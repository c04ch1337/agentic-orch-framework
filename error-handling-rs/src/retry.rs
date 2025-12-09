//! # Retry Mechanism
//!
//! This module provides retry functionality with exponential backoff and jitter
//! for handling transient failures in distributed systems.

use std::time::Duration;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fmt;

use rand::Rng;
use serde::{Serialize, Deserialize};
use tokio::time::sleep;
use tracing::{trace, debug, info, warn, error};
use metrics::{counter, gauge, histogram};

use crate::types::{Error, Result, ErrorKind};
use crate::logging::current_correlation_id;

/// The result of a retry operation
#[derive(Debug)]
pub enum RetryResult<T> {
    /// The operation succeeded with the given result
    Success(T),
    /// All retries failed, returning the final error
    Failure(Error),
}

impl<T> RetryResult<T> {
    /// Converts the result to a standard Result
    pub fn into_result(self) -> Result<T> {
        match self {
            RetryResult::Success(value) => Ok(value),
            RetryResult::Failure(err) => Err(err),
        }
    }

    /// Returns true if the result is a success
    pub fn is_success(&self) -> bool {
        matches!(self, RetryResult::Success(_))
    }

    /// Returns true if the result is a failure
    pub fn is_failure(&self) -> bool {
        matches!(self, RetryResult::Failure(_))
    }

    /// Maps a function over the success value
    pub fn map<U, F>(self, f: F) -> RetryResult<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            RetryResult::Success(value) => RetryResult::Success(f(value)),
            RetryResult::Failure(err) => RetryResult::Failure(err),
        }
    }

    /// Unwraps the success value or panics
    pub fn unwrap(self) -> T {
        match self {
            RetryResult::Success(value) => value,
            RetryResult::Failure(err) => panic!("Called unwrap on a RetryResult::Failure: {:?}", err),
        }
    }

    /// Unwraps the success value or returns the default
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            RetryResult::Success(value) => value,
            RetryResult::Failure(_) => default,
        }
    }

    /// Unwraps the success value or computes a default
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce(Error) -> T,
    {
        match self {
            RetryResult::Success(value) => value,
            RetryResult::Failure(err) => f(err),
        }
    }
}

/// A trait for errors that can be retried
pub trait RetryableError {
    /// Returns true if the error is transient and the operation might succeed on retry
    fn is_transient(&self) -> bool;

    /// Returns the suggested delay before retrying
    fn suggested_delay(&self) -> Option<Duration> {
        None
    }

    /// Error categorization based on status codes or other indicators
    fn categorize(&self) -> RetryCategory {
        RetryCategory::Normal
    }
}

/// Categories of errors for different retry strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryCategory {
    /// Standard transient error
    Normal,
    /// Rate limiting or throttling error
    RateLimit,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Timeout
    Timeout,
    /// Connectivity issue
    Connectivity,
    /// Server error
    Server,
    /// Client error (likely not retriable)
    Client,
    /// Unavailable or maintenance
    Unavailable,
}

/// Implementation for our standard Error type
impl RetryableError for Error {
    fn is_transient(&self) -> bool {
        // Use the transient flag in our Error type
        self.transient
    }

    fn suggested_delay(&self) -> Option<Duration> {
        // If the error has a "retry-after" header value in context, use it
        self.context.get("retry-after")
            .and_then(|value| value.as_u64())
            .map(Duration::from_secs)
    }

    fn categorize(&self) -> RetryCategory {
        match self.kind {
            ErrorKind::RateLimit => RetryCategory::RateLimit,
            ErrorKind::Resource => RetryCategory::ResourceExhaustion,
            ErrorKind::Timeout => RetryCategory::Timeout,
            ErrorKind::Communication => RetryCategory::Connectivity,
            ErrorKind::External => RetryCategory::Server,
            ErrorKind::Unavailable => RetryCategory::Unavailable,
            ErrorKind::Validation => RetryCategory::Client,
            ErrorKind::Authentication => RetryCategory::Client,
            _ => RetryCategory::Normal,
        }
    }
}

/// Configuration for a retry policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: usize,
    
    /// Base duration for exponential backoff
    pub base_backoff: Duration,
    
    /// Maximum backoff time
    pub max_backoff: Duration,
    
    /// Jitter factor (0.0 - 1.0) to add randomness to backoff
    pub jitter_factor: f64,
    
    /// Overall timeout for the entire operation
    pub operation_timeout: Option<Duration>,
    
    /// Whether to retry only transient errors
    pub retry_only_transient: bool,
    
    /// Optional list of error kinds to retry
    pub retryable_errors: Option<Vec<ErrorKind>>,
    
    /// Optional list of error kinds to never retry
    pub non_retryable_errors: Option<Vec<ErrorKind>>,
    
    /// Whether to record metrics
    pub record_metrics: bool,
    
    /// Backpressure limit - max concurrent retries
    pub backpressure_limit: Option<usize>,
    
    /// Whether to use circuit breaker integration
    pub use_circuit_breaker: bool,
    
    /// Special policy for rate limiting errors
    pub rate_limit_policy: RateLimitPolicy,
}

/// Default settings for the retry policy
impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(30),
            jitter_factor: 0.1,
            operation_timeout: Some(Duration::from_secs(60)),
            retry_only_transient: true,
            retryable_errors: None,
            non_retryable_errors: None,
            record_metrics: true,
            backpressure_limit: Some(100),
            use_circuit_breaker: true,
            rate_limit_policy: RateLimitPolicy::default(),
        }
    }
}

/// Special policy for handling rate limiting errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitPolicy {
    /// Whether to respect Retry-After headers
    pub respect_retry_after: bool,
    
    /// Multiplier for backoff when rate limited
    pub backoff_multiplier: f64,
    
    /// Whether to use a separate counter for rate limit errors
    pub separate_counter: bool,
}

impl Default for RateLimitPolicy {
    fn default() -> Self {
        Self {
            respect_retry_after: true,
            backoff_multiplier: 2.0,
            separate_counter: true,
        }
    }
}

/// A retry policy that determines how to handle retries
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Configuration for this policy
    config: RetryConfig,
    
    /// Name of the policy (for metrics)
    name: String,
    
    /// Concurrent retries counter for backpressure
    concurrent_retries: Arc<AtomicUsize>,
}

// Thread-safe reference counting for the concurrent retries counter
use std::sync::Arc;

impl RetryPolicy {
    /// Creates a new retry policy with the given name and configuration
    pub fn new<S: Into<String>>(name: S, config: Option<RetryConfig>) -> Self {
        let config = config.unwrap_or_default();
        
        Self {
            config,
            name: name.into(),
            concurrent_retries: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    /// Creates a policy for operations that should never be retried
    pub fn never() -> Self {
        let mut config = RetryConfig::default();
        config.max_retries = 0;
        
        Self {
            config,
            name: "never".to_string(),
            concurrent_retries: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    /// Creates a simple policy with a fixed number of retries
    pub fn fixed<S: Into<String>>(name: S, retries: usize) -> Self {
        let mut config = RetryConfig::default();
        config.max_retries = retries;
        
        Self {
            config,
            name: name.into(),
            concurrent_retries: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    /// Creates a policy for network operations
    pub fn network() -> Self {
        let config = RetryConfig {
            max_retries: 5,
            base_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(10),
            jitter_factor: 0.2,
            operation_timeout: Some(Duration::from_secs(30)),
            retry_only_transient: true,
            retryable_errors: Some(vec![
                ErrorKind::Communication,
                ErrorKind::Timeout,
                ErrorKind::Unavailable,
                ErrorKind::External,
            ]),
            ..Default::default()
        };
        
        Self {
            config,
            name: "network".to_string(),
            concurrent_retries: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    /// Creates a policy for database operations
    pub fn database() -> Self {
        let config = RetryConfig {
            max_retries: 3,
            base_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_secs(2),
            jitter_factor: 0.1,
            operation_timeout: Some(Duration::from_secs(10)),
            retry_only_transient: true,
            retryable_errors: Some(vec![
                ErrorKind::Storage,
                ErrorKind::Timeout,
                ErrorKind::Concurrency,
            ]),
            ..Default::default()
        };
        
        Self {
            config,
            name: "database".to_string(),
            concurrent_retries: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    /// Checks if an error is retryable according to this policy
    pub fn is_retryable<E: RetryableError>(&self, error: &E, attempt: usize) -> bool {
        // Check maximum retries
        if attempt >= self.config.max_retries {
            return false;
        }
        
        // Check if we only retry transient errors
        if self.config.retry_only_transient && !error.is_transient() {
            return false;
        }
        
        // Check backpressure limit if configured
        if let Some(limit) = self.config.backpressure_limit {
            let current = self.concurrent_retries.load(Ordering::Relaxed);
            if current >= limit {
                debug!(
                    policy = %self.name,
                    current = %current,
                    limit = %limit,
                    "Retry rejected due to backpressure limit"
                );
                return false;
            }
        }
        
        // If we have a specific error implementation, use it for additional checks
        if let Some(err) = error.downcast_ref::<Error>() {
            // Check if this error kind is in the non-retryable list
            if let Some(non_retryable) = &self.config.non_retryable_errors {
                if non_retryable.contains(&err.kind) {
                    return false;
                }
            }
            
            // Check if we have a whitelist of retryable errors
            if let Some(retryable) = &self.config.retryable_errors {
                return retryable.contains(&err.kind);
            }
        }
        
        true
    }
    
    /// Calculates the backoff duration for a retry
    pub fn calculate_backoff<E: RetryableError>(&self, error: &E, attempt: usize) -> Duration {
        // Check for suggested delay from the error
        if let Some(delay) = error.suggested_delay() {
            return delay;
        }
        
        // For rate limiting errors, use special policy
        let category = error.categorize();
        if category == RetryCategory::RateLimit && self.config.rate_limit_policy.respect_retry_after {
            // This is a rate limit error, so use a more aggressive backoff
            let multiplier = self.config.rate_limit_policy.backoff_multiplier;
            let base = self.config.base_backoff.as_millis() as f64;
            
            // Calculate exponential backoff with the rate limit multiplier
            let backoff_ms = base * multiplier.powf(attempt as f64);
            let max_ms = self.config.max_backoff.as_millis() as f64;
            
            let backoff_ms = backoff_ms.min(max_ms);
            return Duration::from_millis(backoff_ms as u64);
        }
        
        // Standard exponential backoff with full jitter
        let base_ms = self.config.base_backoff.as_millis() as f64;
        let max_ms = self.config.max_backoff.as_millis() as f64;
        
        // Calculate raw exponential backoff
        let exp_backoff = base_ms * 2.0_f64.powf(attempt as f64);
        let capped_backoff = exp_backoff.min(max_ms);
        
        // Add jitter to avoid thundering herd
        let jitter_range = capped_backoff * self.config.jitter_factor;
        let jitter = rand::thread_rng().gen_range(-jitter_range..jitter_range);
        
        let final_backoff_ms = (capped_backoff + jitter).max(0.0);
        Duration::from_millis(final_backoff_ms as u64)
    }
    
    /// Records metrics for retry operations
    fn record_metrics(&self, attempt: usize, success: bool, error_category: Option<RetryCategory>, duration: Duration) {
        if !self.config.record_metrics {
            return;
        }
        
        let metric_prefix = format!("retry.{}", self.name);
        
        // Record attempt count
        counter!(&format!("{}.attempts", metric_prefix), 1);
        
        // Record success/failure
        if success {
            counter!(&format!("{}.success", metric_prefix), 1);
        } else {
            counter!(&format!("{}.failure", metric_prefix), 1);
        }
        
        // Record final attempt number
        counter!(&format!("{}.attempt.{}", metric_prefix, attempt), 1);
        
        // Error category metrics if available
        if let Some(category) = error_category {
            let category_name = match category {
                RetryCategory::Normal => "normal",
                RetryCategory::RateLimit => "rate_limit",
                RetryCategory::ResourceExhaustion => "resource_exhaustion",
                RetryCategory::Timeout => "timeout",
                RetryCategory::Connectivity => "connectivity",
                RetryCategory::Server => "server",
                RetryCategory::Client => "client",
                RetryCategory::Unavailable => "unavailable",
            };
            
            counter!(&format!("{}.error.{}", metric_prefix, category_name), 1);
        }
        
        // Record backoff duration
        histogram!(&format!("{}.duration_ms", metric_prefix), duration.as_millis() as f64);
        
        // Record concurrent retries
        gauge!(&format!("{}.concurrent", metric_prefix), self.concurrent_retries.load(Ordering::Relaxed) as f64);
    }
    
    /// Executes a function with retry behavior
    pub async fn retry<F, Fut, T>(&self, operation_name: &str, f: F) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let start_time = std::time::Instant::now();
        let mut last_error: Option<Error> = None;
        let correlation_id = current_correlation_id();
        
        // Use a separate tokio::time::timeout for overall operation if configured
        let operation_result = match self.config.operation_timeout {
            Some(timeout_duration) => {
                // Execute with overall timeout
                match tokio::time::timeout(timeout_duration, self._retry_internal(operation_name, f, start_time)).await {
                    Ok(result) => result,
                    Err(_) => {
                        // Timeout occurred
                        let timeout_error = Error::new(
                            ErrorKind::Timeout,
                            format!("Operation '{}' timed out after {:?}", operation_name, timeout_duration)
                        ).context("operation", operation_name)
                         .context("timeout_ms", timeout_duration.as_millis());
                        
                        if let Some(correlation_id) = correlation_id {
                            timeout_error.context("correlation_id", correlation_id);
                        }
                        
                        error!(
                            operation = %operation_name,
                            timeout_ms = %timeout_duration.as_millis(),
                            "Operation timed out"
                        );
                        
                        RetryResult::Failure(timeout_error)
                    }
                }
            },
            None => {
                // Execute without timeout
                self._retry_internal(operation_name, f, start_time).await
            }
        };
        
        // Log the outcome
        match &operation_result {
            RetryResult::Success(_) => {
                debug!(
                    operation = %operation_name,
                    duration_ms = %start_time.elapsed().as_millis(),
                    "Operation succeeded after retries"
                );
            }
            RetryResult::Failure(error) => {
                warn!(
                    operation = %operation_name,
                    duration_ms = %start_time.elapsed().as_millis(),
                    error = %error,
                    "Operation failed after retries"
                );
            }
        }
        
        operation_result
    }
    
    // Internal implementation of retry logic
    async fn _retry_internal<F, Fut, T>(&self, operation_name: &str, f: F, start_time: std::time::Instant) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut last_error: Option<Error> = None;
        
        loop {
            // Execute the operation
            let attempt_start = std::time::Instant::now();
            let result = f().await;
            let attempt_duration = attempt_start.elapsed();
            
            match result {
                Ok(value) => {
                    // Success!
                    let total_duration = start_time.elapsed();
                    
                    if attempt > 0 {
                        // Only log if we had to retry
                        info!(
                            operation = %operation_name,
                            attempt = %attempt,
                            duration_ms = %total_duration.as_millis(),
                            "Operation succeeded after retries"
                        );
                    }
                    
                    // Record metrics
                    self.record_metrics(attempt, true, None, total_duration);
                    
                    return RetryResult::Success(value);
                }
                Err(error) => {
                    // Record the error
                    last_error = Some(error.clone());
                    
                    // Determine if this error is retryable
                    if self.is_retryable(&error, attempt) {
                        attempt += 1;
                        
                        // Calculate backoff duration
                        let backoff = self.calculate_backoff(&error, attempt);
                        
                        // Get error category for metrics
                        let error_category = Some(error.categorize());
                        
                        // Log the retry attempt
                        debug!(
                            operation = %operation_name,
                            attempt = %attempt,
                            max_retries = %self.config.max_retries,
                            backoff_ms = %backoff.as_millis(),
                            error = %error,
                            "Retrying after error"
                        );
                        
                        // Record that we're about to retry
                        self.concurrent_retries.fetch_add(1, Ordering::Relaxed);
                        
                        // Sleep for the backoff duration
                        sleep(backoff).await;
                        
                        // Record that we're done retrying
                        self.concurrent_retries.fetch_sub(1, Ordering::Relaxed);
                        
                        // Continue to the next iteration
                        continue;
                    } else {
                        // Not retryable or max retries reached
                        let total_duration = start_time.elapsed();
                        
                        warn!(
                            operation = %operation_name,
                            attempt = %attempt,
                            max_retries = %self.config.max_retries,
                            duration_ms = %total_duration.as_millis(),
                            error = %error,
                            "Giving up after retries"
                        );
                        
                        // Record failure metrics
                        self.record_metrics(attempt, false, Some(error.categorize()), total_duration);
                        
                        // Enhance error with retry information
                        let mut final_error = error.clone();
                        final_error = final_error
                            .context("operation", operation_name)
                            .context("attempts", attempt)
                            .context("max_retries", self.config.max_retries)
                            .context("duration_ms", total_duration.as_millis());
                        
                        return RetryResult::Failure(final_error);
                    }
                }
            }
        }
    }
}

impl fmt::Display for RetryPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RetryPolicy({}, max_retries={})", 
            self.name, self.config.max_retries)
    }
}

/// Default policy for common operations
impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new("default", None)
    }
}

/// Helper function to retry an operation with a simple, default policy
pub async fn retry<F, Fut, T>(operation_name: &str, max_retries: usize, f: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let policy = RetryPolicy::fixed("simple", max_retries);
    policy.retry(operation_name, f).await.into_result()
}

/// Executes an operation with an optimized retry policy for network operations
pub async fn retry_network<F, Fut, T>(operation_name: &str, f: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let policy = RetryPolicy::network();
    policy.retry(operation_name, f).await.into_result()
}

/// Executes an operation with an optimized retry policy for database operations
pub async fn retry_database<F, Fut, T>(operation_name: &str, f: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let policy = RetryPolicy::database();
    policy.retry(operation_name, f).await.into_result()
}

/// Defines how backpressure should be applied
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BackpressureStrategy {
    /// Allow the operation but track metrics
    Monitor,
    /// Delay the operation based on the current load
    Delay,
    /// Reject the operation if load is too high
    Reject,
    /// Shed load by rejecting a percentage of requests
    Shed(f64),
}

/// Backpressure configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureConfig {
    /// The target concurrency level
    pub target_concurrency: usize,
    /// The maximum concurrency level
    pub max_concurrency: usize,
    /// How to apply backpressure
    pub strategy: BackpressureStrategy,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            target_concurrency: 100,
            max_concurrency: 200,
            strategy: BackpressureStrategy::Delay,
        }
    }
}

/// The backpressure controller
#[derive(Debug)]
pub struct Backpressure {
    /// Configuration
    config: BackpressureConfig,
    /// Current concurrency level
    concurrency: Arc<AtomicUsize>,
    /// Last time load shedding was evaluated
    last_check: std::sync::Mutex<std::time::Instant>,
}

impl Backpressure {
    /// Creates a new backpressure controller
    pub fn new(config: BackpressureConfig) -> Self {
        Self {
            config,
            concurrency: Arc::new(AtomicUsize::new(0)),
            last_check: std::sync::Mutex::new(std::time::Instant::now()),
        }
    }
    
    /// Acquires a permit to execute an operation
    pub async fn acquire(&self) -> Result<BackpressurePermit> {
        let current = self.concurrency.fetch_add(1, Ordering::Relaxed);
        
        // Apply backpressure strategy
        match self.config.strategy {
            BackpressureStrategy::Monitor => {
                // Just record metrics
                gauge!("backpressure.concurrency", current as f64);
                gauge!("backpressure.utilization", current as f64 / self.config.target_concurrency as f64);
            }
            BackpressureStrategy::Delay => {
                // Add delay based on load
                if current > self.config.target_concurrency {
                    let utilization = current as f64 / self.config.target_concurrency as f64;
                    let delay_ms = ((utilization - 1.0) * 100.0).min(1000.0);
                    
                    debug!(
                        concurrency = %current,
                        target = %self.config.target_concurrency,
                        delay_ms = %delay_ms,
                        "Applying backpressure delay"
                    );
                    
                    sleep(Duration::from_millis(delay_ms as u64)).await;
                }
            }
            BackpressureStrategy::Reject => {
                // Reject if over max concurrency
                if current > self.config.max_concurrency {
                    self.concurrency.fetch_sub(1, Ordering::Relaxed);
                    
                    warn!(
                        concurrency = %current,
                        max = %self.config.max_concurrency,
                        "Rejecting request due to backpressure"
                    );
                    
                    return Err(Error::new(
                        ErrorKind::RateLimit,
                        "Operation rejected due to backpressure"
                    ));
                }
            }
            BackpressureStrategy::Shed(percentage) => {
                // Check if we need to shed load
                if current > self.config.target_concurrency {
                    let mut last_check = self.last_check.lock().unwrap();
                    
                    // Only re-evaluate every 100ms to avoid too much randomness
                    if last_check.elapsed() > Duration::from_millis(100) {
                        *last_check = std::time::Instant::now();
                        
                        // Shed with increasing probability as we approach max
                        let utilization = current as f64 / self.config.max_concurrency as f64;
                        let shed_probability = percentage * utilization;
                        
                        if rand::thread_rng().gen::<f64>() < shed_probability {
                            self.concurrency.fetch_sub(1, Ordering::Relaxed);
                            
                            warn!(
                                concurrency = %current,
                                utilization = %format!("{:.2}", utilization),
                                probability = %format!("{:.2}", shed_probability),
                                "Shedding request due to backpressure"
                            );
                            
                            return Err(Error::new(
                                ErrorKind::RateLimit,
                                "Operation rejected due to load shedding"
                            ));
                        }
                    }
                }
            }
        }
        
        Ok(BackpressurePermit {
            backpressure: self.concurrency.clone(),
        })
    }
    
    /// Gets the current concurrency level
    pub fn current_concurrency(&self) -> usize {
        self.concurrency.load(Ordering::Relaxed)
    }
}

/// A permit for an operation under backpressure control
#[derive(Debug)]
pub struct BackpressurePermit {
    backpressure: Arc<AtomicUsize>,
}

impl Drop for BackpressurePermit {
    fn drop(&mut self) {
        self.backpressure.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_retry_success_first_attempt() {
        let policy = RetryPolicy::default();
        
        let result = policy.retry("test_op", || async {
            Ok::<_, Error>(42)
        }).await;
        
        assert!(result.is_success());
        assert_eq!(result.unwrap(), 42);
    }
    
    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let policy = RetryPolicy::fixed("test", 3);
        let counter = Arc::new(AtomicU32::new(0));
        
        let counter_clone = counter.clone();
        let result = policy.retry("test_op", || {
            let counter = counter_clone.clone();
            async move {
                let attempt = counter.fetch_add(1, Ordering::SeqCst);
                
                if attempt < 2 {
                    // Fail the first two attempts
                    Err(Error::new(ErrorKind::Timeout, "Temporary failure").transient())
                } else {
                    // Succeed on the third attempt
                    Ok(42)
                }
            }
        }).await;
        
        assert!(result.is_success());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
    
    #[tokio::test]
    async fn test_retry_permanent_failure() {
        let policy = RetryPolicy::fixed("test", 3);
        
        let result = policy.retry("test_op", || async {
            Err(Error::new(ErrorKind::Validation, "Permanent failure"))
        }).await;
        
        assert!(result.is_failure());
        
        if let RetryResult::Failure(err) = result {
            assert_eq!(err.kind, ErrorKind::Validation);
            assert_eq!(err.message, "Permanent failure");
            
            // Should have context about the retries
            assert!(err.context.contains_key("attempts"));
            assert!(err.context.contains_key("max_retries"));
        } else {
            panic!("Expected failure");
        }
    }
    
    #[tokio::test]
    async fn test_retry_timeout() {
        let mut config = RetryConfig::default();
        config.operation_timeout = Some(Duration::from_millis(50));
        
        let policy = RetryPolicy::new("test", Some(config));
        
        let result = policy.retry("test_op", || async {
            // Sleep longer than the timeout
            sleep(Duration::from_millis(100)).await;
            Ok::<_, Error>(42)
        }).await;
        
        assert!(result.is_failure());
        
        if let RetryResult::Failure(err) = result {
            assert_eq!(err.kind, ErrorKind::Timeout);
        } else {
            panic!("Expected timeout failure");
        }
    }
    
    #[tokio::test]
    async fn test_backoff_calculation() {
        let mut config = RetryConfig::default();
        config.base_backoff = Duration::from_millis(10);
        config.jitter_factor = 0.0; // Disable jitter for testing
        
        let policy = RetryPolicy::new("test", Some(config));
        
        let error = Error::new(ErrorKind::Timeout, "Test").transient();
        
        // First retry should be base delay
        let backoff1 = policy.calculate_backoff(&error, 1);
        assert_eq!(backoff1, Duration::from_millis(10));
        
        // Second retry should double
        let backoff2 = policy.calculate_backoff(&error, 2);
        assert_eq!(backoff2, Duration::from_millis(20));
        
        // Third retry should double again
        let backoff3 = policy.calculate_backoff(&error, 3);
        assert_eq!(backoff3, Duration::from_millis(40));
    }
}