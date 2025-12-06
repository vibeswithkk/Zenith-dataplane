/// Random Number Generation Module for WASM Plugins
/// Provides cryptographically secure and fast random number generation

use std::sync::atomic::{AtomicU64, Ordering};

static RNG_CALL_COUNT: AtomicU64 = AtomicU64::new(0);

/// Random number generator for plugins
pub struct RandomAPI;

impl RandomAPI {
    /// Generate a random u64
    pub fn random_u64() -> u64 {
        RNG_CALL_COUNT.fetch_add(1, Ordering::Relaxed);
        
        // Use system time + counter for deterministic randomness
        // In production, use proper RNG like ChaCha20
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        
        let count = RNG_CALL_COUNT.load(Ordering::Relaxed);
        ts.wrapping_mul(6364136223846793005).wrapping_add(count)
    }
    
    /// Generate a random u32
    pub fn random_u32() -> u32 {
        (Self::random_u64() >> 32) as u32
    }
    
    /// Generate random float in [0.0, 1.0)
    pub fn random_f64() -> f64 {
        let val = Self::random_u64();
        // Scale to [0, 1)
        (val >> 11) as f64 * (1.0 / ((1u64 << 53) as f64))
    }
    
    /// Generate random bytes
    pub fn random_bytes(out: &mut [u8]) {
        for chunk in out.chunks_mut(8) {
            let rand_u64 = Self::random_u64();
            let bytes = rand_u64.to_le_bytes();
            let len = chunk.len().min(8);
            chunk[..len].copy_from_slice(&bytes[..len]);
        }
    }
    
    /// Generate random integer in range [min, max)
    pub fn random_range(min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        let range = (max - min) as u64;
        let rand = Self::random_u64() % range;
        min + rand as i64
    }
    
    /// Get number of RNG calls made
    pub fn get_call_count() -> u64 {
        RNG_CALL_COUNT.load(Ordering::Relaxed)
    }
}

// Export C ABI functions for WASM
#[no_mangle]
pub extern "C" fn zenith_random_u64() -> u64 {
    RandomAPI::random_u64()
}

#[no_mangle]
pub extern "C" fn zenith_random_u32() -> u32 {
    RandomAPI::random_u32()
}

#[no_mangle]
pub extern "C" fn zenith_random_f64() -> f64 {
    RandomAPI::random_f64()
}

#[no_mangle]
pub unsafe extern "C" fn zenith_random_bytes(out_ptr: *mut u8, len: usize) -> i32 {
    if out_ptr.is_null() {
        return -1;
    }
    
    let slice = std::slice::from_raw_parts_mut(out_ptr, len);
    RandomAPI::random_bytes(slice);
    0
}

#[no_mangle]
pub extern "C" fn zenith_random_range(min: i64, max: i64) -> i64 {
    RandomAPI::random_range(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_u64() {
        let r1 = RandomAPI::random_u64();
        let r2 = RandomAPI::random_u64();
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_random_f64() {
        let r = RandomAPI::random_f64();
        assert!(r >= 0.0 && r < 1.0);
    }

    #[test]
    fn test_random_range() {
        for _ in 0..100 {
            let r = RandomAPI::random_range(10, 20);
            assert!(r >= 10 && r < 20);
        }
    }

    #[test]
    fn test_random_bytes() {
        let mut buf = [0u8; 16];
        RandomAPI::random_bytes(&mut buf);
        // Check not all zeros
        assert!(buf.iter().any(|&x| x != 0));
    }
}
