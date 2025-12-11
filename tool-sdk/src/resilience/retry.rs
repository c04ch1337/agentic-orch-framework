//! Retry with exponential backoff for recoverable errors
//!
//! This module provides a retry mechanism with configurable exponential backoff
//! for handling transient failures when interacting with external services.

use std::future::Future;
use std::marker::{Send, Sync};
use std::time::Duration;
use backoff::{ExponentialBackoff, backoff::Backoff};
use std::fmt;

use crate::error::{Result, ServiceError};

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (0 means no retries)
    pub max_retries: u32,
    
    /// Initial backoff duration
    pub initial_interval: Duration,
    
    /// Maximum backoff duration
    pub max_interval: Duration,
    
    /// Multiplier for backoff between retries
    pub multiplier: f64,
    
    /// Whether to add randomization to backoff intervals
    pub randomization_factor: f64,
    
    /// Maximum total time to spend retrying
    pub max_elapsed_time: Option<Duration>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_interval: Duration::from_millis(100),
            max_interval: Duration::from_secs(10),
            multiplier: 2.0,
            randomization_factor: 0.2,
            max_elapsed_time: Some(Duration::from_secs(30)),
        }
    }
}

impl fmt::Display for RetryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RetryConfig {{ max_retries: {}, initial_interval: {:?}, max_interval: {:?}, multiplier: {}, randomization_factor: {}, max_elapsed_time: {:?} }}",
            self.max_retries,
            self.initial_interval,
            self.max_interval,
            self.multiplier,
            self.randomization_factor,
            self.max_elapsed_time
        )
    }
}

/// Executor for retry operations with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryExecutor {
    /// Retry configuration
    config: RetryConfig,
}

impl RetryExecutor {
    /// Create a new retry executor with the specified configuration
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }
    
    /// Execute a fallible operation with retries according to the configuration
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        // Build the exponential backoff from our config
        let mut backoff = ExponentialBackoff {
            initial_interval: self.config.initial_interval,
            max_interval: self.config.max_interval,
            multiplier: self.config.multiplier,
            randomization_factor: self.config.randomization_factor,
            max_elapsed_time: self.config.max_elapsed_time,
            ..ExponentialBackoff::default()
        };
        
        // Track retry attempts
        let mut attempts = 0;
        let op = operation.clone();
        
        loop {
            let result = op().await;
            
            match result {
                Ok(value) => return Ok(value),
                Err(err) if self.should_retry(&err) && attempts < self.config.max_retries => {
                    // Calculate next backoff duration
                    if let Some(backoff_duration) = backoff.next_backoff() {
                        // Log the retry
                        log::warn!(
                            "Operation failed with retryable error, retrying in {:?} (attempt {}/{}): {}",
                            backoff_duration,
                            attempts + 1,
                            self.config.max_retries,
                            err
                        );
                        
                        // Wait for the backoff duration
                        tokio::time::sleep(backoff_duration).await;
                        
                        // Increment attempt counter
                        attempts += 1;
                    } else {
                        // Max elapsed time exceeded
                        return Err(err.with_context_value("max_retries", attempts));
                    }
                }
                Err(err) => {
                    // Not retryable or max retries exceeded
                    if attempts > 0 {
                        return Err(err.with_context_value("attempts", attempts));
                    } else {
                        return Err(err);
                    }
                }
            }
        }
    }
    
    /// Determine if an error should be retried
    fn should_retry(&self, error: &ServiceError) -> bool {
        error.is_retryable()
    }
    
    /// Get the current retry configuration
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }
    
    /// Update the retry configuration
    pub fn update_config(&mut self, config: RetryConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_successful_operation() {
        let retry = RetryExecutor::new(RetryConfig::default());
        let result = retry.execute(|| async { Ok::<_, ServiceError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }
    
    #[tokio::test]
    async fn test_retry_on_failure() {
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let retry_config = RetryConfig {
            max_retries: 2,
            initial_interval: Duration::from_millis(10),
            max_interval: Duration::from_millis(100),
            ..RetryConfig::default()
        };
        
        let retry = RetryExecutor::new(retry_config);
        let attempt_count_clone = Arc::clone(&attempt_count);
        
        let result = retry.execute(move || {
            let attempt_count_clone = Arc::clone(&attempt_count_clone);
            async move {
                let current_attempt = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
                
                if current_attempt < 2 {
                    // Fail the first two attempts with a retryable error
                    Err(ServiceError::network("Test failure"))
                } else {
                    // Succeed on the third attempt
                    Ok::<_, ServiceError>(42)
                }
            }
        }).await;
        
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }
    
    #[tokio::test]
    async fn test_no_retry_on_non_retryable_error() {
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let retry = RetryExecutor::new(RetryConfig::default());
        let attempt_count_clone = Arc::clone(&attempt_count);
        
        let result = retry.execute(move || {
            let attempt_count_clone = Arc::clone(&attempt_count_clone);
            async move {
                attempt_count_clone.fetch_add(1, Ordering::SeqCst);
                // Return a non-retryable error
                Err(ServiceError::validation("Invalid input"))
            }
        }).await;
        
        assert!(result.is_err());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }
    
    #[tokio::test]
    async fn test_max_retries_exceeded() {
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let retry_config = RetryConfig {
            max_retries: 2,
            initial_interval: Duration::from_millis(10),
            max_interval: Duration::from_millis(100),
            ..RetryConfig::default()
        };
        
        let retry = RetryExecutor::new(retry_config);
        let attempt_count_clone = Arc::clone(&attempt_count);
        
        let result = retry.execute(move || {
            let attempt_count_clone = Arc::clone(&attempt_count_clone);
            async move {
                attempt_count_clone.fetch_add(1, Ordering::SeqCst);
                // Always fail with a retryable error
                Err(ServiceError::network("Persistent failure"))
            }
        }).await;
        
        assert!(result.is_err());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3); // Initial + 2 retries
    }
}