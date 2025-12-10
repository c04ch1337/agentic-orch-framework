//! Integration tests for the Tool SDK
//!
//! These tests verify interactions between different components of the SDK and 
//! test complete workflows from client creation to execution.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    
    use serde::{Serialize, Deserialize};
    use async_trait::async_trait;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};
    
    use crate::core::{ServiceClient, RequestExecutor, AuthenticatedClient, RateLimited, Telemetry};
    use crate::error::{ServiceError, ErrorContext, Result};
    use crate::config::{MemoryConfigProvider, ServiceConfig};
    use crate::resilience::{Resilience, RetryConfig, CircuitBreakerConfig};
    
    // Test data structures
    
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestRequest {
        query: String,
        limit: usize,
    }
    
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestResponse {
        results: Vec<String>,
        total: usize,
    }
    
    #[derive(Debug, Clone)]
    struct TestServiceConfig {
        api_key: String,
        base_url: String,
        timeout_seconds: u64,
    }
    
    impl Default for TestServiceConfig {
        fn default() -> Self {
            Self {
                api_key: "test_api_key".to_string(),
                base_url: "http://localhost:8080".to_string(),
                timeout_seconds: 30,
            }
        }
    }
    
    impl ServiceConfig for TestServiceConfig {
        fn from_provider(provider: &dyn crate::config::ConfigProvider) -> Result<Self> {
            let mut config = Self::default();
            
            if let Ok(api_key) = provider.get_string("TEST_API_KEY") {
                config.api_key = api_key;
            }
            
            if let Ok(base_url) = provider.get_string("TEST_BASE_URL") {
                config.base_url = base_url;
            }
            
            if let Ok(timeout) = provider.get_int("TEST_TIMEOUT") {
                config.timeout_seconds = timeout as u64;
            }
            
            Ok(config)
        }
        
        fn validate(&self) -> Result<()> {
            if self.api_key.is_empty() {
                return Err(ServiceError::validation("API key cannot be empty"));
            }
            
            if self.base_url.is_empty() {
                return Err(ServiceError::validation("Base URL cannot be empty"));
            }
            
            if self.timeout_seconds == 0 {
                return Err(ServiceError::validation("Timeout must be greater than zero"));
            }
            
            Ok(())
        }
    }
    
    // Complete test client that implements all traits
    
    struct TestClient {
        config: TestServiceConfig,
        resilience: Resilience,
        http_client: reqwest::Client,
        auth_headers: Arc<Mutex<HashMap<String, String>>>,
        rate_limits: Arc<Mutex<Option<crate::core::RateLimitStatus>>>,
        metrics: Arc<Mutex<HashMap<String, String>>>,
    }
    
    impl TestClient {
        fn new(config: TestServiceConfig) -> Self {
            let http_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(config.timeout_seconds))
                .build()
                .expect("Failed to create HTTP client");
            
            let resilience = Resilience::new(
                RetryConfig {
                    max_retries: 2,
                    initial_interval: Duration::from_millis(50),
                    ..RetryConfig::default()
                },
                CircuitBreakerConfig {
                    failure_threshold: 3,
                    ..CircuitBreakerConfig::default()
                },
            );
            
            Self {
                config,
                resilience,
                http_client,
                auth_headers: Arc::new(Mutex::new(HashMap::new())),
                rate_limits: Arc::new(Mutex::new(None)),
                metrics: Arc::new(Mutex::new(HashMap::new())),
            }
        }
        
        async fn execute_search(&self, query: &str, limit: usize) -> Result<TestResponse> {
            let request = TestRequest {
                query: query.to_string(),
                limit,
            };
            
            self.execute("search", &request).await
        }
    }
    
    #[async_trait]
    impl ServiceClient for TestClient {
        fn name(&self) -> &str {
            "test_service"
        }
        
        fn base_url(&self) -> &str {
            &self.config.base_url
        }
        
        fn version(&self) -> &str {
            "v1"
        }
        
        async fn health_check(&self) -> Result<bool> {
            // Simple health check implementation
            let response = self.http_client.get(format!("{}/health", self.base_url()))
                .send()
                .await;
            
            match response {
                Ok(res) => Ok(res.status().is_success()),
                Err(_) => Ok(false),
            }
        }
        
        fn metrics(&self) -> Option<HashMap<String, String>> {
            Some(self.metrics.lock().unwrap().clone())
        }
    }
    
    #[async_trait]
    impl RequestExecutor for TestClient {
        async fn execute<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
        where
            T: Serialize + Send + Sync,
            R: for<'de> Deserialize<'de> + Send,
        {
            // Use resilience patterns
            self.resilience.execute(move || {
                let auth_headers = self.auth_headers.clone();
                let base_url = self.base_url().to_string();
                let client = self.http_client.clone();
                let request_json = serde_json::to_vec(request)
                    .map_err(|e| ServiceError::validation(format!("Invalid request: {}", e)))?;
                
                async move {
                    let url = format!("{}/{}", base_url, endpoint);
                    let headers = auth_headers.lock().unwrap().clone();
                    
                    let mut builder = client.post(&url);
                    
                    // Add auth headers
                    for (key, value) in headers {
                        builder = builder.header(key, value);
                    }
                    
                    let response = builder
                        .header("Content-Type", "application/json")
                        .body(request_json)
                        .send()
                        .await
                        .map_err(|e| ServiceError::network(format!("Request failed: {}", e)))?;
                    
                    let status = response.status();
                    
                    if status.is_success() {
                        let json = response.json::<R>().await
                            .map_err(|e| ServiceError::parsing(format!("Invalid response: {}", e)))?;
                        Ok(json)
                    } else {
                        // Parse error response
                        let error_text = response.text().await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        
                        let error = if status.as_u16() == 429 {
                            ServiceError::rate_limit(error_text)
                        } else if status.as_u16() == 401 {
                            ServiceError::authentication(error_text)
                        } else {
                            ServiceError::service(error_text)
                        };
                        
                        Err(error)
                    }
                }
            }).await
        }
        
        async fn get<R>(&self, endpoint: &str, query_params: Option<HashMap<String, String>>) -> Result<R>
        where
            R: for<'de> Deserialize<'de> + Send,
        {
            self.resilience.execute(move || {
                let auth_headers = self.auth_headers.clone();
                let base_url = self.base_url().to_string();
                let client = self.http_client.clone();
                
                async move {
                    let url = format!("{}/{}", base_url, endpoint);
                    let headers = auth_headers.lock().unwrap().clone();
                    
                    let mut builder = client.get(&url);
                    
                    // Add query parameters if provided
                    if let Some(params) = query_params {
                        builder = builder.query(&params);
                    }
                    
                    // Add auth headers
                    for (key, value) in headers {
                        builder = builder.header(key, value);
                    }
                    
                    let response = builder
                        .send()
                        .await
                        .map_err(|e| ServiceError::network(format!("Request failed: {}", e)))?;
                    
                    let status = response.status();
                    
                    if status.is_success() {
                        let json = response.json::<R>().await
                            .map_err(|e| ServiceError::parsing(format!("Invalid response: {}", e)))?;
                        Ok(json)
                    } else {
                        // Parse error response
                        let error_text = response.text().await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        
                        Err(ServiceError::service(error_text))
                    }
                }
            }).await
        }
        
        async fn post<T, R>(&self, endpoint: &str, body: &T) -> Result<R>
        where
            T: Serialize + Send + Sync,
            R: for<'de> Deserialize<'de> + Send,
        {
            self.execute(endpoint, body).await
        }
        
        async fn put<T, R>(&self, endpoint: &str, body: &T) -> Result<R>
        where
            T: Serialize + Send + Sync,
            R: for<'de> Deserialize<'de> + Send,
        {
            self.resilience.execute(move || {
                let auth_headers = self.auth_headers.clone();
                let base_url = self.base_url().to_string();
                let client = self.http_client.clone();
                let request_json = serde_json::to_vec(body)
                    .map_err(|e| ServiceError::validation(format!("Invalid request: {}", e)))?;
                
                async move {
                    let url = format!("{}/{}", base_url, endpoint);
                    let headers = auth_headers.lock().unwrap().clone();
                    
                    let mut builder = client.put(&url);
                    
                    // Add auth headers
                    for (key, value) in headers {
                        builder = builder.header(key, value);
                    }
                    
                    let response = builder
                        .header("Content-Type", "application/json")
                        .body(request_json)
                        .send()
                        .await
                        .map_err(|e| ServiceError::network(format!("Request failed: {}", e)))?;
                    
                    let status = response.status();
                    
                    if status.is_success() {
                        let json = response.json::<R>().await
                            .map_err(|e| ServiceError::parsing(format!("Invalid response: {}", e)))?;
                        Ok(json)
                    } else {
                        // Parse error response
                        let error_text = response.text().await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        
                        Err(ServiceError::service(error_text))
                    }
                }
            }).await
        }
        
        async fn delete<R>(&self, endpoint: &str) -> Result<R>
        where
            R: for<'de> Deserialize<'de> + Send,
        {
            self.resilience.execute(move || {
                let auth_headers = self.auth_headers.clone();
                let base_url = self.base_url().to_string();
                let client = self.http_client.clone();
                
                async move {
                    let url = format!("{}/{}", base_url, endpoint);
                    let headers = auth_headers.lock().unwrap().clone();
                    
                    let mut builder = client.delete(&url);
                    
                    // Add auth headers
                    for (key, value) in headers {
                        builder = builder.header(key, value);
                    }
                    
                    let response = builder
                        .send()
                        .await
                        .map_err(|e| ServiceError::network(format!("Request failed: {}", e)))?;
                    
                    let status = response.status();
                    
                    if status.is_success() {
                        let json = response.json::<R>().await
                            .map_err(|e| ServiceError::parsing(format!("Invalid response: {}", e)))?;
                        Ok(json)
                    } else {
                        // Parse error response
                        let error_text = response.text().await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        
                        Err(ServiceError::service(error_text))
                    }
                }
            }).await
        }
    }
    
    #[async_trait]
    impl AuthenticatedClient for TestClient {
        fn auth_type(&self) -> &str {
            "Bearer"
        }
        
        fn set_auth(&mut self, auth: impl Into<String> + Send) -> Result<()> {
            let auth_value = auth.into();
            let mut headers = self.auth_headers.lock().unwrap();
            headers.insert("Authorization".to_string(), format!("Bearer {}", auth_value));
            Ok(())
        }
        
        fn is_authenticated(&self) -> bool {
            let headers = self.auth_headers.lock().unwrap();
            headers.contains_key("Authorization")
        }
        
        async fn refresh_auth(&mut self) -> Result<()> {
            // In a real implementation, this would call an auth service
            // Here we just simulate a token refresh
            let mut headers = self.auth_headers.lock().unwrap();
            if let Some(auth) = headers.get("Authorization") {
                if let Some(token) = auth.strip_prefix("Bearer ") {
                    let new_token = format!("refreshed_{}", token);
                    headers.insert("Authorization".to_string(), format!("Bearer {}", new_token));
                }
            }
            
            Ok(())
        }
        
        fn apply_auth(&self, headers: &mut HashMap<String, String>) -> Result<()> {
            let auth_headers = self.auth_headers.lock().unwrap();
            if let Some(auth) = auth_headers.get("Authorization") {
                headers.insert("Authorization".to_string(), auth.clone());
                Ok(())
            } else {
                Err(ServiceError::authentication("Not authenticated"))
            }
        }
    }
    
    #[async_trait]
    impl RateLimited for TestClient {
        fn rate_limit_status(&self) -> Option<crate::core::RateLimitStatus> {
            self.rate_limits.lock().unwrap().clone()
        }
        
        fn configure_rate_limit(&mut self, max_requests: u32, period: Duration) {
            let mut rate_limits = self.rate_limits.lock().unwrap();
            *rate_limits = Some(crate::core::RateLimitStatus {
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
                        Err(ServiceError::rate_limit("Rate limit exceeded"))
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
    impl Telemetry for TestClient {
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
            
            // Status code tracking
            let status_key = format!("status_{}", status);
            let status_count = metrics.get(&status_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0) + 1;
            metrics.insert(status_key, status_count.to_string());
            
            // Record duration
            let duration_ms = duration.as_millis() as u64;
            metrics.insert(format!("{}_last_duration_ms", endpoint), duration_ms.to_string());
        }
        
        fn record_error(&self, endpoint: &str, error: &str) {
            let mut metrics = self.metrics.lock().unwrap();
            
            // Error count
            let error_key = "error_count";
            let error_count = metrics.get(error_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0) + 1;
            metrics.insert(error_key.to_string(), error_count.to_string());
            
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
    
    /// A builder for TestClient to manage its construction
    struct TestClientBuilder {
        config: Option<TestServiceConfig>,
        api_key: Option<String>,
        base_url: Option<String>,
        timeout: Option<u64>,
        retry_config: Option<RetryConfig>,
        circuit_breaker_config: Option<CircuitBreakerConfig>,
    }
    
    impl Default for TestClientBuilder {
        fn default() -> Self {
            Self {
                config: None,
                api_key: None,
                base_url: None,
                timeout: None,
                retry_config: None,
                circuit_breaker_config: None,
            }
        }
    }
    
    impl TestClientBuilder {
        fn new() -> Self {
            Self::default()
        }
        
        fn with_config(mut self, config: TestServiceConfig) -> Self {
            self.config = Some(config);
            self
        }
        
        fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
            self.api_key = Some(api_key.into());
            self
        }
        
        fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
            self.base_url = Some(base_url.into());
            self
        }
        
        fn with_timeout(mut self, timeout_seconds: u64) -> Self {
            self.timeout = Some(timeout_seconds);
            self
        }
        
        fn with_retry_config(mut self, config: RetryConfig) -> Self {
            self.retry_config = Some(config);
            self
        }
        
        fn with_circuit_breaker_config(mut self, config: CircuitBreakerConfig) -> Self {
            self.circuit_breaker_config = Some(config);
            self
        }
        
        fn build(self) -> Result<TestClient> {
            // Start with default or provided config
            let mut config = self.config.unwrap_or_default();
            
            // Override with explicit values if provided
            if let Some(api_key) = self.api_key {
                config.api_key = api_key;
            }
            
            if let Some(base_url) = self.base_url {
                config.base_url = base_url;
            }
            
            if let Some(timeout) = self.timeout {
                config.timeout_seconds = timeout;
            }
            
            // Validate the configuration
            config.validate()?;
            
            // Create client with the validated configuration
            let mut client = TestClient::new(config);
            
            // Apply custom resilience configurations if provided
            if let Some(retry_config) = self.retry_config {
                client.resilience.configure_retry(retry_config);
            }
            
            if let Some(circuit_breaker_config) = self.circuit_breaker_config {
                client.resilience.configure_circuit_breaker(circuit_breaker_config);
            }
            
            Ok(client)
        }
    }
    
    /// Integration tests for client lifecycle and components
    #[tokio::test]
    async fn test_client_lifecycle() -> Result<()> {
        // Initialize mock server
        let mock_server = MockServer::start().await;
        
        // Create a config for our test client
        let config = TestServiceConfig {
            api_key: "test_key_123".to_string(),
            base_url: mock_server.uri(),
            timeout_seconds: 10,
        };
        
        // Test successful client creation
        let mut client = TestClientBuilder::new()
            .with_config(config)
            .with_retry_config(RetryConfig {
                max_retries: 1,
                initial_interval: Duration::from_millis(10),
                ..RetryConfig::default()
            })
            .build()?;
        
        // Test authentication
        assert!(!client.is_authenticated());
        client.set_auth("test_token")?;
        assert!(client.is_authenticated());
        
        // Test rate limiting configuration
        client.configure_rate_limit(5, Duration::from_secs(60));
        let rate_limit = client.rate_limit_status().unwrap();
        assert_eq!(rate_limit.max_requests, 5);
        
        // Set up mock for a successful request
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&TestResponse {
                    results: vec!["result1".to_string(), "result2".to_string()],
                    total: 2,
                }))
            .mount(&mock_server)
            .await;
        
        // Execute the search request
        let response = client.execute_search("test query", 10).await?;
        
        // Verify response
        assert_eq!(response.total, 2);
        assert_eq!(response.results, vec!["result1".to_string(), "result2".to_string()]);
        
        // Verify metrics were recorded
        let metrics = client.metrics();
        assert!(metrics.contains_key("request_count"));
        assert!(metrics.contains_key("search_count"));
        assert!(metrics.contains_key("status_200"));
        
        // Test successful client creation with builder pattern
        let client = TestClientBuilder::new()
            .with_api_key("different_key")
            .with_base_url(mock_server.uri())
            .with_timeout(5)
            .build()?;
        
        assert_eq!(client.config.api_key, "different_key");
        assert_eq!(client.config.timeout_seconds, 5);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_error_handling_across_components() -> Result<()> {
        // Initialize mock server
        let mock_server = MockServer::start().await;
        
        // Create a config for our test client
        let config = TestServiceConfig {
            api_key: "test_key".to_string(),
            base_url: mock_server.uri(),
            timeout_seconds: 5,
        };
        
        // Create client
        let mut client = TestClient::new(config);
        
        // Set auth
        client.set_auth("test_token")?;
        
        // Set up mock for rate limited response
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(429)
                .set_body_string("Rate limit exceeded"))
            .mount(&mock_server)
            .await;
        
        // Execute request that will get rate limited
        let result = client.execute_search("test", 10).await;
        
        // Verify the error type and retryability
        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            ServiceError::RateLimit(msg) => {
                assert!(msg.contains("Rate limit exceeded"));
                assert!(error.is_retryable());
            }
            _ => panic!("Expected RateLimit error"),
        }
        
        // Set up mock for authentication error
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(401)
                .set_body_string("Invalid credentials"))
            .mount(&mock_server)
            .await;
        
        // Execute request that will get auth error
        let result = client.execute_search("test", 10).await;
        
        // Verify the error type
        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            ServiceError::Authentication(msg) => {
                assert!(msg.contains("Invalid credentials"));
                assert!(!error.is_retryable());
            }
            _ => panic!("Expected Authentication error"),
        }
        
        // Verify error metrics were recorded
        let metrics = client.metrics();
        assert!(metrics.contains_key("error_count"));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_resilience_patterns_integration() -> Result<()> {
        // Initialize mock server
        let mock_server = MockServer::start().await;
        
        // Create client with specific resilience config
        let config = TestServiceConfig {
            api_key: "test_key".to_string(),
            base_url: mock_server.uri(),
            timeout_seconds: 5,
        };
        
        let mut client = TestClientBuilder::new()
            .with_config(config)
            .with_retry_config(RetryConfig {
                max_retries: 2,
                initial_interval: Duration::from_millis(50),
                ..RetryConfig::default()
            })
            .with_circuit_breaker_config(CircuitBreakerConfig {
                failure_threshold: 3,
                reset_timeout: Duration::from_millis(100),
                ..CircuitBreakerConfig::default()
            })
            .build()?;
        
        // Set auth
        client.set_auth("test_token")?;
        
        // Track attempts
        let attempts = Arc::new(Mutex::new(0));
        
        // Set up mock for server error that will be retried
        {
            let attempts_ref = Arc::clone(&attempts);
            Mock::given(method("POST"))
                .and(path("/search"))
                .respond_with(move |_| {
                    let mut count = attempts_ref.lock().unwrap();
                    *count += 1;
                    
                    if *count < 3 {
                        // Return error for first two attempts
                        ResponseTemplate::new(500).set_body_string("Server error")
                    } else {
                        // Return success on third attempt
                        ResponseTemplate::new(200).set_body_json(&TestResponse {
                            results: vec!["result1".to_string()],
                            total: 1,
                        })
                    }
                })
                .mount(&mock_server)
                .await;
        }
        
        // Execute request that will be retried and eventually succeed
        let result = client.execute_search("test", 10).await;
        
        // Verify it succeeded after retries
        assert!(result.is_ok());
        assert_eq!(*attempts.lock().unwrap(), 3); // Initial + 2 retries
        
        // Reset attempts counter
        *attempts.lock().unwrap() = 0;
        
        // Set up mock for persistent failure
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(500)
                .set_body_string("Persistent server error"))
            .mount(&mock_server)
            .await;
        
        // Execute requests until circuit breaker opens
        for _ in 0..4 {
            let _ = client.execute_search("test", 10).await;
        }
        
        // Circuit should be open now
        assert_eq!(client.resilience.circuit_breaker_status(), crate::resilience::CircuitBreakerStatus::Open);
        
        // Next request should fail fast due to open circuit breaker
        let result = client.execute_search("test", 10).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Circuit breaker is open"));
        
        // Wait for circuit breaker timeout
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Set up mock to respond with success when circuit is half-open
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&TestResponse {
                    results: vec!["success".to_string()],
                    total: 1,
                }))
            .mount(&mock_server)
            .await;
        
        // Next request should go through (circuit half-open) and succeed
        let result = client.execute_search("test", 10).await;
        assert!(result.is_ok());
        
        // Circuit should be closed now
        assert_eq!(client.resilience.circuit_breaker_status(), crate::resilience::CircuitBreakerStatus::Closed);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_client_with_memory_config_provider() -> Result<()> {
        // Create memory config provider
        let mut provider = MemoryConfigProvider::new();
        provider.set("TEST_API_KEY", "memory_provider_key");
        provider.set("TEST_BASE_URL", "https://example.com");
        provider.set("TEST_TIMEOUT", "15");
        
        // Load config from provider
        let config = TestServiceConfig::from_provider(&provider)?;
        
        // Verify loaded values
        assert_eq!(config.api_key, "memory_provider_key");
        assert_eq!(config.base_url, "https://example.com");
        assert_eq!(config.timeout_seconds, 15);
        
        // Create client using the loaded config
        let client = TestClient::new(config);
        
        // Verify client has the correct configuration
        assert_eq!(client.config.api_key, "memory_provider_key");
        assert_eq!(client.config.base_url, "https://example.com");
        assert_eq!(client.config.timeout_seconds, 15);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_telemetry_metrics_collection() -> Result<()> {
        // Initialize mock server
        let mock_server = MockServer::start().await;
        
        // Create client
        let mut client = TestClient::new(TestServiceConfig {
            api_key: "test_key".to_string(),
            base_url: mock_server.uri(),
            timeout_seconds: 5,
        });
        
        // Set auth
        client.set_auth("test_token")?;
        
        // Set up mock for successful response
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&TestResponse {
                    results: vec!["result".to_string()],
                    total: 1,
                }))
            .mount(&mock_server)
            .await;
        
        // Execute successful request
        let _ = client.execute_search("test", 10).await?;
        
        // Set up mock for error response
        Mock::given(method("POST"))
            .and(path("/error"))
            .respond_with(ResponseTemplate::new(500)
                .set_body_string("Server error"))
            .mount(&mock_server)
            .await;
        
        // Execute request that generates an error
        let error_result = client.post::<_, TestResponse>("error", &TestRequest {
            query: "test".to_string(),
            limit: 10,
        }).await;
        
        assert!(error_result.is_err());
        
        // Verify metrics were collected
        let metrics = client.metrics();
        
        // Check success metrics
        assert!(metrics.contains_key("request_count"));
        assert!(metrics.contains_key("search_count"));
        assert!(metrics.contains_key("status_200"));
        
        // Check error metrics
        assert!(metrics.contains_key("error_count"));
        
        // Test metrics reset
        client.reset_metrics();
        let metrics = client.metrics();
        assert_eq!(metrics.len(), 0);
        
        Ok(())
    }
}