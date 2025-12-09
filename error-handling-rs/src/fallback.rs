//! # Fallback Strategies
//!
//! This module provides fallback mechanisms for graceful degradation
//! when critical services or operations fail.
//!
//! Fallback strategies include:
//! - Cached value fallbacks
//! - Default value fallbacks
//! - Alternative implementation fallbacks
//! - Graceful degradation with reduced functionality
//! - Feature flag-controlled fallbacks

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use tokio::sync::Semaphore;
use tracing::{debug, info, warn, error};
use metrics::{counter, gauge, histogram};

use crate::types::{Error, Result, ErrorKind};
use crate::logging::current_correlation_id;

/// Result of a fallback operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackResult<T> {
    /// The primary operation succeeded
    Primary(T),
    /// The primary operation failed but the fallback succeeded
    Fallback(T),
    /// Both primary and fallback operations failed
    Failure(Vec<Error>),
}

impl<T> FallbackResult<T> {
    /// Converts to a standard Result
    pub fn into_result(self) -> Result<T> {
        match self {
            FallbackResult::Primary(value) => Ok(value),
            FallbackResult::Fallback(value) => Ok(value),
            FallbackResult::Failure(errors) => {
                if errors.is_empty() {
                    Err(Error::new(
                        ErrorKind::Internal,
                        "Fallback failed with no specific errors"
                    ))
                } else {
                    // Return the last error with previous errors in context
                    let mut final_error = errors.last().unwrap().clone();
                    for (i, err) in errors.iter().enumerate().take(errors.len() - 1) {
                        final_error = final_error.context(format!("previous_error_{}", i), err.message.clone());
                    }
                    Err(final_error)
                }
            }
        }
    }

    /// Returns true if the result used a fallback
    pub fn is_fallback(&self) -> bool {
        matches!(self, FallbackResult::Fallback(_))
    }

    /// Returns true if the result is from the primary operation
    pub fn is_primary(&self) -> bool {
        matches!(self, FallbackResult::Primary(_))
    }

    /// Returns true if all operations failed
    pub fn is_failure(&self) -> bool {
        matches!(self, FallbackResult::Failure(_))
    }

    /// Gets the value regardless of source
    pub fn unwrap(self) -> T {
        match self {
            FallbackResult::Primary(value) | FallbackResult::Fallback(value) => value,
            FallbackResult::Failure(errors) => panic!(
                "Called unwrap on a FallbackResult::Failure: {:?}", 
                errors
            ),
        }
    }
}

/// A cache entry with expiration
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    /// The cached data
    data: T,
    /// When the data was cached
    timestamp: Instant,
    /// Time-to-live for this entry
    ttl: Duration,
    /// Whether the data is from a fallback
    is_fallback: bool,
}

impl<T> CacheEntry<T> {
    /// Creates a new cache entry
    fn new(data: T, ttl: Duration, is_fallback: bool) -> Self {
        Self {
            data,
            timestamp: Instant::now(),
            ttl,
            is_fallback,
        }
    }

    /// Checks if the entry has expired
    fn is_expired(&self) -> bool {
        self.timestamp.elapsed() > self.ttl
    }
}

/// Configuration for fallback behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// Whether to enable caching
    pub enable_caching: bool,
    /// Default cache TTL
    pub default_ttl: Duration,
    /// Maximum number of concurrent operations
    pub concurrency_limit: Option<usize>,
    /// Whether to record metrics
    pub record_metrics: bool,
    /// Maximum stale time for cache entries
    pub max_stale_time: Option<Duration>,
    /// Whether to attempt fallbacks in parallel
    pub parallel_fallbacks: bool,
    /// Whether to use feature flags for controlling fallbacks
    pub use_feature_flags: bool,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            default_ttl: Duration::from_secs(60),
            concurrency_limit: Some(100),
            record_metrics: true,
            max_stale_time: Some(Duration::from_secs(3600)), // 1 hour max staleness
            parallel_fallbacks: false,
            use_feature_flags: true,
        }
    }
}

/// Types of fallback strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackType {
    /// Return a specified default value
    DefaultValue,
    /// Return cached data if available
    Cache,
    /// Use an alternative implementation
    AlternativeImplementation,
    /// Degrade functionality gracefully
    GracefulDegradation,
    /// Use a simplified backup implementation
    SimplifiedBackup,
}

/// Identifies the source of a result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultSource {
    /// From primary operation
    Primary,
    /// From fallback
    Fallback(FallbackType),
}

/// Cache implementation for fallback values
#[derive(Debug)]
pub struct FallbackCache<K, V> 
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    /// The cache data
    cache: RwLock<HashMap<K, CacheEntry<V>>>,
    /// Configuration
    config: FallbackConfig,
    /// Name for metrics
    name: String,
    /// Cache hit count
    hit_count: std::sync::atomic::AtomicUsize,
    /// Cache miss count
    miss_count: std::sync::atomic::AtomicUsize,
}

impl<K, V> FallbackCache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    /// Creates a new fallback cache
    pub fn new<S: Into<String>>(name: S, config: Option<FallbackConfig>) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            config: config.unwrap_or_default(),
            name: name.into(),
            hit_count: std::sync::atomic::AtomicUsize::new(0),
            miss_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Gets a value from the cache
    pub fn get(&self, key: &K) -> Option<(V, ResultSource)> {
        let cache = self.cache.read().unwrap();
        
        if let Some(entry) = cache.get(key) {
            if entry.is_expired() {
                // Expired entry
                self.miss_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                // If max stale time is configured and not exceeded, return stale data
                if let Some(max_stale) = self.config.max_stale_time {
                    if entry.timestamp.elapsed() <= entry.ttl + max_stale {
                        debug!(
                            cache = %self.name,
                            key = ?key,
                            age_secs = %entry.timestamp.elapsed().as_secs(),
                            "Using stale cache entry"
                        );
                        
                        let source = if entry.is_fallback {
                            ResultSource::Fallback(FallbackType::Cache)
                        } else {
                            ResultSource::Primary
                        };
                        
                        return Some((entry.data.clone(), source));
                    }
                }
                
                None
            } else {
                // Valid entry
                self.hit_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                let source = if entry.is_fallback {
                    ResultSource::Fallback(FallbackType::Cache)
                } else {
                    ResultSource::Primary
                };
                
                Some((entry.data.clone(), source))
            }
        } else {
            // No entry
            self.miss_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            None
        }
    }

    /// Puts a value in the cache
    pub fn put(&self, key: K, value: V, ttl: Option<Duration>, is_fallback: bool) {
        if !self.config.enable_caching {
            return;
        }
        
        let entry = CacheEntry::new(
            value,
            ttl.unwrap_or(self.config.default_ttl),
            is_fallback
        );
        
        let mut cache = self.cache.write().unwrap();
        cache.insert(key, entry);
        
        // Record metrics
        if self.config.record_metrics {
            let size = cache.len();
            gauge!(&format!("fallback_cache.{}.size", self.name), size as f64);
        }
    }

    /// Removes a value from the cache
    pub fn remove(&self, key: &K) -> bool {
        let mut cache = self.cache.write().unwrap();
        cache.remove(key).is_some()
    }

    /// Clears the entire cache
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
        
        // Record metrics
        if self.config.record_metrics {
            gauge!(&format!("fallback_cache.{}.size", self.name), 0.0);
        }
    }

    /// Gets current cache size
    pub fn size(&self) -> usize {
        self.cache.read().unwrap().len()
    }

    /// Records cache metrics
    pub fn record_metrics(&self) {
        if !self.config.record_metrics {
            return;
        }
        
        let hit_count = self.hit_count.load(std::sync::atomic::Ordering::Relaxed);
        let miss_count = self.miss_count.load(std::sync::atomic::Ordering::Relaxed);
        let total = hit_count + miss_count;
        
        let hit_rate = if total > 0 {
            hit_count as f64 / total as f64
        } else {
            0.0
        };
        
        gauge!(&format!("fallback_cache.{}.hit_count", self.name), hit_count as f64);
        gauge!(&format!("fallback_cache.{}.miss_count", self.name), miss_count as f64);
        gauge!(&format!("fallback_cache.{}.hit_rate", self.name), hit_rate);
        gauge!(&format!("fallback_cache.{}.size", self.name), self.size() as f64);
    }
}

/// A fallback strategy for handling failures
#[derive(Debug)]
pub struct FallbackStrategy {
    /// Name of this strategy for metrics
    name: String,
    /// Configuration
    config: FallbackConfig,
    /// Concurrency limiter
    semaphore: Option<Arc<Semaphore>>,
}

impl FallbackStrategy {
    /// Creates a new fallback strategy
    pub fn new<S: Into<String>>(name: S, config: Option<FallbackConfig>) -> Self {
        let config = config.unwrap_or_default();
        let semaphore = config.concurrency_limit.map(|limit| {
            Arc::new(Semaphore::new(limit))
        });
        
        Self {
            name: name.into(),
            config,
            semaphore,
        }
    }
    
    /// Executes an operation with a default value fallback
    pub async fn with_default<F, Fut, T>(&self, operation_name: &str, operation: F, default: T) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
        T: Clone,
    {
        let result = self.execute_with_fallbacks(
            operation_name,
            operation,
            vec![Box::new(move || Box::pin(async move { Ok(default.clone()) }))]
        ).await;
        
        result.into_result()
    }
    
    /// Executes an operation with a cached value fallback
    pub async fn with_cache<F, Fut, T, K>(&self, operation_name: &str, key: K, cache: &FallbackCache<K, T>, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
        T: Clone,
        K: std::hash::Hash + Eq + Clone + std::fmt::Debug,
    {
        // First check cache
        if let Some((cached_value, source)) = cache.get(&key) {
            // Record cache hit metrics
            if self.config.record_metrics {
                match source {
                    ResultSource::Primary => {
                        counter!(&format!("fallback.{}.cache.hit.primary", self.name), 1);
                    }
                    ResultSource::Fallback(_) => {
                        counter!(&format!("fallback.{}.cache.hit.fallback", self.name), 1);
                    }
                }
            }
            
            debug!(
                strategy = %self.name,
                operation = %operation_name,
                key = ?key,
                source = ?source,
                "Using cached value"
            );
            
            return Ok(cached_value);
        }
        
        // Cache miss, try primary operation
        match self.execute_primary(operation_name, operation).await {
            Ok(value) => {
                // Cache the result from primary
                cache.put(key.clone(), value.clone(), None, false);
                
                // Record metrics
                if self.config.record_metrics {
                    counter!(&format!("fallback.{}.cache.miss.primary_success", self.name), 1);
                }
                
                Ok(value)
            }
            Err(error) => {
                // Record metrics
                if self.config.record_metrics {
                    counter!(&format!("fallback.{}.cache.miss.primary_failure", self.name), 1);
                }
                
                warn!(
                    strategy = %self.name,
                    operation = %operation_name,
                    key = ?key,
                    error = %error,
                    "Primary operation failed, no cache entry available"
                );
                
                Err(error)
            }
        }
    }
    
    /// Executes an operation with a single fallback
    pub async fn with_fallback<F1, Fut1, F2, Fut2, T>(&self, operation_name: &str, primary: F1, fallback: F2) -> Result<T>
    where
        F1: FnOnce() -> Fut1,
        Fut1: Future<Output = Result<T>>,
        F2: FnOnce() -> Fut2,
        Fut2: Future<Output = Result<T>>,
    {
        let fallback_fn = Box::new(move || Box::pin(fallback()) as Pin<Box<dyn Future<Output = Result<T>> + Send>>);
        
        let result = self.execute_with_fallbacks(
            operation_name,
            primary,
            vec![fallback_fn]
        ).await;
        
        result.into_result()
    }
    
    /// Executes an operation with multiple fallbacks, trying each in sequence
    pub async fn with_multiple_fallbacks<F, Fut, T>(&self, 
        operation_name: &str,
        primary: F, 
        fallbacks: Vec<Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = Result<T>> + Send>> + Send>>
    ) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let result = self.execute_with_fallbacks(operation_name, primary, fallbacks).await;
        result.into_result()
    }
    
    /// Feature flag controlled fallback
    pub async fn with_feature_flag<F1, Fut1, F2, Fut2, T>(&self, 
        operation_name: &str,
        flag_name: &str,
        flag_enabled: bool,
        enabled_implementation: F1,
        fallback_implementation: F2,
    ) -> Result<T>
    where
        F1: FnOnce() -> Fut1,
        Fut1: Future<Output = Result<T>>,
        F2: FnOnce() -> Fut2,
        Fut2: Future<Output = Result<T>>,
    {
        let start = Instant::now();
        let correlation_id = current_correlation_id();
        
        // Record metrics
        if self.config.record_metrics {
            counter!(&format!("fallback.{}.feature_flag.{}.attempted", self.name, flag_name), 1);
            
            if flag_enabled {
                counter!(&format!("fallback.{}.feature_flag.{}.enabled", self.name, flag_name), 1);
            } else {
                counter!(&format!("fallback.{}.feature_flag.{}.disabled", self.name, flag_name), 1);
            }
        }
        
        let result = if flag_enabled {
            // Use the enabled implementation
            let result = enabled_implementation().await;
            
            if let Err(ref e) = result {
                warn!(
                    strategy = %self.name,
                    operation = %operation_name,
                    flag = %flag_name,
                    error = %e,
                    "Enabled implementation failed"
                );
                
                // Record failure
                if self.config.record_metrics {
                    counter!(&format!("fallback.{}.feature_flag.{}.enabled_failure", self.name, flag_name), 1);
                }
            } else {
                // Record success
                if self.config.record_metrics {
                    counter!(&format!("fallback.{}.feature_flag.{}.enabled_success", self.name, flag_name), 1);
                }
            }
            
            result
        } else {
            // Use the fallback implementation
            let result = fallback_implementation().await;
            
            if let Err(ref e) = result {
                warn!(
                    strategy = %self.name,
                    operation = %operation_name,
                    flag = %flag_name,
                    error = %e,
                    "Fallback implementation failed"
                );
                
                // Record failure
                if self.config.record_metrics {
                    counter!(&format!("fallback.{}.feature_flag.{}.fallback_failure", self.name, flag_name), 1);
                }
            } else {
                // Record success
                if self.config.record_metrics {
                    counter!(&format!("fallback.{}.feature_flag.{}.fallback_success", self.name, flag_name), 1);
                }
            }
            
            result
        };
        
        // Record duration
        let duration = start.elapsed();
        if self.config.record_metrics {
            histogram!(
                &format!("fallback.{}.feature_flag.{}.duration_ms", self.name, flag_name),
                duration.as_millis() as f64
            );
        }
        
        result
    }
    
    /// Executes the primary operation
    async fn execute_primary<F, Fut, T>(&self, operation_name: &str, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let start = Instant::now();
        let correlation_id = current_correlation_id();
        
        // Acquire semaphore permit if concurrency limiting is enabled
        let _permit = if let Some(semaphore) = &self.semaphore {
            match semaphore.clone().acquire_owned().await {
                Ok(permit) => Some(permit),
                Err(_) => {
                    return Err(Error::new(
                        ErrorKind::Concurrency,
                        format!("Failed to acquire semaphore for operation: {}", operation_name)
                    ));
                }
            }
        } else {
            None
        };
        
        // Execute operation
        let result = operation().await;
        
        // Record metrics
        if self.config.record_metrics {
            let duration = start.elapsed();
            
            histogram!(
                &format!("fallback.{}.primary.duration_ms", self.name),
                duration.as_millis() as f64
            );
            
            if result.is_ok() {
                counter!(&format!("fallback.{}.primary.success", self.name), 1);
            } else {
                counter!(&format!("fallback.{}.primary.failure", self.name), 1);
            }
        }
        
        result
    }
    
    /// Executes with fallbacks
    async fn execute_with_fallbacks<F, Fut, T>(&self, 
        operation_name: &str,
        primary: F, 
        fallbacks: Vec<Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = Result<T>> + Send>> + Send>>
    ) -> FallbackResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let start = Instant::now();
        let correlation_id = current_correlation_id();
        
        // Try primary operation first
        match self.execute_primary(operation_name, primary).await {
            Ok(value) => {
                // Primary succeeded
                if self.config.record_metrics {
                    counter!(&format!("fallback.{}.result.primary", self.name), 1);
                }
                
                debug!(
                    strategy = %self.name,
                    operation = %operation_name,
                    duration_ms = %start.elapsed().as_millis(),
                    "Primary operation succeeded"
                );
                
                return FallbackResult::Primary(value);
            }
            Err(primary_error) => {
                // Primary failed, try fallbacks
                warn!(
                    strategy = %self.name,
                    operation = %operation_name,
                    error = %primary_error,
                    "Primary operation failed, trying fallbacks"
                );
                
                let mut errors = vec![primary_error];
                
                // Try each fallback in sequence
                for (i, fallback) in fallbacks.into_iter().enumerate() {
                    let fallback_start = Instant::now();
                    
                    // Execute fallback
                    match fallback().await {
                        Ok(value) => {
                            // Fallback succeeded
                            if self.config.record_metrics {
                                counter!(&format!("fallback.{}.result.fallback", self.name), 1);
                                counter!(&format!("fallback.{}.fallback.{}.success", self.name, i), 1);
                                histogram!(
                                    &format!("fallback.{}.fallback.{}.duration_ms", self.name, i),
                                    fallback_start.elapsed().as_millis() as f64
                                );
                            }
                            
                            info!(
                                strategy = %self.name,
                                operation = %operation_name,
                                fallback_index = %i,
                                duration_ms = %fallback_start.elapsed().as_millis(),
                                "Fallback succeeded"
                            );
                            
                            return FallbackResult::Fallback(value);
                        }
                        Err(error) => {
                            // Fallback failed too
                            if self.config.record_metrics {
                                counter!(&format!("fallback.{}.fallback.{}.failure", self.name, i), 1);
                                histogram!(
                                    &format!("fallback.{}.fallback.{}.duration_ms", self.name, i),
                                    fallback_start.elapsed().as_millis() as f64
                                );
                            }
                            
                            warn!(
                                strategy = %self.name,
                                operation = %operation_name,
                                fallback_index = %i,
                                error = %error,
                                "Fallback failed"
                            );
                            
                            errors.push(error);
                        }
                    }
                }
                
                // All fallbacks failed
                if self.config.record_metrics {
                    counter!(&format!("fallback.{}.result.failure", self.name), 1);
                }
                
                error!(
                    strategy = %self.name,
                    operation = %operation_name,
                    error_count = %errors.len(),
                    duration_ms = %start.elapsed().as_millis(),
                    "All fallbacks failed"
                );
                
                FallbackResult::Failure(errors)
            }
        }
    }
}

impl Default for FallbackStrategy {
    fn default() -> Self {
        Self::new("default", None)
    }
}

/// A simple helper to execute with a default value fallback
pub async fn with_fallback<F, Fut, T, FB>(operation_name: &str, primary: F, fallback: FB) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T>>,
    FB: FnOnce() -> Result<T>,
{
    let strategy = FallbackStrategy::default();
    
    strategy.with_fallback(
        operation_name,
        primary,
        || async move { fallback() }
    ).await
}

/// A simple helper to execute with a default value
pub async fn with_default<F, Fut, T>(operation_name: &str, primary: F, default: T) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T>>,
    T: Clone,
{
    let strategy = FallbackStrategy::default();
    strategy.with_default(operation_name, primary, default).await
}

/// Bulkhead for isolating failures across components
#[derive(Debug)]
pub struct Bulkhead {
    /// Name of this bulkhead for metrics
    name: String,
    /// Maximum concurrent executions
    max_concurrency: usize,
    /// Current concurrent executions
    current: Arc<std::sync::atomic::AtomicUsize>,
    /// Semaphore for limiting concurrency
    semaphore: Arc<Semaphore>,
    /// Whether to record metrics
    record_metrics: bool,
}

impl Bulkhead {
    /// Creates a new bulkhead
    pub fn new<S: Into<String>>(name: S, max_concurrency: usize) -> Self {
        Self {
            name: name.into(),
            max_concurrency,
            current: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
            record_metrics: true,
        }
    }
    
    /// Sets whether to record metrics
    pub fn with_metrics(mut self, record_metrics: bool) -> Self {
        self.record_metrics = record_metrics;
        self
    }
    
    /// Gets current concurrency level
    pub fn current_concurrency(&self) -> usize {
        self.current.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    /// Gets maximum concurrency level
    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency
    }
    
    /// Gets available permits
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
    
    /// Executes an operation with bulkhead isolation
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        // Record attempt
        if self.record_metrics {
            counter!(&format!("bulkhead.{}.attempts", self.name), 1);
        }
        
        // Try to acquire a permit
        let permit = match self.semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => {
                // Semaphore closed
                if self.record_metrics {
                    counter!(&format!("bulkhead.{}.rejected", self.name), 1);
                }
                
                return Err(Error::new(
                    ErrorKind::RateLimit,
                    format!("Bulkhead {} rejected execution due to concurrency limit", self.name)
                ));
            }
        };
        
        // Increment counter
        let current = self.current.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        
        // Record metrics
        if self.record_metrics {
            counter!(&format!("bulkhead.{}.executed", self.name), 1);
            gauge!(&format!("bulkhead.{}.concurrent", self.name), current as f64);
            gauge!(&format!("bulkhead.{}.utilization", self.name), 
                   current as f64 / self.max_concurrency as f64);
        }
        
        // Execute operation
        let start = Instant::now();
        let result = operation().await;
        let duration = start.elapsed();
        
        // Record result and metrics
        if self.record_metrics {
            histogram!(&format!("bulkhead.{}.duration_ms", self.name), duration.as_millis() as f64);
            
            if result.is_ok() {
                counter!(&format!("bulkhead.{}.success", self.name), 1);
            } else {
                counter!(&format!("bulkhead.{}.failure", self.name), 1);
            }
        }
        
        // Decrement counter
        let current = self.current.fetch_sub(1, std::sync::atomic::Ordering::Relaxed) - 1;
        
        // Update metrics
        if self.record_metrics {
            gauge!(&format!("bulkhead.{}.concurrent", self.name), current as f64);
        }
        
        // Return result (permit is dropped automatically)
        result
    }
}

/// Degraded mode management for services
#[derive(Debug)]
pub struct DegradedMode {
    /// The current degraded modes
    modes: Arc<RwLock<HashMap<String, DegradedModeInfo>>>,
    /// Fallback strategy for degraded operations
    fallback_strategy: FallbackStrategy,
}

/// Information about a degraded mode
#[derive(Debug, Clone)]
struct DegradedModeInfo {
    /// Name of the mode
    name: String,
    /// Whether the mode is active
    active: bool,
    /// When the mode was activated
    activated_at: Option<Instant>,
    /// Why the mode was activated
    reason: Option<String>,
    /// Severity level
    severity: DegradedSeverity,
}

/// Severity of degraded mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DegradedSeverity {
    /// Minor degradation, most functionality works
    Minor,
    /// Moderate degradation, core functionality works
    Moderate,
    /// Severe degradation, limited functionality
    Severe,
    /// Critical degradation, minimal functionality
    Critical,
}

impl DegradedMode {
    /// Creates a new degraded mode manager
    pub fn new() -> Self {
        Self {
            modes: Arc::new(RwLock::new(HashMap::new())),
            fallback_strategy: FallbackStrategy::new("degraded", None),
        }
    }
    
    /// Activates a degraded mode
    pub fn activate<S1, S2>(&self, mode: S1, reason: S2, severity: DegradedSeverity)
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        let mode_name = mode.into();
        let reason = reason.into();
        
        let mut modes = self.modes.write().unwrap();
        
        let info = DegradedModeInfo {
            name: mode_name.clone(),
            active: true,
            activated_at: Some(Instant::now()),
            reason: Some(reason.clone()),
            severity,
        };
        
        modes.insert(mode_name.clone(), info);
        
        // Log and record metric
        warn!(
            mode = %mode_name,
            reason = %reason,
            severity = ?severity,
            "Activated degraded mode"
        );
        
        counter!(&format!("degraded_mode.{}.activations", mode_name), 1);
        counter!(&format!("degraded_mode.{}.severity.{:?}", mode_name, severity), 1);
    }
    
    /// Deactivates a degraded mode
    pub fn deactivate<S: Into<String>>(&self, mode: S) {
        let mode_name = mode.into();
        
        let mut modes = self.modes.write().unwrap();
        
        if let Some(info) = modes.get_mut(&mode_name) {
            if info.active {
                info.active = false;
                
                // Calculate duration
                if let Some(activated_at) = info.activated_at {
                    let duration = activated_at.elapsed();
                    
                    // Log and record metric
                    info!(
                        mode = %mode_name,
                        duration_secs = %duration.as_secs(),
                        "Deactivated degraded mode"
                    );
                    
                    gauge!(&format!("degraded_mode.{}.duration_secs", mode_name), duration.as_secs() as f64);
                }
            }
        }
    }
    
    /// Checks if a degraded mode is active
    pub fn is_active<S: Into<String>>(&self, mode: S) -> bool {
        let mode_name = mode.into();
        let modes = self.modes.read().unwrap();
        
        modes.get(&mode_name)
            .map(|info| info.active)
            .unwrap_or(false)
    }
    
    /// Gets information about a degraded mode
    pub fn get_info<S: Into<String>>(&self, mode: S) -> Option<(bool, DegradedSeverity, Option<Duration>)> {
        let mode_name = mode.into();
        let modes = self.modes.read().unwrap();
        
        modes.get(&mode_name).map(|info| {
            let duration = info.activated_at.map(|time| time.elapsed());
            (info.active, info.severity, duration)
        })
    }
    
    /// Gets all active degraded modes
    pub fn active_modes(&self) -> Vec<String> {
        let modes = self.modes.read().unwrap();
        
        modes.iter()
            .filter(|(_, info)| info.active)
            .map(|(name, _)| name.clone())
            .collect()
    }
    
    /// Executes an operation with degraded mode awareness
    pub async fn execute<F, Fut, FB, FBFut, T>(&self, 
        mode: &str,
        primary: F, 
        degraded: FB
    ) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
        FB: FnOnce() -> FBFut,
        FBFut: Future<Output = Result<T>>,
    {
        // Check if mode is active
        let is_active = self.is_active(mode);
        
        if is_active {
            // Execute degraded implementation directly
            debug!(
                mode = %mode,
                "Using degraded implementation (mode active)"
            );
            
            counter!(&format!("degraded_mode.{}.executions.degraded", mode), 1);
            return degraded().await;
        }
        
        // Try primary with fallback to degraded
        let operation_name = format!("degraded_mode_{}", mode);
        
        self.fallback_strategy.with_fallback(
            &operation_name,
            primary,
            degraded
        ).await
    }
}

impl Default for DegradedMode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    #[tokio::test]
    async fn test_fallback_result() {
        let primary = FallbackResult::Primary(42);
        let fallback = FallbackResult::Fallback(42);
        let failure = FallbackResult::Failure(vec![Error::new(ErrorKind::Internal, "Test error")]);
        
        assert_eq!(primary.is_primary(), true);
        assert_eq!(primary.is_fallback(), false);
        assert_eq!(primary.is_failure(), false);
        
        assert_eq!(fallback.is_primary(), false);
        assert_eq!(fallback.is_fallback(), true);
        assert_eq!(fallback.is_failure(), false);
        
        assert_eq!(failure.is_primary(), false);
        assert_eq!(failure.is_fallback(), false);
        assert_eq!(failure.is_failure(), true);
        
        // Test into_result
        assert_eq!(primary.into_result().unwrap(), 42);
        assert_eq!(fallback.into_result().unwrap(), 42);
        assert!(failure.into_result().is_err());
    }
    
    #[tokio::test]
    async fn test_with_default() {
        // Primary succeeds
        let result = with_default("test_op", || async { Ok::<_, Error>(42) }, 0).await;
        assert_eq!(result.unwrap(), 42);
        
        // Primary fails, use default
        let result = with_default(
            "test_op",
            || async { Err::<i32, _>(Error::new(ErrorKind::Internal, "Failed")) },
            99
        ).await;
        assert_eq!(result.unwrap(), 99);
    }
    
    #[tokio::test]
    async fn test_with_fallback() {
        // Primary succeeds
        let result = with_fallback(
            "test_op",
            || async { Ok::<_, Error>(42) },
            || Ok::<_, Error>(99)
        ).await;
        assert_eq!(result.unwrap(), 42);
        
        // Primary fails, fallback succeeds
        let result = with_fallback(
            "test_op",
            || async { Err::<i32, _>(Error::new(ErrorKind::Internal, "Failed")) },
            || Ok::<_, Error>(99)
        ).await;
        assert_eq!(result.unwrap(), 99);
        
        // Both fail
        let result = with_fallback(
            "test_op",
            || async { Err::<i32, _>(Error::new(ErrorKind::Internal, "Failed 1")) },
            || Err::<i32, _>(Error::new(ErrorKind::Internal, "Failed 2"))
        ).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_fallback_cache() {
        let cache: FallbackCache<String, i32> = FallbackCache::new("test", None);
        
        // Initially empty
        assert_eq!(cache.get(&"key1".to_string()), None);
        
        // Add value
        cache.put("key1".to_string(), 42, None, false);
        
        // Read value
        let (value, source) = cache.get(&"key1".to_string()).unwrap();
        assert_eq!(value, 42);
        assert!(matches!(source, ResultSource::Primary));
        
        // Add fallback value
        cache.put("key2".to_string(), 99, None, true);
        
        // Read fallback value
        let (value, source) = cache.get(&"key2".to_string()).unwrap();
        assert_eq!(value, 99);
        assert!(matches!(source, ResultSource::Fallback(_)));
    }
    
    #[tokio::test]
    async fn test_bulkhead() {
        let bulkhead = Bulkhead::new("test", 2);
        
        // Execute first task (should succeed)
        let task1 = bulkhead.execute(|| async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<_, Error>(1)
        });
        
        // Execute second task (should succeed)
        let task2 = bulkhead.execute(|| async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<_, Error>(2)
        });
        
        // Execute third task (should be rejected due to bulkhead)
        let task3 = bulkhead.execute(|| async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<_, Error>(3)
        });
        
        // All tasks should complete
        let (result1, result2, result3) = tokio::join!(task1, task2, task3);
        
        // First two should succeed
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        // Third should be rejected
        assert!(result3.is_err());
        assert_eq!(result3.unwrap_err().kind, ErrorKind::RateLimit);
    }
    
    #[tokio::test]
    async fn test_degraded_mode() {
        let degraded = DegradedMode::new();
        
        // Initially not active
        assert_eq!(degraded.is_active("test_mode"), false);
        
        // Activate
        degraded.activate("test_mode", "Service unavailable", DegradedSeverity::Moderate);
        
        // Now active
        assert_eq!(degraded.is_active("test_mode"), true);
        
        // Should be in active modes list
        let active = degraded.active_modes();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], "test_mode");
        
        // Execute with degraded mode - should use degraded implementation
        let counter = Arc::new(AtomicU32::new(0));
        
        let primary_counter = counter.clone();
        let degraded_counter = counter.clone();
        
        let result = degraded.execute(
            "test_mode",
            || async move {
                primary_counter.fetch_add(1, Ordering::SeqCst);
                Ok::<_, Error>(42)
            },
            || async move {
                degraded_counter.fetch_add(100, Ordering::SeqCst);
                Ok::<_, Error>(99)
            }
        ).await;
        
        // Should return degraded result
        assert_eq!(result.unwrap(), 99);
        
        // Only degraded counter should be updated
        assert_eq!(counter.load(Ordering::SeqCst), 100);
        
        // Deactivate
        degraded.deactivate("test_mode");
        
        // Now inactive
        assert_eq!(degraded.is_active("test_mode"), false);
        assert_eq!(degraded.active_modes().len(), 0);
        
        // Reset counter
        counter.store(0, Ordering::SeqCst);
        
        // Execute again - should try primary first
        let primary_counter = counter.clone();
        let degraded_counter = counter.clone();
        
        let result = degraded.execute(
            "test_mode",
            || async move {
                primary_counter.fetch_add(1, Ordering::SeqCst);
                Ok::<_, Error>(42)
            },
            || async move {
                degraded_counter.fetch_add(100, Ordering::SeqCst);
                Ok::<_, Error>(99)
            }
        ).await;
        
        // Should return primary result
        assert_eq!(result.unwrap(), 42);
        
        // Only primary counter should be updated
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}