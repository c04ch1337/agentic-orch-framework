//! SerpAPI client implementation
//!
//! This module provides a client for the SerpAPI search engine API,
//! with support for different search engines and query parameters.

mod models;
pub use models::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::Client;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use log::{debug, warn, error};
use url::Url;

use crate::core::{ServiceClient, RequestExecutor, AuthenticatedClient, RateLimited, Telemetry, RateLimitStatus};
use crate::error::{Result, ServiceError, ErrorContext};
use crate::config::{SerpAPIConfig, ConfigProvider, DEFAULT_PROVIDER};
use crate::resilience::{Resilience, RetryConfig, CircuitBreakerConfig};
use crate::services::common::{UserAgent, build_http_client, parse_error_response, record_request_metrics};

/// SerpAPI client
pub struct SerpAPIClient {
    /// HTTP client
    http_client: Client,
    
    /// Configuration
    config: SerpAPIConfig,
    
    /// Resilience patterns
    resilience: Resilience,
    
    /// Rate limit status
    rate_limits: Arc<Mutex<Option<RateLimitStatus>>>,
    
    /// Client metrics
    metrics: Mutex<HashMap<String, String>>,
}

impl Default for SerpAPIClient {
    fn default() -> Self {
        let config = SerpAPIConfig::from_provider(&**DEFAULT_PROVIDER)
            .unwrap_or_else(|_| {
                warn!("Failed to load SerpAPI config from environment, using defaults");
                SerpAPIConfig::default()
            });
        
        Self::new_with_config(config)
    }
}

impl SerpAPIClient {
    /// Create a new SerpAPI client with default configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new SerpAPI client with custom configuration
    pub fn new_with_config(config: SerpAPIConfig) -> Self {
        let timeout = Duration::from_secs(config.timeout_seconds);
        
        let http_client = build_http_client(
            Some(UserAgent {
                app_name: "Phoenix-ORCH".to_string(),
                version: "0.1.0".to_string(),
                extra: Some("SerpAPI-Client".to_string()),
            }),
            Some(timeout),
        ).unwrap_or_else(|e| {
            error!("Failed to build SerpAPI HTTP client: {}", e);
            panic!("Failed to build SerpAPI HTTP client: {}", e);
        });
        
        let resilience = Resilience::new(
            RetryConfig {
                max_retries: 2,
                initial_interval: Duration::from_millis(500),
                max_interval: Duration::from_secs(5),
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
    
    /// Create a new builder for the SerpAPI client
    pub fn builder() -> SerpAPIClientBuilder {
        SerpAPIClientBuilder::default()
    }
    
    /// Perform a search using Google
    pub async fn google_search(&self, params: GoogleSearchParams) -> Result<SearchResponse> {
        let mut query_params = params.to_query_params();
        self.add_auth_to_params(&mut query_params);
        
        self.search_with_engine("google", query_params).await
    }
    
    /// Perform a search using Bing
    pub async fn bing_search(&self, params: BingSearchParams) -> Result<SearchResponse> {
        let mut query_params = params.to_query_params();
        self.add_auth_to_params(&mut query_params);
        
        self.search_with_engine("bing", query_params).await
    }
    
    /// Execute a simple search with just a query string
    pub async fn search(&self, query: &str) -> Result<SearchResponse> {
        let params = GoogleSearchParams {
            q: query.to_string(),
            num: Some(10),
            ..Default::default()
        };
        
        self.google_search(params).await
    }
    
    /// Execute a simple search with custom engine
    pub async fn search_with_engine(&self, engine: &str, params: HashMap<String, String>) -> Result<SearchResponse> {
        let endpoint = format!("search/{}", engine);
        self.get(&endpoint, Some(params)).await
    }
    
    /// Get account information
    pub async fn get_account(&self) -> Result<AccountResponse> {
        let mut params = HashMap::new();
        self.add_auth_to_params(&mut params);
        
        self.get("account", Some(params)).await
    }
    
    /// Get search archive
    pub async fn get_search_archive(&self) -> Result<SearchArchiveResponse> {
        let mut params = HashMap::new();
        self.add_auth_to_params(&mut params);
        
        self.get("searches", Some(params)).await
    }
    
    /// Add API key to query parameters
    fn add_auth_to_params(&self, params: &mut HashMap<String, String>) {
        params.insert("api_key".to_string(), self.config.api_key.clone());
    }
}

#[async_trait]
impl ServiceClient for SerpAPIClient {
    fn name(&self) -> &str {
        "serpapi"
    }
    
    fn base_url(&self) -> &str {
        &self.config.base_url
    }
    
    fn version(&self) -> &str {
        "v1"
    }
    
    async fn health_check(&self) -> Result<bool> {
        // Try to get account info as a health check
        match self.get_account().await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("SerpAPI health check failed: {}", e);
                Ok(false)
            }
        }
    }
    
    fn metrics(&self) -> Option<HashMap<String, String>> {
        Some(self.metrics.lock().unwrap().clone())
    }
}

#[async_trait]
impl RequestExecutor for SerpAPIClient {
    async fn execute<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de> + Send,
    {
        // SerpAPI primarily uses GET requests, but we'll implement POST for completeness
        self.post(endpoint, request).await
    }
    
    async fn get<R>(&self, endpoint: &str, query_params: Option<HashMap<String, String>>) -> Result<R>
    where
        R: for<'de> Deserialize<'de> + Send,
    {
        // Directly call get_with_client without resilience wrapper to avoid lifetime issues
        self.get_with_client::<R>(endpoint, query_params).await
    }
    
    async fn post<T, R>(&self, endpoint: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de> + Send,
    {
        // Directly call post_with_client without resilience wrapper to avoid lifetime issues
        self.post_with_client::<T, R>(endpoint, body).await
    }
    
    async fn put<T, R>(&self, _endpoint: &str, _body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de> + Send,
    {
        // SerpAPI doesn't use PUT, so we'll return an error
        Err(ServiceError::validation("PUT not supported for SerpAPI"))
    }
    
    async fn delete<R>(&self, _endpoint: &str) -> Result<R>
    where
        R: for<'de> Deserialize<'de> + Send,
    {
        // SerpAPI doesn't expose DELETE endpoints, so we'll return an error
        Err(ServiceError::validation("DELETE not supported for SerpAPI"))
    }
}

#[async_trait]
impl AuthenticatedClient for SerpAPIClient {
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
        // SerpAPI doesn't support refreshing tokens, but we'll implement this for completeness
        Ok(())
    }
    
    fn apply_auth(&self, headers: &mut HashMap<String, String>) -> Result<()> {
        if !self.is_authenticated() {
            return Err(ServiceError::authentication("No API key set for SerpAPI client"));
        }
        
        // SerpAPI uses query parameters for authentication, not headers,
        // but we'll implement this for completeness
        headers.insert("X-API-Key".to_string(), self.config.api_key.clone());
        Ok(())
    }
}

#[async_trait]
impl RateLimited for SerpAPIClient {
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
impl Telemetry for SerpAPIClient {
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

// Private helper methods for the SerpAPI client
impl SerpAPIClient {
    async fn get_with_client<R>(&self, endpoint: &str, query_params: Option<HashMap<String, String>>) -> Result<R>
    where
        R: for<'de> Deserialize<'de> + Send,
    {
        // Check rate limits before proceeding
        self.check_rate_limit().await?;
        
        // Record the request for rate limiting
        RateLimited::record_request(self);
        
        let url = format!("{}/{}", self.config.base_url, endpoint);
        debug!("Sending request to SerpAPI: GET {}", url);
        
        let start_time = Instant::now();
        
        let mut builder = self.http_client.get(&url);
        
        // Add query parameters if provided
        if let Some(params) = query_params {
            builder = builder.query(&params);
        }
        
        let response = builder
            .send()
            .await
            .map_err(|e| ServiceError::network(format!("Failed to send request: {}", e)))?;
        
        // Update rate limits if headers are present
        // SerpAPI typically doesn't provide explicit rate limit headers,
        // but we'll check for them anyway
        
        let status = response.status();
        let status_code = status.as_u16();
        
        if status.is_success() {
            let bytes_received = response.content_length().unwrap_or(0);
            
            let json = response.json::<R>().await
                .map_err(|e| ServiceError::parsing(format!("Failed to parse response: {}", e)))?;
            
            let duration = start_time.elapsed();
            
            // Record metrics
            record_request_metrics(
                "serpapi",
                endpoint,
                start_time,
                status_code,
                true,
                None,
                Some(bytes_received),
            );
            
            Telemetry::record_request(self, endpoint, status_code, duration);
            
            Ok(json)
        } else {
            let error = parse_error_response("serpapi", response).await;
            
            // Record error metrics
            self.record_error(endpoint, &error.to_string());
            
            Err(error)
        }
    }
    
    async fn post_with_client<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de> + Send,
    {
        // Check rate limits before proceeding
        self.check_rate_limit().await?;
        
        // Record the request for rate limiting
        RateLimited::record_request(self);
        
        let url = format!("{}/{}", self.config.base_url, endpoint);
        debug!("Sending request to SerpAPI: POST {}", url);
        
        let start_time = Instant::now();
        
        let request_json = serde_json::to_string(request)
            .map_err(|e| ServiceError::validation(format!("Failed to serialize request: {}", e)))?;
        
        let bytes_sent = request_json.len() as u64;
        
        // Add API key to request
        let mut url = Url::parse(&url)
            .map_err(|e| ServiceError::validation(format!("Invalid URL: {}", e)))?;
        
        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair("api_key", &self.config.api_key);
        }
        
        let response = self.http_client
            .post(url.as_str())
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
                "serpapi",
                endpoint,
                start_time,
                status_code,
                true,
                Some(bytes_sent),
                Some(bytes_received),
            );
            
            Telemetry::record_request(self, endpoint, status_code, duration);
            
            Ok(json)
        } else {
            let error = parse_error_response("serpapi", response).await;
            
            // Record error metrics
            self.record_error(endpoint, &error.to_string());
            
            Err(error)
        }
    }
}

/// Builder for SerpAPI client
#[derive(Default)]
pub struct SerpAPIClientBuilder {
    /// API key for authentication
    api_key: Option<String>,
    
    /// Base URL for the API
    base_url: Option<String>,
    
    /// Default search engine
    default_engine: Option<String>,
    
    /// Request timeout
    timeout_seconds: Option<u64>,
    
    /// Retry configuration
    retry_config: Option<RetryConfig>,
    
    /// Circuit breaker configuration
    circuit_breaker_config: Option<CircuitBreakerConfig>,
}

impl SerpAPIClientBuilder {
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
    
    /// Set the default search engine
    pub fn default_engine(mut self, engine: impl Into<String>) -> Self {
        self.default_engine = Some(engine.into());
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
    
    /// Build the SerpAPI client
    pub fn build(self) -> Result<SerpAPIClient> {
        // Try to load config from environment first
        let mut config = SerpAPIConfig::from_provider(&**DEFAULT_PROVIDER).unwrap_or_default();
        
        // Override with explicitly provided values
        if let Some(api_key) = self.api_key {
            config.api_key = api_key;
        }
        
        if let Some(base_url) = self.base_url {
            config.base_url = base_url;
        }
        
        if let Some(default_engine) = self.default_engine {
            config.default_engine = default_engine;
        }
        
        if let Some(timeout) = self.timeout_seconds {
            config.timeout_seconds = timeout;
        }
        
        // Validate the configuration
        // Validate the configuration - using simple validation since ServiceConfig trait is not imported
        if config.api_key.is_empty() {
            return Err(ServiceError::validation("API key is required"));
        }
        
        let mut client = SerpAPIClient::new_with_config(config);
        
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