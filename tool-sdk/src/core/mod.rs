//! Core abstractions for the Tool SDK
//!
//! This module provides the fundamental trait interfaces that all service
//! clients will implement or use:
//!
//! - `ServiceClient`: The base trait for all service clients
//! - `RequestExecutor`: Handles actual HTTP requests
//! - `AuthenticatedClient`: Adds authentication capabilities
//! - `RateLimited`: Adds rate limiting capabilities
//! - `Telemetry`: Adds observability/metrics 
//! - `ClientBuilder`: Builder pattern for creating clients

pub mod builder;
pub use builder::ClientBuilder;

use async_trait::async_trait;
use std::future::Future;
use std::time::Duration;
use std::fmt::Debug;
use std::collections::HashMap;
use serde::{Serialize, de::DeserializeOwned};

use crate::error::Result;

/// Base trait for all service clients
#[async_trait]
pub trait ServiceClient: Send + Sync {
    /// The client name/identifier
    fn name(&self) -> &str;
    
    /// The base URL for the service
    fn base_url(&self) -> &str;
    
    /// Service version
    fn version(&self) -> &str;
    
    /// Health check for the service
    async fn health_check(&self) -> Result<bool>;
    
    /// Returns the client's metrics and telemetry if available
    fn metrics(&self) -> Option<HashMap<String, String>>;
}

/// Trait responsible for executing HTTP requests with strong typing
#[async_trait]
pub trait RequestExecutor: Send + Sync {
    /// Execute a request that returns a response of type R
    async fn execute<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: DeserializeOwned + Send;
    
    /// Execute a GET request
    async fn get<R>(&self, endpoint: &str, query_params: Option<HashMap<String, String>>) -> Result<R>
    where
        R: DeserializeOwned + Send;
    
    /// Execute a POST request
    async fn post<T, R>(&self, endpoint: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: DeserializeOwned + Send;
    
    /// Execute a PUT request
    async fn put<T, R>(&self, endpoint: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: DeserializeOwned + Send;
    
    /// Execute a DELETE request
    async fn delete<R>(&self, endpoint: &str) -> Result<R>
    where
        R: DeserializeOwned + Send;
}

/// Trait for clients that require authentication
#[async_trait]
pub trait AuthenticatedClient: Send + Sync {
    /// Authentication type (e.g., "Bearer", "ApiKey")
    fn auth_type(&self) -> &str;
    
    /// Set authentication credentials
    fn set_auth(&mut self, auth: impl Into<String> + Send) -> Result<()>;
    
    /// Check if client is authenticated
    fn is_authenticated(&self) -> bool;
    
    /// Refresh authentication credentials if needed
    async fn refresh_auth(&mut self) -> Result<()>;
    
    /// Add authentication headers to a request
    fn apply_auth(&self, headers: &mut HashMap<String, String>) -> Result<()>;
}

/// Trait for clients with rate limiting capabilities
#[async_trait]
pub trait RateLimited: Send + Sync {
    /// Get current rate limit status
    fn rate_limit_status(&self) -> Option<RateLimitStatus>;
    
    /// Set rate limiting configuration
    fn configure_rate_limit(&mut self, max_requests: u32, period: Duration);
    
    /// Check if a request would exceed rate limits
    async fn check_rate_limit(&self) -> Result<bool>;
    
    /// Record a request for rate limiting purposes
    fn record_request(&self);
}

/// Rate limit status information
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    /// Maximum requests allowed in the period
    pub max_requests: u32,
    
    /// Time period for the rate limit
    pub period: Duration,
    
    /// Current request count in this period
    pub current_count: u32,
    
    /// Time until rate limit resets
    pub reset_after: Duration,
    
    /// Whether the rate limit is currently being enforced
    pub enforced: bool,
}

/// Trait for clients that support telemetry
#[async_trait]
pub trait Telemetry: Send + Sync {
    /// Record a request event with timing
    fn record_request(&self, endpoint: &str, status: u16, duration: Duration);
    
    /// Record an error event
    fn record_error(&self, endpoint: &str, error: &str);
    
    /// Get current metrics
    fn metrics(&self) -> HashMap<String, String>;
    
    /// Reset metrics
    fn reset_metrics(&mut self);
}