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
    use arrow::datatypes::{DataType, Field};
    use arrow::array::Int32Array;
    
    // ===================== FileFormat Tests =====================
    
    #[test]
    fn test_file_format_detection() {
        assert_eq!(FileFormat::from_extension("data.parquet"), FileFormat::Parquet);
        assert_eq!(FileFormat::from_extension("data.csv"), FileFormat::Csv);
        assert_eq!(FileFormat::from_extension("data.arrow"), FileFormat::ArrowIpc);
        assert_eq!(FileFormat::from_extension("data.unknown"), FileFormat::Unknown);
    }
    
    #[test]
    fn test_file_format_feather() {
        // 'ipc' is not supported, but 'feather' is an alias for ArrowIpc
        assert_eq!(FileFormat::from_extension("data.feather"), FileFormat::ArrowIpc);
    }
    
    #[test]
    fn test_file_format_jsonl() {
        assert_eq!(FileFormat::from_extension("data.jsonl"), FileFormat::JsonLines);
        assert_eq!(FileFormat::from_extension("data.ndjson"), FileFormat::JsonLines);
    }
    
    #[test]
    fn test_file_format_uppercase() {
        // Extensions are case-sensitive, uppercase should be unknown
        assert_eq!(FileFormat::from_extension("data.PARQUET"), FileFormat::Unknown);
    }
    
    #[test]
    fn test_file_format_no_extension() {
        assert_eq!(FileFormat::from_extension("data"), FileFormat::Unknown);
    }
    
    #[test]
    fn test_file_format_clone_copy() {
        let format = FileFormat::Parquet;
        let cloned = format.clone();
        let copied = format;
        assert_eq!(format, cloned);
        assert_eq!(format, copied);
    }
    
    #[test]
    fn test_file_format_debug() {
        let format = FileFormat::Parquet;
        let debug_str = format!("{:?}", format);
        assert!(debug_str.contains("Parquet"));
    }
    
    // ===================== LoaderConfig Tests =====================
    
    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert_eq!(config.batch_size, 1024);
        assert_eq!(config.num_workers, 4);
        assert_eq!(config.prefetch_count, 4);
        assert!(config.memory_map);
        assert_eq!(config.io_buffer_size, 8 * 1024 * 1024);
    }
    
    #[test]
    fn test_loader_config_custom() {
        let config = LoaderConfig {
            batch_size: 2048,
            num_workers: 8,
            prefetch_count: 8,
            memory_map: false,
            io_buffer_size: 4 * 1024 * 1024,
        };
        assert_eq!(config.batch_size, 2048);
        assert_eq!(config.num_workers, 8);
        assert!(!config.memory_map);
    }
    
    #[test]
    fn test_loader_config_clone() {
        let config = LoaderConfig::default();
        let cloned = config.clone();
        assert_eq!(config.batch_size, cloned.batch_size);
        assert_eq!(config.num_workers, cloned.num_workers);
    }
    
    #[test]
    fn test_loader_config_debug() {
        let config = LoaderConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("LoaderConfig"));
        assert!(debug_str.contains("batch_size"));
    }
    
    // ===================== DataSource Tests =====================
    
    #[test]
    fn test_data_source_from_path_file() {
        let source = DataSource::from_path("/tmp/test.parquet");
        match source {
            DataSource::File(p) => assert!(p.contains("test.parquet")),
            _ => panic!("Expected File variant"),
        }
    }
    
    #[test]
    fn test_data_source_directory() {
        // DataSource::Directory variant can be created directly
        let source = DataSource::Directory("/tmp/data".to_string());
        match source {
            DataSource::Directory(p) => assert!(p.contains("data")),
            _ => panic!("Expected Directory variant"),
        }
    }
    
    #[test]
    fn test_data_source_memory() {
        let data = vec![1u8, 2, 3, 4, 5];
        let source = DataSource::Memory(data.clone());
        match source {
            DataSource::Memory(d) => assert_eq!(d.len(), 5),
            _ => panic!("Expected Memory variant"),
        }
    }
    
    #[test]
    fn test_data_source_clone() {
        let source = DataSource::File("test.parquet".to_string());
        let cloned = source.clone();
        match (source, cloned) {
            (DataSource::File(a), DataSource::File(b)) => assert_eq!(a, b),
            _ => panic!("Clone mismatch"),
        }
    }
    
    #[test]
    fn test_data_source_debug() {
        let source = DataSource::File("test.parquet".to_string());
        let debug_str = format!("{:?}", source);
        assert!(debug_str.contains("File"));
        assert!(debug_str.contains("test.parquet"));
    }
    
    // ===================== DataLoader Tests =====================
    
    #[test]
    fn test_data_loader_creation() {
        let source = DataSource::File("test.parquet".to_string());
        let config = LoaderConfig::default();
        let loader = DataLoader::new(source, config);
        
        assert_eq!(loader.config().batch_size, 1024);
    }
    
    #[test]
    fn test_data_loader_with_defaults() {
        let loader = DataLoader::with_defaults("/tmp/test.parquet");
        assert_eq!(loader.config().batch_size, 1024);
    }
    
    #[test]
    fn test_data_loader_config_access() {
        let config = LoaderConfig {
            batch_size: 512,
            num_workers: 2,
            prefetch_count: 1,
            memory_map: false,
            io_buffer_size: 1024 * 1024,
        };
        let loader = DataLoader::new(DataSource::File("test.csv".to_string()), config);
        
        assert_eq!(loader.config().batch_size, 512);
        assert_eq!(loader.config().num_workers, 2);
    }
    
    #[test]
    fn test_data_loader_clear_cache() {
        let loader = DataLoader::with_defaults("/tmp/test.parquet");
        // Should not panic even when cache is empty
        loader.clear_cache();
    }
    
    #[test]
    fn test_data_loader_schema_before_load() {
        let loader = DataLoader::with_defaults("/tmp/test.parquet");
        // Schema should be None before loading
        assert!(loader.schema().is_none());
    }
    
    #[test]
    fn test_data_loader_load_nonexistent_file() {
        let loader = DataLoader::with_defaults("/nonexistent/path/data.parquet");
        let result = loader.load();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_data_loader_load_unsupported_format() {
        let loader = DataLoader::with_defaults("/tmp/data.xyz");
        let result = loader.load();
        assert!(result.is_err());
    }
    
    // ===================== BatchIterator Tests =====================
    
    fn create_test_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
        ]))
    }
    
    fn create_test_batch(schema: &Arc<Schema>, values: Vec<i32>) -> RecordBatch {
        let array = Int32Array::from(values);
        RecordBatch::try_new(schema.clone(), vec![Arc::new(array)]).unwrap()
    }
    
    #[test]
    fn test_batch_iterator_creation() {
        let schema = create_test_schema();
        let batches = vec![
            create_test_batch(&schema, vec![1, 2, 3]),
            create_test_batch(&schema, vec![4, 5, 6]),
        ];
        
        let iter = BatchIterator::new(schema.clone(), batches);
        assert_eq!(iter.num_batches(), 2);
        assert_eq!(iter.total_rows(), 6);
    }
    
    #[test]
    fn test_batch_iterator_schema() {
        let schema = create_test_schema();
        let batches = vec![create_test_batch(&schema, vec![1, 2, 3])];
        
        let iter = BatchIterator::new(schema.clone(), batches);
        let iter_schema = iter.schema();
        
        assert_eq!(iter_schema.fields().len(), 1);
        assert_eq!(iter_schema.field(0).name(), "id");
    }
    
    #[test]
    fn test_batch_iterator_empty() {
        let schema = create_test_schema();
        let iter = BatchIterator::new(schema, vec![]);
        
        assert_eq!(iter.num_batches(), 0);
        assert_eq!(iter.total_rows(), 0);
    }
    
    #[test]
    fn test_batch_iterator_iteration() {
        let schema = create_test_schema();
        let batches = vec![
            create_test_batch(&schema, vec![1, 2]),
            create_test_batch(&schema, vec![3, 4]),
        ];
        
        let mut iter = BatchIterator::new(schema, batches);
        
        let first = iter.next();
        assert!(first.is_some());
        assert_eq!(first.unwrap().num_rows(), 2);
        
        let second = iter.next();
        assert!(second.is_some());
        assert_eq!(second.unwrap().num_rows(), 2);
        
        let third = iter.next();
        assert!(third.is_none());
    }
    
    #[test]
    fn test_batch_iterator_reset() {
        let schema = create_test_schema();
        let batches = vec![create_test_batch(&schema, vec![1, 2, 3])];
        
        let mut iter = BatchIterator::new(schema, batches);
        
        // Consume the iterator
        let _ = iter.next();
        assert!(iter.next().is_none());
        
        // Reset and iterate again
        iter.reset();
        assert!(iter.next().is_some());
    }
    
    // ===================== DataLoaderError Tests =====================
    
    #[test]
    fn test_error_io() {
        let err = DataLoaderError::Io("file not found".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("I/O error"));
        assert!(msg.contains("file not found"));
    }
    
    #[test]
    fn test_error_parse() {
        let err = DataLoaderError::Parse("invalid format".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Parse error"));
    }
    
    #[test]
    fn test_error_unsupported_format() {
        let err = DataLoaderError::UnsupportedFormat(".xyz".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Unsupported format"));
    }
    
    #[test]
    fn test_error_empty() {
        let err = DataLoaderError::Empty("no data".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Empty source"));
    }
    
    #[test]
    fn test_error_config() {
        let err = DataLoaderError::Config("invalid batch size".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Configuration error"));
    }
    
    #[test]
    fn test_error_debug() {
        let err = DataLoaderError::Io("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Io"));
    }
    
    #[test]
    fn test_error_is_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(DataLoaderError::Io("test".to_string()));
        assert!(err.to_string().contains("I/O error"));
    }
    
    // ========================================================================
    // MUTATION-KILLING TESTS
    // ========================================================================
    
    /// Test that clear_cache actually clears the cache (not a no-op)
    /// Kills mutation: replace clear_cache with ()
    #[test]
    fn test_clear_cache_actually_clears() {
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        // Create a temporary parquet file
        let schema = create_test_schema();
        let batch = create_test_batch(&schema, vec![1, 2, 3, 4, 5]);
        
        let mut temp_file = NamedTempFile::with_suffix(".parquet").unwrap();
        {
            use parquet::arrow::ArrowWriter;
            let mut writer = ArrowWriter::try_new(temp_file.as_file_mut(), schema.clone(), None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }
        
        let loader = DataLoader::with_defaults(temp_file.path().to_str().unwrap());
        
        // Load to populate cache
        let result = loader.load();
        assert!(result.is_ok(), "Should load parquet file");
        
        // Verify cache is populated
        {
            let cache = loader.cached_batches.read();
            assert!(cache.is_some(), "Cache should be populated after load");
        }
        
        // Clear the cache
        loader.clear_cache();
        
        // Verify cache is now empty
        {
            let cache = loader.cached_batches.read();
            assert!(cache.is_none(), 
                "clear_cache must actually clear the cache, not be a no-op");
        }
    }
    
    /// Test loading a real parquet file through load_file match arm
    /// Kills mutation: delete match arm FileFormat::Parquet
    #[test]
    fn test_load_parquet_file() {
        use tempfile::NamedTempFile;
        
        let schema = create_test_schema();
        let batch = create_test_batch(&schema, vec![10, 20, 30]);
        
        let mut temp_file = NamedTempFile::with_suffix(".parquet").unwrap();
        {
            use parquet::arrow::ArrowWriter;
            let mut writer = ArrowWriter::try_new(temp_file.as_file_mut(), schema.clone(), None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }
        
        let loader = DataLoader::with_defaults(temp_file.path().to_str().unwrap());
        let result = loader.load();
        
        assert!(result.is_ok(), "Must be able to load parquet files");
        let iter = result.unwrap();
        assert_eq!(iter.total_rows(), 3, "Should have 3 rows from parquet");
    }
    
    /// Test loading a real CSV file through load_file match arm
    /// Kills mutation: delete match arm FileFormat::Csv
    #[test]
    fn test_load_csv_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        let mut temp_file = NamedTempFile::with_suffix(".csv").unwrap();
        writeln!(temp_file, "id,value").unwrap();
        writeln!(temp_file, "1,100").unwrap();
        writeln!(temp_file, "2,200").unwrap();
        writeln!(temp_file, "3,300").unwrap();
        temp_file.flush().unwrap();
        
        let loader = DataLoader::with_defaults(temp_file.path().to_str().unwrap());
        let result = loader.load();
        
        assert!(result.is_ok(), "Must be able to load CSV files");
        let iter = result.unwrap();
        assert_eq!(iter.total_rows(), 3, "Should have 3 rows from CSV");
    }
    
    /// Test loading Arrow IPC file through load_file match arm  
    /// Kills mutation: delete match arm FileFormat::ArrowIpc
    #[test]
    fn test_load_arrow_ipc_file() {
        use tempfile::NamedTempFile;
        use arrow::ipc::writer::FileWriter;
        
        let schema = create_test_schema();
        let batch = create_test_batch(&schema, vec![100, 200]);
        
        let temp_file = NamedTempFile::with_suffix(".arrow").unwrap();
        {
            let mut writer = FileWriter::try_new(temp_file.as_file(), &schema).unwrap();
            writer.write(&batch).unwrap();
            writer.finish().unwrap();
        }
        
        let loader = DataLoader::with_defaults(temp_file.path().to_str().unwrap());
        let result = loader.load();
        
        assert!(result.is_ok(), "Must be able to load Arrow IPC files");
        let iter = result.unwrap();
        assert_eq!(iter.total_rows(), 2, "Should have 2 rows from Arrow IPC");
    }
    
    /// Test cache is populated and subsequent loads use cache
    /// Kills mutations: cache size comparisons
    #[test]
    fn test_cache_is_populated_on_small_data() {
        use tempfile::NamedTempFile;
        
        let schema = create_test_schema();
        let batch = create_test_batch(&schema, vec![1, 2, 3]);
        
        let mut temp_file = NamedTempFile::with_suffix(".parquet").unwrap();
        {
            use parquet::arrow::ArrowWriter;
            let mut writer = ArrowWriter::try_new(temp_file.as_file_mut(), schema.clone(), None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }
        
        let loader = DataLoader::with_defaults(temp_file.path().to_str().unwrap());
        
        // First load
        let result1 = loader.load();
        assert!(result1.is_ok());
        
        // Verify cache populated (small data < 100MB)
        {
            let cache = loader.cached_batches.read();
            assert!(cache.is_some(), "Small data should be cached");
            let batches = cache.as_ref().unwrap();
            assert!(!batches.is_empty(), "Cached batches should not be empty");
        }
        
        // Second load should use cache (test this indirectly)
        let result2 = loader.load();
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap().total_rows(), 3);
    }
    
    /// Test FileFormat detection for all supported formats
    /// Strengthens match arm testing
    #[test]
    fn test_file_format_all_variants() {
        // Parquet variants
        assert_eq!(FileFormat::from_extension("data.parquet"), FileFormat::Parquet);
        assert_eq!(FileFormat::from_extension("path/to/file.pq"), FileFormat::Parquet);
        
        // CSV variants
        assert_eq!(FileFormat::from_extension("data.csv"), FileFormat::Csv);
        assert_eq!(FileFormat::from_extension("data.tsv"), FileFormat::Csv);
        
        // Arrow IPC variants
        assert_eq!(FileFormat::from_extension("data.arrow"), FileFormat::ArrowIpc);
        assert_eq!(FileFormat::from_extension("data.feather"), FileFormat::ArrowIpc);
        
        // JSON Lines
        assert_eq!(FileFormat::from_extension("data.jsonl"), FileFormat::JsonLines);
        assert_eq!(FileFormat::from_extension("data.ndjson"), FileFormat::JsonLines);
        
        // Unknown
        assert_eq!(FileFormat::from_extension("data.txt"), FileFormat::Unknown);
        assert_eq!(FileFormat::from_extension("no_extension"), FileFormat::Unknown);
    }
    
    /// Test that load returns error for unsupported format
    /// Verifies the _ match arm in load_file
    #[test]
    fn test_load_file_unsupported_returns_error() {
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        // Create a file with unknown extension
        let mut temp_file = NamedTempFile::with_suffix(".xyz").unwrap();
        writeln!(temp_file, "some data").unwrap();
        temp_file.flush().unwrap();
        
        let loader = DataLoader::with_defaults(temp_file.path().to_str().unwrap());
        let result = loader.load();
        
        assert!(result.is_err(), "Unknown format should return error");
        match result {
            Err(DataLoaderError::UnsupportedFormat(_)) => {}
            _ => panic!("Expected UnsupportedFormat error"),
        }
    }
    
    /// Test config() returns the correct configuration
    #[test] 
    fn test_config_returns_correct_values() {
        let config = LoaderConfig {
            batch_size: 512,
            prefetch_count: 2,
            num_workers: 8,
            memory_map: false,
            io_buffer_size: 4 * 1024 * 1024,
        };
        let loader = DataLoader::new(DataSource::File("test.parquet".to_string()), config);
        
        let c = loader.config();
        assert_eq!(c.batch_size, 512);
        assert_eq!(c.prefetch_count, 2);
        assert_eq!(c.num_workers, 8);
        assert!(!c.memory_map);
        assert_eq!(c.io_buffer_size, 4 * 1024 * 1024);
    }
}
