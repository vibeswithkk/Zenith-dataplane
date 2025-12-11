//! Mixed Precision Engine
//!
//! Support for FP16/BF16 training and inference acceleration.

use std::sync::atomic::{AtomicU64, Ordering};

/// Half precision (FP16) representation
#[derive(Debug, Clone, Copy, Default)]
#[repr(transparent)]
pub struct Float16(u16);

/// Brain floating point (BF16) representation
#[derive(Debug, Clone, Copy, Default)]
#[repr(transparent)]
pub struct BFloat16(u16);

impl Float16 {
    /// Convert from f32 to fp16
    pub fn from_f32(value: f32) -> Self {
        let bits = value.to_bits();
        
        // Extract components
        let sign = (bits >> 31) & 1;
        let exp = ((bits >> 23) & 0xFF) as i32;
        let frac = bits & 0x7FFFFF;
        
        // Handle special cases
        if exp == 0xFF {
            // Inf or NaN
            if frac == 0 {
                return Self(((sign << 15) | 0x7C00) as u16);
            } else {
                return Self(0x7E00); // NaN
            }
        }
        
        // Rebias exponent
        let new_exp = exp - 127 + 15;
        
        if new_exp <= 0 {
            // Subnormal or zero
            Self((sign << 15) as u16)
        } else if new_exp >= 31 {
            // Overflow to infinity
            Self(((sign << 15) | 0x7C00) as u16)
        } else {
            // Normal number
            let new_frac = (frac >> 13) as u16;
            Self(((sign << 15) | ((new_exp as u32) << 10) | new_frac as u32) as u16)
        }
    }
    
    /// Convert from fp16 to f32
    pub fn to_f32(self) -> f32 {
        let bits = self.0 as u32;
        
        let sign = (bits >> 15) & 1;
        let exp = (bits >> 10) & 0x1F;
        let frac = bits & 0x3FF;
        
        if exp == 0 {
            if frac == 0 {
                // Zero
                f32::from_bits(sign << 31)
            } else {
                // Subnormal
                let new_frac = (frac as f32) / 1024.0 * (2.0f32).powi(-14);
                if sign == 0 { new_frac } else { -new_frac }
            }
        } else if exp == 31 {
            if frac == 0 {
                // Infinity
                f32::from_bits((sign << 31) | 0x7F800000)
            } else {
                // NaN
                f32::NAN
            }
        } else {
            // Normal
            let new_exp = exp + 127 - 15;
            let new_frac = frac << 13;
            f32::from_bits((sign << 31) | (new_exp << 23) | new_frac)
        }
    }
    
    /// Get raw bits
    pub fn to_bits(self) -> u16 {
        self.0
    }
    
    /// Create from raw bits
    pub fn from_bits(bits: u16) -> Self {
        Self(bits)
    }
}

impl BFloat16 {
    /// Convert from f32 to bf16 (just truncate lower 16 bits)
    pub fn from_f32(value: f32) -> Self {
        let bits = value.to_bits();
        Self((bits >> 16) as u16)
    }
    
    /// Convert from bf16 to f32 (just add 16 zero bits)
    pub fn to_f32(self) -> f32 {
        f32::from_bits((self.0 as u32) << 16)
    }
    
    /// Get raw bits
    pub fn to_bits(self) -> u16 {
        self.0
    }
    
    /// Create from raw bits
    pub fn from_bits(bits: u16) -> Self {
        Self(bits)
    }
}

/// Mixed precision configuration
#[derive(Debug, Clone)]
pub struct MixedPrecisionConfig {
    /// Data type for forward pass
    pub compute_dtype: PrecisionType,
    /// Data type for master weights
    pub master_dtype: PrecisionType,
    /// Enable dynamic loss scaling
    pub dynamic_loss_scale: bool,
    /// Initial loss scale
    pub initial_scale: f32,
    /// Scale growth factor
    pub growth_factor: f32,
    /// Scale reduction factor  
    pub backoff_factor: f32,
    /// Growth interval (steps)
    pub growth_interval: u32,
}

/// Precision type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrecisionType {
    Float32,
    Float16,
    BFloat16,
}

impl Default for MixedPrecisionConfig {
    fn default() -> Self {
        Self {
            compute_dtype: PrecisionType::BFloat16,
            master_dtype: PrecisionType::Float32,
            dynamic_loss_scale: true,
            initial_scale: 65536.0,
            growth_factor: 2.0,
            backoff_factor: 0.5,
            growth_interval: 2000,
        }
    }
}

/// Dynamic loss scaler for mixed precision training
pub struct LossScaler {
    config: MixedPrecisionConfig,
    current_scale: f32,
    growth_tracker: u32,
    num_overflows: AtomicU64,
    num_underflows: AtomicU64,
}

impl LossScaler {
    /// Create new loss scaler
    pub fn new(config: MixedPrecisionConfig) -> Self {
        let current_scale = config.initial_scale;
        Self {
            config,
            current_scale,
            growth_tracker: 0,
            num_overflows: AtomicU64::new(0),
            num_underflows: AtomicU64::new(0),
        }
    }
    
    /// Get current scale
    pub fn scale(&self) -> f32 {
        self.current_scale
    }
    
    /// Scale up gradients before backward pass
    pub fn scale_loss(&self, loss: f32) -> f32 {
        loss * self.current_scale
    }
    
    /// Unscale gradients after backward pass
    pub fn unscale(&self, grad: f32) -> f32 {
        grad / self.current_scale
    }
    
    /// Check if gradient has overflow/underflow
    pub fn check_overflow(&self, grad: f32) -> bool {
        !grad.is_finite()
    }
    
    /// Update scale based on gradient health
    pub fn update(&mut self, overflow_detected: bool) {
        if overflow_detected {
            // Reduce scale on overflow
            self.current_scale *= self.config.backoff_factor;
            self.growth_tracker = 0;
            self.num_overflows.fetch_add(1, Ordering::Relaxed);
        } else {
            // Increment growth tracker
            self.growth_tracker += 1;
            
            if self.growth_tracker >= self.config.growth_interval {
                // Increase scale
                self.current_scale *= self.config.growth_factor;
                self.growth_tracker = 0;
            }
        }
        
        // Clamp scale
        self.current_scale = self.current_scale.clamp(1.0, 65536.0 * 65536.0);
    }
    
    /// Get statistics
    pub fn stats(&self) -> (u64, u64, f32) {
        (
            self.num_overflows.load(Ordering::Relaxed),
            self.num_underflows.load(Ordering::Relaxed),
            self.current_scale,
        )
    }
}

/// Mixed precision converter for batch processing
pub struct PrecisionConverter {
    config: MixedPrecisionConfig,
}

impl PrecisionConverter {
    /// Create new converter
    pub fn new(config: MixedPrecisionConfig) -> Self {
        Self { config }
    }
    
    /// Convert f32 slice to bf16
    pub fn f32_to_bf16(&self, input: &[f32], output: &mut [u16]) {
        assert_eq!(input.len(), output.len());
        for (i, &val) in input.iter().enumerate() {
            output[i] = BFloat16::from_f32(val).to_bits();
        }
    }
    
    /// Convert bf16 slice to f32
    pub fn bf16_to_f32(&self, input: &[u16], output: &mut [f32]) {
        assert_eq!(input.len(), output.len());
        for (i, &val) in input.iter().enumerate() {
            output[i] = BFloat16::from_bits(val).to_f32();
        }
    }
    
    /// Convert f32 slice to fp16
    pub fn f32_to_fp16(&self, input: &[f32], output: &mut [u16]) {
        assert_eq!(input.len(), output.len());
        for (i, &val) in input.iter().enumerate() {
            output[i] = Float16::from_f32(val).to_bits();
        }
    }
    
    /// Convert fp16 slice to f32
    pub fn fp16_to_f32(&self, input: &[u16], output: &mut [f32]) {
        assert_eq!(input.len(), output.len());
        for (i, &val) in input.iter().enumerate() {
            output[i] = Float16::from_bits(val).to_f32();
        }
    }
    
    /// Get compute dtype
    pub fn compute_dtype(&self) -> PrecisionType {
        self.config.compute_dtype
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bf16_conversion() {
        let values = [0.0f32, 1.0, -1.0, std::f32::consts::PI, 100.0, 0.001];
        
        for &val in &values {
            let bf16 = BFloat16::from_f32(val);
            let back = bf16.to_f32();
            
            // BF16 should preserve ~3 digits
            let error = (val - back).abs() / val.abs().max(1e-6);
            assert!(error < 0.01, "BF16 error too large for {}: got {}", val, back);
        }
    }
    
    #[test]
    fn test_fp16_conversion() {
        let values = [0.0f32, 1.0, -1.0, std::f32::consts::PI, 100.0, 0.001];
        
        for &val in &values {
            let fp16 = Float16::from_f32(val);
            let back = fp16.to_f32();
            
            // FP16 should preserve ~3-4 digits
            let error = (val - back).abs() / val.abs().max(1e-6);
            assert!(error < 0.01, "FP16 error too large for {}: got {}", val, back);
        }
    }
    
    #[test]
    fn test_loss_scaler() {
        let config = MixedPrecisionConfig::default();
        let mut scaler = LossScaler::new(config);
        
        let initial_scale = scaler.scale();
        assert!(initial_scale > 0.0);
        
        // Simulate overflow
        scaler.update(true);
        assert!(scaler.scale() < initial_scale);
        
        // Simulate many successful steps
        for _ in 0..3000 {
            scaler.update(false);
        }
        assert!(scaler.scale() > initial_scale * 0.5);
    }
    
    #[test]
    fn test_precision_converter() {
        let config = MixedPrecisionConfig::default();
        let converter = PrecisionConverter::new(config);
        
        let input = vec![1.0f32, 2.0, 3.0, 4.0];
        let mut bf16_output = vec![0u16; 4];
        let mut f32_output = vec![0.0f32; 4];
        
        converter.f32_to_bf16(&input, &mut bf16_output);
        converter.bf16_to_f32(&bf16_output, &mut f32_output);
        
        for (i, (&orig, &back)) in input.iter().zip(f32_output.iter()).enumerate() {
            let error = (orig - back).abs();
            assert!(error < 0.1, "Conversion error at {}: {} vs {}", i, orig, back);
        }
    }
}
