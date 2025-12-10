# Extending the Tool SDK

This guide provides detailed instructions for extending the Tool SDK with new service integrations, enhancing error handling capabilities, and implementing additional resilience features.

## Table of Contents

- [Adding New Service Integrations](#adding-new-service-integrations)
- [Enhancing Error Handling](#enhancing-error-handling)
- [Implementing New Resilience Features](#implementing-new-resilience-features)
- [Testing Extensions](#testing-extensions)
- [Contribution Guidelines](#contribution-guidelines)

## Adding New Service Integrations

### Overview

The Tool SDK is designed to be easily extended with new service integrations. This section walks through the process of adding a new service client to the SDK.

### Step 1: Create Service-Specific Module

First, create a new module under `src/services/` for your service:

```
src/
└── services/
    └── my_service/
        ├── mod.rs        # Main module file
        └── models.rs     # Service-specific data models
```

### Step 2: Define Service-Specific Models

In `models.rs`, define the request and response types for the service:

```rust
use serde::{Deserialize, Serialize};

/// Request for MyService API
#[derive(Debug, Clone, Serialize)]
pub struct MyServiceRequest {
    pub query: String,
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

/// Response from MyService API
#[derive(Debug, Clone, Deserialize)]
pub struct MyServiceResponse {
    pub results: Vec<MyServiceResult>,
    pub count: u32,
    pub metadata: Option<MyServiceMetadata>,
}

/// Single result item from MyService
#[derive(Debug, Clone, Deserialize)]
pub struct MyServiceResult {
    pub id: String,
    pub title: String,
    pub content: String,
}

/// Metadata for MyService response
#[derive(Debug, Clone, Deserialize)]
pub struct MyServiceMetadata {
    pub total: u32,
    pub page: u32,
}

impl Default for MyServiceRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            limit: None,
            filter: None,
        }
    }
}
```

### Step 3: Implement Service-Specific Configuration

Create a configuration struct for your service in `src/config/mod.rs`:

```rust
/// Configuration for MyService
#[derive(Debug, Clone)]
pub struct MyServiceConfig {
    /// API key for authentication
    pub api_key: String,
    
    /// Base URL for the API
    pub base_url: String,
    
    /// Timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for MyServiceConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.myservice.com".to_string(),
            timeout_seconds: 30,
        }
    }
}

impl MyServiceConfig {
    /// Load configuration from a provider
    pub fn from_provider(provider: &dyn ConfigProvider) -> Result<Self> {
        let api_key = provider.get_string_or("API_KEY", "");
        let base_url = provider.get_string_or("BASE_URL", "https://api.myservice.com");
        let timeout = provider.get_u64_or("TIMEOUT_SECONDS", 30);
        
        Ok(Self {
            api_key,
            base_url,
            timeout_seconds: timeout,
        })
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(ServiceError::configuration("MyService API key is required"));
        }
        
        Ok(())
    }
}
```

### Step 4: Implement the Service Client

In `src/services/my_service/mod.rs`, implement the service client:

```rust
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::core::{ServiceClient, RequestExecutor, AuthenticatedClient, RateLimited, Telemetry, RateLimitStatus};
use crate::error::{Result, ServiceError, ErrorContext};
use crate::config::{MyServiceConfig, ConfigProvider, DEFAULT_PROVIDER};
use crate::resilience::{Resilience, RetryConfig, CircuitBreakerConfig};
use crate::services::common::{UserAgent, build_http_client, parse_error_response, record_request_metrics};

pub use super::models::*;

/// MyService API client
pub struct MyServiceClient {
    /// HTTP client
    http_client: reqwest::Client,
    
    /// Configuration
    config: MyServiceConfig,
    
    /// Resilience patterns
    resilience: Resilience,
    
    /// Rate limit status
    rate_limits: Arc<Mutex<Option<RateLimitStatus>>>,
    
    /// Client metrics
    metrics: Mutex<HashMap<String, String>>,
}

impl Default for MyServiceClient {
    fn default() -> Self {
        let config = MyServiceConfig::from_provider(&**DEFAULT_PROVIDER)
            .unwrap_or_else(|_| {
                log::warn!("Failed to load MyService config from environment, using defaults");
                MyServiceConfig::default()
            });
        
        Self::new_with_config(config)
    }
}

impl MyServiceClient {
    /// Create a new MyService client with default configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new MyService client with custom configuration
    pub fn new_with_config(config: MyServiceConfig) -> Self {
        let timeout = Duration::from_secs(config.timeout_seconds);
        
        let http_client = build_http_client(
            Some(UserAgent {
                app_name: "Phoenix-ORCH".to_string(),
                version: "0.1.0".to_string(),
                extra: Some("MyService-Client".to_string()),
            }),
            Some(timeout),
        ).unwrap_or_else(|e| {
            log::error!("Failed to build MyService HTTP client: {}", e);
            panic!("Failed to build MyService HTTP client: {}", e);
        });
        
        let resilience = Resilience::new(
            RetryConfig {
                max_retries: 3,
                initial_interval: Duration::from_millis(500),
                max_interval: Duration::from_secs(10),
                ..RetryConfig::default()
            },
            CircuitBreakerConfig {
                failure_threshold: 5,
                reset_timeout: Duration::from_secs(60),
                ..CircuitBreakerConfig::default()
            },
        );
        
        Self {
            http_client,
            config,
            resilience,
            rate_limits: Arc::new(Mutex::new(None)),
            metrics: Mutex::new(HashMap::new()),
        }
    }
    
    /// Create a new builder for the MyService client
    pub fn builder() -> MyServiceClientBuilder {
        MyServiceClientBuilder::default()
    }
    
    /// Execute a search request
    pub async fn search(&self, request: MyServiceRequest) -> Result<MyServiceResponse> {
        self.execute_request("search", &request).await
    }
    
    /// Simple search with just a query string
    pub async fn simple_search(&self, query: &str) -> Result<Vec<MyServiceResult>> {
        let request = MyServiceRequest {
            query: query.to_string(),
            limit: Some(10),
            filter: None,
        };
        
        let response = self.search(request).await?;
        Ok(response.results)
    }
    
    // Private helper methods
    
    async fn execute_request<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
    where
        T: serde::Serialize + Send + Sync,
        R: for<'de> serde::de::DeserializeOwned + Send,
    {
        // Use the resilience facade to retry on certain errors
        self.resilience.execute(move || {
            self.execute_with_client(endpoint, request)
        }).await
    }
    
    async fn execute_with_client<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
    where
        T: serde::Serialize + Send + Sync,
        R: for<'de> serde::de::DeserializeOwned + Send,
    {
        // Check rate limits before proceeding
        self.check_rate_limit().await?;
        
        // Record the request for rate limiting
        self.record_request();
        
        let url = format!("{}/{}", self.config.base_url, endpoint);
        log::debug!("Sending request to MyService: POST {}", url);
        
        let start_time = Instant::now();
        let mut auth_headers = HashMap::new();
        self.apply_auth(&mut auth_headers)?;
        
        let request_json = serde_json::to_string(request)
            .map_err(|e| ServiceError::validation(format!("Failed to serialize request: {}", e)))?;
        
        let bytes_sent = request_json.len() as u64;
        
        let mut builder = self.http_client.post(&url);
        
        // Add authentication headers
        for (key, value) in &auth_headers {
            builder = builder.header(key, value);
        }
        
        let response = builder
            .header("Content-Type", "application/json")
            .body(request_json)
            .send()
            .await
            .map_err(|e| ServiceError::network(format!("Failed to send request: {}", e)))?;
        
        let status = response.status();
        let status_code = status.as_u16();
        
        if status.is_success() {
            let bytes_received = response.content_length().unwrap_or(0);
            
            let json = response.json::<R>().await
                .map_err(|e| ServiceError::parsing(format!("Failed to parse response: {}", e)))?;
            
            let duration = start_time.elapsed();
            
            // Record metrics
            record_request_metrics(
                "my_service",
                endpoint,
                start_time,
                status_code,
                true,
                Some(bytes_sent),
                Some(bytes_received),
            );
            
            self.record_request(endpoint, status_code, duration);
            
            Ok(json)
        } else {
            let error = parse_error_response("my_service", response).await;
            
            // Record error metrics
            self.record_error(endpoint, &error.to_string());
            
            Err(error)
        }
    }
}

#[async_trait]
impl ServiceClient for MyServiceClient {
    fn name(&self) -> &str {
        "my_service"
    }
    
    fn base_url(&self) -> &str {
        &self.config.base_url
    }
    
    fn version(&self) -> &str {
        "v1"
    }
    
    async fn health_check(&self) -> Result<bool> {
        // Implement a health check for the service
        match self.simple_search("test").await {
            Ok(_) => Ok(true),
            Err(e) => {
                log::warn!("MyService health check failed: {}", e);
                Ok(false)
            }
        }
    }
    
    fn metrics(&self) -> Option<HashMap<String, String>> {
        Some(self.metrics.lock().unwrap().clone())
    }
}

#[async_trait]
impl RequestExecutor for MyServiceClient {
    async fn execute<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
    where
        T: serde::Serialize + Send + Sync,
        R: for<'de> serde::de::DeserializeOwned + Send,
    {
        self.execute_request(endpoint, request).await
    }
    
    async fn get<R>(&self, endpoint: &str, query_params: Option<HashMap<String, String>>) -> Result<R>
    where
        R: for<'de> serde::de::DeserializeOwned + Send,
    {
        // Implement GET method
        unimplemented!()
    }
    
    async fn post<T, R>(&self, endpoint: &str, body: &T) -> Result<R>
    where
        T: serde::Serialize + Send + Sync,
        R: for<'de> serde::de::DeserializeOwned + Send,
    {
        self.execute(endpoint, body).await
    }
    
    async fn put<T, R>(&self, _endpoint: &str, _body: &T) -> Result<R>
    where
        T: serde::Serialize + Send + Sync,
        R: for<'de> serde::de::DeserializeOwned + Send,
    {
        Err(ServiceError::validation("PUT not supported for MyService API"))
    }
    
    async fn delete<R>(&self, _endpoint: &str) -> Result<R>
    where
        R: for<'de> serde::de::DeserializeOwned + Send,
    {
        Err(ServiceError::validation("DELETE not supported for MyService API"))
    }
}

#[async_trait]
impl AuthenticatedClient for MyServiceClient {
    fn auth_type(&self) -> &str {
        "ApiKey"
    }
    
    fn set_auth(&mut self, auth: impl Into<String> + Send) -> Result<()> {
        self.config.api_key = auth.into();
        Ok(())
    }
    
    fn is_authenticated(&self) -> bool {
        !self.config.api_key.is_empty()
    }
    
    async fn refresh_auth(&mut self) -> Result<()> {
        // No refresh needed for API key auth
        Ok(())
    }
    
    fn apply_auth(&self, headers: &mut HashMap<String, String>) -> Result<()> {
        if !self.is_authenticated() {
            return Err(ServiceError::authentication("No API key set for MyService client"));
        }
        
        headers.insert("X-Api-Key".to_string(), self.config.api_key.clone());
        
        Ok(())
    }
}

#[async_trait]
impl RateLimited for MyServiceClient {
    fn rate_limit_status(&self) -> Option<RateLimitStatus> {
        self.rate_limits.lock().unwrap().clone()
    }
    
    fn configure_rate_limit(&mut self, max_requests: u32, period: Duration) {
        let mut rate_limits = self.rate_limits.lock().unwrap();
        *rate_limits = Some(RateLimitStatus {
            max_requests,
            period,
            current_count: 0,
            reset_after: period,
            enforced: true,
        });
    }
    
    async fn check_rate_limit(&self) -> Result<bool> {
        let rate_limit = self.rate_limits.lock().unwrap().clone();
        
        match rate_limit {
            Some(limit) if limit.enforced => {
                if limit.current_count >= limit.max_requests {
                    Err(ServiceError::rate_limit(format!(
                        "Rate limit exceeded. Reset in {} seconds",
                        limit.reset_after.as_secs()
                    )))
                } else {
                    Ok(true)
                }
            }
            _ => Ok(true),
        }
    }
    
    fn record_request(&self) {
        let mut rate_limits = self.rate_limits.lock().unwrap();
        
        if let Some(ref mut limit) = *rate_limits {
            limit.current_count += 1;
        }
    }
}

#[async_trait]
impl Telemetry for MyServiceClient {
    fn record_request(&self, endpoint: &str, status: u16, duration: Duration) {
        let mut metrics = self.metrics.lock().unwrap();
        
        // Overall request count
        let count_key = "request_count";
        let count = metrics.get(count_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0) + 1;
        metrics.insert(count_key.to_string(), count.to_string());
        
        // Endpoint-specific count
        let endpoint_key = format!("{}_count", endpoint.replace("/", "_"));
        let endpoint_count = metrics.get(&endpoint_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0) + 1;
        metrics.insert(endpoint_key, endpoint_count.to_string());
        
        // Average duration for this endpoint
        let duration_key = format!("{}_avg_ms", endpoint.replace("/", "_"));
        let duration_ms = duration.as_millis() as u64;
        let old_avg = metrics.get(&duration_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0);
        let new_avg = ((old_avg * (endpoint_count - 1)) + duration_ms) / endpoint_count;
        metrics.insert(duration_key, new_avg.to_string());
        
        // Status code tracking
        let status_key = format!("status_{}", status);
        let status_count = metrics.get(&status_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0) + 1;
        metrics.insert(status_key, status_count.to_string());
    }
    
    fn record_error(&self, endpoint: &str, error: &str) {
        let mut metrics = self.metrics.lock().unwrap();
        
        // Overall error count
        let error_key = "error_count";
        let error_count = metrics.get(error_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0) + 1;
        metrics.insert(error_key.to_string(), error_count.to_string());
        
        // Track error types
        let error_type_key = if error.contains("rate limit") {
            "rate_limit_errors"
        } else if error.contains("authentication") {
            "auth_errors"
        } else {
            "other_errors"
        };
        
        let type_count = metrics.get(error_type_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0) + 1;
        metrics.insert(error_type_key.to_string(), type_count.to_string());
        
        // Endpoint-specific error count
        let endpoint_error_key = format!("{}_errors", endpoint.replace("/", "_"));
        let endpoint_error_count = metrics.get(&endpoint_error_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0) + 1;
        metrics.insert(endpoint_error_key, endpoint_error_count.to_string());
    }
    
    fn metrics(&self) -> HashMap<String, String> {
        self.metrics.lock().unwrap().clone()
    }
    
    fn reset_metrics(&mut self) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.clear();
    }
}

/// Builder for MyService client
#[derive(Default)]
pub struct MyServiceClientBuilder {
    /// API key for authentication
    api_key: Option<String>,
    
    /// Base URL for the API
    base_url: Option<String>,
    
    /// Request timeout
    timeout_seconds: Option<u64>,
    
    /// Retry configuration
    retry_config: Option<RetryConfig>,
    
    /// Circuit breaker configuration
    circuit_breaker_config: Option<CircuitBreakerConfig>,
}

impl MyServiceClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the API key
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
    
    /// Set the base URL
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }
    
    /// Set the timeout in seconds
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }
    
    /// Set retry configuration
    pub fn retry(mut self, config: RetryConfig) -> Self {
        self.retry_config = Some(config);
        self
    }
    
    /// Set circuit breaker configuration
    pub fn circuit_breaker(mut self, config: CircuitBreakerConfig) -> Self {
        self.circuit_breaker_config = Some(config);
        self
    }
    
    /// Build the MyService client
    pub fn build(self) -> Result<MyServiceClient> {
        // Try to load config from environment first
        let mut config = MyServiceConfig::from_provider(&**DEFAULT_PROVIDER).unwrap_or_default();
        
        // Override with explicitly provided values
        if let Some(api_key) = self.api_key {
            config.api_key = api_key;
        }
        
        if let Some(base_url) = self.base_url {
            config.base_url = base_url;
        }
        
        if let Some(timeout) = self.timeout_seconds {
            config.timeout_seconds = timeout;
        }
        
        // Validate the configuration
        config.validate()?;
        
        let mut client = MyServiceClient::new_with_config(config);
        
        // Apply custom resilience configurations if provided
        let mut resilience = Resilience::default();
        
        if let Some(retry_config) = self.retry_config {
            resilience.configure_retry(retry_config);
        }
        
        if let Some(circuit_breaker_config) = self.circuit_breaker_config {
            resilience.configure_circuit_breaker(circuit_breaker_config);
        }
        
        client.resilience = resilience;
        
        Ok(client)
    }
}
```

### Step 5: Export the New Service in the Module System

Update `src/services/mod.rs` to expose the new service:

```rust
pub mod openai;
pub mod serpapi;
pub mod my_service; // Add your new service
mod common;

pub use common::UserAgent;
```

### Step 6: Add Factory Function

In `src/lib.rs`, add a factory function for your new service:

```rust
/// Create a pre-configured MyService client
pub fn my_service_client() -> services::my_service::MyServiceClientBuilder {
    services::my_service::MyServiceClientBuilder::new()
}
```

### Step 7: Add Error Mapping

Implement service-specific error mapping in `src/error/mapping.rs`:

```rust
/// Map MyService API errors to ServiceError
pub fn map_my_service_error(status: u16, body: &str) -> ServiceError {
    // Parse the error response
    let error_data = match serde_json::from_str::<Value>(body) {
        Ok(data) => data,
        Err(_) => {
            return ServiceError::parsing(format!(
                "Failed to parse MyService error response: {}", body
            ));
        }
    };
    
    let error_message = error_data["error"]["message"]
        .as_str()
        .unwrap_or("Unknown MyService error");
        
    let error_code = error_data["error"]["code"]
        .as_str()
        .unwrap_or("unknown");
        
    let context = ErrorContext::for_service("my_service")
        .status_code(status)
        .error_code(error_code);
        
    // Map based on status code and/or error code
    match (status, error_code) {
        (401, _) => ServiceError::authentication(error_message),
        (403, _) => ServiceError::authorization(error_message),
        (429, _) => ServiceError::rate_limit(error_message),
        (400, _) => ServiceError::validation(error_message),
        (404, _) => ServiceError::service(format!("Resource not found: {}", error_message)),
        (500..=599, _) => ServiceError::service(format!("MyService server error: {}", error_message)),
        _ => ServiceError::service(format!("MyService API error: {}", error_message)),
    }
    .with_context(context)
}
```

### Step 8: Create an Example

Create an example in `examples/my_service_example.rs`:

```rust
//! MyService Example
//!
//! This example demonstrates how to use the MyService client.
//! 
//! To run this example:
//! ```
//! PHOENIX_MY_SERVICE_API_KEY=your_api_key cargo run --example my_service_example
//! ```

use tool_sdk::{
    my_service_client,
    services::my_service::MyServiceRequest,
    config::{EnvConfigProvider, ConfigProvider},
    error::Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    println!("MyService Example");
    
    // Create a config provider that reads from environment variables
    let config_provider = EnvConfigProvider::new()
        .with_prefix("PHOENIX")
        .with_namespace("MY_SERVICE");
    
    // Load API key from environment
    let api_key = config_provider.get_string_or("API_KEY", "");
    if api_key.is_empty() {
        eprintln!("Please set PHOENIX_MY_SERVICE_API_KEY environment variable");
        std::process::exit(1);
    }
    
    // Create a MyService client
    let client = my_service_client()
        .api_key(api_key)
        .build()?;
    
    // Create a request
    let request = MyServiceRequest {
        query: "example query".to_string(),
        limit: Some(5),
        filter: Some("category:example".to_string()),
    };
    
    println!("Sending request to MyService...");
    
    // Send the request
    let response = client.search(request).await?;
    
    // Print the response
    println!("\nResponse from MyService:");
    println!("Found {} results (total: {})", 
             response.results.len(), 
             response.metadata.map_or(0, |m| m.total));
             
    for result in response.results {
        println!("- {}: {}", result.title, result.content);
    }
    
    Ok(())
}
```

### Step 9: Add Tests

Create tests for your service in `src/tests/my_service_tests.rs`:

```rust
//! Tests for MyService client

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};
    use serde_json::json;
    
    use crate::services::my_service::{MyServiceClient, MyServiceRequest};
    
    #[tokio::test]
    async fn test_my_service_search() {
        // Start a mock server
        let mock_server = MockServer::start().await;
        
        // Create a mock response
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "results": [
                        {
                            "id": "123",
                            "title": "Test Result",
                            "content": "This is a test result"
                        }
                    ],
                    "count": 1,
                    "metadata": {
                        "total": 1,
                        "page": 1
                    }
                })))
            .mount(&mock_server)
            .await;
            
        // Create a client pointing to the mock server
        let client = MyServiceClient::builder()
            .api_key("test-api-key")
            .base_url(mock_server.uri())
            .build()
            .unwrap();
            
        // Create a request
        let request = MyServiceRequest {
            query: "test".to_string(),
            limit: Some(10),
            filter: None,
        };
        
        // Execute the request
        let response = client.search(request).await.unwrap();
        
        // Verify the response
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].id, "123");
        assert_eq!(response.results[0].title, "Test Result");
        assert_eq!(response.results[0].content, "This is a test result");
        assert_eq!(response.count, 1);
        assert!(response.metadata.is_some());
        assert_eq!(response.metadata.unwrap().total, 1);
    }
    
    #[tokio::test]
    async fn test_my_service_error_handling() {
        // Start a mock server
        let mock_server = MockServer::start().await;
        
        // Create a mock error response
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(401)
                .set_body_json(json!({
                    "error": {
                        "message": "Invalid API key",
                        "code": "unauthorized"
                    }
                })))
            .mount(&mock_server)
            .await;
            
        // Create a client pointing to the mock server
        let client = MyServiceClient::builder()
            .api_key("invalid-api-key")
            .base_url(mock_server.uri())
            .build()
            .unwrap();
            
        // Create a request
        let request = MyServiceRequest {
            query: "test".to_string(),
            limit: Some(10),
            filter: None,
        };
        
        // Execute the request and expect an error
        let result = client.search(request).await;
        assert!(result.is_err());
        
        // Verify the error details
        let error = result.unwrap_err();
        match error {
            ServiceError::Authentication(_) => {
                // This is expected
            },
            _ => {
                panic!("Expected Authentication error, got: {:?}", error);
            }
        }
    }
}
```

## Enhancing Error Handling

### Adding New Error Types

To add a new error type, extend the `ServiceError` enum in `src/error/mod.rs`:

```rust
/// Main error type for the Tool SDK
#[derive(Error, Debug)]
pub enum ServiceError {
    // ... existing error types ...
    
    /// New error type: Quota exceeded errors
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),
    
    /// New error type: Throttled request
    #[error("Request throttled: {0}")]
    Throttled(String),
}

impl ServiceError {
    // ... existing factory methods ...
    
    /// Create a quota exceeded error
    pub fn quota_exceeded(message: impl Into<String>) -> Self {
        ServiceError::QuotaExceeded(message.into())
    }
    
    /// Create a throttled error
    pub fn throttled(message: impl Into<String>) -> Self {
        ServiceError::Throttled(message.into())
    }
    
    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        match self {
            // ... existing retryable cases ...
            ServiceError::Network(_) => true,
            ServiceError::Timeout(_) => true,
            ServiceError::RateLimit(_) => true,
            ServiceError::Throttled(_) => true, // Add new retryable type
            ServiceError::WithContext { inner, .. } => inner.is_retryable(),
            _ => false,
        }
    }
    
    // Add other methods as needed...
}
```

### Implementing Custom Error Mapping

To add custom error mapping for a specific service:

```rust
// In src/error/mapping.rs

/// Map custom service errors to ServiceError
pub fn map_custom_service_error(status: u16, body: &str) -> ServiceError {
    // Parse the error response
    let error_data = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(data) => data,
        Err(_) => {
            return ServiceError::parsing(format!("Failed to parse error response: {}", body));
        }
    };
    
    // Extract error details
    let error_message = error_data["error"]["message"]
        .as_str()
        .unwrap_or("Unknown error");
        
    let error_type = error_data["error"]["type"]
        .as_str()
        .unwrap_or("unknown_error");
        
    let error_code = error_data["error"]["code"]
        .as_str()
        .unwrap_or("unknown");
        
    // Create context
    let context = ErrorContext::for_service("custom_service")
        .status_code(status)
        .error_code(error_code);
        
    // Map to ServiceError based on status, error type, and error code
    let service_error = match (status, error_type, error_code) {
        (401, _, _) => ServiceError::authentication(error_message),
        (403, _, _) => ServiceError::authorization(error_message),
        (429, _, "rate_limit_exceeded") => ServiceError::rate_limit(error_message),
        (429, _, "quota_exceeded") => ServiceError::quota_exceeded(error_message),
        (429, _, _) => ServiceError::throttled(error_message),
        (400, _, _) => ServiceError::validation(error_message),
        (404, _, _) => ServiceError::service(format!("Resource not found: {}", error_message)),
        (500..=599, _, _) => ServiceError::service(format!("Server error: {}", error_message)),
        _ => ServiceError::service(format!("API error: {}", error_message)),
    };
    
    // Add context to the error
    service_error.with_context(context)
}
```

### Adding Context Enrichers

To create a utility for enriching errors with context:

```rust
// In src/error/context.rs

/// Error context enricher
pub struct ContextEnricher {
    /// Service name
    service: String,
    
    /// Request ID
    request_id: Option<String>,
    
    /// User ID
    user_id: Option<String>,
    
    /// Additional context data
    additional: HashMap<String, String>,
}

impl ContextEnricher {
    /// Create a new context enricher for a service
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            request_id: None,
            user_id: None,
            additional: HashMap::new(),
        }
    }
    
    /// Set the request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
    
    /// Set the user ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
    
    /// Add additional context data
    pub fn with_data(mut self, key: impl Into<String>, value: impl Display) -> Self {
        self.additional.insert(key.into(), value.to_string());
        self
    }
    
    /// Enrich an error with context
    pub fn enrich(&self, error: ServiceError) -> ServiceError {
        let mut context = ErrorContext::for_service(&self.service);
        
        if let Some(ref request_id) = self.request_id {
            context = context.request_id(request_id);
        }
        
        for (key, value) in &self.additional {
            context = context.with(key, value);
        }
        
        if let Some(ref user_id) = self.user_id {
            context = context.with("user_id", user_id);
        }
        
        error.with_context(context)
    }
    
    /// Wrap an async operation with context enrichment
    pub async fn wrap<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        match operation().await {
            Ok(value) => Ok(value),
            Err(error) => Err(self.enrich(error)),
        }
    }
}
```

## Implementing New Resilience Features

### Adding a New Resilience Pattern: Bulkhead

The bulkhead pattern limits concurrent requests to prevent resource exhaustion:

```rust
// In src/resilience/bulkhead.rs

use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use std::future::Future;
use std::time::Duration;
use tokio::time::timeout;

use crate::error::{Result, ServiceError};

/// Configuration for the bulkhead pattern
#[derive(Debug, Clone)]
pub struct BulkheadConfig {
    /// Maximum concurrent requests
    pub max_concurrent_calls: usize,
    
    /// Timeout for acquiring a permit
    pub acquire_timeout: Duration,
}

impl Default for BulkheadConfig {
    fn default() -> Self {
        Self {
            max_concurrent_calls: 10,
            acquire_timeout: Duration::from_secs(1),
        }
    }
}

/// Implementation of the bulkhead pattern to limit concurrent requests
pub struct Bulkhead {
    /// Semaphore to limit concurrent requests
    semaphore: Arc<Semaphore>,
    
    /// Configuration
    config: BulkheadConfig,
    
    /// Metrics
    metrics: Arc<Mutex<BulkheadMetrics>>,
}

/// Metrics for the bulkhead
#[derive(Debug, Default)]
pub struct BulkheadMetrics {
    /// Total successful acquisitions
    pub successful_acquisitions: usize,
    
    /// Total failed acquisitions
    pub failed_acquisitions: usize,
    
    /// Total completed calls
    pub completed_calls: usize,
}

impl Bulkhead {
    /// Create a new bulkhead with the given configuration
    pub fn new(config: BulkheadConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.max_concurrent_calls)),
            config,
            metrics: Arc::new(Mutex::new(BulkheadMetrics::default())),
        }
    }
    
    /// Execute an operation with bulkhead protection
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        // Try to acquire a permit with timeout
        let permit = match timeout(self.config.acquire_timeout, self.semaphore.acquire()).await {
            Ok(Ok(permit)) => {
                // Successfully acquired a permit
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.successful_acquisitions += 1;
                }
                permit
            },
            Ok(Err(_)) => {
                // Semaphore was closed
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.failed_acquisitions += 1;
                }
                return Err(ServiceError::service("Bulkhead semaphore was closed"));
            },
            Err(_) => {
                // Timed out waiting for a permit
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.failed_acquisitions += 1;
                }
                return Err(ServiceError::service(format!(
                    "Bulkhead full, timed out after {:?} waiting for execution permit",
                    self.config.acquire_timeout
                )));
            }
        };
        
        // Execute the operation with the permit
        // When permit is dropped, it releases a slot in the bulkhead
        let result = operation().await;
        
        // Drop permit explicitly (though it would be dropped automatically)
        drop(permit);
        
        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.completed_calls += 1;
        }
        
        result
    }
    
    /// Get the current available capacity
    pub fn available_capacity(&self) -> usize {
        self.semaphore.available_permits()
    }
    
    /// Get the current metrics
    pub fn metrics(&self) -> BulkheadMetrics {
        self.metrics.lock().unwrap().clone()
    }
}
```

### Integrating the New Pattern in the Resilience Facade

Update the Resilience facade in `src/resilience/mod.rs` to include the new bulkhead pattern:

```rust
mod retry;
mod circuit_breaker;
mod bulkhead;

pub use retry::{RetryExecutor, RetryConfig};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
pub use bulkhead::{Bulkhead, BulkheadConfig};

/// A unified resilience facade that composes multiple resilience strategies
pub struct Resilience {
    /// Retry executor
    retry: RetryExecutor,
    
    /// Circuit breaker
    circuit_breaker: Arc<CircuitBreaker>,
    
    /// Bulkhead (optional)
    bulkhead: Option<Bulkhead>,
}

impl Clone for Resilience {
    fn clone(&self) -> Self {
        Self {
            retry: self.retry.clone(),
            circuit_breaker: Arc::clone(&self.circuit_breaker),
            bulkhead: self.bulkhead.clone(),
        }
    }
}

impl Resilience {
    /// Create a new resilience facade with the specified configurations
    pub fn new(retry_config: RetryConfig, circuit_breaker_config: CircuitBreakerConfig) -> Self {
        let retry = RetryExecutor::new(retry_config);
        let circuit_breaker = Arc::new(CircuitBreaker::new(circuit_breaker_config));
        
        Self {
            retry,
            circuit_breaker,
            bulkhead: None,
        }
    }
    
    /// Create a new resilience facade with bulkhead
    pub fn with_bulkhead(
        retry_config: RetryConfig,
        circuit_breaker_config: CircuitBreakerConfig,
        bulkhead_config: BulkheadConfig,
    ) -> Self {
        let mut resilience = Self::new(retry_config, circuit_breaker_config);
        resilience.bulkhead = Some(Bulkhead::new(bulkhead_config));
        resilience
    }
    
    /// Execute a fallible operation with all configured resilience patterns
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T>> + Send,
        T: Send + 'static,
    {
        // First check if circuit breaker allows the request
        self.circuit_breaker.check()?;
        
        // Use bulkhead if configured
        let operation_with_cb = |cb = Arc::clone(&self.circuit_breaker)| {
            async move {
                match operation().await {
                    Ok(value) => {
                        cb.record_success();
                        Ok(value)
                    }
                    Err(err) => {
                        if err.is_retryable() {
                            cb.record_failure();
                        }
                        Err(err)
                    }
                }
            }
        };
        
        // Apply bulkhead if configured
        let result = if let Some(ref bulkhead) = self.bulkhead {
            // Execute with bulkhead
            self.retry.execute(move || {
                let operation = operation_with_cb();
                bulkhead.execute(move || operation)
            }).await
        } else {
            // Execute without bulkhead
            self.retry.execute(move || operation_with_cb()).await
        };
        
        result
    }
    
    // ... other methods ...
    
    /// Configure the bulkhead
    pub fn configure_bulkhead(&mut self, config: BulkheadConfig) {
        self.bulkhead = Some(Bulkhead::new(config));
    }
}
```

### Adding Fallback Pattern

The fallback pattern provides an alternative when the primary operation fails:

```rust
// In src/resilience/fallback.rs

use std::future::Future;
use crate::error::{Result, ServiceError};

/// A utility for providing fallbacks when operations fail
pub struct Fallback;

impl Fallback {
    /// Execute a primary operation with a fallback in case of failure
    pub async fn execute<F, FB, FutF, FutFB, T>(primary: F, fallback: FB) -> Result<T>
    where
        F: FnOnce() -> FutF,
        FB: FnOnce(ServiceError) -> FutFB,
        FutF: Future<Output = Result<T>>,
        FutFB: Future<Output = Result<T>>,
    {
        match primary().await {
            Ok(value) => Ok(value),
            Err(error) => {
                log::debug!("Primary operation failed, using fallback: {}", error);
                fallback(error).await
            }
        }
    }
    
    /// Execute a primary operation with a static fallback value
    pub async fn with_value<F, Fut, T: Clone>(primary: F, fallback_value: T) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        match primary().await {
            Ok(value) => Ok(value),
            Err(error) => {
                log::debug!("Primary operation failed, using fallback value: {}", error);
                Ok(fallback_value)
            }
        }
    }
    
    /// Execute a primary operation with a fallback function to compute a value
    pub async fn with_function<F, FB, FutF, T>(primary: F, fallback: FB) -> Result<T>
    where
        F: FnOnce() -> FutF,
        FB: FnOnce() -> T,
        FutF: Future<Output = Result<T>>,
    {
        match primary().await {
            Ok(value) => Ok(value),
            Err(error) => {
                log::debug!("Primary operation failed, computing fallback: {}", error);
                Ok(fallback())
            }
        }
    }
}
```

## Testing Extensions

### Unit Testing

Create comprehensive unit tests for your extensions:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ServiceError, Result};
    use tokio_test::block_on;
    use std::time::Duration;
    
    // Test the bulkhead pattern
    #[tokio::test]
    async fn test_bulkhead() {
        // Create a bulkhead with 2 concurrent calls
        let config = BulkheadConfig {
            max_concurrent_calls: 2,
            acquire_timeout: Duration::from_millis(100),
        };
        let bulkhead = Bulkhead::new(config);
        
        // Create a counter to track concurrent calls
        let counter = Arc::new(AtomicUsize::new(0));
        let max_counter = Arc::new(AtomicUsize::new(0));
        
        // Create 5 tasks
        let mut handles = vec![];
        for i in 0..5 {
            let bulkhead = bulkhead.clone();
            let counter = Arc::clone(&counter);
            let max_counter = Arc::clone(&max_counter);
            
            let handle = tokio::spawn(async move {
                match bulkhead.execute(|| async {
                    // Increment counter when entering
                    let current = counter.fetch_add(1, Ordering::SeqCst) + 1;
                    
                    // Update max counter
                    let mut max = max_counter.load(Ordering::SeqCst);
                    while current > max {
                        match max_counter.compare_exchange(
                            max, current, Ordering::SeqCst, Ordering::Relaxed
                        ) {
                            Ok(_) => break,
                            Err(actual) => max = actual,
                        }
                    }
                    
                    // Simulate work
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    
                    // Decrement counter when leaving
                    counter.fetch_sub(1, Ordering::SeqCst);
                    
                    Ok(i)
                }).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(e),
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        let results = futures::future::join_all(handles).await;
        
        // Count successes and failures
        let successes = results.iter()
            .filter(|r| r.as_ref().map(|r| r.is_ok()).unwrap_or(false))
            .count();
        
        let failures = results.len() - successes;
        
        // Verify that max concurrency was respected
        assert!(max_counter.load(Ordering::SeqCst) <= 2);
        
        // Check bulkhead metrics
        let metrics = bulkhead.metrics();
        assert_eq!(metrics.completed_calls, successes);
        assert_eq!(metrics.failed_acquisitions, failures);
    }
    
    // Test the fallback pattern
    #[tokio::test]
    async fn test_fallback() {
        // Test with a successful primary function
        let result = Fallback::execute(
            || async { Ok(42) },
            |_| async { Ok(0) }
        ).await;
        assert_eq!(result.unwrap(), 42);
        
        // Test with a failing primary function
        let result = Fallback::execute(
            || async { Err(ServiceError::network("Connection failed")) },
            |_| async { Ok(0) }
        ).await;
        assert_eq!(result.unwrap(), 0);
        
        // Test with a static fallback value
        let result = Fallback::with_value(
            || async { Err(ServiceError::network("Connection failed")) },
            42
        ).await;
        assert_eq!(result.unwrap(), 42);
        
        // Test with a fallback function
        let result = Fallback::with_function(
            || async { Err(ServiceError::network("Connection failed")) },
            || 42
        ).await;
        assert_eq!(result.unwrap(), 42);
    }
}
```

### Integration Testing

Create integration tests for your service client:

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::my_service_client;
    use crate::services::my_service::MyServiceRequest;
    
    // This test requires a real API key and will be skipped unless enabled
    #[tokio::test]
    #[ignore]
    async fn test_real_my_service_api() {
        // Load API key from environment
        let api_key = match std::env::var("PHOENIX_MY_SERVICE_API_KEY") {
            Ok(key) if !key.is_empty() => key,
            _ => {
                println!("Skipping test: PHOENIX_MY_SERVICE_API_KEY not set");
                return;
            }
        };
        
        // Create client
        let client = my_service_client()
            .api_key(api_key)
            .build()
            .unwrap();
            
        // Create request
        let request = MyServiceRequest {
            query: "test".to_string(),
            limit: Some(3),
            filter: None,
        };
        
        // Execute request
        let result = client.search(request).await;
        assert!(result.is_ok());
        
        // Verify response structure
        let response = result.unwrap();
        println!("Got {} results", response.results.len());
        assert!(response.results.len() > 0);
        
        // Verify simple search
        let simple_results = client.simple_search("test").await.unwrap();
        assert!(!simple_results.is_empty());
    }
}
```

## Contribution Guidelines

### Code Style

When extending the Tool SDK, follow these style guidelines:

1. **Trait-based interfaces**: Define functionality through traits for flexibility
2. **Error handling**: Properly categorize errors and add rich context
3. **Documentation**: Include comprehensive documentation with examples
4. **Testing**: Add both unit and integration tests
5. **Builder pattern**: Implement a builder for complex objects
6. **Async/await**: Use async/await for all I/O operations
7. **Naming**: Use consistent and descriptive naming
8. **Visibility**: Make only necessary items public

### PR Process

1. Ensure your extension follows the architecture patterns in the SDK
2. Write comprehensive tests for your implementation
3. Document new functionality with examples
4. Update the relevant documentation files
5. Ensure all tests pass before submitting a PR
6. Provide a clear description of the changes and their purpose

### Performance Guidelines

1. **Reuse clients**: Create clients once and reuse them
2. **Connection pooling**: Leverage connection pooling for HTTP clients
3. **Avoid blocking**: Use async operations for I/O
4. **Resource cleanup**: Properly clean up resources
5. **Timeout configuration**: Set appropriate timeouts for operations
6. **Metrics collection**: Include metrics for performance monitoring

## Conclusion

Extending the Tool SDK with new service integrations and resilience patterns allows the Phoenix ORCH project to interface with additional external services in a consistent and robust manner. By following this guide, you can ensure that your extensions maintain the architectural principles and quality standards of the core SDK.

Remember to:

1. **Follow the existing patterns**: Maintain consistency with the core SDK
2. **Add comprehensive tests**: Ensure reliability and correctness
3. **Document thoroughly**: Make your extensions easy to use
4. **Consider edge cases**: Handle errors, timeouts, and rate limits appropriately

By leveraging the extension points provided by the SDK, you can add new functionality while benefiting from the existing resilience patterns and error handling mechanisms, ensuring that your service integrations are robust and reliable.