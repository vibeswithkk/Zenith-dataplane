/// Key-Value Store Module for WASM Plugins
/// Provides persistent state storage for plugins

use std::sync::RwLock;
use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref KV_STORE: RwLock<HashMap<String, Vec<u8>>> = RwLock::new(HashMap::new());
}

/// Key-Value store API
pub struct KvAPI;

impl KvAPI {
    /// Set a key-value pair
    pub fn set(key: &str, value: &[u8]) -> Result<(), String> {
        let mut store = KV_STORE.write().unwrap();
        store.insert(key.to_string(), value.to_vec());
        Ok(())
    }
    
    /// Get a value by key
    pub fn get(key: &str) -> Option<Vec<u8>> {
        let store = KV_STORE.read().unwrap();
        store.get(key).cloned()
    }
    
    /// Delete a key
    pub fn delete(key: &str) -> bool {
        let mut store = KV_STORE.write().unwrap();
        store.remove(key).is_some()
    }
    
    /// Check if key exists
    pub fn exists(key: &str) -> bool {
        let store = KV_STORE.read().unwrap();
        store.contains_key(key)
    }
    
    /// Get all keys
    pub fn keys() -> Vec<String> {
        let store = KV_STORE.read().unwrap();
        store.keys().cloned().collect()
    }
    
    /// Clear all entries
    pub fn clear() {
        let mut store = KV_STORE.write().unwrap();
        store.clear();
    }
    
    /// Get number of entries
    pub fn count() -> usize {
        let store = KV_STORE.read().unwrap();
        store.len()
    }
}

// C ABI exports
#[no_mangle]
pub unsafe extern "C" fn zenith_kv_set(
    key_ptr: *const u8,
    key_len: usize,
    value_ptr: *const u8,
    value_len: usize,
) -> i32 {
    if key_ptr.is_null() || value_ptr.is_null() {
        return -1;
    }
    
    let key_slice = std::slice::from_raw_parts(key_ptr, key_len);
    let key = match std::str::from_utf8(key_slice) {
        Ok(s) => s,
        Err(_) => return -2,
    };
    
    let value = std::slice::from_raw_parts(value_ptr, value_len);
    
    match KvAPI::set(key, value) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

#[no_mangle]
pub unsafe extern "C" fn zenith_kv_get(
    key_ptr: *const u8,
    key_len: usize,
    out_ptr: *mut u8,
    out_len: usize,
) -> i32 {
    if key_ptr.is_null() || out_ptr.is_null() {
        return -1;
    }
    
    let key_slice = std::slice::from_raw_parts(key_ptr, key_len);
    let key = match std::str::from_utf8(key_slice) {
        Ok(s) => s,
        Err(_) => return -2,
    };
    
    match KvAPI::get(key) {
        Some(value) => {
            let copy_len = value.len().min(out_len);
            let out_slice = std::slice::from_raw_parts_mut(out_ptr, copy_len);
            out_slice.copy_from_slice(&value[..copy_len]);
            copy_len as i32
        }
        None => -3, // Not found
    }
}

#[no_mangle]
pub unsafe extern "C" fn zenith_kv_delete(
    key_ptr: *const u8,
    key_len: usize,
) -> i32 {
    if key_ptr.is_null() {
        return -1;
    }
    
    let key_slice = std::slice::from_raw_parts(key_ptr, key_len);
    let key = match std::str::from_utf8(key_slice) {
        Ok(s) => s,
        Err(_) => return -2,
    };
    
    if KvAPI::delete(key) {
        0
    } else {
        -3 // Not found
    }
}

#[no_mangle]
pub extern "C" fn zenith_kv_count() -> usize {
    KvAPI::count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_operations() {
        KvAPI::clear();
        
        // Set
        KvAPI::set("test_key", b"test_value").unwrap();
        assert_eq!(KvAPI::count(), 1);
        
        // Get
        let value = KvAPI::get("test_key").unwrap();
        assert_eq!(value, b"test_value");
        
        // Exists
        assert!(KvAPI::exists("test_key"));
        assert!(!KvAPI::exists("nonexistent"));
        
        // Delete
        assert!(KvAPI::delete("test_key"));
        assert_eq!(KvAPI::count(), 0);
        
        // Delete non-existent
        assert!(!KvAPI::delete("test_key"));
    }

    #[test]
    fn test_kv_keys() {
        KvAPI::clear();
        
        KvAPI::set("key1", b"val1").unwrap();
        KvAPI::set("key2", b"val2").unwrap();
        KvAPI::set("key3", b"val3").unwrap();
        
        let keys = KvAPI::keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }
}
