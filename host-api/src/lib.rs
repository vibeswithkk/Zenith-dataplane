/// Zenith Host API
/// Functions that WASM plugins can call back to the host runtime
///
/// This provides a safe, capability-based interface for plugins to interact
/// with the Zenith runtime without compromising security.

use std::sync::atomic::{AtomicU64, Ordering};

// Host API modules
pub mod random;
pub mod logging;
pub mod kv;
pub mod http;
pub mod fs;

// Re-exports
pub use random::RandomAPI;
pub use logging::{LoggingAPI, LogLevel, LogEntry};
pub use kv::KvAPI;
pub use http::{HttpAPI, HttpMethod, HttpResponse};
pub use fs::FsAPI;

/// Global counters for host API usage tracking
static HOST_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
static LOG_COUNT: AtomicU64 = AtomicU64::new(0);

/// Host API functions callable from WASM
pub struct HostAPI;

impl HostAPI {
    /// Log a message from the plugin
    /// 
    /// # Safety
    /// message_ptr must point to valid UTF-8 data of length message_len
    pub unsafe fn log(level: u32, message_ptr: *const u8, message_len: usize) -> i32 {
        HOST_CALL_COUNT.fetch_add(1, Ordering::Relaxed);
        LOG_COUNT.fetch_add(1, Ordering::Relaxed);

        if message_ptr.is_null() {
            return -1;
        }

        let message_slice = std::slice::from_raw_parts(message_ptr, message_len);
        let message = match std::str::from_utf8(message_slice) {
            Ok(s) => s,
            Err(_) => return -2,
        };

        let level: LogLevel = level.into();
        match level {
            LogLevel::Trace => tracing::trace!("[Plugin] {}", message),
            LogLevel::Debug => tracing::debug!("[Plugin] {}", message),
            LogLevel::Info => tracing::info!("[Plugin] {}", message),
            LogLevel::Warn => tracing::warn!("[Plugin] {}", message),
            LogLevel::Error => tracing::error!("[Plugin] {}", message),
        }

        0
    }

    /// Get current timestamp in nanoseconds since UNIX epoch
    pub fn get_timestamp_ns() -> u64 {
        HOST_CALL_COUNT.fetch_add(1, Ordering::Relaxed);
        
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    /// Get a random u64 value
    pub fn get_random_u64() -> u64 {
        HOST_CALL_COUNT.fetch_add(1, Ordering::Relaxed);
        
        // In production, use proper RNG
        // For now, use timestamp + counter
        let ts = Self::get_timestamp_ns();
        let count = HOST_CALL_COUNT.load(Ordering::Relaxed);
        ts.wrapping_add(count)
    }

    /// Read event metadata field by index
    /// 
    /// # Safety
    /// out_buffer must be valid for out_buffer_len bytes
    pub unsafe fn read_event_field(
        field_index: u32,
        out_buffer: *mut u8,
        out_buffer_len: usize,
    ) -> i32 {
        HOST_CALL_COUNT.fetch_add(1, Ordering::Relaxed);

        if out_buffer.is_null() {
            return -1;
        }

        // In real implementation, this would access thread-local event context
        // For now, return placeholder data
        let placeholder = format!("field_{}", field_index);
        let bytes = placeholder.as_bytes();
        let copy_len = bytes.len().min(out_buffer_len);

        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buffer, copy_len);
        copy_len as i32
    }

    /// Get total number of host calls made
    pub fn get_host_call_count() -> u64 {
        HOST_CALL_COUNT.load(Ordering::Relaxed)
    }

    /// Get total number of log calls made
    pub fn get_log_count() -> u64 {
        LOG_COUNT.load(Ordering::Relaxed)
    }

    /// Reset all counters (for testing)
    pub fn reset_counters() {
        HOST_CALL_COUNT.store(0, Ordering::Relaxed);
        LOG_COUNT.store(0, Ordering::Relaxed);
    }
}

// Export functions for WASM linking
#[no_mangle]
pub unsafe extern "C" fn zenith_host_log(
    level: u32,
    message_ptr: *const u8,
    message_len: usize,
) -> i32 {
    HostAPI::log(level, message_ptr, message_len)
}

#[no_mangle]
pub extern "C" fn zenith_host_get_timestamp_ns() -> u64 {
    HostAPI::get_timestamp_ns()
}

#[no_mangle]
pub extern "C" fn zenith_host_get_random_u64() -> u64 {
    HostAPI::get_random_u64()
}

#[no_mangle]
pub unsafe extern "C" fn zenith_host_read_event_field(
    field_index: u32,
    out_buffer: *mut u8,
    out_buffer_len: usize,
) -> i32 {
    HostAPI::read_event_field(field_index, out_buffer, out_buffer_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp() {
        let ts1 = HostAPI::get_timestamp_ns();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = HostAPI::get_timestamp_ns();
        assert!(ts2 > ts1);
    }

    #[test]
    fn test_log() {
        HostAPI::reset_counters();
        let msg = "Test message";
        unsafe {
            let result = HostAPI::log(1, msg.as_ptr(), msg.len());
            assert_eq!(result, 0);
        }
        assert_eq!(HostAPI::get_log_count(), 1);
    }

    #[test]
    fn test_counters() {
        HostAPI::reset_counters();
        assert_eq!(HostAPI::get_host_call_count(), 0);
        
        let _ = HostAPI::get_timestamp_ns();
        assert_eq!(HostAPI::get_host_call_count(), 1);
        
        let _ = HostAPI::get_random_u64();
        // get_random_u64 internally calls get_timestamp_ns, so count is 3 (1 + 2)
        assert_eq!(HostAPI::get_host_call_count(), 3);
    }
}
