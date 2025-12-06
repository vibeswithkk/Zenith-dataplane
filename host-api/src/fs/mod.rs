/// Sandboxed Filesystem Module for WASM Plugins
/// Provides restricted filesystem access with safety guarantees

use std::path::{Path, PathBuf};
use std::fs;
use std::io::{Read, Write};
use std::sync::RwLock;

lazy_static::lazy_static! {
    static ref SANDBOX_ROOT: RwLock<PathBuf> = RwLock::new(PathBuf::from("/tmp/zenith_sandbox"));
}

/// Filesystem API with sandboxing
pub struct FsAPI;

impl FsAPI {
    /// Set sandbox root directory
    pub fn set_sandbox_root(path: PathBuf) {
        let mut root = SANDBOX_ROOT.write().unwrap();
        *root = path;
    }
    
    /// Get sandbox root
    pub fn get_sandbox_root() -> PathBuf {
        SANDBOX_ROOT.read().unwrap().clone()
    }
    
    /// Resolve path within sandbox
    fn resolve_path(relative_path: &str) -> Result<PathBuf, String> {
        let root = Self::get_sandbox_root();
        let full_path = root.join(relative_path);
        
        // Security: Ensure path doesn't escape sandbox
        let canonical = full_path.canonicalize()
            .or_else(|_| {
                // If doesn't exist yet, check parent
                if let Some(parent) = full_path.parent() {
                    parent.canonicalize()
                        .map(|p| p.join(full_path.file_name().unwrap()))
                } else {
                    Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Invalid path"))
                }
            })
            .map_err(|e| format!("Path resolution failed: {}", e))?;
        
        if !canonical.starts_with(&root) {
            return Err("Path escapes sandbox".to_string());
        }
        
        Ok(canonical)
    }
    
    /// Read file contents
    pub fn read_file(path: &str) -> Result<Vec<u8>, String> {
        let full_path = Self::resolve_path(path)?;
        
        fs::read(&full_path)
            .map_err(|e| format!("Failed to read file: {}", e))
    }
    
    /// Write file contents
    pub fn write_file(path: &str, data: &[u8]) -> Result<(), String> {
        let full_path = Self::resolve_path(path)?;
        
        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        
        fs::write(&full_path, data)
            .map_err(|e| format!("Failed to write file: {}", e))
    }
    
    /// Check if file exists
    pub fn exists(path: &str) -> bool {
        Self::resolve_path(path)
            .ok()
            .map(|p| p.exists())
            .unwrap_or(false)
    }
    
    /// Delete file
    pub fn delete_file(path: &str) -> Result<(), String> {
        let full_path = Self::resolve_path(path)?;
        
        if !full_path.exists() {
            return Err("File not found".to_string());
        }
        
        fs::remove_file(&full_path)
            .map_err(|e| format!("Failed to delete file: {}", e))
    }
    
    /// List directory contents
    pub fn list_dir(path: &str) -> Result<Vec<String>, String> {
        let full_path = Self::resolve_path(path)?;
        
        let entries = fs::read_dir(&full_path)
            .map_err(|e| format!("Failed to read directory: {}", e))?;
        
        let mut names = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    names.push(name.to_string());
                }
            }
        }
        
        Ok(names)
    }
}

// C ABI exports
#[no_mangle]
pub unsafe extern "C" fn zenith_fs_read(
    path_ptr: *const u8,
    path_len: usize,
    out_ptr: *mut u8,
    out_len: usize,
) -> i32 {
    if path_ptr.is_null() || out_ptr.is_null() {
        return -1;
    }
    
    let path_slice = std::slice::from_raw_parts(path_ptr, path_len);
    let path = match std::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return -2,
    };
    
    match FsAPI::read_file(path) {
        Ok(data) => {
            let copy_len = data.len().min(out_len);
            let out_slice = std::slice::from_raw_parts_mut(out_ptr, copy_len);
            out_slice.copy_from_slice(&data[..copy_len]);
            copy_len as i32
        }
        Err(_) => -3,
    }
}

#[no_mangle]
pub unsafe extern "C" fn zenith_fs_write(
    path_ptr: *const u8,
    path_len: usize,
    data_ptr: *const u8,
    data_len: usize,
) -> i32 {
    if path_ptr.is_null() || data_ptr.is_null() {
        return -1;
    }
    
    let path_slice = std::slice::from_raw_parts(path_ptr, path_len);
    let path = match std::str::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return -2,
    };
    
    let data = std::slice::from_raw_parts(data_ptr, data_len);
    
    match FsAPI::write_file(path, data) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_sandbox_escape_prevention() {
        FsAPI::set_sandbox_root(PathBuf::from("/tmp/test_sandbox"));
        
        // Try to escape with ../
        let result = FsAPI::resolve_path("../etc/passwd");
        assert!(result.is_err() || !result.unwrap().to_str().unwrap().contains("/etc/passwd"));
    }

    #[test]
    fn test_file_operations() {
        let sandbox = std::env::temp_dir().join("zenith_test");
        FsAPI::set_sandbox_root(sandbox.clone());
        fs::create_dir_all(&sandbox).unwrap();
        
        // Write
        FsAPI::write_file("test.txt", b"Hello, World!").unwrap();
        
        // Read
        let data = FsAPI::read_file("test.txt").unwrap();
        assert_eq!(data, b"Hello, World!");
        
        // Exists
        assert!(FsAPI::exists("test.txt"));
        
        // Delete
        FsAPI::delete_file("test.txt").unwrap();
        assert!(!FsAPI::exists("test.txt"));
        
        // Cleanup
        fs::remove_dir_all(&sandbox).ok();
    }
}
