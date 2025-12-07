//! Health Check and Readiness Probe System
//!
//! Provides liveness and readiness checks for Kubernetes/container deployments.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Service is healthy
    Healthy,
    /// Service is degraded but operational
    Degraded,
    /// Service is unhealthy
    Unhealthy,
    /// Service is starting up
    Starting,
    /// Service is shutting down
    ShuttingDown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Starting => write!(f, "starting"),
            Self::ShuttingDown => write!(f, "shutting_down"),
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Component name
    pub component: String,
    /// Status
    pub status: HealthStatus,
    /// Message
    pub message: Option<String>,
    /// Latency in microseconds
    pub latency_us: u64,
    /// Last check timestamp (Unix millis)
    pub last_check: u64,
}

/// Readiness check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessResult {
    /// Overall ready status
    pub ready: bool,
    /// Individual check results
    pub checks: Vec<HealthCheckResult>,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Version
    pub version: String,
}

/// Health check function type
pub type HealthCheckFn = Box<dyn Fn() -> HealthCheckResult + Send + Sync>;

/// Health check manager
pub struct HealthManager {
    start_time: Instant,
    status: AtomicU8Wrapper,
    checks: RwLock<Vec<(String, HealthCheckFn)>>,
    ready: AtomicBool,
    last_check: AtomicU64,
}

// Wrapper for AtomicU8 since HealthStatus needs atomic operations
struct AtomicU8Wrapper(std::sync::atomic::AtomicU8);

impl AtomicU8Wrapper {
    fn new(status: HealthStatus) -> Self {
        Self(std::sync::atomic::AtomicU8::new(status as u8))
    }
    
    fn load(&self) -> HealthStatus {
        match self.0.load(Ordering::SeqCst) {
            0 => HealthStatus::Healthy,
            1 => HealthStatus::Degraded,
            2 => HealthStatus::Unhealthy,
            3 => HealthStatus::Starting,
            _ => HealthStatus::ShuttingDown,
        }
    }
    
    fn store(&self, status: HealthStatus) {
        self.0.store(status as u8, Ordering::SeqCst);
    }
}

impl HealthManager {
    /// Create a new health manager
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            status: AtomicU8Wrapper::new(HealthStatus::Starting),
            checks: RwLock::new(Vec::new()),
            ready: AtomicBool::new(false),
            last_check: AtomicU64::new(0),
        }
    }
    
    /// Register a health check
    pub fn register_check<F>(&self, name: &str, check: F)
    where
        F: Fn() -> HealthCheckResult + Send + Sync + 'static,
    {
        let mut checks = self.checks.write();
        checks.push((name.to_string(), Box::new(check)));
    }
    
    /// Mark service as ready
    pub fn set_ready(&self) {
        self.ready.store(true, Ordering::SeqCst);
        self.status.store(HealthStatus::Healthy);
    }
    
    /// Mark service as not ready
    pub fn set_not_ready(&self) {
        self.ready.store(false, Ordering::SeqCst);
    }
    
    /// Mark service as shutting down
    pub fn set_shutting_down(&self) {
        self.status.store(HealthStatus::ShuttingDown);
        self.ready.store(false, Ordering::SeqCst);
    }
    
    /// Get current status
    pub fn status(&self) -> HealthStatus {
        self.status.load()
    }
    
    /// Check if ready
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }
    
    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Run all health checks
    pub fn check_health(&self) -> ReadinessResult {
        let checks = self.checks.read();
        let mut results = Vec::with_capacity(checks.len());
        let mut all_healthy = true;
        
        for (_name, check_fn) in checks.iter() {
            let start = Instant::now();
            let mut result = check_fn();
            result.latency_us = start.elapsed().as_micros() as u64;
            result.last_check = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            if result.status != HealthStatus::Healthy {
                all_healthy = false;
            }
            
            results.push(result);
        }
        
        // Update overall status
        if all_healthy && self.is_ready() {
            self.status.store(HealthStatus::Healthy);
        } else if self.is_ready() {
            self.status.store(HealthStatus::Degraded);
        }
        
        self.last_check.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            Ordering::SeqCst,
        );
        
        ReadinessResult {
            ready: self.is_ready() && all_healthy,
            checks: results,
            uptime_secs: self.uptime().as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
    
    /// Liveness check (simple alive check)
    pub fn liveness(&self) -> bool {
        self.status.load() != HealthStatus::ShuttingDown
    }
}

impl Default for HealthManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a memory health check
pub fn memory_health_check(threshold_percent: f64) -> impl Fn() -> HealthCheckResult + Send + Sync {
    move || {
        let sys = sysinfo::System::new_all();
        let total = sys.total_memory();
        let used = total - sys.available_memory();
        let percent = (used as f64 / total as f64) * 100.0;
        
        let status = if percent > threshold_percent {
            HealthStatus::Unhealthy
        } else if percent > threshold_percent * 0.8 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        
        HealthCheckResult {
            component: "memory".to_string(),
            status,
            message: Some(format!("{:.1}% used", percent)),
            latency_us: 0,
            last_check: 0,
        }
    }
}

/// Create a disk health check
pub fn disk_health_check(path: &str, threshold_percent: f64) -> impl Fn() -> HealthCheckResult + Send + Sync {
    let path = path.to_string();
    move || {
        use sysinfo::Disks;
        let disks = Disks::new_with_refreshed_list();
        
        for disk in disks.list() {
            if disk.mount_point().to_string_lossy().starts_with(&path) {
                let total = disk.total_space();
                let available = disk.available_space();
                let used_percent = ((total - available) as f64 / total as f64) * 100.0;
                
                let status = if used_percent > threshold_percent {
                    HealthStatus::Unhealthy
                } else if used_percent > threshold_percent * 0.8 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                };
                
                return HealthCheckResult {
                    component: format!("disk:{}", path),
                    status,
                    message: Some(format!("{:.1}% used", used_percent)),
                    latency_us: 0,
                    last_check: 0,
                };
            }
        }
        
        HealthCheckResult {
            component: format!("disk:{}", path),
            status: HealthStatus::Healthy,
            message: Some("Path not found".to_string()),
            latency_us: 0,
            last_check: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_health_manager() {
        let manager = HealthManager::new();
        
        assert_eq!(manager.status(), HealthStatus::Starting);
        assert!(!manager.is_ready());
        
        manager.set_ready();
        assert!(manager.is_ready());
        assert_eq!(manager.status(), HealthStatus::Healthy);
        
        manager.set_shutting_down();
        assert!(!manager.is_ready());
        assert!(!manager.liveness());
    }
    
    #[test]
    fn test_health_check() {
        let manager = HealthManager::new();
        
        manager.register_check("test", || HealthCheckResult {
            component: "test".to_string(),
            status: HealthStatus::Healthy,
            message: None,
            latency_us: 0,
            last_check: 0,
        });
        
        manager.set_ready();
        let result = manager.check_health();
        
        assert!(result.ready);
        assert_eq!(result.checks.len(), 1);
    }
}
