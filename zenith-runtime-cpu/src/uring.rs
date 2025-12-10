//! Full io_uring Implementation
//!
//! Production-ready async I/O using Linux io_uring.

use std::os::unix::io::RawFd;
use std::path::Path;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::{Error, Result};

/// io_uring operation types
#[derive(Debug, Clone, Copy)]
pub enum IoOp {
    /// Read operation
    Read,
    /// Write operation
    Write,
    /// Fsync operation
    Fsync,
    /// Close operation
    Close,
}

/// Completion entry from io_uring
#[derive(Debug)]
pub struct Completion {
    /// User data (request ID)
    pub user_data: u64,
    /// Result (bytes transferred or error)
    pub result: i32,
    /// Operation type
    pub op: IoOp,
}

/// io_uring configuration
#[derive(Debug, Clone)]
pub struct UringConfig {
    /// Number of submission queue entries
    pub sq_entries: u32,
    /// Enable submission queue polling
    pub sq_poll: bool,
    /// SQ poll idle timeout (milliseconds)
    pub sq_poll_idle_ms: u32,
    /// Enable IO polling
    pub io_poll: bool,
    /// Use registered buffers
    pub registered_buffers: bool,
    /// Number of registered buffers
    pub num_buffers: usize,
    /// Size of each buffer
    pub buffer_size: usize,
}

impl Default for UringConfig {
    fn default() -> Self {
        Self {
            sq_entries: 4096,
            sq_poll: false,
            sq_poll_idle_ms: 1000,
            io_poll: false,
            registered_buffers: false,
            num_buffers: 64,
            buffer_size: 64 * 1024, // 64KB
        }
    }
}

/// High-performance io_uring engine
/// 
/// Provides async I/O operations with minimal syscall overhead.
pub struct UringEngine {
    _config: UringConfig,
    ring: parking_lot::Mutex<io_uring::IoUring>,
    next_id: AtomicU64,
    pending: parking_lot::Mutex<VecDeque<PendingOp>>,
}

struct PendingOp {
    id: u64,
    op: IoOp,
}

impl UringEngine {
    /// Create a new io_uring engine
    pub fn new(config: UringConfig) -> Result<Self> {
        let mut builder = io_uring::IoUring::builder();
        
        if config.sq_poll {
            builder.setup_sqpoll(config.sq_poll_idle_ms);
        }
        
        if config.io_poll {
            builder.setup_iopoll();
        }
        
        let ring = builder
            .build(config.sq_entries)
            .map_err(|e| Error::IoUring(format!("Failed to create io_uring: {}", e)))?;
        
        Ok(Self {
            _config: config,
            ring: parking_lot::Mutex::new(ring),
            next_id: AtomicU64::new(1),
            pending: parking_lot::Mutex::new(VecDeque::new()),
        })
    }
    
    /// Get next request ID
    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
    
    /// Submit a read operation
    pub fn submit_read(&self, fd: RawFd, buf: &mut [u8], offset: u64) -> Result<u64> {
        let id = self.next_id();
        
        // Create read entry
        let read_e = io_uring::opcode::Read::new(
            io_uring::types::Fd(fd),
            buf.as_mut_ptr(),
            buf.len() as u32,
        )
        .offset(offset)
        .build()
        .user_data(id);
        
        // Submit to ring
        unsafe {
            self.ring.lock().submission()
                .push(&read_e)
                .map_err(|_| Error::IoUring("Submission queue full".to_string()))?;
        }
        
        self.pending.lock().push_back(PendingOp { id, op: IoOp::Read });
        
        Ok(id)
    }
    
    /// Submit a write operation
    pub fn submit_write(&self, fd: RawFd, buf: &[u8], offset: u64) -> Result<u64> {
        let id = self.next_id();
        
        let write_e = io_uring::opcode::Write::new(
            io_uring::types::Fd(fd),
            buf.as_ptr(),
            buf.len() as u32,
        )
        .offset(offset)
        .build()
        .user_data(id);
        
        unsafe {
            self.ring.lock().submission()
                .push(&write_e)
                .map_err(|_| Error::IoUring("Submission queue full".to_string()))?;
        }
        
        self.pending.lock().push_back(PendingOp { id, op: IoOp::Write });
        
        Ok(id)
    }
    
    /// Submit a vectored read (readv)
    pub fn submit_readv(&self, fd: RawFd, iovecs: &mut [libc::iovec], offset: u64) -> Result<u64> {
        let id = self.next_id();
        
        let readv_e = io_uring::opcode::Readv::new(
            io_uring::types::Fd(fd),
            iovecs.as_mut_ptr(),
            iovecs.len() as u32,
        )
        .offset(offset)
        .build()
        .user_data(id);
        
        unsafe {
            self.ring.lock().submission()
                .push(&readv_e)
                .map_err(|_| Error::IoUring("Submission queue full".to_string()))?;
        }
        
        self.pending.lock().push_back(PendingOp { id, op: IoOp::Read });
        
        Ok(id)
    }
    
    /// Submit fsync
    pub fn submit_fsync(&self, fd: RawFd) -> Result<u64> {
        let id = self.next_id();
        
        let fsync_e = io_uring::opcode::Fsync::new(io_uring::types::Fd(fd))
            .build()
            .user_data(id);
        
        unsafe {
            self.ring.lock().submission()
                .push(&fsync_e)
                .map_err(|_| Error::IoUring("Submission queue full".to_string()))?;
        }
        
        self.pending.lock().push_back(PendingOp { id, op: IoOp::Fsync });
        
        Ok(id)
    }
    
    /// Submit all pending operations
    pub fn submit(&self) -> Result<usize> {
        self.ring.lock().submit()
            .map_err(|e| Error::IoUring(format!("Submit failed: {}", e)))
    }
    
    /// Submit and wait for at least one completion
    pub fn submit_and_wait(&self, want: usize) -> Result<usize> {
        self.ring.lock().submit_and_wait(want)
            .map_err(|e| Error::IoUring(format!("Submit and wait failed: {}", e)))
    }
    
    /// Get completions
    pub fn completions(&self) -> Vec<Completion> {
        let mut completions = Vec::new();
        let pending = self.pending.lock();
        let mut ring = self.ring.lock();
        
        for cqe in ring.completion() {
            let user_data = cqe.user_data();
            let result = cqe.result();
            
            // Find the operation type
            let op = pending.iter()
                .find(|p| p.id == user_data)
                .map(|p| p.op)
                .unwrap_or(IoOp::Read);
            
            completions.push(Completion {
                user_data,
                result,
                op,
            });
        }
        
        completions
    }
    
    /// Get number of pending operations
    pub fn pending_count(&self) -> usize {
        self.pending.lock().len()
    }
}

/// Async file handle using io_uring
pub struct AsyncFile {
    fd: RawFd,
    engine: Arc<UringEngine>,
}

impl AsyncFile {
    /// Open a file for reading
    pub fn open<P: AsRef<Path>>(path: P, engine: Arc<UringEngine>) -> Result<Self> {
        let fd = nix::fcntl::open(
            path.as_ref(),
            nix::fcntl::OFlag::O_RDONLY | nix::fcntl::OFlag::O_DIRECT,
            nix::sys::stat::Mode::empty(),
        ).map_err(|e| Error::Io(std::io::Error::other(
            e.to_string(),
        )))?;
        
        Ok(Self { fd, engine })
    }
    
    /// Create a file for writing
    pub fn create<P: AsRef<Path>>(path: P, engine: Arc<UringEngine>) -> Result<Self> {
        let fd = nix::fcntl::open(
            path.as_ref(),
            nix::fcntl::OFlag::O_WRONLY | nix::fcntl::OFlag::O_CREAT | nix::fcntl::OFlag::O_TRUNC | nix::fcntl::OFlag::O_DIRECT,
            nix::sys::stat::Mode::from_bits(0o644).unwrap(),
        ).map_err(|e| Error::Io(std::io::Error::other(
            e.to_string(),
        )))?;
        
        Ok(Self { fd, engine })
    }
    
    /// Submit a read operation
    pub fn read(&self, buf: &mut [u8], offset: u64) -> Result<u64> {
        self.engine.submit_read(self.fd, buf, offset)
    }
    
    /// Submit a write operation
    pub fn write(&self, buf: &[u8], offset: u64) -> Result<u64> {
        self.engine.submit_write(self.fd, buf, offset)
    }
    
    /// Sync to disk
    pub fn fsync(&self) -> Result<u64> {
        self.engine.submit_fsync(self.fd)
    }
}

impl Drop for AsyncFile {
    fn drop(&mut self) {
        let _ = nix::unistd::close(self.fd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_uring_config() {
        let config = UringConfig::default();
        assert_eq!(config.sq_entries, 4096);
        assert!(!config.sq_poll);
    }
    
    #[test]
    fn test_uring_creation() {
        // May fail if io_uring not supported
        let config = UringConfig {
            sq_entries: 32,
            ..Default::default()
        };
        
        match UringEngine::new(config) {
            Ok(engine) => {
                assert_eq!(engine.pending_count(), 0);
            }
            Err(_) => {
                // io_uring may not be available in test environment
            }
        }
    }
}
