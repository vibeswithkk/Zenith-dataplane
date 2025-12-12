//! Safe Rust wrapper for C++ NUMA backend
//!
//! This module provides safe Rust bindings to the native libnuma C++ backend.
//! It is only compiled when the `numa_cpp` feature is enabled.
//!
//! # Usage
//!
//! ```rust,ignore
//! use zenith_runtime_cpu::numa_ffi::{NumaAllocator, bind_thread_to_node};
//!
//! // Initialize and bind thread to NUMA node 0
//! bind_thread_to_node(0)?;
//!
//! // Allocate memory on NUMA node 0
//! let allocator = NumaAllocator::new(0)?;
//! let ptr = unsafe { allocator.alloc(4096)? };
//!
//! // Use the memory...
//!
//! // Free when done
//! unsafe { allocator.free(ptr, 4096) };
//! ```

use std::ffi::c_void;

/// Error codes from the C++ NUMA backend
pub const ZENITH_NUMA_OK: i32 = 0;
pub const ZENITH_NUMA_ERR_UNAVAILABLE: i32 = -1;
pub const ZENITH_NUMA_ERR_INVALID_NODE: i32 = -2;
pub const ZENITH_NUMA_ERR_ALLOC_FAILED: i32 = -3;
pub const ZENITH_NUMA_ERR_BIND_FAILED: i32 = -4;
pub const ZENITH_NUMA_ERR_NULL_PTR: i32 = -5;

/// Information about a NUMA node
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ZenithNumaNodeInfo {
    pub node_id: i32,
    pub total_memory: u64,
    pub free_memory: u64,
    pub num_cpus: i32,
}

// FFI bindings to C++ backend
#[cfg(feature = "numa_cpp")]
mod ffi {
    use super::*;

    extern "C" {
        pub fn zenith_numa_init() -> i32;
        pub fn zenith_numa_cleanup();
        pub fn zenith_numa_available() -> i32;
        pub fn zenith_numa_num_nodes() -> i32;
        pub fn zenith_numa_num_cpus() -> i32;
        pub fn zenith_numa_node_of_cpu(cpu: i32) -> i32;
        pub fn zenith_numa_preferred_node() -> i32;
        pub fn zenith_numa_alloc_onnode(size: usize, node: i32) -> *mut c_void;
        pub fn zenith_numa_alloc_interleaved(size: usize) -> *mut c_void;
        pub fn zenith_numa_alloc_local(size: usize) -> *mut c_void;
        pub fn zenith_numa_free(ptr: *mut c_void, size: usize);
        pub fn zenith_numa_bind_thread_to_node(node: i32) -> i32;
        pub fn zenith_numa_bind_thread_to_cpu(cpu: i32) -> i32;
        pub fn zenith_numa_unbind_thread() -> i32;
        pub fn zenith_numa_set_preferred(node: i32) -> i32;
        pub fn zenith_numa_set_interleave(nodemask: u64) -> i32;
        pub fn zenith_numa_set_membind(nodemask: u64) -> i32;
        pub fn zenith_numa_get_node_info(node: i32, info: *mut ZenithNumaNodeInfo) -> i32;
        pub fn zenith_numa_distance(node1: i32, node2: i32) -> i32;
    }
}

/// NUMA backend error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaError {
    /// NUMA is not available on this system
    Unavailable,
    /// Invalid NUMA node specified
    InvalidNode,
    /// Memory allocation failed
    AllocFailed,
    /// Thread binding failed
    BindFailed,
    /// Null pointer passed
    NullPtr,
    /// Unknown error
    Unknown(i32),
}

impl std::fmt::Display for NumaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumaError::Unavailable => write!(f, "NUMA not available"),
            NumaError::InvalidNode => write!(f, "Invalid NUMA node"),
            NumaError::AllocFailed => write!(f, "NUMA allocation failed"),
            NumaError::BindFailed => write!(f, "Thread binding failed"),
            NumaError::NullPtr => write!(f, "Null pointer"),
            NumaError::Unknown(code) => write!(f, "Unknown NUMA error: {}", code),
        }
    }
}

impl std::error::Error for NumaError {}

impl From<i32> for NumaError {
    fn from(code: i32) -> Self {
        match code {
            ZENITH_NUMA_ERR_UNAVAILABLE => NumaError::Unavailable,
            ZENITH_NUMA_ERR_INVALID_NODE => NumaError::InvalidNode,
            ZENITH_NUMA_ERR_ALLOC_FAILED => NumaError::AllocFailed,
            ZENITH_NUMA_ERR_BIND_FAILED => NumaError::BindFailed,
            ZENITH_NUMA_ERR_NULL_PTR => NumaError::NullPtr,
            _ => NumaError::Unknown(code),
        }
    }
}

/// Result type for NUMA operations
pub type NumaResult<T> = Result<T, NumaError>;

/// Convert FFI result to Rust Result
fn check_result(code: i32) -> NumaResult<()> {
    if code == ZENITH_NUMA_OK {
        Ok(())
    } else {
        Err(NumaError::from(code))
    }
}

// ============================================================================
// Safe Public API
// ============================================================================

/// Initialize the NUMA subsystem
///
/// Must be called before any other NUMA functions.
#[cfg(feature = "numa_cpp")]
pub fn init() -> NumaResult<()> {
    check_result(unsafe { ffi::zenith_numa_init() })
}

/// Cleanup NUMA subsystem resources
#[cfg(feature = "numa_cpp")]
pub fn cleanup() {
    unsafe { ffi::zenith_numa_cleanup() }
}

/// Check if NUMA is available on this system
#[cfg(feature = "numa_cpp")]
pub fn is_available() -> bool {
    unsafe { ffi::zenith_numa_available() != 0 }
}

/// Get the number of NUMA nodes
#[cfg(feature = "numa_cpp")]
pub fn num_nodes() -> i32 {
    unsafe { ffi::zenith_numa_num_nodes() }
}

/// Get the total number of CPUs
#[cfg(feature = "numa_cpp")]
pub fn num_cpus() -> i32 {
    unsafe { ffi::zenith_numa_num_cpus() }
}

/// Get the NUMA node for a given CPU
#[cfg(feature = "numa_cpp")]
pub fn node_of_cpu(cpu: i32) -> Option<i32> {
    let node = unsafe { ffi::zenith_numa_node_of_cpu(cpu) };
    if node >= 0 {
        Some(node)
    } else {
        None
    }
}

/// Get the preferred NUMA node for the current thread
#[cfg(feature = "numa_cpp")]
pub fn preferred_node() -> i32 {
    unsafe { ffi::zenith_numa_preferred_node() }
}

/// Bind the current thread to a NUMA node
#[cfg(feature = "numa_cpp")]
pub fn bind_thread_to_node(node: i32) -> NumaResult<()> {
    check_result(unsafe { ffi::zenith_numa_bind_thread_to_node(node) })
}

/// Bind the current thread to a specific CPU
#[cfg(feature = "numa_cpp")]
pub fn bind_thread_to_cpu(cpu: i32) -> NumaResult<()> {
    check_result(unsafe { ffi::zenith_numa_bind_thread_to_cpu(cpu) })
}

/// Unbind the current thread
#[cfg(feature = "numa_cpp")]
pub fn unbind_thread() -> NumaResult<()> {
    check_result(unsafe { ffi::zenith_numa_unbind_thread() })
}

/// Set the preferred NUMA node for future allocations
#[cfg(feature = "numa_cpp")]
pub fn set_preferred(node: i32) -> NumaResult<()> {
    check_result(unsafe { ffi::zenith_numa_set_preferred(node) })
}

/// Get information about a NUMA node
#[cfg(feature = "numa_cpp")]
pub fn get_node_info(node: i32) -> NumaResult<ZenithNumaNodeInfo> {
    let mut info = ZenithNumaNodeInfo::default();
    check_result(unsafe { ffi::zenith_numa_get_node_info(node, &mut info) })?;
    Ok(info)
}

/// Get the distance between two NUMA nodes
#[cfg(feature = "numa_cpp")]
pub fn distance(node1: i32, node2: i32) -> Option<i32> {
    let dist = unsafe { ffi::zenith_numa_distance(node1, node2) };
    if dist >= 0 {
        Some(dist)
    } else {
        None
    }
}

// ============================================================================
// NUMA Allocator
// ============================================================================

/// NUMA-aware memory allocator
///
/// Allocates memory on a specific NUMA node for optimal memory locality.
#[cfg(feature = "numa_cpp")]
pub struct NumaAllocator {
    node: i32,
}

#[cfg(feature = "numa_cpp")]
impl NumaAllocator {
    /// Create a new allocator for a specific NUMA node
    pub fn new(node: i32) -> NumaResult<Self> {
        init()?;
        if node >= num_nodes() {
            return Err(NumaError::InvalidNode);
        }
        Ok(Self { node })
    }

    /// Create an allocator for the local NUMA node
    pub fn local() -> NumaResult<Self> {
        init()?;
        Ok(Self {
            node: preferred_node(),
        })
    }

    /// Get the NUMA node this allocator uses
    pub fn node(&self) -> i32 {
        self.node
    }

    /// Allocate memory on this allocator's NUMA node
    ///
    /// # Safety
    ///
    /// The caller must:
    /// - Ensure the memory is freed with `free()` when no longer needed
    /// - Not use the pointer after freeing
    pub unsafe fn alloc(&self, size: usize) -> NumaResult<*mut u8> {
        let ptr = ffi::zenith_numa_alloc_onnode(size, self.node);
        if ptr.is_null() {
            Err(NumaError::AllocFailed)
        } else {
            Ok(ptr as *mut u8)
        }
    }

    /// Allocate memory interleaved across all NUMA nodes
    ///
    /// # Safety
    ///
    /// Same requirements as `alloc()`
    pub unsafe fn alloc_interleaved(&self, size: usize) -> NumaResult<*mut u8> {
        let ptr = ffi::zenith_numa_alloc_interleaved(size);
        if ptr.is_null() {
            Err(NumaError::AllocFailed)
        } else {
            Ok(ptr as *mut u8)
        }
    }

    /// Free NUMA-allocated memory
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `ptr` was allocated by this allocator
    /// - `size` matches the original allocation size
    /// - The pointer is not used after freeing
    pub unsafe fn free(&self, ptr: *mut u8, size: usize) {
        ffi::zenith_numa_free(ptr as *mut c_void, size);
    }
}

// ============================================================================
// Fallback implementations when numa_cpp is not enabled
// ============================================================================

#[cfg(not(feature = "numa_cpp"))]
pub fn init() -> NumaResult<()> {
    Err(NumaError::Unavailable)
}

#[cfg(not(feature = "numa_cpp"))]
pub fn cleanup() {}

#[cfg(not(feature = "numa_cpp"))]
pub fn is_available() -> bool {
    false
}

#[cfg(not(feature = "numa_cpp"))]
pub fn num_nodes() -> i32 {
    1
}

#[cfg(not(feature = "numa_cpp"))]
pub fn num_cpus() -> i32 {
    num_cpus::get() as i32
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        assert_eq!(NumaError::from(ZENITH_NUMA_ERR_UNAVAILABLE), NumaError::Unavailable);
        assert_eq!(NumaError::from(ZENITH_NUMA_ERR_INVALID_NODE), NumaError::InvalidNode);
    }

    #[test]
    #[cfg(feature = "numa_cpp")]
    fn test_numa_init() {
        // This will succeed or fail based on system NUMA support
        let result = init();
        // Either it works or NUMA is unavailable, both are valid
        assert!(result.is_ok() || matches!(result, Err(NumaError::Unavailable)));
    }
}
