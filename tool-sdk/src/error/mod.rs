//! Error handling for the Tool SDK
//!
//! This module provides a comprehensive error system that:
//! - Categorizes errors by type (network, auth, rate limit, etc.)
//! - Adds rich context to errors for better debugging
//! - Maps service-specific errors to normalized formats
//! - Provides convenient Result type alias

use std::fmt;
use std::collections::HashMap;
use thiserror::Error;

pub mod mapping;

/// Result type for Tool SDK operations
pub type Result<T> = std::result::Result<T, ServiceError>;

/// Main error type for the Tool SDK
#[derive(Error, Debug)]
pub enum ServiceError {
    /// Network or connection errors
    #[error("Network error: {0}")]
    Network(String),
    
    /// Authentication errors
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    /// Authorization errors (permission issues)
    #[error("Authorization error: {0}")]
    Authorization(String),
    
    /// Rate limiting errors
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),
    
    /// Service-specific errors
    #[error("Service error: {0}")]
    Service(String),
    
    /// Request validation errors
    #[error("Validation error: {0}")]
    Validation(String),
    
    /// Response parsing errors
    #[error("Parsing error: {0}")]
    Parsing(String),
    
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    /// Timeout errors
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    /// Unexpected or internal errors
    #[error("Internal error: {0}")]
    Internal(String),
    
    /// Resource not found errors
    #[error("Not found: {0}")]
    NotFound(String),
    
    /// Circuit breaker open errors
    #[error("Circuit broken: {0}")]
    CircuitBroken(String),
    
    /// External service errors
    #[error("External service error: {0}")]
    ExternalService(String),
    
    /// Unknown errors
    #[error("Unknown error: {0}")]
    Unknown(String),
    
    /// Errors with additional context
    #[error("{inner}")]
    WithContext {
        inner: Box<ServiceError>,
        context: ErrorContext,
    },
}

impl ServiceError {
    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        ServiceError::Network(message.into())
    }
    
    /// Create an authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        ServiceError::Authentication(message.into())
    }
    
    /// Create an authorization error
    pub fn authorization(message: impl Into<String>) -> Self {
        ServiceError::Authorization(message.into())
    }
    
    /// Create a rate limit error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        ServiceError::RateLimit(message.into())
    }
    
    /// Create a service-specific error
    pub fn service(message: impl Into<String>) -> Self {
        ServiceError::Service(message.into())
    }
    
    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        ServiceError::Validation(message.into())
    }
    
    /// Create a parsing error
    pub fn parsing(message: impl Into<String>) -> Self {
        ServiceError::Parsing(message.into())
    }
    
    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        ServiceError::Configuration(message.into())
    }
    
    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        ServiceError::Timeout(message.into())
    }
    
    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        ServiceError::Internal(message.into())
    }
    
    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        ServiceError::NotFound(message.into())
    }
    
    /// Create a circuit broken error
    pub fn circuit_broken(message: impl Into<String>) -> Self {
        ServiceError::CircuitBroken(message.into())
    }
    
    /// Create an external service error
    pub fn external_service(message: impl Into<String>) -> Self {
        ServiceError::ExternalService(message.into())
    }
    
    /// Create an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        ServiceError::Unknown(message.into())
    }
    
    /// Add context to an existing error
    pub fn with_context(self, context: ErrorContext) -> Self {
        ServiceError::WithContext {
            inner: Box::new(self),
            context,
        }
    }
    
    /// Add a single context key/value to an existing error
    pub fn with_context_value(self, key: impl Into<String>, value: impl fmt::Display) -> Self {
        let mut context = ErrorContext::new();
        context.add(key, value);
        self.with_context(context)
    }
    
    /// Get the error code if available
    pub fn error_code(&self) -> Option<&str> {
        match self {
            ServiceError::WithContext { context, .. } => context.error_code.as_deref(),
            _ => None,
        }
    }
    
    /// Get the service name if available
    pub fn service_name(&self) -> Option<&str> {
        match self {
            ServiceError::WithContext { context, .. } => Some(&context.service),
            _ => None,
        }
    }
    
    /// Get the HTTP status code if available
    pub fn status_code(&self) -> Option<u16> {
        match self {
            ServiceError::WithContext { context, .. } => context.status_code,
            _ => None,
        }
    }
    
    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        match self {
            ServiceError::Network(_) => true,
            ServiceError::Timeout(_) => true,
            ServiceError::RateLimit(_) => true,
            ServiceError::CircuitBroken(_) => true,
            ServiceError::WithContext { inner, .. } => inner.is_retryable(),
            _ => false,
        }
    }
    
    /// Check if this is a permanent error (not retryable)
    pub fn is_permanent(&self) -> bool {
        !self.is_retryable()
    }
}

/// Error context information
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Service that generated the error
    pub service: String,
    
    /// Request timestamp
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    
    /// HTTP status code if applicable
    pub status_code: Option<u16>,
    
    /// Service-specific error code
    pub error_code: Option<String>,
    
    /// Request ID for tracing
    pub request_id: Option<String>,
    
    /// Endpoint that was called
    pub endpoint: Option<String>,
    
    /// Additional context data
    pub data: HashMap<String, String>,
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            service: "unknown".to_string(),
            timestamp: Some(chrono::Utc::now()),
            status_code: None,
            error_code: None,
            request_id: None,
            endpoint: None,
            data: HashMap::new(),
        }
    }
}

impl ErrorContext {
    /// Create a new error context
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new error context for a specific service
    pub fn for_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            ..Self::default()
        }
    }
    
    /// Add an HTTP status code
    pub fn status_code(mut self, code: u16) -> Self {
        self.status_code = Some(code);
        self
    }
    
    /// Add an error code
    pub fn error_code(mut self, code: impl Into<String>) -> Self {
        self.error_code = Some(code.into());
        self
    }
    
    /// Add a request ID
    pub fn request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }
    
    /// Add an endpoint
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }
    
    /// Add a context value
    pub fn add<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: fmt::Display,
    {
        self.data.insert(key.into(), value.to_string());
    }
    
    /// Add a context value and return self (builder pattern)
    pub fn with<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: fmt::Display,
    {
        self.add(key, value);
        self
    }
}

/// Convert reqwest errors to ServiceError
impl From<reqwest::Error> for ServiceError {
    fn from(err: reqwest::Error) -> Self {
        let context = ErrorContext::for_service("http_client");
        
        let service_error = if err.is_timeout() {
            ServiceError::timeout(format!("Request timed out: {}", err))
        } else if err.is_connect() {
            ServiceError::network(format!("Connection error: {}", err))
        } else if err.is_request() {
            ServiceError::validation(format!("Invalid request: {}", err))
        } else if err.is_redirect() {
            ServiceError::network(format!("Too many redirects: {}", err))
        } else if err.is_decode() {
            ServiceError::parsing(format!("Response decode error: {}", err))
        } else {
            ServiceError::internal(format!("HTTP client error: {}", err))
        };
        
        // Add status code if available
        if let Some(status) = err.status() {
            service_error.with_context(context.status_code(status.as_u16()))
        } else {
            service_error.with_context(context)
        }
    }
}

/// Convert serde_json errors to ServiceError
impl From<serde_json::Error> for ServiceError {
    fn from(err: serde_json::Error) -> Self {
        ServiceError::parsing(format!("JSON error: {}", err))
            .with_context(ErrorContext::for_service("json"))
    }
}
