//! Circuit breaker implementation for preventing cascading failures
//!
//! This module implements the circuit breaker pattern to prevent
//! overloading services that are failing or experiencing issues.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use crate::error::{Result, ServiceError};

use super::CircuitBreakerStatus;

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before the circuit opens
    pub failure_threshold: usize,
    
    /// Recovery timeout before allowing test requests
    pub reset_timeout: Duration,
    
    /// Number of successful test requests needed to close the circuit
    pub success_threshold: usize,
    
    /// Size of the sliding window for tracking error rates, if used
    pub sliding_window_size: usize,
    
    /// Error threshold rate to open the circuit (0.0-1.0)
    pub error_threshold_percentage: f64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            success_threshold: 2,
            sliding_window_size: 100,
            error_threshold_percentage: 0.5, // 50% error rate
        }
    }
}

/// A thread-safe circuit breaker implementation
pub struct CircuitBreaker {
    /// Current circuit status (atomic for thread safety)
    status: RwLock<CircuitBreakerStatus>,
    
    /// Time when the circuit was opened
    opened_at: RwLock<Option<Instant>>,
    
    /// Count of consecutive failures in closed state
    failure_count: AtomicUsize,
    
    /// Count of consecutive successes in half-open state
    success_count: AtomicUsize,
    
    /// Total number of failures
    total_failures: AtomicUsize,
    
    /// Total number of successes
    total_successes: AtomicUsize,
    
    /// Configuration
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the specified configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            status: RwLock::new(CircuitBreakerStatus::Closed),
            opened_at: RwLock::new(None),
            failure_count: AtomicUsize::new(0),
            success_count: AtomicUsize::new(0),
            total_failures: AtomicUsize::new(0),
            total_successes: AtomicUsize::new(0),
            config,
        }
    }
    
    /// Check if the circuit allows a request
    pub fn check(&self) -> Result<()> {
        let status = self.status();
        
        match status {
            CircuitBreakerStatus::Closed => {
                // Circuit is closed, allow the request
                Ok(())
            }
            CircuitBreakerStatus::Open => {
                // Check if reset timeout has elapsed
                let should_reset = {
                    let opened_at = self.opened_at.read().unwrap();
                    if let Some(instant) = *opened_at {
                        instant.elapsed() >= self.config.reset_timeout
                    } else {
                        // No opened_at time recorded, default to allowing transition
                        true
                    }
                };
                
                if should_reset {
                    // Transition to half-open
                    self.transition_to_half_open();
                    Ok(())
                } else {
                    // Circuit is still open, reject the request
                    Err(ServiceError::service(format!("Circuit breaker is open, rejecting requests for {} more seconds", 
                        self.config.reset_timeout.as_secs().saturating_sub(
                            self.opened_at.read().unwrap().unwrap().elapsed().as_secs()
                        )
                    )))
                }
            }
            CircuitBreakerStatus::HalfOpen => {
                // Allow test requests in half-open state
                Ok(())
            }
        }
    }
    
    /// Record a successful request
    pub fn record_success(&self) {
        let status = self.status();
        self.total_successes.fetch_add(1, Ordering::SeqCst);
        
        match status {
            CircuitBreakerStatus::Closed => {
                // Reset failure counter on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitBreakerStatus::HalfOpen => {
                // Increment success counter
                let successes = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                
                // Check if we've reached the success threshold
                if successes >= self.config.success_threshold {
                    self.close_circuit();
                }
            }
            CircuitBreakerStatus::Open => {
                // This shouldn't happen normally, but just in case
                log::warn!("Received success in Open state, ignoring");
            }
        }
    }
    
    /// Record a failed request
    pub fn record_failure(&self) {
        let status = self.status();
        self.total_failures.fetch_add(1, Ordering::SeqCst);
        
        match status {
            CircuitBreakerStatus::Closed => {
                // Increment failure counter
                let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                
                // Check if we've reached the failure threshold
                if failures >= self.config.failure_threshold {
                    self.open_circuit();
                }
            }
            CircuitBreakerStatus::HalfOpen => {
                // Any failure in half-open state reopens the circuit
                self.open_circuit();
            }
            CircuitBreakerStatus::Open => {
                // Already open, just log
                log::debug!("Received failure in Open state, ignoring");
            }
        }
    }
    
    /// Reset the circuit breaker to closed state
    pub fn reset(&self) {
        *self.status.write().unwrap() = CircuitBreakerStatus::Closed;
        *self.opened_at.write().unwrap() = None;
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
    }
    
    /// Get the current circuit status
    pub fn status(&self) -> CircuitBreakerStatus {
        *self.status.read().unwrap()
    }
    
    /// Get the current number of consecutive failures
    pub fn failure_count(&self) -> usize {
        self.failure_count.load(Ordering::SeqCst)
    }
    
    /// Get the current number of consecutive successes in half-open state
    pub fn success_count(&self) -> usize {
        self.success_count.load(Ordering::SeqCst)
    }
    
    /// Get metrics about the circuit breaker
    pub fn metrics(&self) -> CircuitBreakerMetrics {
        let status = self.status();
        let opened_duration = {
            let opened_at = self.opened_at.read().unwrap();
            opened_at.map(|instant| instant.elapsed())
        };
        
        CircuitBreakerMetrics {
            status,
            failure_count: self.failure_count.load(Ordering::SeqCst),
            success_count: self.success_count.load(Ordering::SeqCst),
            total_failures: self.total_failures.load(Ordering::SeqCst),
            total_successes: self.total_successes.load(Ordering::SeqCst),
            opened_duration,
            config: self.config.clone(),
        }
    }
    
    // Private methods
    
    /// Transition to open state
    fn open_circuit(&self) {
        log::warn!("Circuit breaker transitioning to Open state");
        *self.status.write().unwrap() = CircuitBreakerStatus::Open;
        *self.opened_at.write().unwrap() = Some(Instant::now());
        self.success_count.store(0, Ordering::SeqCst);
    }
    
    /// Transition to closed state
    fn close_circuit(&self) {
        log::info!("Circuit breaker transitioning to Closed state");
        *self.status.write().unwrap() = CircuitBreakerStatus::Closed;
        *self.opened_at.write().unwrap() = None;
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
    }
    
    /// Transition to half-open state
    fn transition_to_half_open(&self) {
        log::info!("Circuit breaker transitioning to Half-Open state");
        *self.status.write().unwrap() = CircuitBreakerStatus::HalfOpen;
        self.success_count.store(0, Ordering::SeqCst);
    }
}

/// Metrics for a circuit breaker
#[derive(Debug)]
pub struct CircuitBreakerMetrics {
    /// Current status
    pub status: CircuitBreakerStatus,
    
    /// Current failure count
    pub failure_count: usize,
    
    /// Current success count (in half-open state)
    pub success_count: usize,
    
    /// Total failures seen
    pub total_failures: usize,
    
    /// Total successes seen
    pub total_successes: usize,
    
    /// Duration the circuit has been open, if applicable
    pub opened_duration: Option<Duration>,
    
    /// Current configuration
    pub config: CircuitBreakerConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_circuit_closed_initially() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.status(), CircuitBreakerStatus::Closed);
        assert!(cb.check().is_ok());
    }
    
    #[test]
    fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..CircuitBreakerConfig::default()
        };
        
        let cb = CircuitBreaker::new(config);
        
        // Record failures
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.status(), CircuitBreakerStatus::Closed);
        
        // Third failure should open the circuit
        cb.record_failure();
        assert_eq!(cb.status(), CircuitBreakerStatus::Open);
        
        // Check should fail when circuit is open
        assert!(cb.check().is_err());
    }
    
    #[test]
    fn test_circuit_transitions_to_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(100),
            ..CircuitBreakerConfig::default()
        };
        
        let cb = CircuitBreaker::new(config);
        
        // Open the circuit
        cb.record_failure();
        assert_eq!(cb.status(), CircuitBreakerStatus::Open);
        
        // Wait for reset timeout
        thread::sleep(Duration::from_millis(200));
        
        // Next check should transition to half-open
        assert!(cb.check().is_ok());
        assert_eq!(cb.status(), CircuitBreakerStatus::HalfOpen);
    }
    
    #[test]
    fn test_circuit_closes_after_successes_in_half_open() {
        let config = CircuitBreakerConfig {
            success_threshold: 2,
            ..CircuitBreakerConfig::default()
        };
        
        let cb = CircuitBreaker::new(config);
        
        // Manually transition to half-open
        cb.transition_to_half_open();
        assert_eq!(cb.status(), CircuitBreakerStatus::HalfOpen);
        
        // First success
        cb.record_success();
        assert_eq!(cb.status(), CircuitBreakerStatus::HalfOpen);
        
        // Second success should close the circuit
        cb.record_success();
        assert_eq!(cb.status(), CircuitBreakerStatus::Closed);
    }
    
    #[test]
    fn test_failure_in_half_open_reopens_circuit() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        
        // Manually transition to half-open
        cb.transition_to_half_open();
        assert_eq!(cb.status(), CircuitBreakerStatus::HalfOpen);
        
        // Failure should reopen the circuit
        cb.record_failure();
        assert_eq!(cb.status(), CircuitBreakerStatus::Open);
    }
    
    #[test]
    fn test_reset() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        
        // Open the circuit
        cb.open_circuit();
        assert_eq!(cb.status(), CircuitBreakerStatus::Open);
        
        // Reset should close the circuit
        cb.reset();
        assert_eq!(cb.status(), CircuitBreakerStatus::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.success_count(), 0);
    }
}