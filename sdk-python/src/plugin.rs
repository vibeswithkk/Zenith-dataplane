//! WASM Plugin Manager
//!
//! This module handles loading, execution, and lifecycle management
//! of WebAssembly preprocessing plugins.

use std::collections::HashMap;
use std::path::Path;

/// Information about a loaded plugin
#[derive(Clone, Debug)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub path: String,
    pub size_bytes: usize,
}

/// Manages WASM plugin lifecycle
pub struct PluginManager {
    plugins: HashMap<String, PluginInfo>,
    // In production, this would hold wasmtime::Module instances
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }
    
    /// Load a WASM plugin from file
    pub fn load(&mut self, path: &Path) -> Result<PluginInfo, String> {
        let wasm_bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read plugin: {}", e))?;
        
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let info = PluginInfo {
            name: name.clone(),
            version: "0.1.0".to_string(),
            path: path.to_string_lossy().to_string(),
            size_bytes: wasm_bytes.len(),
        };
        
        // In production, this would:
        // 1. Compile WASM module with wasmtime
        // 2. Validate plugin interface
        // 3. Store compiled module for execution
        
        self.plugins.insert(name.clone(), info.clone());
        
        Ok(info)
    }
    
    /// Unload a plugin by name
    pub fn unload(&mut self, name: &str) -> bool {
        self.plugins.remove(name).is_some()
    }
    
    /// Get information about a loaded plugin
    pub fn get(&self, name: &str) -> Option<&PluginInfo> {
        self.plugins.get(name)
    }
    
    /// List all loaded plugins
    pub fn list(&self) -> Vec<&PluginInfo> {
        self.plugins.values().collect()
    }
    
    /// Get the number of loaded plugins
    pub fn count(&self) -> usize {
        self.plugins.len()
    }
    
    /// Execute a plugin on data
    pub fn execute(
        &self,
        name: &str,
        input: &[u8],
    ) -> Result<Vec<u8>, String> {
        let _plugin = self.plugins.get(name)
            .ok_or_else(|| format!("Plugin not found: {}", name))?;
        
        // In production, this would:
        // 1. Get the compiled WASM module
        // 2. Create a new instance with memory
        // 3. Copy input data to WASM memory
        // 4. Call the process function
        // 5. Copy output data from WASM memory
        
        // Placeholder: return input unchanged
        Ok(input.to_vec())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_plugin_manager() {
        let mut manager = PluginManager::new();
        assert_eq!(manager.count(), 0);
        
        // Create a fake WASM file
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"\x00asm\x01\x00\x00\x00").unwrap();
        
        let result = manager.load(temp_file.path());
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert_eq!(info.size_bytes, 8);
        assert_eq!(manager.count(), 1);
    }
}
