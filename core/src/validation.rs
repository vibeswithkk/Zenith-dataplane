//! Input Validation Module
//!
//! Provides validation utilities for sanitizing and validating input
//! at API boundaries to prevent security vulnerabilities.

use std::collections::HashSet;

/// Maximum allowed string length for user inputs
pub const MAX_STRING_LENGTH: usize = 10_000;
/// Maximum allowed job name length
pub const MAX_JOB_NAME_LENGTH: usize = 256;
/// Maximum allowed path length
pub const MAX_PATH_LENGTH: usize = 4096;
/// Maximum allowed command length
pub const MAX_COMMAND_LENGTH: usize = 65536;
/// Maximum number of environment variables
pub const MAX_ENV_VARS: usize = 1000;
/// Maximum number of arguments
pub const MAX_ARGUMENTS: usize = 1000;

/// Validation error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Input is empty when it shouldn't be
    Empty(String),
    /// Input exceeds maximum length
    TooLong { field: String, max: usize, actual: usize },
    /// Input contains invalid characters
    InvalidChars { field: String, invalid: String },
    /// Input contains forbidden patterns
    ForbiddenPattern { field: String, pattern: String },
    /// Input is out of valid range
    OutOfRange { field: String, min: i64, max: i64, actual: i64 },
    /// Generic validation failure
    Invalid(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty(field) => write!(f, "{} cannot be empty", field),
            Self::TooLong { field, max, actual } => {
                write!(f, "{} too long: {} > {} max", field, actual, max)
            }
            Self::InvalidChars { field, invalid } => {
                write!(f, "{} contains invalid characters: {}", field, invalid)
            }
            Self::ForbiddenPattern { field, pattern } => {
                write!(f, "{} contains forbidden pattern: {}", field, pattern)
            }
            Self::OutOfRange { field, min, max, actual } => {
                write!(f, "{} out of range: {} not in [{}, {}]", field, actual, min, max)
            }
            Self::Invalid(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Input validator with configurable rules
pub struct Validator {
    /// Forbidden command patterns (for security)
    forbidden_patterns: HashSet<String>,
}

impl Default for Validator {
    fn default() -> Self {
        let mut forbidden_patterns = HashSet::new();
        // Prevent shell injection
        forbidden_patterns.insert("$((".to_string());
        forbidden_patterns.insert("$(".to_string());
        forbidden_patterns.insert("`".to_string());
        forbidden_patterns.insert("&&".to_string());
        forbidden_patterns.insert("||".to_string());
        forbidden_patterns.insert(";".to_string());
        forbidden_patterns.insert("|".to_string());
        forbidden_patterns.insert(">".to_string());
        forbidden_patterns.insert("<".to_string());
        forbidden_patterns.insert("..".to_string());  // Path traversal
        
        Self { forbidden_patterns }
    }
}

impl Validator {
    /// Create a new validator with default rules
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Validate a string is not empty
    pub fn require_non_empty(&self, field: &str, value: &str) -> ValidationResult<()> {
        if value.trim().is_empty() {
            Err(ValidationError::Empty(field.to_string()))
        } else {
            Ok(())
        }
    }
    
    /// Validate string length
    pub fn validate_length(&self, field: &str, value: &str, max: usize) -> ValidationResult<()> {
        if value.len() > max {
            Err(ValidationError::TooLong {
                field: field.to_string(),
                max,
                actual: value.len(),
            })
        } else {
            Ok(())
        }
    }
    
    /// Validate a job name (alphanumeric, dashes, underscores)
    pub fn validate_job_name(&self, name: &str) -> ValidationResult<()> {
        self.require_non_empty("job_name", name)?;
        self.validate_length("job_name", name, MAX_JOB_NAME_LENGTH)?;
        
        let invalid: String = name.chars()
            .filter(|c| !c.is_alphanumeric() && *c != '-' && *c != '_')
            .collect();
        
        if !invalid.is_empty() {
            Err(ValidationError::InvalidChars {
                field: "job_name".to_string(),
                invalid,
            })
        } else {
            Ok(())
        }
    }
    
    /// Validate a path (no traversal attacks)
    pub fn validate_path(&self, path: &str) -> ValidationResult<()> {
        self.validate_length("path", path, MAX_PATH_LENGTH)?;
        
        // Check for path traversal
        if path.contains("..") {
            return Err(ValidationError::ForbiddenPattern {
                field: "path".to_string(),
                pattern: "..".to_string(),
            });
        }
        
        // Check for null bytes
        if path.contains('\0') {
            return Err(ValidationError::InvalidChars {
                field: "path".to_string(),
                invalid: "null byte".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Validate a command (check for injection patterns)
    pub fn validate_command(&self, command: &str) -> ValidationResult<()> {
        self.require_non_empty("command", command)?;
        self.validate_length("command", command, MAX_COMMAND_LENGTH)?;
        
        for pattern in &self.forbidden_patterns {
            if command.contains(pattern) {
                return Err(ValidationError::ForbiddenPattern {
                    field: "command".to_string(),
                    pattern: pattern.clone(),
                });
            }
        }
        
        Ok(())
    }
    
    /// Validate a numeric value is in range
    pub fn validate_range(&self, field: &str, value: i64, min: i64, max: i64) -> ValidationResult<()> {
        if value < min || value > max {
            Err(ValidationError::OutOfRange {
                field: field.to_string(),
                min,
                max,
                actual: value,
            })
        } else {
            Ok(())
        }
    }
    
    /// Validate GPU count
    pub fn validate_gpu_count(&self, count: u32) -> ValidationResult<()> {
        self.validate_range("gpu_count", count as i64, 0, 1024)
    }
    
    /// Validate priority
    pub fn validate_priority(&self, priority: i32) -> ValidationResult<()> {
        self.validate_range("priority", priority as i64, -1000, 1000)
    }
    
    /// Validate buffer size
    pub fn validate_buffer_size(&self, size: usize) -> ValidationResult<()> {
        self.validate_range("buffer_size", size as i64, 1, 1024 * 1024 * 1024)  // 1GB max
    }
}

/// Sanitize a string by removing control characters
pub fn sanitize_string(input: &str) -> String {
    input.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

/// Sanitize a log message
pub fn sanitize_log_message(message: &str) -> String {
    let sanitized = sanitize_string(message);
    if sanitized.len() > MAX_STRING_LENGTH {
        format!("{}... [truncated]", &sanitized[..MAX_STRING_LENGTH])
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_job_name() {
        let v = Validator::new();
        
        assert!(v.validate_job_name("my-job-123").is_ok());
        assert!(v.validate_job_name("test_job").is_ok());
        assert!(v.validate_job_name("").is_err());
        assert!(v.validate_job_name("my job").is_err());  // space not allowed
        assert!(v.validate_job_name("job;rm -rf").is_err());  // injection attempt
    }
    
    #[test]
    fn test_validate_path() {
        let v = Validator::new();
        
        assert!(v.validate_path("/home/user/data").is_ok());
        assert!(v.validate_path("../../../etc/passwd").is_err());
        assert!(v.validate_path("/path/with\0null").is_err());
    }
    
    #[test]
    fn test_validate_command() {
        let v = Validator::new();
        
        // Valid commands
        assert!(v.validate_command("python train.py").is_ok());
        assert!(v.validate_command("python").is_ok());
        assert!(v.validate_command("python3 -m pytest").is_ok());
        
        // Invalid - shell injection patterns
        assert!(v.validate_command("$(cat /etc/passwd)").is_err());
        assert!(v.validate_command("echo `whoami`").is_err());
        assert!(v.validate_command("cmd1 && cmd2").is_err());
        assert!(v.validate_command("cmd1 || cmd2").is_err());
        assert!(v.validate_command("cmd ; rm -rf /").is_err());
        assert!(v.validate_command("cat file | grep secret").is_err());
        assert!(v.validate_command("echo > /etc/passwd").is_err());
    }
    
    #[test]
    fn test_validate_range() {
        let v = Validator::new();
        
        assert!(v.validate_gpu_count(0).is_ok());
        assert!(v.validate_gpu_count(8).is_ok());
        assert!(v.validate_priority(0).is_ok());
        assert!(v.validate_priority(-100).is_ok());
    }
    
    #[test]
    fn test_sanitize() {
        assert_eq!(sanitize_string("hello\x00world"), "helloworld");
        assert_eq!(sanitize_string("line1\nline2"), "line1\nline2");
    }
}
