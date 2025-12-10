//! Client builder implementation
//!
//! Provides a unified builder pattern for creating and configuring service clients.

use std::collections::HashMap;
use std::time::Duration;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client as ReqwestClient, ClientBuilder as ReqwestBuilder};
use std::str::FromStr;

use crate::error::{Result, ServiceError};
use crate::resilience::{RetryConfig, CircuitBreakerConfig, Resilience};

/// Unified client builder for all service clients
pub struct ClientBuilder {
    /// Base URL for the service
    base_url: Option<String>,
    
    /// Authentication token or key
    auth_token: Option<String>,
    
    /// Authentication type (Bearer, ApiKey, etc.)
    auth_type: Option<String>,
    
    /// Custom headers to include with all requests
    custom_headers: HashMap<String, String>,
    
    /// Request timeout
    timeout: Option<Duration>,
    
    /// Retry configuration
    retry_config: Option<RetryConfig>,
    
    /// Circuit breaker configuration
    circuit_breaker_config: Option<CircuitBreakerConfig>,
    
    /// Rate limit configuration
    max_requests: Option<u32>,
    
    /// Rate limit period
    rate_limit_period: Option<Duration>,
    
    /// User agent
    user_agent: Option<String>,
    
    /// Enable request compression
    compression: bool,
    
    /// Enable metrics collection
    metrics_enabled: bool,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            base_url: None,
            auth_token: None,
            auth_type: None,
            custom_headers: HashMap::new(),
            timeout: Some(Duration::from_secs(30)), // Default 30s timeout
            retry_config: Some(RetryConfig::default()),
            circuit_breaker_config: Some(CircuitBreakerConfig::default()),
            max_requests: None,
            rate_limit_period: None,
            user_agent: Some("Phoenix-Tool-SDK/0.1.0".to_string()),
            compression: true,
            metrics_enabled: true,
        }
    }
}

impl ClientBuilder {
    /// Create a new client builder with default settings
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the base URL for the service
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }
    
    /// Set authentication token/key
    pub fn auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }
    
    /// Set authentication type
    pub fn auth_type(mut self, auth_type: impl Into<String>) -> Self {
        self.auth_type = Some(auth_type.into());
        self
    }
    
    /// Add a custom header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_headers.insert(key.into(), value.into());
        self
    }
    
    /// Set request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
    
    /// Configure retry behavior
    pub fn retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = Some(config);
        self
    }
    
    /// Configure circuit breaker
    pub fn circuit_breaker(mut self, config: CircuitBreakerConfig) -> Self {
        self.circuit_breaker_config = Some(config);
        self
    }
    
    /// Configure rate limiting
    pub fn rate_limit(mut self, max_requests: u32, period: Duration) -> Self {
        self.max_requests = Some(max_requests);
        self.rate_limit_period = Some(period);
        self
    }
    
    /// Set user agent
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }
    
    /// Enable or disable compression
    pub fn compression(mut self, enabled: bool) -> Self {
        self.compression = enabled;
        self
    }
    
    /// Enable or disable metrics collection
    pub fn metrics(mut self, enabled: bool) -> Self {
        self.metrics_enabled = enabled;
        self
    }
    
    /// Build an HTTP client with the configured settings
    pub fn build_http_client(&self) -> Result<ReqwestClient> {
        let mut builder = ReqwestClient::builder();
        
        // Apply timeout if set
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }
        
        // Set user agent
        if let Some(ref user_agent) = self.user_agent {
            builder = builder.user_agent(user_agent);
        }
        
        // Configure compression
        builder = builder.gzip(self.compression);
        
        // Build default headers
        let mut headers = HeaderMap::new();
        for (key, value) in &self.custom_headers {
            let header_name = HeaderName::from_str(key)
                .map_err(|e| ServiceError::configuration(format!("Invalid header name: {}", e)))?;
            
            let header_value = HeaderValue::from_str(value)
                .map_err(|e| ServiceError::configuration(format!("Invalid header value: {}", e)))?;
            
            headers.insert(header_name, header_value);
        }
        
        // Add authentication header if provided
        if let (Some(ref token), Some(ref auth_type)) = (&self.auth_token, &self.auth_type) {
            let auth_header_value = match auth_type.as_str() {
                "Bearer" => format!("Bearer {}", token),
                "ApiKey" => token.clone(),
                _ => format!("{} {}", auth_type, token),
            };
            
            headers.insert(
                reqwest::header::AUTHORIZATION,
                HeaderValue::from_str(&auth_header_value)
                    .map_err(|e| ServiceError::configuration(format!("Invalid auth header: {}", e)))?,
            );
        }
        
        builder = builder.default_headers(headers);
        
        // Build the client
        builder.build()
            .map_err(|e| ServiceError::configuration(format!("Failed to build HTTP client: {}", e)))
    }
    
    /// Build a resilience facade with the configured settings
    pub fn build_resilience(&self) -> Result<Resilience> {
        let retry_config = self.retry_config.clone().unwrap_or_default();
        let circuit_breaker_config = self.circuit_breaker_config.clone().unwrap_or_default();
        
        Ok(Resilience::new(retry_config, circuit_breaker_config))
    }
}