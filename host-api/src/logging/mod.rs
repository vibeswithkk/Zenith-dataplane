/// Structured Logging Module for WASM Plugins
/// Provides leveled, structured logging with context

use std::sync::Mutex;
use std::collections::VecDeque;

/// Maximum log entries to keep in memory
const MAX_LOG_ENTRIES: usize = 1000;

/// Log level
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl From<u32> for LogLevel {
    fn from(val: u32) -> Self {
        match val {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            _ => LogLevel::Error,
        }
    }
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: LogLevel,
    pub message: String,
    pub plugin_id: Option<String>,
}

lazy_static::lazy_static! {
    static ref LOG_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
}

/// Logging API
pub struct LoggingAPI;

impl LoggingAPI {
    /// Log a message
    pub fn log(level: LogLevel, message: &str, plugin_id: Option<&str>) {
        let entry = LogEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            level,
            message: message.to_string(),
            plugin_id: plugin_id.map(String::from),
        };
        
        // Print to tracing
        match level {
            LogLevel::Trace => tracing::trace!("[{}] {}", plugin_id.unwrap_or("unknown"), message),
            LogLevel::Debug => tracing::debug!("[{}] {}", plugin_id.unwrap_or("unknown"), message),
            LogLevel::Info => tracing::info!("[{}] {}", plugin_id.unwrap_or("unknown"), message),
            LogLevel::Warn => tracing::warn!("[{}] {}", plugin_id.unwrap_or("unknown"), message),
            LogLevel::Error => tracing::error!("[{}] {}", plugin_id.unwrap_or("unknown"), message),
        }
        
        // Store in buffer
        let mut buffer = LOG_BUFFER.lock().unwrap();
        buffer.push_back(entry);
        
        // Trim if too large
        while buffer.len() > MAX_LOG_ENTRIES {
            buffer.pop_front();
        }
    }
    
    /// Get recent log entries
    pub fn get_recent_logs(count: usize) -> Vec<LogEntry> {
        let buffer = LOG_BUFFER.lock().unwrap();
        buffer.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }
    
    /// Clear log buffer
    pub fn clear_logs() {
        LOG_BUFFER.lock().unwrap().clear();
    }
    
    /// Get log count
    pub fn get_log_count() -> usize {
        LOG_BUFFER.lock().unwrap().len()
    }
}

// C ABI exports
#[no_mangle]
pub unsafe extern "C" fn zenith_log(
    level: u32,
    message_ptr: *const u8,
    message_len: usize,
) -> i32 {
    if message_ptr.is_null() {
        return -1;
    }
    
    let slice = std::slice::from_raw_parts(message_ptr, message_len);
    let message = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => return -2,
    };
    
    LoggingAPI::log(level.into(), message, Some("plugin"));
    0
}

#[no_mangle]
pub extern "C" fn zenith_log_count() -> usize {
    LoggingAPI::get_log_count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging() {
        // Use a mutex to serialize this test
        static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = TEST_LOCK.lock().unwrap();
        
        LoggingAPI::clear_logs();
        
        LoggingAPI::log(LogLevel::Info, "Test message", Some("test_plugin"));
        assert_eq!(LoggingAPI::get_log_count(), 1);
        
        let logs = LoggingAPI::get_recent_logs(10);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, "Test message");
        assert_eq!(logs[0].level, LogLevel::Info);
    }

    #[test]
    fn test_log_buffer_limit() {
        LoggingAPI::clear_logs();
        
        for i in 0..1500 {
            LoggingAPI::log(LogLevel::Debug, &format!("Log {}", i), None);
        }
        
        assert_eq!(LoggingAPI::get_log_count(), MAX_LOG_ENTRIES);
    }
}
