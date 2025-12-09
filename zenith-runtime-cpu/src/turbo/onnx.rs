//! ONNX Runtime Integration
//!
//! Fast inference using ONNX Runtime for any ML model.

use std::path::Path;

/// ONNX execution provider
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionProvider {
    /// CPU execution
    CPU,
    /// CUDA GPU execution
    CUDA,
    /// TensorRT optimized execution
    TensorRT,
    /// ROCm for AMD GPUs
    ROCm,
    /// DirectML for Windows
    DirectML,
    /// CoreML for Apple devices
    CoreML,
}

impl ExecutionProvider {
    /// Get provider name
    pub fn name(&self) -> &'static str {
        match self {
            Self::CPU => "CPUExecutionProvider",
            Self::CUDA => "CUDAExecutionProvider",
            Self::TensorRT => "TensorrtExecutionProvider",
            Self::ROCm => "ROCMExecutionProvider",
            Self::DirectML => "DmlExecutionProvider",
            Self::CoreML => "CoreMLExecutionProvider",
        }
    }
    
    /// Check if provider is available
    pub fn is_available(&self) -> bool {
        match self {
            Self::CPU => true, // Always available
            Self::CUDA => std::env::var("CUDA_VISIBLE_DEVICES").is_ok() 
                || Path::new("/usr/local/cuda").exists(),
            Self::TensorRT => Path::new("/usr/lib/x86_64-linux-gnu/libnvinfer.so").exists(),
            _ => false,
        }
    }
}

/// ONNX session configuration
#[derive(Debug, Clone)]
pub struct OnnxConfig {
    /// Execution providers in priority order
    pub providers: Vec<ExecutionProvider>,
    /// Number of intra-op threads
    pub intra_op_threads: usize,
    /// Number of inter-op threads
    pub inter_op_threads: usize,
    /// Enable memory arena
    pub enable_mem_arena: bool,
    /// Enable memory pattern optimization
    pub enable_mem_pattern: bool,
    /// Graph optimization level (0-3)
    pub optimization_level: u32,
    /// Enable profiling
    pub enable_profiling: bool,
}

impl Default for OnnxConfig {
    fn default() -> Self {
        Self {
            providers: vec![ExecutionProvider::CUDA, ExecutionProvider::CPU],
            intra_op_threads: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
            inter_op_threads: 2,
            enable_mem_arena: true,
            enable_mem_pattern: true,
            optimization_level: 3, // All optimizations
            enable_profiling: false,
        }
    }
}

/// Tensor shape information
#[derive(Debug, Clone)]
pub struct TensorInfo {
    /// Tensor name (e.g., "input", "output")
    pub name: String,
    /// Shape dimensions (-1 indicates dynamic dimension)
    pub shape: Vec<i64>,
    /// Data type of the tensor
    pub dtype: TensorType,
}

/// Tensor data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorType {
    Float32,
    Float16,
    Int32,
    Int64,
    UInt8,
    Bool,
    String,
}

impl TensorType {
    /// Size in bytes
    pub fn size(&self) -> usize {
        match self {
            Self::Float32 | Self::Int32 => 4,
            Self::Float16 => 2,
            Self::Int64 => 8,
            Self::UInt8 | Self::Bool => 1,
            Self::String => 0, // Variable
        }
    }
}

/// ONNX inference session wrapper
pub struct OnnxSession {
    model_path: String,
    config: OnnxConfig,
    input_info: Vec<TensorInfo>,
    output_info: Vec<TensorInfo>,
    loaded: bool,
}

impl OnnxSession {
    /// Create new ONNX session
    pub fn new(model_path: &str, config: OnnxConfig) -> Result<Self, OnnxError> {
        // Validate model path
        if !Path::new(model_path).exists() {
            return Err(OnnxError::ModelNotFound(model_path.to_string()));
        }
        
        // Parse model metadata (placeholder - real impl would use onnxruntime-rs)
        let input_info = vec![TensorInfo {
            name: "input".to_string(),
            shape: vec![-1, 3, 224, 224], // Dynamic batch
            dtype: TensorType::Float32,
        }];
        
        let output_info = vec![TensorInfo {
            name: "output".to_string(),
            shape: vec![-1, 1000],
            dtype: TensorType::Float32,
        }];
        
        Ok(Self {
            model_path: model_path.to_string(),
            config,
            input_info,
            output_info,
            loaded: true,
        })
    }
    
    /// Get input tensor info
    pub fn inputs(&self) -> &[TensorInfo] {
        &self.input_info
    }
    
    /// Get output tensor info
    pub fn outputs(&self) -> &[TensorInfo] {
        &self.output_info
    }
    
    /// Run inference (placeholder - real impl would use onnxruntime-rs)
    pub fn run(&self, inputs: &[&[f32]]) -> Result<Vec<Vec<f32>>, OnnxError> {
        if !self.loaded {
            return Err(OnnxError::SessionNotLoaded);
        }
        
        if inputs.is_empty() {
            return Err(OnnxError::InvalidInput("No inputs provided".into()));
        }
        
        // Placeholder output
        let output_size = self.output_info[0].shape.iter()
            .map(|&s| if s < 0 { 1 } else { s as usize })
            .product();
        
        Ok(vec![vec![0.0f32; output_size]])
    }
    
    /// Get model path
    pub fn model_path(&self) -> &str {
        &self.model_path
    }
    
    /// Get active execution provider
    pub fn active_provider(&self) -> ExecutionProvider {
        for provider in &self.config.providers {
            if provider.is_available() {
                return *provider;
            }
        }
        ExecutionProvider::CPU
    }
}

/// ONNX error types
#[derive(Debug)]
pub enum OnnxError {
    ModelNotFound(String),
    SessionNotLoaded,
    InvalidInput(String),
    RuntimeError(String),
}

impl std::fmt::Display for OnnxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModelNotFound(p) => write!(f, "Model not found: {}", p),
            Self::SessionNotLoaded => write!(f, "Session not loaded"),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Self::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl std::error::Error for OnnxError {}

/// Model converter utilities
pub struct ModelConverter;

impl ModelConverter {
    /// Convert PyTorch model to ONNX (command helper)
    pub fn pytorch_to_onnx_cmd(
        model_path: &str,
        output_path: &str,
        input_shape: &[i64],
    ) -> String {
        format!(
            r#"python -c "
import torch
model = torch.load('{}')
model.eval()
dummy = torch.randn({:?})
torch.onnx.export(model, dummy, '{}', opset_version=17)
""#,
            model_path, input_shape, output_path
        )
    }
    
    /// Convert TensorFlow model to ONNX (command helper)
    pub fn tensorflow_to_onnx_cmd(
        model_path: &str,
        output_path: &str,
    ) -> String {
        format!(
            "python -m tf2onnx.convert --saved-model {} --output {} --opset 17",
            model_path, output_path
        )
    }
}

/// Inference benchmark helper
pub struct InferenceBenchmark {
    /// Name of the model being benchmarked
    pub model_name: String,
    /// Execution provider to use for inference
    pub provider: ExecutionProvider,
    /// Number of warmup runs before timing
    pub warmup_runs: u32,
    /// Number of timed benchmark runs
    pub benchmark_runs: u32,
    /// Batch size for inference
    pub batch_size: usize,
}

impl InferenceBenchmark {
    /// Create new benchmark
    pub fn new(model_name: &str, provider: ExecutionProvider) -> Self {
        Self {
            model_name: model_name.to_string(),
            provider,
            warmup_runs: 10,
            benchmark_runs: 100,
            batch_size: 32,
        }
    }
    
    /// Run benchmark (returns samples/sec)
    pub fn run(&self, session: &OnnxSession, input_data: &[f32]) -> Result<f64, OnnxError> {
        use std::time::Instant;
        
        // Warmup
        for _ in 0..self.warmup_runs {
            session.run(&[input_data])?;
        }
        
        // Benchmark
        let start = Instant::now();
        for _ in 0..self.benchmark_runs {
            session.run(&[input_data])?;
        }
        let elapsed = start.elapsed();
        
        let total_samples = self.benchmark_runs as usize * self.batch_size;
        let samples_per_sec = total_samples as f64 / elapsed.as_secs_f64();
        
        Ok(samples_per_sec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_execution_provider() {
        assert!(ExecutionProvider::CPU.is_available());
        assert_eq!(ExecutionProvider::CPU.name(), "CPUExecutionProvider");
    }
    
    #[test]
    fn test_onnx_config() {
        let config = OnnxConfig::default();
        assert_eq!(config.optimization_level, 3);
        assert!(!config.providers.is_empty());
    }
    
    #[test]
    fn test_tensor_type() {
        assert_eq!(TensorType::Float32.size(), 4);
        assert_eq!(TensorType::Float16.size(), 2);
        assert_eq!(TensorType::UInt8.size(), 1);
    }
    
    #[test]
    fn test_model_converter_commands() {
        let cmd = ModelConverter::pytorch_to_onnx_cmd(
            "model.pt",
            "model.onnx",
            &[1, 3, 224, 224]
        );
        assert!(cmd.contains("torch.onnx.export"));
        
        let cmd = ModelConverter::tensorflow_to_onnx_cmd("saved_model", "model.onnx");
        assert!(cmd.contains("tf2onnx"));
    }
}
