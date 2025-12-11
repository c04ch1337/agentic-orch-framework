//! Resilience patterns for service clients
//!
//! This module provides implementations of common resilience patterns:
//! - Retry with exponential backoff
//! - Circuit breaker
//! - Unified resilience facade

mod retry;
mod circuit_breaker;

pub use retry::{RetryExecutor, RetryConfig};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};

use std::future::Future;
use std::marker::{Send, Sync};
use std::sync::Arc;
use std::time::Duration;

use crate::error::{Result, ServiceError};

/// A unified resilience facade that composes multiple resilience strategies
pub struct Resilience {
    /// Retry executor
    retry: RetryExecutor,
    
    /// Circuit breaker
    circuit_breaker: Arc<CircuitBreaker>,
}

impl Clone for Resilience {
    fn clone(&self) -> Self {
        Self {
            retry: self.retry.clone(),
            circuit_breaker: Arc::clone(&self.circuit_breaker),
        }
    }
}

impl Default for Resilience {
    fn default() -> Self {
        Self::new(RetryConfig::default(), CircuitBreakerConfig::default())
    }
}

impl Resilience {
    /// Create a new resilience facade with specified configurations
    pub fn new(retry_config: RetryConfig, circuit_breaker_config: CircuitBreakerConfig) -> Self {
        let retry = RetryExecutor::new(retry_config);
        let circuit_breaker = Arc::new(CircuitBreaker::new(circuit_breaker_config));
        
        Self {
            retry,
            circuit_breaker,
        }
    }
    
    /// Execute a fallible operation with all configured resilience patterns
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        // Check circuit breaker first
        self.circuit_breaker.check()?;
        
        // Use retry with circuit breaker for each attempt
        let cb = Arc::clone(&self.circuit_breaker);
        let operation = operation.clone();
        self.retry.execute(move || {
            let cb = Arc::clone(&cb);
            let op = operation.clone();
            async move {
                match op().await {
                    Ok(value) => {
                        cb.record_success();
                        Ok(value)
                    }
                    Err(err) => {
                        // Don't record non-retryable errors in circuit breaker
                        if err.is_retryable() {
                            cb.record_failure();
                        }
                        Err(err)
                    }
                }
            }
        }).await
    }
    
    /// Get the current status of the circuit breaker
    pub fn circuit_breaker_status(&self) -> CircuitBreakerStatus {
        self.circuit_breaker.status()
    }
    
    /// Reset the circuit breaker state
    pub fn reset_circuit_breaker(&self) {
        self.circuit_breaker.reset();
    }
    
    /// Configure the retry executor
    pub fn configure_retry(&mut self, config: RetryConfig) {
        self.retry = RetryExecutor::new(config);
    }
    
    /// Configure the circuit breaker
    pub fn configure_circuit_breaker(&mut self, config: CircuitBreakerConfig) {
        self.circuit_breaker = Arc::new(CircuitBreaker::new(config));
    }
}

/// Status of a circuit breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerStatus {
    /// Circuit is closed, allowing requests
    Closed,
    
    /// Circuit is open, rejecting requests
    Open,
    
    /// Circuit is half-open, allowing a limited number of test requests
    HalfOpen,
}

impl std::fmt::Display for CircuitBreakerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "Closed"),
            Self::Open => write!(f, "Open"),
            Self::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}