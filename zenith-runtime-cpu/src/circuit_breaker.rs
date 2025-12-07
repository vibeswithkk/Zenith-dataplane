//! Circuit Breaker Pattern Implementation
//!
//! Provides fault tolerance through circuit breaker pattern.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use parking_lot::RwLock;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed (normal operation)
    Closed,
    /// Circuit is open (failing, rejecting calls)
    Open,
    /// Circuit is half-open (testing if service recovered)
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Duration to wait before trying half-open
    pub reset_timeout: Duration,
    /// Number of successes in half-open before closing
    pub success_threshold: u32,
    /// Timeout for individual calls
    pub call_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            success_threshold: 3,
            call_timeout: Duration::from_secs(10),
        }
    }
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: RwLock<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: RwLock<Option<Instant>>,
    total_calls: AtomicU64,
    total_failures: AtomicU64,
    total_successes: AtomicU64,
    total_rejections: AtomicU64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: RwLock::new(None),
            total_calls: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            total_successes: AtomicU64::new(0),
            total_rejections: AtomicU64::new(0),
        }
    }
    
    /// Get current state
    pub fn state(&self) -> CircuitState {
        *self.state.read()
    }
    
    /// Check if calls are allowed
    pub fn is_allowed(&self) -> bool {
        let state = *self.state.read();
        
        match state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true, // Allow limited calls
            CircuitState::Open => {
                // Check if we should try half-open
                if let Some(last_failure) = *self.last_failure_time.read() {
                    if last_failure.elapsed() >= self.config.reset_timeout {
                        *self.state.write() = CircuitState::HalfOpen;
                        self.success_count.store(0, Ordering::SeqCst);
                        return true;
                    }
                }
                false
            }
        }
    }
    
    /// Execute a function through the circuit breaker
    pub fn call<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Result<T, E>,
    {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        
        if !self.is_allowed() {
            self.total_rejections.fetch_add(1, Ordering::Relaxed);
            return Err(CircuitBreakerError::CircuitOpen);
        }
        
        match f() {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(e) => {
                self.on_failure();
                Err(CircuitBreakerError::CallFailed(e))
            }
        }
    }
    
    /// Record a success
    pub fn on_success(&self) {
        self.total_successes.fetch_add(1, Ordering::Relaxed);
        
        let state = *self.state.read();
        
        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= self.config.success_threshold {
                    // Enough successes, close the circuit
                    *self.state.write() = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                }
            }
            CircuitState::Open => {}
        }
    }
    
    /// Record a failure
    pub fn on_failure(&self) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        *self.last_failure_time.write() = Some(Instant::now());
        
        let state = *self.state.read();
        
        match state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= self.config.failure_threshold {
                    // Too many failures, open the circuit
                    *self.state.write() = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                // Failure in half-open, go back to open
                *self.state.write() = CircuitState::Open;
                self.success_count.store(0, Ordering::SeqCst);
            }
            CircuitState::Open => {}
        }
    }
    
    /// Force reset the circuit breaker
    pub fn reset(&self) {
        *self.state.write() = CircuitState::Closed;
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        *self.last_failure_time.write() = None;
    }
    
    /// Get statistics
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            state: *self.state.read(),
            total_calls: self.total_calls.load(Ordering::Relaxed),
            total_successes: self.total_successes.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            total_rejections: self.total_rejections.load(Ordering::Relaxed),
            current_failure_count: self.failure_count.load(Ordering::Relaxed),
        }
    }
}

/// Circuit breaker error
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, call was rejected
    CircuitOpen,
    /// Call failed with underlying error
    CallFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CircuitOpen => write!(f, "Circuit breaker is open"),
            Self::CallFailed(e) => write!(f, "Call failed: {}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CircuitOpen => None,
            Self::CallFailed(e) => Some(e),
        }
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// Current state
    pub state: CircuitState,
    /// Total calls attempted
    pub total_calls: u64,
    /// Total successful calls
    pub total_successes: u64,
    /// Total failed calls
    pub total_failures: u64,
    /// Total rejected calls (circuit open)
    pub total_rejections: u64,
    /// Current failure count
    pub current_failure_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_circuit_breaker_normal() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        });
        
        // Normal operation
        let result = cb.call(|| Ok::<i32, &str>(42));
        assert!(result.is_ok());
        assert_eq!(cb.state(), CircuitState::Closed);
    }
    
    #[test]
    fn test_circuit_breaker_opens() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        });
        
        // Cause failures
        for _ in 0..3 {
            let _ = cb.call(|| Err::<i32, &str>("error"));
        }
        
        assert_eq!(cb.state(), CircuitState::Open);
        
        // Next call should be rejected
        let result = cb.call(|| Ok::<i32, &str>(42));
        assert!(matches!(result, Err(CircuitBreakerError::CircuitOpen)));
    }
    
    #[test]
    fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        
        // Force some state
        for _ in 0..5 {
            let _ = cb.call(|| Err::<i32, &str>("error"));
        }
        
        assert_eq!(cb.state(), CircuitState::Open);
        
        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
    }
}
