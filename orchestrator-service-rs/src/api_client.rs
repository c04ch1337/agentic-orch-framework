//! # Orchestrator Service Client
//!
//! This module provides a client interface for external systems to communicate with the Orchestrator service.
//! The client handles authentication, connection pooling, retry logic, and proper serialization/deserialization
//! of requests and responses.
//!
//! ## Features
//!
//! - Connection pooling for efficient resource usage
//! - Retry logic with exponential backoff
//! - Authentication support with API keys/tokens
//! - Comprehensive error handling
//! - Transparent request/response serialization
//!
//! ## Examples
//!
//! ```rust
//! use orchestrator_service_rs::api_client::{OrchestratorClient, ClientConfig};
//! use agi_core::{ProtoRequest, HealthRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client with default configuration
//!     let mut client = OrchestratorClient::new("http://localhost:50051").await?;
//!
//!     // Or with custom configuration
//!     let config = ClientConfig {
//!         auth_token: Some("api-key-123".to_string()),
//!         timeout_ms: 5000,
//!         max_retries: 3,
//!         ..Default::default()
//!     };
//!     let mut client = OrchestratorClient::with_config("http://localhost:50051", config).await?;
//!
//!     // Check service health
//!     let health = client.get_health(HealthRequest {}).await?;
//!     println!("Service health: {}", health.status);
//!
//!     // Process a request
//!     let request = ProtoRequest {
//!         id: "req-123".to_string(),
//!         service: "llm-service".to_string(),
//!         method: "generate_text".to_string(),
//!         payload: b"Generate a response to: What is AGI?".to_vec(),
//!         metadata: std::collections::HashMap::new(),
//!     };
//!
//!     let response = client.process_request(request).await?;
//!     println!("Response: {}", String::from_utf8_lossy(&response.payload));
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use std::str::FromStr;

use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use futures::future::BoxFuture;
use futures::{FutureExt, TryFutureExt};
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use thiserror::Error;
use tokio::sync::Semaphore;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::transport::{Channel, Endpoint};
use tonic::{Code, Request, Response, Status};

// Import the generated Protobuf code
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    health_service_client::HealthServiceClient, 
    orchestrator_service_client::OrchestratorServiceClient,
    HealthRequest, HealthResponse, ProtoRequest, ProtoResponse, 
    RouteRequest, RouteResponse,
};

// Constants 
const DEFAULT_TIMEOUT_MS: u64 = 30_000; // 30 seconds
const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_CONNECTION_POOL_SIZE: usize = 10;
const DEFAULT_KEEPALIVE_MS: u64 = 60_000; // 1 minute
const RETRY_BASE_DURATION_MS: u64 = 100; // Start with 100ms then exponential
const CONNECTION_TIMEOUT_MS: u64 = 5_000; // 5 seconds for initial connection

/// Custom error types for the Orchestrator client
#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Failed to connect to Orchestrator service: {0}")]
    ConnectionError(String),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Request timed out after {0}ms")]
    TimeoutError(u64),
    
    #[error("Service error: {0}")]
    ServiceError(String),
    
    #[error("Invalid argument: {0}")]
    InvalidArgumentError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Max retry attempts ({0}) exceeded")]
    RetryExhaustedError(u32),
    
    #[error("Connection pool exhausted")]
    ConnectionPoolExhausted,
    
    #[error("Transport error: {0}")]
    TransportError(#[from] tonic::transport::Error),
    
    #[error("Status error: {0}")]
    StatusError(#[from] Status),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Authentication configuration for the client
#[derive(Debug, Clone)]
pub enum AuthConfig {
    /// No authentication
    None,
    /// API Key authentication
    ApiKey(String),
    /// OAuth bearer token
    BearerToken(String),
    /// Custom authentication method with header name and value
    Custom {
        header_name: String,
        header_value: String,
    },
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self::None
    }
}

/// Configuration options for the Orchestrator client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Authentication token or API key (if any)
    pub auth_token: Option<String>,
    
    /// Authentication configuration
    pub auth_config: AuthConfig,
    
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    
    /// Maximum number of retry attempts
    pub max_retries: u32,
    
    /// Connection pool size
    pub pool_size: usize,
    
    /// Keep-alive interval in milliseconds
    pub keepalive_ms: u64,
    
    /// Additional headers to include with each request
    pub headers: HashMap<String, String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            auth_token: None,
            auth_config: AuthConfig::None,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_retries: DEFAULT_MAX_RETRIES,
            pool_size: DEFAULT_CONNECTION_POOL_SIZE,
            keepalive_ms: DEFAULT_KEEPALIVE_MS,
            headers: HashMap::new(),
        }
    }
}

/// A pooled connection to the Orchestrator service
struct PooledConnection {
    orchestrator: OrchestratorServiceClient<Channel>,
    health: HealthServiceClient<Channel>,
    last_used: std::time::Instant,
}

impl PooledConnection {
    /// Create a new connection to the service
    async fn new(endpoint: Endpoint) -> Result<Self, OrchestratorError> {
        let channel = endpoint
            .connect()
            .await
            .map_err(|e| OrchestratorError::ConnectionError(e.to_string()))?;
        
        let orchestrator = OrchestratorServiceClient::new(channel.clone());
        let health = HealthServiceClient::new(channel);
        
        Ok(Self {
            orchestrator,
            health,
            last_used: std::time::Instant::now(),
        })
    }
    
    /// Check if the connection is still valid
    fn is_valid(&self, max_idle_time: Duration) -> bool {
        self.last_used.elapsed() < max_idle_time
    }
    
    /// Update the last used timestamp
    fn touch(&mut self) {
        self.last_used = std::time::Instant::now();
    }
}

/// A type-safe wrapper for the Orchestrator service client
#[derive(Clone)]
pub struct OrchestratorClient {
    endpoint: Endpoint,
    config: ClientConfig,
    connection_pool: Arc<StdMutex<Vec<PooledConnection>>>,
    semaphore: Arc<Semaphore>,
}

impl OrchestratorClient {
    /// Create a new client with default configuration
    ///
    /// # Arguments
    ///
    /// * `addr` - The address of the Orchestrator service
    ///
    /// # Returns
    ///
    /// A new OrchestratorClient instance
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint could not be created
    pub async fn new(addr: &str) -> Result<Self, OrchestratorError> {
        Self::with_config(addr, ClientConfig::default()).await
    }
    
    /// Create a new client with custom configuration
    ///
    /// # Arguments
    ///
    /// * `addr` - The address of the Orchestrator service
    /// * `config` - Client configuration options
    ///
    /// # Returns
    ///
    /// A new OrchestratorClient instance
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint could not be created
    pub async fn with_config(addr: &str, config: ClientConfig) -> Result<Self, OrchestratorError> {
        let timeout = Duration::from_millis(config.timeout_ms);
        let conn_timeout = Duration::from_millis(CONNECTION_TIMEOUT_MS);
        let keepalive = Duration::from_millis(config.keepalive_ms);
        
        let endpoint = Endpoint::from_str(addr)
            .map_err(|e| OrchestratorError::InvalidArgumentError(format!("Invalid address: {}", e)))?
            .timeout(timeout)
            .connect_timeout(conn_timeout)
            .keep_alive_timeout(keepalive)
            .tcp_keepalive(Some(keepalive));
        
        // Initialize connection pool
        let pool_size = config.pool_size;
        let semaphore = Arc::new(Semaphore::new(pool_size));
        
        let client = Self {
            endpoint,
            config,
            connection_pool: Arc::new(StdMutex::new(Vec::with_capacity(pool_size))),
            semaphore,
        };
        
        // Pre-warm the connection pool with a few connections
        let initial_conns = pool_size.min(3); // Start with at most 3 connections
        for _ in 0..initial_conns {
            match client.get_connection().await {
                Ok(conn) => {
                    if let Ok(mut pool) = client.connection_pool.lock() {
                        pool.push(conn);
                    }
                },
                Err(e) => {
                    warn!("Failed to pre-warm connection pool: {}", e);
                    break;
                }
            }
        }
        
        Ok(client)
    }
    
    /// Get a connection from the pool or create a new one
    async fn get_connection(&self) -> Result<PooledConnection, OrchestratorError> {
        // Acquire a permit from the semaphore
        let _permit = self.semaphore.acquire().await.map_err(|_| {
            OrchestratorError::InternalError("Failed to acquire semaphore permit".to_string())
        })?;
        
        // Try to get a connection from the pool
        let mut pool = self.connection_pool.lock().map_err(|e| {
            OrchestratorError::InternalError(format!("Failed to lock connection pool: {}", e))
        })?;
        
        let max_idle = Duration::from_millis(self.config.keepalive_ms);
        
        // Find a valid connection in the pool
        if let Some(index) = pool.iter().position(|conn| conn.is_valid(max_idle)) {
            let mut conn = pool.remove(index);
            conn.touch();
            return Ok(conn);
        }
        
        // Create a new connection
        debug!("Creating new connection to Orchestrator service");
        PooledConnection::new(self.endpoint.clone()).await
    }
    
    /// Return a connection to the pool
    fn return_connection(&self, conn: PooledConnection) {
        if let Ok(mut pool) = self.connection_pool.lock() {
            pool.push(conn);
        }
    }
    
    /// Apply authentication and custom headers to a request
    fn prepare_request<T>(&self, mut request: Request<T>) -> Request<T> {
        match &self.config.auth_config {
            AuthConfig::ApiKey(key) => {
                if let Ok(value) = MetadataValue::from_str(key.as_str()) {
                    request.metadata_mut().insert("x-api-key", value);
                }
            },
            AuthConfig::BearerToken(token) => {
                if let Ok(value) = MetadataValue::from_str(&format!("Bearer {}", token)) {
                    request.metadata_mut().insert("authorization", value);
                }
            },
            AuthConfig::Custom { header_name, header_value } => {
                if let Ok(value) = MetadataValue::from_str(header_value.as_str()) {
                    request.metadata_mut().insert_bin(header_name.as_str(), value.into());
                }
            },
            AuthConfig::None => {},
        }
        
        // Add any additional headers from config
        for (key, value) in &self.config.headers {
            if let Ok(header_value) = MetadataValue::from_str(value.as_str()) {
                request.metadata_mut().insert(key.as_str(), header_value);
            }
        }
        
        // Backward compatibility - use auth_token if auth_config is None
        if let (Some(token), AuthConfig::None) = (&self.config.auth_token, &self.config.auth_config) {
            if let Ok(value) = MetadataValue::from_str(&format!("Bearer {}", token)) {
                request.metadata_mut().insert("authorization", value);
            }
        }
        
        request
    }
    
    /// Create a backoff strategy with the configured parameters
    fn create_backoff(&self) -> ExponentialBackoff {
        ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_millis(RETRY_BASE_DURATION_MS))
            .with_max_elapsed_time(Some(Duration::from_millis(self.config.timeout_ms)))
            .with_max_interval(Duration::from_secs(5))
            .with_multiplier(2.0)
            .build()
    }
    
    /// Execute a request with retry logic
    async fn execute_with_retry<T, F, Fut, R>(
        &self,
        operation: F,
    ) -> Result<R, OrchestratorError>
    where
        F: Fn() -> Fut,
        Fut: futures::Future<Output = Result<T, Status>>,
        R: From<T>,
    {
        let backoff = self.create_backoff();
        let max_retries = self.config.max_retries;
        
        let operation_with_retry = || async {
            let result = operation().await;
            
            match result {
                Ok(response) => {
                    Ok(Ok(response))
                }
                Err(status) => {
                    // Determine if the error is retriable
                    match status.code() {
                        // Transient errors that warrant a retry
                        Code::Unavailable | Code::ResourceExhausted | Code::Aborted | Code::DeadlineExceeded => {
                            warn!("Retriable error occurred: {} ({})", status.message(), status.code());
                            Ok(Err(backoff::Error::transient(status)))
                        },
                        // Permanent errors that should not be retried
                        _ => {
                            error!("Non-retriable error occurred: {} ({})", status.message(), status.code());
                            Ok(Err(backoff::Error::permanent(status)))
                        }
                    }
                }
            }
        };
        
        let mut retry_count = 0;
        let retry_future = backoff::future::retry_notify(
            backoff,
            operation_with_retry,
            |err, duration| {
                retry_count += 1;
                warn!(
                    "Retrying request after error: {} (retry {}/{}; waiting {:?})",
                    err, retry_count, max_retries, duration
                );
            },
        );
        
        retry_future
            .await
            .map_err(|e| {
                if retry_count >= max_retries {
                    OrchestratorError::RetryExhaustedError(max_retries)
                } else {
                    match e {
                        backoff::Error::Transient(status) => OrchestratorError::StatusError(status),
                        backoff::Error::Permanent(status) => OrchestratorError::StatusError(status),
                    }
                }
            })?
            .map(R::from)
    }
    
    /// Process a general request through the Orchestrator service
    ///
    /// # Arguments
    ///
    /// * `request` - The request to process
    ///
    /// # Returns
    ///
    /// The processed response
    ///
    /// # Errors
    ///
    /// Returns an error if the request failed
    pub async fn process_request(&self, request: ProtoRequest) -> Result<ProtoResponse, OrchestratorError> {
        let client_clone = self.clone();
        
        self.execute_with_retry::<_, _, _, ProtoResponse>(|| async move {
            let conn = client_clone.get_connection().await?;
            
            let request = client_clone.prepare_request(Request::new(request.clone()));
            
            let result = conn.orchestrator.process_request(request).await;
            
            // Return the connection to the pool
            client_clone.return_connection(conn);
            
            // Map the result
            result.map(|r| r.into_inner())
        })
        .await
    }
    
    /// Plan and execute a complex request through the Orchestrator service
    ///
    /// This method handles breaking down complex requests into sub-tasks,
    /// planning their execution, and aggregating the results.
    ///
    /// # Arguments
    ///
    /// * `request` - The request to plan and execute
    ///
    /// # Returns
    ///
    /// The processed response
    ///
    /// # Errors
    ///
    /// Returns an error if the request failed
    pub async fn plan_and_execute(&self, request: ProtoRequest) -> Result<ProtoResponse, OrchestratorError> {
        let client_clone = self.clone();
        
        self.execute_with_retry::<_, _, _, ProtoResponse>(|| async move {
            let conn = client_clone.get_connection().await?;
            
            let request = client_clone.prepare_request(Request::new(request.clone()));
            
            let result = conn.orchestrator.plan_and_execute(request).await;
            
            // Return the connection to the pool
            client_clone.return_connection(conn);
            
            // Map the result
            result.map(|r| r.into_inner())
        })
        .await
    }
    
    /// Route a request to a specific service through the Orchestrator
    ///
    /// # Arguments
    ///
    /// * `target_service` - The target service to route to
    /// * `request` - The request to route
    ///
    /// # Returns
    ///
    /// The response from the target service
    ///
    /// # Errors
    ///
    /// Returns an error if the routing failed
    pub async fn route(&self, target_service: String, request: Option<ProtoRequest>) -> Result<RouteResponse, OrchestratorError> {
        let client_clone = self.clone();
        
        let route_req = RouteRequest {
            target_service,
            request,
        };
        
        self.execute_with_retry::<_, _, _, RouteResponse>(|| async move {
            let conn = client_clone.get_connection().await?;
            
            let request = client_clone.prepare_request(Request::new(route_req.clone()));
            
            let result = conn.orchestrator.route(request).await;
            
            // Return the connection to the pool
            client_clone.return_connection(conn);
            
            // Map the result
            result.map(|r| r.into_inner())
        })
        .await
    }
    
    /// Get health information from the Orchestrator service
    ///
    /// # Arguments
    ///
    /// * `request` - The health request (usually empty)
    ///
    /// # Returns
    ///
    /// The health response containing service status
    ///
    /// # Errors
    ///
    /// Returns an error if the health check failed
    pub async fn get_health(&self, request: HealthRequest) -> Result<HealthResponse, OrchestratorError> {
        let client_clone = self.clone();
        
        self.execute_with_retry::<_, _, _, HealthResponse>(|| async move {
            let conn = client_clone.get_connection().await?;
            
            let request = client_clone.prepare_request(Request::new(request.clone()));
            
            let result = conn.health.get_health(request).await;
            
            // Return the connection to the pool
            client_clone.return_connection(conn);
            
            // Map the result
            result.map(|r| r.into_inner())
        })
        .await
    }
    
    /// Create a proto request for use with the client
    ///
    /// # Arguments
    ///
    /// * `id` - The request ID
    /// * `service` - The target service
    /// * `method` - The method to call
    /// * `payload` - The request payload
    ///
    /// # Returns
    ///
    /// A new ProtoRequest instance
    pub fn create_request(
        id: String,
        service: String,
        method: String,
        payload: Vec<u8>,
    ) -> ProtoRequest {
        ProtoRequest {
            id,
            service,
            method,
            payload,
            metadata: HashMap::new(),
        }
    }
    
    /// Add metadata to a request
    ///
    /// # Arguments
    ///
    /// * `request` - The request to modify
    /// * `key` - The metadata key
    /// * `value` - The metadata value
    ///
    /// # Returns
    ///
    /// The modified request
    pub fn add_metadata(mut request: ProtoRequest, key: String, value: String) -> ProtoRequest {
        request.metadata.insert(key, value);
        request
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_client_creation() {
        let client = OrchestratorClient::new("http://localhost:50051").await;
        assert!(client.is_ok());
    }
    
    #[tokio::test]
    async fn test_client_with_custom_config() {
        let config = ClientConfig {
            auth_token: Some("token123".to_string()),
            timeout_ms: 10000,
            max_retries: 5,
            ..Default::default()
        };
        
        let client = OrchestratorClient::with_config("http://localhost:50051", config).await;
        assert!(client.is_ok());
    }
}