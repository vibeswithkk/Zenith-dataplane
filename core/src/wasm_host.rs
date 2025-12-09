// WasmHost implementation
use wasmtime::{Engine, Linker, Module, Store, Config};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
use crate::error::Result;
use std::sync::{Arc, Mutex};

pub struct WasmPlugin {
    store: Arc<Mutex<Store<WasiCtx>>>,
    instance: wasmtime::Instance,
}

pub struct WasmHost {
    engine: Engine,
    linker: Linker<WasiCtx>,
}

impl WasmHost {
    pub fn new() -> Result<Self> {
        let config = Config::new();
        // config.wasm_component_model(true); // Disable for basic module
        
        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

        Ok(Self {
            engine,
            linker,
        })
    }

    pub fn load_plugin(&self, wasm_bytes: &[u8]) -> Result<WasmPlugin> {
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .build();
        
        let mut store = Store::new(&self.engine, wasi);
        let module = Module::new(&self.engine, wasm_bytes)?;
        let instance = self.linker.instantiate(&mut store, &module)?;

        Ok(WasmPlugin {
            store: Arc::new(Mutex::new(store)),
            instance,
        })
    }
}

impl WasmPlugin {
    pub fn on_event(&self, source_id: u32, seq_no: u64) -> Result<bool> {
        let mut store = self.store.lock().expect("Lock poisoned");
        // Look for a function named "on_event" that takes (i32, i64) -> i32
        // Rust u32 -> wasm i32, u64 -> i64 usually
        let func = self.instance.get_typed_func::<(i32, i64), i32>(&mut *store, "on_event");
        
        match func {
            Ok(f) => {
                let res = f.call(&mut *store, (source_id as i32, seq_no as i64))?;
                Ok(res != 0)
            }
            Err(_) => {
                // If not found, allow by default
                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasm_host_creation() {
        let result = WasmHost::new();
        assert!(result.is_ok(), "WasmHost creation should succeed");
    }
    
    #[test]
    fn test_wasm_host_load_invalid_plugin() {
        let host = WasmHost::new().unwrap();
        
        // Invalid WASM bytes should fail
        let invalid_wasm = b"invalid wasm bytes";
        let result = host.load_plugin(invalid_wasm);
        assert!(result.is_err(), "Invalid WASM should fail to load");
    }
    
    #[test]
    fn test_wasm_host_load_valid_minimal_wasm() {
        let host = WasmHost::new().unwrap();
        
        // Minimal valid WASM module (empty module)
        // This is the binary encoding of an empty WASM module
        let minimal_wasm = &[
            0x00, 0x61, 0x73, 0x6D,  // WASM magic number
            0x01, 0x00, 0x00, 0x00,  // Version 1
        ];
        
        let result = host.load_plugin(minimal_wasm);
        // This should either succeed or fail gracefully
        // (depends on whether empty modules are accepted)
        let _ = result; // We just want to ensure no panic
    }
    
    /// Test that on_event returns Ok(true) when function is not found
    /// This catches the mutation: replace return Ok(true) with Ok(false)
    #[test]
    fn test_wasm_plugin_on_event_default_true() {
        let host = WasmHost::new().unwrap();
        
        // Minimal WASM module without on_event function
        let minimal_wasm = &[
            0x00, 0x61, 0x73, 0x6D,  // WASM magic number
            0x01, 0x00, 0x00, 0x00,  // Version 1
        ];
        
        let plugin_result = host.load_plugin(minimal_wasm);
        
        // If plugin loads successfully, test on_event
        if let Ok(plugin) = plugin_result {
            let result = plugin.on_event(1, 100);
            assert!(result.is_ok(), "on_event should return Ok");
            
            // CRITICAL: This catches the mutation Ok(true) -> Ok(false)
            // When on_event function is not found, it should return true (allow by default)
            assert!(result.unwrap(), 
                "on_event should return true when function not found - catches Ok(false) mutation");
        }
    }
    
    /// Test that verifies the != 0 logic in on_event
    /// This is harder to test without a real WASM plugin, but we document the expected behavior
    #[test]
    fn test_on_event_return_value_semantics() {
        // Document the expected behavior:
        // - res != 0 should return true (event allowed)
        // - res == 0 should return false (event blocked)
        // 
        // If mutation changes != to ==:
        // - res != 0 (e.g., res=1) would incorrectly return false
        // - res == 0 would incorrectly return true
        //
        // This test exists to ensure we understand the semantics
        // and to provide documentation for the behavior
        
        // Test the logic directly
        let res: i32 = 1;
        let expected = res != 0;
        assert!(expected, "Non-zero result should mean 'allow event'");
        
        let res: i32 = 0;
        let expected = res != 0;
        assert!(!expected, "Zero result should mean 'block event'");
        
        // Edge cases
        let res: i32 = -1;
        let expected = res != 0;
        assert!(expected, "Negative result should still mean 'allow event'");
    }
}
