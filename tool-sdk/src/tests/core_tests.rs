//! Tests for core abstractions
//!
//! These tests verify that the core traits and interfaces work correctly.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;
    use std::sync::{Arc, Mutex};
    
    use async_trait::async_trait;
    use serde::{Serialize, Deserialize};
    
    use crate::core::{
        ServiceClient, RequestExecutor, AuthenticatedClient,
        RateLimited, Telemetry, RateLimitStatus
    };
    use crate::error::{ServiceError, Result};
    
    // Mock implementations for testing
    
    /// Simple mock service client for testing
    struct MockServiceClient {
        name: String,
        base_url: String,
        version: String,
        health_status: bool,
        metrics: HashMap<String, String>,
    }
    
    impl Default for MockServiceClient {
        fn default() -> Self {
            Self {
                name: "mock_service".to_string(),
                base_url: "https://api.example.com".to_string(),
                version: "v1".to_string(),
                health_status: true,
                metrics: HashMap::new(),
            }
        }
    }
    
    #[async_trait]
    impl ServiceClient for MockServiceClient {
        fn name(&self) -> &str {
            &self.name
        }
        
        fn base_url(&self) -> &str {
            &self.base_url
        }
        
        fn version(&self) -> &str {
            &self.version
        }
        
        async fn health_check(&self) -> Result<bool> {
            Ok(self.health_status)
        }
        
        fn metrics(&self) -> Option<HashMap<String, String>> {
            Some(self.metrics.clone())
        }
    }
    
    // Test data structures
    
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct MockRequest {
        query: String,
        limit: usize,
    }
    
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct MockResponse {
        results: Vec<String>,
        total: usize,
    }
    
    /// Mock request executor for testing
    struct MockRequestExecutor {
        responses: HashMap<String, String>, // endpoint -> JSON response
        auth_headers: HashMap<String, String>,
        should_fail: bool,
    }
    
    impl MockRequestExecutor {
        fn new() -> Self {
            let mut responses = HashMap::new();
            responses.insert(
                "search".to_string(),
                r#"{"results": ["result1", "result2"], "total": 2}"#.to_string(),
            );
            
            Self {
                responses,
                auth_headers: HashMap::new(),
                should_fail: false,
            }
        }
        
        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }
    }
    
    #[async_trait]
    impl RequestExecutor for MockRequestExecutor {
        async fn execute<T, R>(&self, endpoint: &str, _request: &T) -> Result<R>
        where
            T: Serialize + Send + Sync,
            R: for<'de> Deserialize<'de> + Send,
        {
            if self.should_fail {
                return Err(ServiceError::network("Mock network failure"));
            }
            
            if let Some(response_json) = self.responses.get(endpoint) {
                let response: R = serde_json::from_str(response_json)
                    .map_err(|e| ServiceError::parsing(format!("Mock parsing error: {}", e)))?;
                Ok(response)
            } else {
                Err(ServiceError::not_found(format!("Endpoint {} not found in mock", endpoint)))
            }
        }
        
        async fn get<R>(&self, endpoint: &str, _query_params: Option<HashMap<String, String>>) -> Result<R>
        where
            R: for<'de> Deserialize<'de> + Send,
        {
            if self.should_fail {
                return Err(ServiceError::network("Mock network failure"));
            }
            
            if let Some(response_json) = self.responses.get(endpoint) {
                let response: R = serde_json::from_str(response_json)
                    .map_err(|e| ServiceError::parsing(format!("Mock parsing error: {}", e)))?;
                Ok(response)
            } else {
                Err(ServiceError::not_found(format!("Endpoint {} not found in mock", endpoint)))
            }
        }
        
        async fn post<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
        where
            T: Serialize + Send + Sync,
            R: for<'de> Deserialize<'de> + Send,
        {
            self.execute(endpoint, request).await
        }
        
        async fn put<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
        where
            T: Serialize + Send + Sync,
            R: for<'de> Deserialize<'de> + Send,
        {
            self.execute(endpoint, request).await
        }
        
        async fn delete<R>(&self, endpoint: &str) -> Result<R>
        where
            R: for<'de> Deserialize<'de> + Send,
        {
            self.get(endpoint, None).await
        }
    }
    
    /// Mock authenticated client for testing
    struct MockAuthenticatedClient {
        auth_type: String,
        api_key: Option<String>,
    }
    
    impl Default for MockAuthenticatedClient {
        fn default() -> Self {
            Self {
                auth_type: "Bearer".to_string(),
                api_key: None,
            }
        }
    }
    
    #[async_trait]
    impl AuthenticatedClient for MockAuthenticatedClient {
        fn auth_type(&self) -> &str {
            &self.auth_type
        }
        
        fn set_auth(&mut self, auth: impl Into<String> + Send) -> Result<()> {
            self.api_key = Some(auth.into());
            Ok(())
        }
        
        fn is_authenticated(&self) -> bool {
            self.api_key.is_some()
        }
        
        async fn refresh_auth(&mut self) -> Result<()> {
            if !self.is_authenticated() {
                return Err(ServiceError::authentication("No auth token to refresh"));
            }
            
            // Simulate token refresh by adding a prefix
            if let Some(ref key) = self.api_key {
                self.api_key = Some(format!("refreshed_{}", key));
            }
            
            Ok(())
        }
        
        fn apply_auth(&self, headers: &mut HashMap<String, String>) -> Result<()> {
            if !self.is_authenticated() {
                return Err(ServiceError::authentication("Not authenticated"));
            }
            
            headers.insert(
                "Authorization".to_string(),
                format!("{} {}", self.auth_type, self.api_key.as_ref().unwrap()),
            );
            
            Ok(())
        }
    }
    
    /// Mock rate limited client for testing
    struct MockRateLimited {
        rate_limit: Arc<Mutex<Option<RateLimitStatus>>>,
        request_count: Arc<Mutex<u32>>,
    }
    
    impl Default for MockRateLimited {
        fn default() -> Self {
            Self {
                rate_limit: Arc::new(Mutex::new(None)),
                request_count: Arc::new(Mutex::new(0)),
            }
        }
    }
    
    #[async_trait]
    impl RateLimited for MockRateLimited {
        fn rate_limit_status(&self) -> Option<RateLimitStatus> {
            self.rate_limit.lock().unwrap().clone()
        }
        
        fn configure_rate_limit(&mut self, max_requests: u32, period: Duration) {
            let mut rate_limit = self.rate_limit.lock().unwrap();
            *rate_limit = Some(RateLimitStatus {
                max_requests,
                period,
                current_count: 0,
                reset_after: period,
                enforced: true,
            });
        }
        
        async fn check_rate_limit(&self) -> Result<bool> {
            let rate_limit = self.rate_limit.lock().unwrap().clone();
            
            match rate_limit {
                Some(limit) if limit.enforced => {
                    let count = *self.request_count.lock().unwrap();
                    if count >= limit.max_requests {
                        Err(ServiceError::rate_limit("Rate limit exceeded in mock"))
                    } else {
                        Ok(true)
                    }
                }
                _ => Ok(true),
            }
        }
        
        fn record_request(&self) {
            let mut count = self.request_count.lock().unwrap();
            *count += 1;
        }
    }
    
    /// Mock telemetry for testing
    struct MockTelemetry {
        metrics: Arc<Mutex<HashMap<String, String>>>,
    }
    
    impl Default for MockTelemetry {
        fn default() -> Self {
            Self {
                metrics: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }
    
    #[async_trait]
    impl Telemetry for MockTelemetry {
        fn record_request(&self, endpoint: &str, status: u16, duration: Duration) {
            let mut metrics = self.metrics.lock().unwrap();
            
            let count_key = format!("{}_count", endpoint);
            let count = metrics.get(&count_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0) + 1;
            metrics.insert(count_key, count.to_string());
            
            let status_key = format!("status_{}", status);
            let status_count = metrics.get(&status_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0) + 1;
            metrics.insert(status_key, status_count.to_string());
            
            let duration_ms = duration.as_millis() as u64;
            metrics.insert(format!("{}_last_duration_ms", endpoint), duration_ms.to_string());
        }
        
        fn record_error(&self, endpoint: &str, error: &str) {
            let mut metrics = self.metrics.lock().unwrap();
            
            let error_key = format!("{}_errors", endpoint);
            let count = metrics.get(&error_key).unwrap_or(&"0".to_string()).parse::<u64>().unwrap_or(0) + 1;
            metrics.insert(error_key, count.to_string());
            
            metrics.insert(format!("{}_last_error", endpoint), error.to_string());
        }
        
        fn metrics(&self) -> HashMap<String, String> {
            self.metrics.lock().unwrap().clone()
        }
        
        fn reset_metrics(&mut self) {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.clear();
        }
    }
    
    // Tests for ServiceClient trait
    
    #[tokio::test]
    async fn test_service_client_interface() {
        let client = MockServiceClient::default();
        
        // Basic properties
        assert_eq!(client.name(), "mock_service");
        assert_eq!(client.base_url(), "https://api.example.com");
        assert_eq!(client.version(), "v1");
        
        // Health check
        let health = client.health_check().await.unwrap();
        assert_eq!(health, true);
        
        // Check metrics interface
        let metrics = client.metrics();
        assert!(metrics.is_some());
        assert_eq!(metrics.unwrap().len(), 0); // Empty initially
    }
    
    // Tests for RequestExecutor trait
    
    #[tokio::test]
    async fn test_request_executor_success() {
        let executor = MockRequestExecutor::new();
        
        // Test execute method
        let request = MockRequest {
            query: "test".to_string(),
            limit: 10,
        };
        
        let response: MockResponse = executor.execute("search", &request).await.unwrap();
        assert_eq!(response.results.len(), 2);
        assert_eq!(response.total, 2);
        
        // Test get method
        let response: MockResponse = executor.get("search", None).await.unwrap();
        assert_eq!(response.results, vec!["result1".to_string(), "result2".to_string()]);
        
        // Test post method
        let response: MockResponse = executor.post("search", &request).await.unwrap();
        assert_eq!(response.total, 2);
    }
    
    #[tokio::test]
    async fn test_request_executor_failure() {
        let executor = MockRequestExecutor::new().with_failure();
        
        let request = MockRequest {
            query: "test".to_string(),
            limit: 10,
        };
        
        // All methods should fail with the same error
        let error = executor.execute::<_, MockResponse>("search", &request).await.unwrap_err();
        assert!(error.to_string().contains("Mock network failure"));
        
        let error = executor.get::<MockResponse>("search", None).await.unwrap_err();
        assert!(error.to_string().contains("Mock network failure"));
    }
    
    #[tokio::test]
    async fn test_request_executor_endpoint_not_found() {
        let executor = MockRequestExecutor::new();
        
        let request = MockRequest {
            query: "test".to_string(),
            limit: 10,
        };
        
        // Test with non-existent endpoint
        let error = executor.execute::<_, MockResponse>("nonexistent", &request).await.unwrap_err();
        assert!(error.to_string().contains("not found in mock"));
    }
    
    // Tests for AuthenticatedClient trait
    
    #[tokio::test]
    async fn test_authenticated_client() {
        let mut client = MockAuthenticatedClient::default();
        
        // Initially not authenticated
        assert_eq!(client.is_authenticated(), false);
        
        // Set auth token
        client.set_auth("test_token").unwrap();
        assert_eq!(client.is_authenticated(), true);
        
        // Apply auth to headers
        let mut headers = HashMap::new();
        client.apply_auth(&mut headers).unwrap();
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer test_token");
        
        // Refresh token
        client.refresh_auth().await.unwrap();
        
        // Check that token was refreshed
        let mut headers = HashMap::new();
        client.apply_auth(&mut headers).unwrap();
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer refreshed_test_token");
    }
    
    #[tokio::test]
    async fn test_authenticated_client_errors() {
        let mut client = MockAuthenticatedClient::default();
        
        // Applying auth without setting token should error
        let mut headers = HashMap::new();
        let result = client.apply_auth(&mut headers);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Not authenticated"));
        
        // Refreshing auth without token should error
        let result = client.refresh_auth().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No auth token"));
    }
    
    // Tests for RateLimited trait
    
    #[tokio::test]
    async fn test_rate_limited() {
        let mut client = MockRateLimited::default();
        
        // Initially no rate limit
        assert_eq!(client.rate_limit_status(), None);
        
        // Configure rate limit
        client.configure_rate_limit(5, Duration::from_secs(60));
        
        // Check rate limit status
        let status = client.rate_limit_status().unwrap();
        assert_eq!(status.max_requests, 5);
        assert_eq!(status.current_count, 0);
        assert!(status.enforced);
        
        // Check rate limit with no requests recorded
        let result = client.check_rate_limit().await;
        assert!(result.is_ok());
        
        // Record requests up to the limit
        for _ in 0..5 {
            client.record_request();
        }
        
        // Next check should fail
        let result = client.check_rate_limit().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Rate limit exceeded"));
    }
    
    // Tests for Telemetry trait
    
    #[tokio::test]
    async fn test_telemetry() {
        let mut telemetry = MockTelemetry::default();
        
        // Initially no metrics
        assert_eq!(telemetry.metrics().len(), 0);
        
        // Record a successful request
        telemetry.record_request("test_endpoint", 200, Duration::from_millis(150));
        
        // Check metrics
        let metrics = telemetry.metrics();
        assert_eq!(metrics.get("test_endpoint_count").unwrap(), "1");
        assert_eq!(metrics.get("status_200").unwrap(), "1");
        assert_eq!(metrics.get("test_endpoint_last_duration_ms").unwrap(), "150");
        
        // Record an error
        telemetry.record_error("test_endpoint", "Test error message");
        
        // Check updated metrics
        let metrics = telemetry.metrics();
        assert_eq!(metrics.get("test_endpoint_errors").unwrap(), "1");
        assert_eq!(metrics.get("test_endpoint_last_error").unwrap(), "Test error message");
        
        // Reset metrics
        telemetry.reset_metrics();
        assert_eq!(telemetry.metrics().len(), 0);
    }
    
    // Combined tests
    
    struct CompleteMockClient {
        service: MockServiceClient,
        executor: MockRequestExecutor,
        auth: MockAuthenticatedClient,
        rate_limit: MockRateLimited,
        telemetry: MockTelemetry,
    }
    
    impl Default for CompleteMockClient {
        fn default() -> Self {
            Self {
                service: MockServiceClient::default(),
                executor: MockRequestExecutor::new(),
                auth: MockAuthenticatedClient::default(),
                rate_limit: MockRateLimited::default(),
                telemetry: MockTelemetry::default(),
            }
        }
    }
    
    #[tokio::test]
    async fn test_combined_traits() {
        // This test verifies that the traits can be used together in a single struct
        let mut client = CompleteMockClient::default();
        
        // Configure the client
        client.auth.set_auth("test_token").unwrap();
        client.rate_limit.configure_rate_limit(10, Duration::from_secs(60));
        
        // Verify auth is set
        assert!(client.auth.is_authenticated());
        
        // Verify rate limit is configured
        let status = client.rate_limit.rate_limit_status().unwrap();
        assert_eq!(status.max_requests, 10);
        
        // Record a request and check telemetry
        client.rate_limit.record_request();
        client.telemetry.record_request("test", 200, Duration::from_millis(100));
        
        let metrics = client.telemetry.metrics();
        assert_eq!(metrics.get("test_count").unwrap(), "1");
        
        // Check rate limit status
        let result = client.rate_limit.check_rate_limit().await;
        assert!(result.is_ok());
    }
}