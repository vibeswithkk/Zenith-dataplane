//! Zenith AI - PyO3 Python Bindings
//!
//! This crate provides native Python bindings for the Zenith AI
//! high-performance data loading engine using PyO3.
//!
//! # Features
//! - Zero-copy data transfer via Apache Arrow
//! - High-performance ring buffer for data streaming
//! - WASM plugin execution for preprocessing
//!
//! # Example (Python)
//! ```python
//! from zenith._core import Engine
//!
//! engine = Engine(buffer_size=4096)
//! engine.load_plugin("preprocess.wasm")
//! ```

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError, PyIOError};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::collections::VecDeque;

mod engine;
mod buffer;
mod plugin;

pub use engine::PyEngine;
pub use buffer::RingBuffer;
pub use plugin::PluginManager;

/// Zenith AI Python Module
///
/// High-performance data infrastructure for machine learning.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register classes
    m.add_class::<PyEngine>()?;
    m.add_class::<PyDataLoader>()?;
    m.add_class::<PyPluginInfo>()?;
    
    // Register functions
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(is_available, m)?)?;
    
    // Module metadata
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", "Zenith Contributors")?;
    
    Ok(())
}

/// Get the Zenith native library version
#[pyfunction]
fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Check if native acceleration is available
#[pyfunction]
fn is_available() -> bool {
    true
}

/// Plugin information exposed to Python
#[pyclass]
#[derive(Clone)]
pub struct PyPluginInfo {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub version: String,
    #[pyo3(get)]
    pub path: String,
}

#[pymethods]
impl PyPluginInfo {
    fn __repr__(&self) -> String {
        format!("<PluginInfo(name='{}', version='{}')>", self.name, self.version)
    }
}

/// High-performance DataLoader for ML training
#[pyclass]
pub struct PyDataLoader {
    source: String,
    batch_size: usize,
    shuffle: bool,
    num_workers: usize,
    engine: Arc<Mutex<engine::EngineCore>>,
}

#[pymethods]
impl PyDataLoader {
    #[new]
    #[pyo3(signature = (source, batch_size=32, shuffle=true, num_workers=4))]
    fn new(
        source: String,
        batch_size: usize,
        shuffle: bool,
        num_workers: usize,
    ) -> PyResult<Self> {
        let engine = engine::EngineCore::new(1024)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create engine: {}", e)))?;
        
        Ok(Self {
            source,
            batch_size,
            shuffle,
            num_workers,
            engine: Arc::new(Mutex::new(engine)),
        })
    }
    
    /// Get the data source path
    #[getter]
    fn source(&self) -> &str {
        &self.source
    }
    
    /// Get the batch size
    #[getter]
    fn batch_size(&self) -> usize {
        self.batch_size
    }
    
    /// Get shuffle setting
    #[getter]
    fn shuffle(&self) -> bool {
        self.shuffle
    }
    
    /// Get number of workers
    #[getter]
    fn num_workers(&self) -> usize {
        self.num_workers
    }
    
    fn __repr__(&self) -> String {
        format!(
            "<DataLoader(source='{}', batch_size={}, shuffle={}, num_workers={})>",
            self.source, self.batch_size, self.shuffle, self.num_workers
        )
    }
    
    fn __len__(&self) -> usize {
        // Placeholder - would calculate based on dataset size
        0
    }
}
