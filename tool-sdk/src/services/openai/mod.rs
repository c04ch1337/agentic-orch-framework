//! OpenAI API client implementation
//!
//! This module provides a strongly-typed client for the OpenAI API,
//! with support for chat completions and embeddings.

mod models;
pub use models::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::{Client, header};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use log::{debug, warn, error};

use crate::core::{ServiceClient, RequestExecutor, AuthenticatedClient, RateLimited, Telemetry, RateLimitStatus};
use crate::error::{Result, ServiceError, ErrorContext};
use crate::config::{OpenAIConfig, ConfigProvider, DEFAULT_PROVIDER};
use crate::resilience::{Resilience, RetryConfig, CircuitBreakerConfig};
use crate::services::common::{UserAgent, build_http_client, parse_error_response, record_request_metrics};

/// OpenAI API client
pub struct OpenAIClient {
    /// HTTP client
    http_client: Client,
    
    /// Configuration
    config: OpenAIConfig,
    
    /// Resilience patterns
    resilience: Resilience,
    
    /// Rate limit status
    rate_limits: Arc<Mutex<Option<RateLimitStatus>>>,
    
    /// Client metrics
    metrics: Mutex<HashMap<String, String>>,
}

impl Default for OpenAIClient {
    fn default() -> Self {
        let config = OpenAIConfig::from_provider(&**DEFAULT_PROVIDER)
            .unwrap_or_else(|_| {
                warn!("Failed to load OpenAI config from environment, using defaults");
                OpenAIConfig::default()
            });
        
        Self::new_with_config(config)
    }
}

impl OpenAIClient {
    /// Create a new OpenAI client with default configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new OpenAI client with custom configuration
    pub fn new_with_config(config: OpenAIConfig) -> Self {
        let timeout = Duration::from_secs(config.timeout_seconds);
        
        let http_client = build_http_client(
            Some(UserAgent {
                app_name: "Phoenix-ORCH".to_string(),
                version: "0.1.0".to_string(),
                extra: Some("OpenAI-Client".to_string()),
            }),
            Some(timeout),
        ).unwrap_or_else(|e| {
            error!("Failed to build OpenAI HTTP client: {}", e);
            panic!("Failed to build OpenAI HTTP client: {}", e);
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
    
    /// Create a new builder for the OpenAI client
    pub fn builder() -> OpenAIClientBuilder {
        OpenAIClientBuilder::default()
    }
    
    /// Send a chat completion request
    pub async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        self.execute("chat/completions", &request).await
    }
    
    /// Send a text embedding request
    pub async fn embeddings(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        self.execute("embeddings", &request).await
    }
    
    /// List available models
    pub async fn list_models(&self) -> Result<ListModelsResponse> {
        self.get("models", None).await
    }
    
    /// Get model details
    pub async fn get_model(&self, model_id: &str) -> Result<Model> {
        let endpoint = format!("models/{}", model_id);
        self.get(&endpoint, None).await
    }
    
    /// Create a simple chat completion with just a message
    pub async fn simple_completion(
        &self,
        message: &str,
        model: Option<&str>,
    ) -> Result<String> {
        let model = model.unwrap_or("gpt-3.5-turbo");
        
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: message.to_string(),
                name: None,
            }],
            temperature: Some(0.7),
            ..Default::default()
        };
        
        let response = self.chat_completion(request).await?;
        if let Some(choice) = response.choices.first() {
            if let Some(content) = &choice.message.content {
                Ok(content.clone())
            } else {
                Err(ServiceError::parsing("Empty completion response"))
            }
        } else {
            Err(ServiceError::parsing("No completion choices returned"))
        }
    }
    
    /// Generate embeddings for a text
    pub async fn embed_text(&self, text: &str, model: Option<&str>) -> Result<Vec<f32>> {
        let model = model.unwrap_or("text-embedding-ada-002");
        
        let request = EmbeddingRequest {
            model: model.to_string(),
            input: EmbeddingInput::String(text.to_string()),
            encoding_format: None,
            user: None,
        };
        
        let response = self.embeddings(request).await?;
        if let Some(embedding) = response.data.first() {
            Ok(embedding.embedding.clone())
        } else {
            Err(ServiceError::parsing("No embeddings returned"))
        }
    }
    
    /// Extracts rate limit information from response headers
    fn extract_rate_limits(&self, headers: &header::HeaderMap) -> Option<RateLimitStatus> {
        let remaining = headers.get("x-ratelimit-remaining-requests")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u32>().ok());
        
        let reset = headers.get("x-ratelimit-reset-requests")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        
        let limit = headers.get("x-ratelimit-limit-requests")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u32>().ok());
        
        match (remaining, reset, limit) {
            (Some(remaining), Some(reset), Some(limit)) => {
                Some(RateLimitStatus {
                    max_requests: limit,
                    period: Duration::from_secs(reset),
                    current_count: limit - remaining,
                    reset_after: Duration::from_secs(reset),
                    enforced: true,
                })
            }
            _ => None,
        }
    }
}

#[async_trait]
impl ServiceClient for OpenAIClient {
    fn name(&self) -> &str {
        "openai"
    }
    
    fn base_url(&self) -> &str {
        &self.config.base_url
    }
    
    fn version(&self) -> &str {
        "v1"
    }
    
    async fn health_check(&self) -> Result<bool> {
        // Just check if we can list models as a health check
        match self.list_models().await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("OpenAI health check failed: {}", e);
                Ok(false)
            }
        }
    }
    
    fn metrics(&self) -> Option<HashMap<String, String>> {
        Some(self.metrics.lock().unwrap().clone())
    }
}

#[async_trait]
impl RequestExecutor for OpenAIClient {
    async fn execute<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de> + Send,
    {
        // Directly call execute_with_client without resilience wrapper to avoid lifetime issues
        self.execute_with_client(endpoint, request).await
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
        self.execute(endpoint, body).await
    }
    
    async fn put<T, R>(&self, _endpoint: &str, _body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de> + Send,
    {
        // OpenAI doesn't use PUT, so we'll return an error
        Err(ServiceError::validation("PUT not supported for OpenAI API"))
    }
    
    async fn delete<R>(&self, _endpoint: &str) -> Result<R>
    where
        R: for<'de> Deserialize<'de> + Send,
    {
        // OpenAI doesn't expose DELETE endpoints in the basic API, so we'll return an error
        Err(ServiceError::validation("DELETE not supported for OpenAI API"))
    }
}

#[async_trait]
impl AuthenticatedClient for OpenAIClient {
    fn auth_type(&self) -> &str {
        "Bearer"
    }
    
    fn set_auth(&mut self, auth: impl Into<String> + Send) -> Result<()> {
        self.config.api_key = auth.into();
        Ok(())
    }
    
    fn is_authenticated(&self) -> bool {
        !self.config.api_key.is_empty()
    }
    
    async fn refresh_auth(&mut self) -> Result<()> {
        // OpenAI doesn't support refreshing tokens, but we'll implement this for completeness
        Ok(())
    }
    
    fn apply_auth(&self, headers: &mut HashMap<String, String>) -> Result<()> {
        if !self.is_authenticated() {
            return Err(ServiceError::authentication("No API key set for OpenAI client"));
        }
        
        headers.insert("Authorization".to_string(), format!("Bearer {}", self.config.api_key));
        
        // Add organization if specified
        if let Some(ref org) = self.config.org_id {
            headers.insert("OpenAI-Organization".to_string(), org.clone());
        }
        
        Ok(())
    }
}

#[async_trait]
impl RateLimited for OpenAIClient {
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
impl Telemetry for OpenAIClient {
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

// Private helper methods for the OpenAI client
impl OpenAIClient {
    async fn execute_with_client<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de> + Send,
    {
        // Check rate limits before proceeding
        self.check_rate_limit().await?;
        
        // Record the request for rate limiting
        RateLimited::record_request(self);
        
        let url = format!("{}/{}", self.config.base_url, endpoint);
        debug!("Sending request to OpenAI: POST {}", url);
        
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
        
        // Extract and update rate limits from response headers
        if let Some(rate_limit) = self.extract_rate_limits(response.headers()) {
            let mut rate_limits = self.rate_limits.lock().unwrap();
            *rate_limits = Some(rate_limit);
        }
        
        let status = response.status();
        let status_code = status.as_u16();
        
        if status.is_success() {
            let bytes_received = response.content_length().unwrap_or(0);
            
            let json = response.json::<R>().await
                .map_err(|e| ServiceError::parsing(format!("Failed to parse response: {}", e)))?;
            
            let duration = start_time.elapsed();
            
            // Record metrics
            record_request_metrics(
                "openai",
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
            let error = parse_error_response("openai", response).await;
            
            // Record error metrics
            self.record_error(endpoint, &error.to_string());
            
            Err(error)
        }
    }
    
    async fn get_with_client<R>(&self, endpoint: &str, query_params: Option<HashMap<String, String>>) -> Result<R>
    where
        R: for<'de> Deserialize<'de> + Send,
    {
        // Check rate limits before proceeding
        self.check_rate_limit().await?;
        
        // Record the request for rate limiting
        RateLimited::record_request(self);
        
        let url = format!("{}/{}", self.config.base_url, endpoint);
        debug!("Sending request to OpenAI: GET {}", url);
        
        let start_time = Instant::now();
        let mut auth_headers = HashMap::new();
        self.apply_auth(&mut auth_headers)?;
        
        let mut builder = self.http_client.get(&url);
        
        // Add query parameters if provided
        if let Some(params) = query_params {
            builder = builder.query(&params);
        }
        
        // Add authentication headers
        for (key, value) in &auth_headers {
            builder = builder.header(key, value);
        }
        
        let response = builder
            .send()
            .await
            .map_err(|e| ServiceError::network(format!("Failed to send request: {}", e)))?;
        
        // Extract and update rate limits from response headers
        if let Some(rate_limit) = self.extract_rate_limits(response.headers()) {
            let mut rate_limits = self.rate_limits.lock().unwrap();
            *rate_limits = Some(rate_limit);
        }
        
        let status = response.status();
        let status_code = status.as_u16();
        
        if status.is_success() {
            let bytes_received = response.content_length().unwrap_or(0);
            
            let json = response.json::<R>().await
                .map_err(|e| ServiceError::parsing(format!("Failed to parse response: {}", e)))?;
            
            let duration = start_time.elapsed();
            
            // Record metrics
            record_request_metrics(
                "openai",
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
            let error = parse_error_response("openai", response).await;
            
            // Record error metrics
            self.record_error(endpoint, &error.to_string());
            
            Err(error)
        }
    }
}

/// Builder for OpenAI client
#[derive(Default)]
pub struct OpenAIClientBuilder {
    /// API key for authentication
    api_key: Option<String>,
    
    /// Organization ID
    org_id: Option<String>,
    
    /// Base URL for the API
    base_url: Option<String>,
    
    /// Request timeout
    timeout_seconds: Option<u64>,
    
    /// Retry configuration
    retry_config: Option<RetryConfig>,
    
    /// Circuit breaker configuration
    circuit_breaker_config: Option<CircuitBreakerConfig>,
}

impl OpenAIClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the API key
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
    
    /// Set the organization ID
    pub fn org_id(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
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
    
    /// Build the OpenAI client
    pub fn build(self) -> Result<OpenAIClient> {
        // Try to load config from environment first
        let mut config = OpenAIConfig::from_provider(&**DEFAULT_PROVIDER).unwrap_or_default();
        
        // Override with explicitly provided values
        if let Some(api_key) = self.api_key {
            config.api_key = api_key;
        }
        
        if let Some(org_id) = self.org_id {
            config.org_id = Some(org_id);
        }
        
        if let Some(base_url) = self.base_url {
            config.base_url = base_url;
        }
        
        if let Some(timeout) = self.timeout_seconds {
            config.timeout_seconds = timeout;
        }
        
        // Validate the configuration
        // Validate the configuration - using simple validation since ServiceConfig trait is not imported
        if config.api_key.is_empty() {
            return Err(ServiceError::validation("API key is required"));
        }
        
        let mut client = OpenAIClient::new_with_config(config);
        
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