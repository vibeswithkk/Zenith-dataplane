/// Virtual Machine abstraction for WASM execution
/// Wraps Wasmtime with additional runtime features
use wasmtime::{Engine as WasmEngine, Store, Module, Linker};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use anyhow::Result;
use std::sync::Arc;

/// Type alias for WASI state in wasmtime v39+
type WasiState = WasiP1Ctx;

/// WASM Virtual Machine
pub struct VM {
    engine: Arc<WasmEngine>,
    module: Module,
}

impl VM {
    /// Create new VM from WASM bytes
    pub fn from_bytes(wasm: &[u8]) -> Result<Self> {
        let engine = Arc::new(WasmEngine::default());
        let module = Module::new(&engine, wasm)?;
        
        Ok(Self { engine, module })
    }

    /// Execute the WASM module's exported function
    pub fn execute(&self, function_name: &str, args: &[i64]) -> Result<Vec<i64>> {
        let mut linker = Linker::new(&self.engine);
        
        // wasmtime v39+ uses p1 module for WASIp1 compatibility
        p1::add_to_linker_sync(&mut linker, |s: &mut WasiState| s)?;
        
        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .build_p1();
        
        let mut store = Store::new(&self.engine, wasi_ctx);
        let instance = linker.instantiate(&mut store, &self.module)?;
        
        // Try to get the function
        let func = instance.get_func(&mut store, function_name)
            .ok_or_else(|| anyhow::anyhow!("Function {} not found", function_name))?;
        
        // For simplicity, assume function signature matches
        // In production, we'd validate this
        let mut results = vec![wasmtime::Val::I64(0)];
        
        let params: Vec<wasmtime::Val> = args.iter()
            .map(|&v| wasmtime::Val::I64(v))
            .collect();
        
        func.call(&mut store, &params, &mut results)?;
        
        Ok(results.iter().map(|v| {
            if let wasmtime::Val::I64(i) = v {
                *i
            } else {
                0
            }
        }).collect())
    }

    /// Get module metadata
    pub fn get_exports(&self) -> Vec<String> {
        self.module.exports()
            .map(|e| e.name().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_creation() {
        // Simple WASM module that exports a function
        let wasm = wat::parse_str(r#"
            (module
                (func (export "test") (result i32)
                    i32.const 42
                )
            )
        "#).unwrap();
        
        let vm = VM::from_bytes(&wasm).unwrap();
        let exports = vm.get_exports();
        assert!(exports.contains(&"test".to_string()));
    }
}
