//! S3 Object Storage Adapter
//!
//! Support for reading data from AWS S3 and compatible object stores.
//! 
//! # Features
//! 
//! Enable the `aws_s3` feature to use real AWS SDK:
//! ```toml
//! zenith-runtime-cpu = { version = "0.3", features = ["aws_s3"] }
//! ```

/// S3 configuration
#[derive(Debug, Clone)]
pub struct S3Config {
    /// S3 bucket name
    pub bucket: String,
    /// AWS region
    pub region: String,
    /// Optional endpoint URL (for MinIO, LocalStack, etc.)
    pub endpoint: Option<String>,
    /// Use path-style addressing
    pub path_style: bool,
    /// Maximum concurrent requests
    pub max_connections: usize,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            region: "us-east-1".to_string(),
            endpoint: None,
            path_style: false,
            max_connections: 8,
            timeout_secs: 30,
        }
    }
}

impl S3Config {
    /// Create config for a bucket
    pub fn new(bucket: &str, region: &str) -> Self {
        Self {
            bucket: bucket.to_string(),
            region: region.to_string(),
            ..Default::default()
        }
    }
    
    /// Use custom endpoint (MinIO, LocalStack)
    pub fn with_endpoint(mut self, endpoint: &str) -> Self {
        self.endpoint = Some(endpoint.to_string());
        self
    }
    
    /// Enable path-style addressing (required for MinIO)
    pub fn with_path_style(mut self, path_style: bool) -> Self {
        self.path_style = path_style;
        self
    }
}

/// S3 object reference
#[derive(Debug, Clone)]
pub struct S3Object {
    /// Object key (path in bucket)
    pub key: String,
    /// Object size in bytes
    pub size: u64,
    /// ETag (for caching/validation)
    pub etag: Option<String>,
    /// Last modified timestamp
    pub last_modified: Option<String>,
}

/// S3 error types
#[derive(Debug)]
pub enum S3Error {
    /// Feature not enabled
    NotEnabled(String),
    /// Connection error
    Connection(String),
    /// Object not found
    NotFound(String),
    /// Access denied
    AccessDenied(String),
    /// General S3 error
    Other(String),
}

impl std::fmt::Display for S3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotEnabled(msg) => write!(f, "Feature not enabled: {}", msg),
            Self::Connection(msg) => write!(f, "Connection error: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::AccessDenied(msg) => write!(f, "Access denied: {}", msg),
            Self::Other(msg) => write!(f, "S3 error: {}", msg),
        }
    }
}

impl std::error::Error for S3Error {}

// ============================================================================
// AWS SDK Implementation (when aws_s3 feature is enabled)
// ============================================================================

#[cfg(feature = "aws_s3")]
mod aws_impl {
    use super::*;
    use aws_sdk_s3::Client;
    use aws_sdk_s3::config::{Region, Builder};
    
    /// S3 adapter with real AWS SDK
    pub struct S3Adapter {
        client: Client,
        config: S3Config,
    }
    
    impl S3Adapter {
        /// Create new S3 adapter with AWS SDK
        pub async fn new(config: S3Config) -> Result<Self, S3Error> {
            let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(Region::new(config.region.clone()))
                .load()
                .await;
            
            let mut s3_config_builder = Builder::from(&sdk_config);
            
            if let Some(endpoint) = &config.endpoint {
                s3_config_builder = s3_config_builder.endpoint_url(endpoint);
            }
            
            if config.path_style {
                s3_config_builder = s3_config_builder.force_path_style(true);
            }
            
            let client = Client::from_conf(s3_config_builder.build());
            
            Ok(Self { client, config })
        }
        
        /// List objects with prefix (Issue #41)
        pub async fn list_objects(&self, prefix: &str) -> Result<Vec<S3Object>, S3Error> {
            let resp = self.client
                .list_objects_v2()
                .bucket(&self.config.bucket)
                .prefix(prefix)
                .send()
                .await
                .map_err(|e| S3Error::Connection(e.to_string()))?;
            
            let objects: Vec<S3Object> = resp.contents()
                .iter()
                .map(|obj| S3Object {
                    key: obj.key().unwrap_or_default().to_string(),
                    size: obj.size().unwrap_or(0) as u64,
                    etag: obj.e_tag().map(|s| s.to_string()),
                    last_modified: obj.last_modified().map(|dt| dt.to_string()),
                })
                .collect();
            
            Ok(objects)
        }
        
        /// Read object contents (Issue #42)
        pub async fn read_object(&self, key: &str) -> Result<Vec<u8>, S3Error> {
            let resp = self.client
                .get_object()
                .bucket(&self.config.bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| {
                    let err_str = e.to_string();
                    if err_str.contains("NoSuchKey") {
                        S3Error::NotFound(key.to_string())
                    } else if err_str.contains("AccessDenied") {
                        S3Error::AccessDenied(key.to_string())
                    } else {
                        S3Error::Connection(err_str)
                    }
                })?;
            
            let data = resp.body
                .collect()
                .await
                .map_err(|e| S3Error::Connection(e.to_string()))?
                .into_bytes()
                .to_vec();
            
            Ok(data)
        }
        
        /// Stream object in chunks (Issue #43)
        pub async fn stream_object<F>(
            &self,
            key: &str,
            chunk_size: usize,
            mut callback: F,
        ) -> Result<u64, S3Error>
        where
            F: FnMut(&[u8]) -> bool,
        {
            let resp = self.client
                .get_object()
                .bucket(&self.config.bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| S3Error::Connection(e.to_string()))?;
            
            let mut total_bytes = 0u64;
            let mut body = resp.body;
            let mut buffer = Vec::with_capacity(chunk_size);
            
            while let Some(chunk) = body
                .try_next()
                .await
                .map_err(|e| S3Error::Connection(e.to_string()))?
            {
                buffer.extend_from_slice(&chunk);
                total_bytes += chunk.len() as u64;
                
                // Process in chunks
                while buffer.len() >= chunk_size {
                    let chunk_data: Vec<u8> = buffer.drain(..chunk_size).collect();
                    if !callback(&chunk_data) {
                        return Ok(total_bytes);
                    }
                }
            }
            
            // Process remaining data
            if !buffer.is_empty() {
                callback(&buffer);
            }
            
            Ok(total_bytes)
        }
        
        /// Check if object exists (Issue #44)
        pub async fn object_exists(&self, key: &str) -> Result<bool, S3Error> {
            match self.client
                .head_object()
                .bucket(&self.config.bucket)
                .key(key)
                .send()
                .await
            {
                Ok(_) => Ok(true),
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("NotFound") || err_str.contains("NoSuchKey") {
                        Ok(false)
                    } else {
                        Err(S3Error::Connection(err_str))
                    }
                }
            }
        }
        
        /// Get object metadata
        pub async fn get_object_info(&self, key: &str) -> Result<S3Object, S3Error> {
            let resp = self.client
                .head_object()
                .bucket(&self.config.bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| S3Error::Connection(e.to_string()))?;
            
            Ok(S3Object {
                key: key.to_string(),
                size: resp.content_length().unwrap_or(0) as u64,
                etag: resp.e_tag().map(|s| s.to_string()),
                last_modified: resp.last_modified().map(|dt| dt.to_string()),
            })
        }
        
        /// Get bucket name
        pub fn bucket(&self) -> &str {
            &self.config.bucket
        }
        
        /// Get region
        pub fn region(&self) -> &str {
            &self.config.region
        }
    }
}

#[cfg(feature = "aws_s3")]
pub use aws_impl::S3Adapter;

// ============================================================================
// Stub Implementation (when aws_s3 feature is NOT enabled)
// ============================================================================

#[cfg(not(feature = "aws_s3"))]
mod stub_impl {
    use super::*;
    
    /// S3 adapter stub (enable aws_s3 feature for real implementation)
    #[derive(Debug)]
    pub struct S3Adapter {
        config: S3Config,
    }
    
    impl S3Adapter {
        /// Create new S3 adapter (stub)
        pub fn new(config: S3Config) -> Self {
            Self { config }
        }
        
        /// List objects - requires aws_s3 feature
        pub fn list_objects(&self, _prefix: &str) -> Result<Vec<S3Object>, S3Error> {
            Err(S3Error::NotEnabled(
                "Enable the 'aws_s3' feature to use S3. \
                 Add `features = [\"aws_s3\"]` to your Cargo.toml.".to_string()
            ))
        }
        
        /// Read object - requires aws_s3 feature
        pub fn read_object(&self, _key: &str) -> Result<Vec<u8>, S3Error> {
            Err(S3Error::NotEnabled(
                "Enable the 'aws_s3' feature to use S3.".to_string()
            ))
        }
        
        /// Stream object - requires aws_s3 feature
        pub fn stream_object(
            &self,
            _key: &str,
            _chunk_size: usize,
        ) -> Result<(), S3Error> {
            Err(S3Error::NotEnabled(
                "Enable the 'aws_s3' feature to use S3.".to_string()
            ))
        }
        
        /// Check if object exists - requires aws_s3 feature
        pub fn object_exists(&self, _key: &str) -> Result<bool, S3Error> {
            Err(S3Error::NotEnabled(
                "Enable the 'aws_s3' feature to use S3.".to_string()
            ))
        }
        
        /// Get bucket name
        pub fn bucket(&self) -> &str {
            &self.config.bucket
        }
        
        /// Get region
        pub fn region(&self) -> &str {
            &self.config.region
        }
    }
}

#[cfg(not(feature = "aws_s3"))]
pub use stub_impl::S3Adapter;

// ============================================================================
// Helper functions
// ============================================================================

/// Helper to parse S3 URI (s3://bucket/key)
pub fn parse_s3_uri(uri: &str) -> Option<(String, String)> {
    if !uri.starts_with("s3://") {
        return None;
    }
    
    let path = &uri[5..]; // Remove "s3://"
    let mut parts = path.splitn(2, '/');
    
    let bucket = parts.next()?.to_string();
    let key = parts.next().unwrap_or("").to_string();
    
    Some((bucket, key))
}

/// Check if path is an S3 URI
pub fn is_s3_path(path: &str) -> bool {
    path.starts_with("s3://") || path.starts_with("s3a://")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_s3_uri() {
        let (bucket, key) = parse_s3_uri("s3://my-bucket/path/to/file.parquet").unwrap();
        assert_eq!(bucket, "my-bucket");
        assert_eq!(key, "path/to/file.parquet");
        
        let (bucket, key) = parse_s3_uri("s3://bucket/").unwrap();
        assert_eq!(bucket, "bucket");
        assert_eq!(key, "");
        
        assert!(parse_s3_uri("http://example.com").is_none());
    }
    
    #[test]
    fn test_is_s3_path() {
        assert!(is_s3_path("s3://bucket/key"));
        assert!(is_s3_path("s3a://bucket/key"));
        assert!(!is_s3_path("/local/path"));
        assert!(!is_s3_path("http://example.com"));
    }
    
    #[test]
    fn test_s3_config() {
        let config = S3Config::new("my-bucket", "us-west-2")
            .with_endpoint("http://localhost:9000")
            .with_path_style(true);
        
        assert_eq!(config.bucket, "my-bucket");
        assert_eq!(config.region, "us-west-2");
        assert_eq!(config.endpoint, Some("http://localhost:9000".to_string()));
        assert!(config.path_style);
    }
    
    #[test]
    #[cfg(not(feature = "aws_s3"))]
    fn test_stub_returns_not_enabled() {
        let adapter = S3Adapter::new(S3Config::new("bucket", "us-east-1"));
        
        assert!(matches!(
            adapter.list_objects("prefix"),
            Err(S3Error::NotEnabled(_))
        ));
        
        assert!(matches!(
            adapter.read_object("key"),
            Err(S3Error::NotEnabled(_))
        ));
    }
}
