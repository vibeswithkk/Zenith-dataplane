//! # Zenith CPU Runtime
//!
//! Ultra-low-latency CPU runtime for high-performance data processing.
//!
//! Copyright 2025 Wahyu Ardiansyah and Zenith AI Contributors
//! Licensed under Apache License 2.0
//!
//! ## Features
//!
//! - **NUMA-aware memory allocation**: Optimized for multi-socket systems
//! - **io_uring async I/O**: High-performance Linux async I/O
//! - **Lock-free ring buffers**: Zero-contention producer/consumer queues
//! - **Thread pinning**: Deterministic core affinity for latency-critical tasks
//! - **Hugepage support**: Reduced TLB misses for large allocations
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Zenith CPU Runtime                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │   Thread    │  │  io_uring   │  │   NUMA Allocator    │  │
//! │  │   Pinning   │  │   I/O Loop  │  │   (Hugepages)       │  │
//! │  └─────────────┘  └─────────────┘  └─────────────────────┘  │
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │              Lock-Free Ring Buffers (SPSC/MPMC)         ││
//! │  └─────────────────────────────────────────────────────────┘│
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │                 Telemetry & Metrics                      ││
//! │  └─────────────────────────────────────────────────────────┘│
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use zenith_runtime_cpu::{CpuEngine, EngineConfig};
//!
//! #[tokio::main]
//! async fn main() -> zenith_runtime_cpu::Result<()> {
//!     let config = EngineConfig::builder()
//!         .numa_aware(true)
//!         .hugepages(true)
//!         .io_uring_entries(4096)
//!         .build()?;
//!     
//!     let engine = CpuEngine::new(config)?;
//!     Ok(engine.run().await?)
//! }
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod allocator;
pub mod buffer;
pub mod config;
pub mod engine;
pub mod io;
pub mod numa;
pub mod telemetry;
pub mod thread;

// Re-exports
pub use config::EngineConfig;
pub use engine::CpuEngine;
pub use buffer::{RingBuffer, SpscRingBuffer, MpmcRingBuffer};
pub use allocator::NumaAllocator;
pub use numa::NumaTopology;
pub use telemetry::TelemetryCollector;

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result type alias for this crate
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for the CPU runtime
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// NUMA-related errors
    #[error("NUMA error: {0}")]
    Numa(String),
    
    /// Memory allocation errors
    #[error("Allocation error: {0}")]
    Allocation(String),
    
    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Thread affinity errors
    #[error("Thread affinity error: {0}")]
    Affinity(String),
    
    /// Buffer errors
    #[error("Buffer error: {0}")]
    Buffer(String),
    
    /// io_uring errors
    #[error("io_uring error: {0}")]
    IoUring(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
