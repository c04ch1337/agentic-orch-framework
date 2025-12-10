//! # Enhanced Circuit Breaker for Data Router
//!
//! This module provides an advanced circuit breaker implementation
//! for resilient service communication in the Data Router service.
//! It wraps the core functionality from error-handling-rs.

use error_handling_rs::circuit_breaker::{
    CircuitBreaker as CoreCircuitBreaker, 
    CircuitBreakerConfig,
    CircuitState as CoreCircuitState,
    CircuitHealth,
};

use std::sync::Arc;
use metrics::{counter, gauge};
use std::time::Duration;
use log;

// Re-export the CircuitState enum with the same variants as before
// for backward compatibility with existing code
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Blocking all requests
    HalfOpen,    // Testing if service recovered
}

// Conversion between our CircuitState and the core library's CircuitState
impl From<CoreCircuitState> for CircuitState {
    fn from(state: CoreCircuitState) -> Self {
        match state {
            CoreCircuitState::Closed => CircuitState::Closed,
            CoreCircuitState::Open => CircuitState::Open,
            CoreCircuitState::HalfOpen => CircuitState::HalfOpen,
        }
    }
}

/// Enhanced circuit breaker wrapping the core implementation
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Core implementation from error-handling-rs
    core: Arc<CoreCircuitBreaker>,
    /// Service name for metrics
    service_name: String,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker with default settings
    pub fn new() -> Self {
        // Create a configuration with appropriate thresholds
        let config = CircuitBreakerConfig {
            window_size: 100,                   // Track last 100 requests
            error_threshold: 0.5,               // 50% error rate to trip
            minimum_request_threshold: 5,       // Need at least 5 requests before considering
            reset_timeout: Duration::from_secs(30), // 30s timeout in open state
            half_open_success_threshold: 3,     // 3 successful requests to close circuit
            half_open_max_calls: 1,             // Allow 1 call in half-open state
            use_error_percentage: true,         // Use percentage threshold
            max_backoff_time: Duration::from_secs(60), // Max 60s backoff
        };
        
        // Create the core circuit breaker with dashboard monitoring enabled
        let core = CoreCircuitBreaker::new("data-router", Some(config));
        
        // Add health state change callback
        let mut core = core.clone();
        core.set_state_change_callback(|service, old_state, new_state, health| {
            // Log state change
            log::info!(
                "Circuit state change for {}: {} -> {}",
                service,
                old_state,
                new_state
            );
            
            // Record metrics
            gauge!(
                &format!("circuit_breaker.{}.state", service),
                match new_state {
                    CoreCircuitState::Closed => 0.0,
                    CoreCircuitState::Open => 1.0,
                    CoreCircuitState::HalfOpen => 0.5,
                }
            );
            
            // Record error rate
            gauge!(&format!("circuit_breaker.{}.error_rate", service), health.error_rate);
        });
        
        Self {
            core: Arc::new(core),
            service_name: "data-router".to_string(),
        }
    }
    
    /// Creates a circuit breaker with a custom configuration
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        let core = CoreCircuitBreaker::new("data-router", Some(config));
        
        Self {
            core: Arc::new(core),
            service_name: "data-router".to_string(),
        }
    }
    
    /// Check if a request to a service is allowed
    pub fn is_allowed(&self, service_name: &str) -> bool {
        // Delegate to core implementation
        self.core.is_allowed(service_name)
    }

    /// Record a successful call
    pub fn record_success(&self, service_name: &str) {
        // Delegate to core implementation
        self.core.record_success(service_name);
        
        // Record metric
        counter!(&format!("circuit_breaker.{}.success", service_name), 1);
    }

    /// Record a failed call
    pub fn record_failure(&self, service_name: &str) {
        // Delegate to core implementation
        self.core.record_failure(service_name);
        
        // Record metric
        counter!(&format!("circuit_breaker.{}.failure", service_name), 1);
    }

    /// Get current state of a service's circuit
    pub fn get_state(&self, service_name: &str) -> CircuitState {
        // Get state from core implementation and convert
        let core_state = self.core.get_state(service_name);
        core_state.into()
    }

    /// Get statistics for a service
    pub fn get_stats(&self, service_name: &str) -> Option<(CircuitState, u32, u32)> {
        // Get health from core implementation
        let health = self.core.get_health(service_name);
        
        // Convert to the format expected by existing code
        Some((
            CircuitState::from(health.state),
            health.failure_count as u32,
            health.success_count as u32,
        ))
    }
    
    /// Get detailed health for a service
    pub fn get_health(&self, service_name: &str) -> CircuitHealth {
        self.core.get_health(service_name)
    }
    
    /// Execute an async function with circuit breaker protection
    pub async fn execute<F, T, E>(&self, service_name: &str, operation: F) -> Result<T, error_handling_rs::types::Error>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::error::Error + 'static,
    {
        self.core.execute_async(service_name, operation).await
    }
    
    /// Reset a circuit to closed state (for testing/admin purposes)
    pub fn reset(&self, service_name: &str) {
        self.core.reset(service_name);
        log::info!("Circuit manually reset for service: {}", service_name);
    }
    
    /// Gets all circuit names being tracked
    pub fn get_circuit_names(&self) -> Vec<String> {
        self.core.get_circuit_names()
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

// Additional utility functions for the Data Router service

/// Creates a standardized error response for circuit open situations
pub fn create_circuit_open_error(service_name: &str) -> tonic::Status {
    tonic::Status::unavailable(format!(
        "Service {} is temporarily unavailable (circuit open)",
        service_name
    ))
}

/// Helper to create a protected service client with circuit breaker
pub struct ProtectedServiceClient<T> {
    client: Option<T>,
    service_name: String,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl<T> ProtectedServiceClient<T> {
    pub fn new(service_name: &str, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            client: None,
            service_name: service_name.to_string(),
            circuit_breaker,
        }
    }
    
    pub fn set_client(&mut self, client: T) {
        self.client = Some(client);
    }
    
    pub fn get_client(&self) -> Option<&T> {
        self.client.as_ref()
    }
    
    pub fn is_allowed(&self) -> bool {
        self.circuit_breaker.is_allowed(&self.service_name)
    }
    
    pub fn record_success(&self) {
        self.circuit_breaker.record_success(&self.service_name);
    }
    
    pub fn record_failure(&self) {
        self.circuit_breaker.record_failure(&self.service_name);
    }
}
