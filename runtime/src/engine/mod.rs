/// Runtime Engine Orchestration
/// Coordinates scheduler, VMs, and sandbox
use crate::sandbox::{Sandbox, SandboxLimits};
use crate::scheduler::{Scheduler, Priority};
use crate::vm::VM;
use crate::host_calls::HostCallInterface;
use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Plugin registry entry
#[allow(dead_code)]
struct PluginEntry {
    id: String,
    vm: VM,
    metadata: PluginMetadata,
}

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub loaded_at: std::time::SystemTime,
}

/// Runtime Engine that orchestrates all components
pub struct RuntimeEngine {
    plugins: Arc<RwLock<HashMap<String, PluginEntry>>>,
    scheduler: Arc<Scheduler>,
    sandbox: Arc<Sandbox>,
    #[allow(dead_code)]
    host_calls: Arc<HostCallInterface>,
}

impl RuntimeEngine {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            scheduler: Arc::new(Scheduler::new(max_concurrent)),
            sandbox: Arc::new(Sandbox::new(SandboxLimits::default())),
            host_calls: Arc::new(HostCallInterface::new()),
        }
    }

    /// Load a plugin into the runtime
    pub async fn load_plugin(&self, id: String, wasm_bytes: &[u8], metadata: PluginMetadata) -> Result<()> {
        // Validate WASM
        self.sandbox.validate_wasm_bytes(wasm_bytes)?;
        
        // Create VM
        let vm = VM::from_bytes(wasm_bytes)?;
        
        // Register plugin
        let entry = PluginEntry {
            id: id.clone(),
            vm,
            metadata,
        };
        
        let mut plugins = self.plugins.write().await;
        plugins.insert(id, entry);
        
        tracing::info!("Plugin loaded successfully");
        Ok(())
    }

    /// Execute a plugin function
    pub async fn execute_plugin(&self, plugin_id: &str, function: &str, args: &[i64]) -> Result<Vec<i64>> {
        let plugins = self.plugins.read().await;
        let entry = plugins.get(plugin_id)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found"))?;
        
        // Create execution context
        let mut ctx = self.sandbox.create_context();
        ctx.start();
        
        // Execute
        let result = entry.vm.execute(function, args)?;
        
        // Check timeout
        ctx.check_timeout()?;
        
        Ok(result)
    }

    /// Schedule task for async execution
    pub fn schedule_task(&self, priority: Priority, payload: Vec<u8>) -> u64 {
        self.scheduler.submit(priority, payload)
    }

    /// Get loaded plugin count
    pub async fn plugin_count(&self) -> usize {
        self.plugins.read().await.len()
    }

    /// Get scheduler stats
    pub fn pending_tasks(&self) -> usize {
        self.scheduler.pending_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_engine() {
        let engine = RuntimeEngine::new(4);
        assert_eq!(engine.plugin_count().await, 0);
    }
}
