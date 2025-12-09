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
