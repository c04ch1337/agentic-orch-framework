//! # Advanced Circuit Breaker
//!
//! This module provides an enhanced circuit breaker implementation for
//! preventing cascading failures in distributed systems.
//! 
//! Features include:
//! - Standard circuit states (Closed, Open, Half-Open)
//! - Error percentage thresholds
//! - Sliding window for error tracking
//! - Configurable timeouts and thresholds
//! - Health metrics reporting
//! - Multi-level circuit breaker support

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::RwLock;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use metrics::{counter, gauge, histogram};
use tokio::time::sleep;
use serde::{Serialize, Deserialize};
use tracing::{debug, info, warn, error};
use crate::types::{Error, Result};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Normal operation, requests allowed
    Closed,
    /// Failing, requests blocked
    Open,
    /// Testing recovery, limited requests allowed
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "CLOSED"),
            CircuitState::Open => write!(f, "OPEN"),
            CircuitState::HalfOpen => write!(f, "HALF-OPEN"),
        }
    }
}

/// Result tracking for a sliding window
#[derive(Debug)]
struct ResultWindow {
    /// Size of the sliding window
    size: usize,
    /// Results in the window (true = success, false = failure)
    results: VecDeque<bool>,
    /// Total successes in the window
    success_count: usize,
    /// Total failures in the window
    failure_count: usize,
}

impl ResultWindow {
    /// Creates a new result window with the given size
    pub fn new(size: usize) -> Self {
        Self {
            size,
            results: VecDeque::with_capacity(size),
            success_count: 0,
            failure_count: 0,
        }
    }

    /// Adds a result to the window
    pub fn add_result(&mut self, success: bool) {
        // Remove oldest result if window is full
        if self.results.len() >= self.size {
            if let Some(old_result) = self.results.pop_front() {
                if old_result {
                    self.success_count = self.success_count.saturating_sub(1);
                } else {
                    self.failure_count = self.failure_count.saturating_sub(1);
                }
            }
        }

        // Add new result
        self.results.push_back(success);
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
    }

    /// Gets the current failure rate (0.0 to 1.0)
    pub fn failure_rate(&self) -> f64 {
        if self.results.is_empty() {
            0.0
        } else {
            self.failure_count as f64 / self.results.len() as f64
        }
    }

    /// Gets the total number of results in the window
    pub fn total(&self) -> usize {
        self.results.len()
    }

    /// Gets the success count
    pub fn success_count(&self) -> usize {
        self.success_count
    }

    /// Gets the failure count
    pub fn failure_count(&self) -> usize {
        self.failure_count
    }

    /// Clears all results from the window
    pub fn clear(&mut self) {
        self.results.clear();
        self.success_count = 0;
        self.failure_count = 0;
    }
}

/// Circuit breaker statistics for a single service
#[derive(Debug)]
struct CircuitStats {
    /// Current state of the circuit
    state: CircuitState,
    /// Time of last state change
    last_state_change: Instant,
    /// Time of last failure
    last_failure: Option<Instant>,
    /// Time of last success
    last_success: Option<Instant>,
    /// Sliding window for tracking results
    window: ResultWindow,
    /// Consecutive successes in half-open state
    consecutive_successes: usize,
    /// Number of requests allowed in half-open state
    current_half_open_allowed: usize,
    /// Exponential backoff multiplier for retries
    backoff_multiplier: u32,
}

/// Configuration for a circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Size of the sliding window for error tracking
    pub window_size: usize,
    /// Error threshold percentage to trip the circuit (0.0 to 1.0)
    pub error_threshold: f64,
    /// Minimum number of requests before error threshold applies
    pub minimum_request_threshold: usize,
    /// Time to keep circuit open before testing
    pub reset_timeout: Duration,
    /// Number of consecutive successes to close circuit from half-open
    pub half_open_success_threshold: usize,
    /// Maximum number of allowed requests in half-open state
    pub half_open_max_calls: usize,
    /// Whether to use error percentage thresholds
    pub use_error_percentage: bool,
    /// Maximum backoff time
    pub max_backoff_time: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            window_size: 100,
            error_threshold: 0.5, // 50% error rate
            minimum_request_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            half_open_success_threshold: 5,
            half_open_max_calls: 10,
            use_error_percentage: true,
            max_backoff_time: Duration::from_secs(60),
        }
    }
}

/// Health metrics for a circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitHealth {
    /// Current state of the circuit
    pub state: CircuitState,
    /// Current error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Total requests in tracking window
    pub request_count: usize,
    /// Number of failures in window
    pub failure_count: usize,
    /// Number of successes in window
    pub success_count: usize,
    /// Time since last state transition
    pub time_in_state: Duration,
    /// Last failure time
    pub last_failure_time: Option<Duration>,
    /// Estimated time until next state change attempt
    pub estimated_time_to_retry: Option<Duration>,
}

/// Advanced implementation of circuit breaker pattern
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Name of this circuit breaker (for metrics and logging)
    name: String,
    /// Circuit configuration
    config: CircuitBreakerConfig,
    /// Circuit breakers by service
    circuits: RwLock<HashMap<String, CircuitStats>>,
    /// Total number of requests processed
    request_count: AtomicUsize,
    /// Total number of successful requests
    success_count: AtomicUsize,
    /// Total number of failed requests
    failure_count: AtomicUsize,
    /// State transition timestamps
    state_transition_times: RwLock<Vec<(Instant, String, CircuitState)>>,
    /// Start time of the circuit breaker
    start_time: Instant,
    /// Last metrics emission time
    last_metrics_time: AtomicU64,
    /// Circuit breaker callback for state changes
    state_change_callback: Option<Arc<dyn Fn(&str, CircuitState, CircuitState, CircuitHealth) + Send + Sync>>,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker with the given name and configuration
    pub fn new<S: Into<String>>(name: S, config: Option<CircuitBreakerConfig>) -> Self {
        let config = config.unwrap_or_default();
        let name = name.into();
        
        Self {
            name: name.clone(),
            config,
            circuits: RwLock::new(HashMap::new()),
            request_count: AtomicUsize::new(0),
            success_count: AtomicUsize::new(0),
            failure_count: AtomicUsize::new(0),
            state_transition_times: RwLock::new(Vec::new()),
            start_time: Instant::now(),
            last_metrics_time: AtomicU64::new(0),
            state_change_callback: None,
        }
    }
    
    /// Sets a callback function to be called on state changes
    pub fn set_state_change_callback<F>(&mut self, callback: F)
    where
        F: Fn(&str, CircuitState, CircuitState, CircuitHealth) + Send + Sync + 'static,
    {
        self.state_change_callback = Some(Arc::new(callback));
    }
    
    /// Executes a function with circuit breaker protection
    pub async fn execute<F, T, E>(&self, service_name: &str, operation: F) -> Result<T>
    where
        F: FnOnce() -> std::result::Result<T, E>,
        E: std::error::Error + 'static,
    {
        // Check if the circuit is open
        if !self.is_allowed(service_name) {
            return Err(Error::new(
                crate::types::ErrorKind::Unavailable, 
                format!("Circuit breaker open for service: {}", service_name)
            ));
        }
        
        // Increment request count
        self.request_count.fetch_add(1, Ordering::Relaxed);
        
        // Execute the operation
        match operation() {
            Ok(result) => {
                // Record success
                self.record_success(service_name);
                Ok(result)
            }
            Err(err) => {
                // Record failure
                self.record_failure(service_name);
                
                Err(Error::new(
                    crate::types::ErrorKind::External,
                    format!("Service call failed: {}", err)
                ).cause(err))
            }
        }
    }
    
    /// Executes an async function with circuit breaker protection
    pub async fn execute_async<F, T, E>(&self, service_name: &str, operation: F) -> Result<T>
    where
        F: std::future::Future<Output = std::result::Result<T, E>>,
        E: std::error::Error + 'static,
    {
        // Check if the circuit is open
        if !self.is_allowed(service_name) {
            return Err(Error::new(
                crate::types::ErrorKind::Unavailable, 
                format!("Circuit breaker open for service: {}", service_name)
            ));
        }
        
        // Increment request count
        self.request_count.fetch_add(1, Ordering::Relaxed);
        
        // Execute the operation
        match operation.await {
            Ok(result) => {
                // Record success
                self.record_success(service_name);
                Ok(result)
            }
            Err(err) => {
                // Record failure
                self.record_failure(service_name);
                
                Err(Error::new(
                    crate::types::ErrorKind::External,
                    format!("Service call failed: {}", err)
                ).cause(err))
            }
        }
    }
    
    /// Checks if a request to the service is allowed
    pub fn is_allowed(&self, service_name: &str) -> bool {
        let circuits = self.circuits.read().unwrap();
        
        match circuits.get(service_name) {
            Some(stats) => {
                match stats.state {
                    CircuitState::Closed => {
                        // Always allow in closed state
                        true
                    }
                    CircuitState::Open => {
                        // Check if reset timeout has elapsed
                        let elapsed = stats.last_state_change.elapsed();
                        if elapsed >= self.config.reset_timeout {
                            // Allow to transition to half-open
                            drop(circuits);
                            self.transition_to_half_open(service_name);
                            true
                        } else {
                            // Calculate time until retry
                            let remaining = self.config.reset_timeout.checked_sub(elapsed)
                                .unwrap_or_default();
                                
                            // Emit metrics occasionally
                            self.emit_metrics(service_name);
                            
                            debug!(
                                circuit = %service_name,
                                remaining_ms = %remaining.as_millis(),
                                "Circuit open, request rejected"
                            );
                            
                            false
                        }
                    }
                    CircuitState::HalfOpen => {
                        // Limited number of requests allowed in half-open
                        let mut circuits = self.circuits.write().unwrap();
                        if let Some(stats) = circuits.get_mut(service_name) {
                            if stats.current_half_open_allowed > 0 {
                                stats.current_half_open_allowed -= 1;
                                true
                            } else {
                                false
                            }
                        } else {
                            true
                        }
                    }
                }
            }
            None => {
                // Initialize new circuit stats for this service
                drop(circuits);
                
                let mut circuits = self.circuits.write().unwrap();
                circuits.insert(service_name.to_string(), CircuitStats {
                    state: CircuitState::Closed,
                    last_state_change: Instant::now(),
                    last_failure: None,
                    last_success: None,
                    window: ResultWindow::new(self.config.window_size),
                    consecutive_successes: 0,
                    current_half_open_allowed: self.config.half_open_max_calls,
                    backoff_multiplier: 1,
                });
                
                true
            }
        }
    }
    
    /// Records a successful call to the service
    pub fn record_success(&self, service_name: &str) {
        let mut circuits = self.circuits.write().unwrap();
        
        // Increment global success counter
        self.success_count.fetch_add(1, Ordering::Relaxed);
        
        // Get or create circuit stats
        let stats = circuits.entry(service_name.to_string()).or_insert_with(|| {
            CircuitStats {
                state: CircuitState::Closed,
                last_state_change: Instant::now(),
                last_failure: None,
                last_success: None,
                window: ResultWindow::new(self.config.window_size),
                consecutive_successes: 0,
                current_half_open_allowed: self.config.half_open_max_calls,
                backoff_multiplier: 1,
            }
        });
        
        // Update last success time
        stats.last_success = Some(Instant::now());
        
        // Add to result window
        stats.window.add_result(true);
        
        // Handle half-open state
        if stats.state == CircuitState::HalfOpen {
            stats.consecutive_successes += 1;
            
            // Check if we should close the circuit
            if stats.consecutive_successes >= self.config.half_open_success_threshold {
                let old_state = stats.state;
                stats.state = CircuitState::Closed;
                stats.last_state_change = Instant::now();
                stats.consecutive_successes = 0;
                stats.backoff_multiplier = 1; // Reset backoff
                
                // Get circuit health for callback
                let health = self.compute_circuit_health(service_name, stats);
                
                // Record state transition
                {
                    let mut transitions = self.state_transition_times.write().unwrap();
                    transitions.push((
                        Instant::now(),
                        service_name.to_string(),
                        CircuitState::Closed,
                    ));
                }
                
                info!(
                    circuit = %service_name,
                    successes = %stats.consecutive_successes,
                    threshold = %self.config.half_open_success_threshold,
                    "Circuit CLOSED: Service recovered"
                );
                
                // Call state change callback if set
                if let Some(callback) = &self.state_change_callback {
                    let callback = callback.clone();
                    let name = service_name.to_string();
                    let health_copy = health.clone();
                    
                    // Execute callback in background task
                    tokio::spawn(async move {
                        callback(&name, old_state, CircuitState::Closed, health_copy);
                    });
                }
                
                // Update metrics
                self.emit_metrics(service_name);
            }
        }
    }
    
    /// Records a failed call to the service
    pub fn record_failure(&self, service_name: &str) {
        let mut circuits = self.circuits.write().unwrap();
        
        // Increment global failure counter
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        
        // Get or create circuit stats
        let stats = circuits.entry(service_name.to_string()).or_insert_with(|| {
            CircuitStats {
                state: CircuitState::Closed,
                last_state_change: Instant::now(),
                last_failure: None,
                last_success: None,
                window: ResultWindow::new(self.config.window_size),
                consecutive_successes: 0,
                current_half_open_allowed: self.config.half_open_max_calls,
                backoff_multiplier: 1,
            }
        });
        
        // Update last failure time
        stats.last_failure = Some(Instant::now());
        
        // Add to result window
        stats.window.add_result(false);
        
        match stats.state {
            CircuitState::Closed => {
                let failure_threshold_reached = if self.config.use_error_percentage {
                    // Using error percentage threshold
                    let window = &stats.window;
                    
                    // Only apply if we have enough samples
                    window.total() >= self.config.minimum_request_threshold
                        && window.failure_rate() >= self.config.error_threshold
                } else {
                    // Using absolute failure count
                    stats.window.failure_count() >= self.config.minimum_request_threshold
                };
                
                // Trip the circuit if threshold reached
                if failure_threshold_reached {
                    let old_state = stats.state;
                    stats.state = CircuitState::Open;
                    stats.last_state_change = Instant::now();
                    stats.consecutive_successes = 0;
                    
                    // Get circuit health for callback
                    let health = self.compute_circuit_health(service_name, stats);
                    
                    // Record state transition
                    {
                        let mut transitions = self.state_transition_times.write().unwrap();
                        transitions.push((
                            Instant::now(),
                            service_name.to_string(),
                            CircuitState::Open,
                        ));
                    }
                    
                    warn!(
                        circuit = %service_name,
                        error_rate = %format!("{:.2}%", stats.window.failure_rate() * 100.0),
                        failure_count = %stats.window.failure_count(),
                        threshold = %format!("{:.2}%", self.config.error_threshold * 100.0),
                        "Circuit OPEN: Failure threshold exceeded"
                    );
                    
                    // Call state change callback if set
                    if let Some(callback) = &self.state_change_callback {
                        let callback = callback.clone();
                        let name = service_name.to_string();
                        let health_copy = health.clone();
                        
                        // Execute callback in background task
                        tokio::spawn(async move {
                            callback(&name, old_state, CircuitState::Open, health_copy);
                        });
                    }
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open state immediately opens the circuit
                let old_state = stats.state;
                stats.state = CircuitState::Open;
                stats.last_state_change = Instant::now();
                stats.consecutive_successes = 0;
                
                // Increase backoff multiplier for exponential backoff
                stats.backoff_multiplier = std::cmp::min(stats.backoff_multiplier * 2, 10);
                
                // Calculate new timeout with exponential backoff
                let backoff_timeout = std::cmp::min(
                    self.config.reset_timeout.mul_f64(stats.backoff_multiplier as f64),
                    self.config.max_backoff_time,
                );
                
                // Get circuit health for callback
                let health = self.compute_circuit_health(service_name, stats);
                
                // Record state transition
                {
                    let mut transitions = self.state_transition_times.write().unwrap();
                    transitions.push((
                        Instant::now(),
                        service_name.to_string(),
                        CircuitState::Open,
                    ));
                }
                
                warn!(
                    circuit = %service_name,
                    backoff_secs = %backoff_timeout.as_secs(),
                    multiplier = %stats.backoff_multiplier,
                    "Circuit REOPENED: Failed in half-open state"
                );
                
                // Call state change callback if set
                if let Some(callback) = &self.state_change_callback {
                    let callback = callback.clone();
                    let name = service_name.to_string();
                    let health_copy = health.clone();
                    
                    // Execute callback in background task
                    tokio::spawn(async move {
                        callback(&name, old_state, CircuitState::Open, health_copy);
                    });
                }
            }
            CircuitState::Open => {
                // Already open, just update metrics
                self.emit_metrics(service_name);
            }
        }
    }
    
    /// Transitions a circuit to half-open state
    fn transition_to_half_open(&self, service_name: &str) {
        let mut circuits = self.circuits.write().unwrap();
        
        if let Some(stats) = circuits.get_mut(service_name) {
            if stats.state == CircuitState::Open &&
               stats.last_state_change.elapsed() >= self.config.reset_timeout {
                let old_state = stats.state;
                stats.state = CircuitState::HalfOpen;
                stats.last_state_change = Instant::now();
                stats.consecutive_successes = 0;
                stats.current_half_open_allowed = self.config.half_open_max_calls;
                
                // Get circuit health for callback
                let health = self.compute_circuit_health(service_name, stats);
                
                // Record state transition
                {
                    let mut transitions = self.state_transition_times.write().unwrap();
                    transitions.push((
                        Instant::now(),
                        service_name.to_string(),
                        CircuitState::HalfOpen,
                    ));
                }
                
                info!(
                    circuit = %service_name,
                    max_test_calls = %stats.current_half_open_allowed,
                    success_threshold = %self.config.half_open_success_threshold,
                    "Circuit HALF-OPEN: Testing service recovery"
                );
                
                // Call state change callback if set
                if let Some(callback) = &self.state_change_callback {
                    let callback = callback.clone();
                    let name = service_name.to_string();
                    let health_copy = health.clone();
                    
                    // Execute callback in background task
                    tokio::spawn(async move {
                        callback(&name, old_state, CircuitState::HalfOpen, health_copy);
                    });
                }
            }
        }
    }
    
    /// Gets the current state of a circuit
    pub fn get_state(&self, service_name: &str) -> CircuitState {
        let circuits = self.circuits.read().unwrap();
        
        circuits.get(service_name)
            .map(|stats| stats.state)
            .unwrap_or(CircuitState::Closed)
    }
    
    /// Gets health metrics for a circuit
    pub fn get_health(&self, service_name: &str) -> CircuitHealth {
        let circuits = self.circuits.read().unwrap();
        
        if let Some(stats) = circuits.get(service_name) {
            self.compute_circuit_health(service_name, stats)
        } else {
            // Default health for unknown circuit
            CircuitHealth {
                state: CircuitState::Closed,
                error_rate: 0.0,
                request_count: 0,
                failure_count: 0,
                success_count: 0,
                time_in_state: Duration::from_secs(0),
                last_failure_time: None,
                estimated_time_to_retry: None,
            }
        }
    }
    
    /// Reset a circuit to closed state
    pub fn reset(&self, service_name: &str) {
        let mut circuits = self.circuits.write().unwrap();
        
        if let Some(stats) = circuits.get_mut(service_name) {
            let old_state = stats.state;
            stats.state = CircuitState::Closed;
            stats.last_state_change = Instant::now();
            stats.window.clear();
            stats.consecutive_successes = 0;
            stats.current_half_open_allowed = self.config.half_open_max_calls;
            stats.backoff_multiplier = 1;
            
            info!(
                circuit = %service_name,
                previous_state = %old_state,
                "Circuit manually reset to CLOSED state"
            );
            
            // Record state transition
            {
                let mut transitions = self.state_transition_times.write().unwrap();
                transitions.push((
                    Instant::now(),
                    service_name.to_string(),
                    CircuitState::Closed,
                ));
            }
            
            // Call state change callback if set
            if let Some(callback) = &self.state_change_callback {
                let callback = callback.clone();
                let name = service_name.to_string();
                let health = self.compute_circuit_health(service_name, stats);
                
                // Execute callback in background task
                tokio::spawn(async move {
                    callback(&name, old_state, CircuitState::Closed, health);
                });
            }
        }
    }
    
    /// Resets all circuits to closed state
    pub fn reset_all(&self) {
        let mut circuits = self.circuits.write().unwrap();
        
        for (service_name, stats) in circuits.iter_mut() {
            let old_state = stats.state;
            stats.state = CircuitState::Closed;
            stats.last_state_change = Instant::now();
            stats.window.clear();
            stats.consecutive_successes = 0;
            stats.current_half_open_allowed = self.config.half_open_max_calls;
            stats.backoff_multiplier = 1;
            
            info!(
                circuit = %service_name,
                previous_state = %old_state,
                "Circuit manually reset to CLOSED state"
            );
            
            // Record state transition
            {
                let mut transitions = self.state_transition_times.write().unwrap();
                transitions.push((
                    Instant::now(),
                    service_name.to_string(),
                    CircuitState::Closed,
                ));
            }
            
            // Call state change callback if set
            if let Some(callback) = &self.state_change_callback {
                let callback = callback.clone();
                let name = service_name.to_string();
                let health = self.compute_circuit_health(service_name, stats);
                
                // Execute callback in background task
                tokio::spawn(async move {
                    callback(&name, old_state, CircuitState::Closed, health.clone());
                });
            }
        }
    }
    
    /// Gets a list of all circuit names
    pub fn get_circuit_names(&self) -> Vec<String> {
        let circuits = self.circuits.read().unwrap();
        circuits.keys().cloned().collect()
    }
    
    /// Gets health metrics for all circuits
    pub fn get_all_health(&self) -> HashMap<String, CircuitHealth> {
        let circuits = self.circuits.read().unwrap();
        let mut health_map = HashMap::with_capacity(circuits.len());
        
        for (name, stats) in circuits.iter() {
            health_map.insert(name.clone(), self.compute_circuit_health(name, stats));
        }
        
        health_map
    }
    
    /// Gets state transition history
    pub fn get_state_transitions(&self) -> Vec<(chrono::DateTime<chrono::Utc>, String, CircuitState)> {
        let transitions = self.state_transition_times.read().unwrap();
        
        transitions.iter()
            .map(|(time, service, state)| {
                // Convert Instant to DateTime
                let elapsed = time.elapsed();
                let now = chrono::Utc::now();
                let timestamp = now - chrono::Duration::from_std(elapsed).unwrap_or_default();
                
                (timestamp, service.clone(), *state)
            })
            .collect()
    }
    
    /// Emits metrics about the circuit breaker
    fn emit_metrics(&self, service_name: &str) {
        let now = Instant::now().elapsed().as_secs();
        let last = self.last_metrics_time.load(Ordering::Relaxed);
        
        // Only emit metrics every 5 seconds to avoid flooding
        if now - last < 5 {
            return;
        }
        
        // Update last metrics time
        self.last_metrics_time.store(now, Ordering::Relaxed);
        
        // Emit metrics
        let circuits = self.circuits.read().unwrap();
        
        if let Some(stats) = circuits.get(service_name) {
            let prefix = format!("circuit_breaker.{}.{}", self.name, service_name);
            
            // Circuit state (0=closed, 1=open, 2=half-open)
            let state_value = match stats.state {
                CircuitState::Closed => 0.0,
                CircuitState::Open => 1.0,
                CircuitState::HalfOpen => 2.0,
            };
            gauge!(&format!("{}.state", prefix), state_value);
            
            // Error rate
            gauge!(&format!("{}.error_rate", prefix), stats.window.failure_rate());
            
            // Sample count
            gauge!(&format!("{}.sample_count", prefix), stats.window.total() as f64);
            
            // Failure count
            gauge!(&format!("{}.failure_count", prefix), stats.window.failure_count() as f64);
            
            // Time in state
            gauge!(&format!("{}.time_in_state_ms", prefix), stats.last_state_change.elapsed().as_millis() as f64);
            
            // Consecutive successes in half-open
            if stats.state == CircuitState::HalfOpen {
                gauge!(&format!("{}.consecutive_successes", prefix), stats.consecutive_successes as f64);
            }
        }
        
        // Global metrics
        gauge!("circuit_breaker.total_requests", self.request_count.load(Ordering::Relaxed) as f64);
        gauge!("circuit_breaker.success_count", self.success_count.load(Ordering::Relaxed) as f64);
        gauge!("circuit_breaker.failure_count", self.failure_count.load(Ordering::Relaxed) as f64);
        gauge!("circuit_breaker.uptime_seconds", self.start_time.elapsed().as_secs() as f64);
    }
    
    /// Computes health metrics for a circuit
    fn compute_circuit_health(&self, service_name: &str, stats: &CircuitStats) -> CircuitHealth {
        let time_in_state = stats.last_state_change.elapsed();
        let estimated_time_to_retry = if stats.state == CircuitState::Open {
            let remaining = self.config.reset_timeout.checked_sub(time_in_state);
            remaining
        } else {
            None
        };
        
        let last_failure_time = stats.last_failure.map(|time| time.elapsed());
        
        CircuitHealth {
            state: stats.state,
            error_rate: stats.window.failure_rate(),
            request_count: stats.window.total(),
            failure_count: stats.window.failure_count(),
            success_count: stats.window.success_count(),
            time_in_state,
            last_failure_time,
            estimated_time_to_retry,
        }
    }
    
    /// Creates a default global circuit breaker
    pub fn global() -> Arc<Self> {
        static mut GLOBAL_BREAKER: Option<Arc<CircuitBreaker>> = None;
        static GLOBAL_INIT: std::sync::Once = std::sync::Once::new();
        
        unsafe {
            GLOBAL_INIT.call_once(|| {
                let breaker = CircuitBreaker::new("global", None);
                GLOBAL_BREAKER = Some(Arc::new(breaker));
            });
            
            GLOBAL_BREAKER.clone().unwrap()
        }
    }
}

/// Default implementation
impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new("default", None)
    }
}

/// Circuit breaker guard for executing code with circuit breaker protection
pub struct CircuitBreakerGuard<'a> {
    circuit_breaker: &'a CircuitBreaker,
    service_name: String,
}

impl<'a> CircuitBreakerGuard<'a> {
    /// Creates a new circuit breaker guard
    pub fn new<S: Into<String>>(circuit_breaker: &'a CircuitBreaker, service_name: S) -> Self {
        Self {
            circuit_breaker,
            service_name: service_name.into(),
        }
    }
    
    /// Executes a function with circuit breaker protection
    pub async fn execute<F, T, E>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> std::result::Result<T, E>,
        E: std::error::Error + 'static,
    {
        self.circuit_breaker.execute(&self.service_name, operation).await
    }
    
    /// Executes an async function with circuit breaker protection
    pub async fn execute_async<F, T, E>(&self, operation: F) -> Result<T>
    where
        F: std::future::Future<Output = std::result::Result<T, E>>,
        E: std::error::Error + 'static,
    {
        self.circuit_breaker.execute_async(&self.service_name, operation).await
    }
}

/// Executes a function with circuit breaker protection
pub async fn with_circuit_breaker<F, T, E>(
    service_name: &str,
    operation: F,
) -> Result<T>
where
    F: FnOnce() -> std::result::Result<T, E>,
    E: std::error::Error + 'static,
{
    CircuitBreaker::global().execute(service_name, operation).await
}

/// Executes an async function with circuit breaker protection
pub async fn with_circuit_breaker_async<F, T, E>(
    service_name: &str,
    operation: F,
) -> Result<T>
where
    F: std::future::Future<Output = std::result::Result<T, E>>,
    E: std::error::Error + 'static,
{
    CircuitBreaker::global().execute_async(service_name, operation).await
}

/// Helper for sleeping with jitter to avoid thundering herd
pub async fn sleep_with_jitter(base_duration: Duration, jitter_factor: f64) -> Duration {
    use rand::Rng;
    
    // Calculate jittered duration
    let mut rng = rand::thread_rng();
    let jitter = rng.gen_range(0.0..jitter_factor);
    let duration = base_duration.mul_f64(1.0 + jitter);
    
    // Sleep
    sleep(duration).await;
    
    duration
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::time::{sleep, timeout};
    
    #[tokio::test]
    async fn test_circuit_breaker_basic() {
        let cb = CircuitBreaker::new("test", Some(CircuitBreakerConfig {
            window_size: 10,
            error_threshold: 0.5,
            minimum_request_threshold: 3,
            reset_timeout: Duration::from_millis(100),
            half_open_success_threshold: 2,
            half_open_max_calls: 2,
            use_error_percentage: true,
            max_backoff_time: Duration::from_secs(1),
        }));
        
        let service = "test-service";
        
        // Initially closed
        assert_eq!(cb.get_state(service), CircuitState::Closed);
        
        // Record failures to trip circuit
        for _ in 0..5 {
            cb.record_failure(service);
        }
        
        // Should be open now
        assert_eq!(cb.get_state(service), CircuitState::Open);
        
        // Wait for timeout
        sleep(Duration::from_millis(150)).await;
        
        // Should be half-open after is_allowed check
        assert!(cb.is_allowed(service));
        assert_eq!(cb.get_state(service), CircuitState::HalfOpen);
        
        // Record successes to close circuit
        cb.record_success(service);
        cb.record_success(service);
        
        // Should be closed again
        assert_eq!(cb.get_state(service), CircuitState::Closed);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_execute() {
        let cb = CircuitBreaker::new("test", Some(CircuitBreakerConfig {
            window_size: 10,
            error_threshold: 0.5,
            minimum_request_threshold: 3,
            reset_timeout: Duration::from_millis(100),
            half_open_success_threshold: 2,
            half_open_max_calls: 2,
            use_error_percentage: true,
            max_backoff_time: Duration::from_secs(1),
        }));
        
        let service = "test-service";
        
        // Execute successful operation
        let result = cb.execute(service, || Ok::<_, std::io::Error>(42)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        
        // Execute failing operations to trip circuit
        for _ in 0..5 {
            let _ = cb.execute(service, || Err::<i32, _>(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "Connection refused"
            ))).await;
        }
        
        // Circuit should be open
        assert_eq!(cb.get_state(service), CircuitState::Open);
        
        // Try to execute again, should fail fast
        let result = cb.execute(service, || Ok::<_, std::io::Error>(42)).await;
        assert!(result.is_err());
        
        // Wait for timeout
        sleep(Duration::from_millis(150)).await;
        
        // Should work again in half-open state
        let result = cb.execute(service, || Ok::<_, std::io::Error>(42)).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_health() {
        let cb = CircuitBreaker::new("test", None);
        let service = "test-service";
        
        // Initial health
        let health = cb.get_health(service);
        assert_eq!(health.state, CircuitState::Closed);
        assert_eq!(health.error_rate, 0.0);
        
        // Record some operations
        cb.record_success(service);
        cb.record_success(service);
        cb.record_failure(service);
        
        // Check updated health
        let health = cb.get_health(service);
        assert_eq!(health.state, CircuitState::Closed);
        assert_eq!(health.request_count, 3);
        assert_eq!(health.success_count, 2);
        assert_eq!(health.failure_count, 1);
        assert!(health.error_rate > 0.0);
    }
}