//! # Standardized Error Types
//! 
//! This module provides a comprehensive set of standardized error types
//! for use throughout the Phoenix ORCH AGI system.

use std::fmt;
use std::error::Error as StdError;
use std::backtrace::Backtrace;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// A type alias for Result with the error type defaulting to our Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The severity level of an error
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Informational message, not an actual error
    Info,
    /// A minor issue that doesn't affect overall functionality
    Minor,
    /// A significant issue that may impact some functionality
    Major,
    /// A critical issue that severely impacts system functionality
    Critical,
    /// A fatal error that requires immediate attention
    Fatal,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Minor => write!(f, "MINOR"),
            Severity::Major => write!(f, "MAJOR"),
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::Fatal => write!(f, "FATAL"),
        }
    }
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Major
    }
}

/// Categorizes different kinds of errors
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorKind {
    // System-level errors
    /// Initialization or configuration error
    Initialization,
    /// Resource allocation or management error
    Resource,
    /// Error in communication between services
    Communication,
    /// Authentication or authorization error
    Authentication,
    /// Error in data validation
    Validation,
    /// Error in data processing or transformation
    Processing,
    /// Database or storage error
    Storage,
    /// External service or API error
    External,
    /// Concurrency or synchronization error
    Concurrency,
    /// Internal server error
    Internal,
    /// Timeout error
    Timeout,
    /// Resource unavailable or service degraded
    Unavailable,
    /// Input/output error
    IO,
    /// Security-related error
    Security,
    /// Error specific to a particular service
    Service(String),
    /// Rate limiting or throttling error
    RateLimit,
    /// Unexpected or unhandled error
    Unexpected,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Initialization => write!(f, "Initialization Error"),
            ErrorKind::Resource => write!(f, "Resource Error"),
            ErrorKind::Communication => write!(f, "Communication Error"),
            ErrorKind::Authentication => write!(f, "Authentication Error"),
            ErrorKind::Validation => write!(f, "Validation Error"),
            ErrorKind::Processing => write!(f, "Processing Error"),
            ErrorKind::Storage => write!(f, "Storage Error"),
            ErrorKind::External => write!(f, "External Service Error"),
            ErrorKind::Concurrency => write!(f, "Concurrency Error"),
            ErrorKind::Internal => write!(f, "Internal Server Error"),
            ErrorKind::Timeout => write!(f, "Timeout Error"),
            ErrorKind::Unavailable => write!(f, "Service Unavailable Error"),
            ErrorKind::IO => write!(f, "I/O Error"),
            ErrorKind::Security => write!(f, "Security Error"),
            ErrorKind::Service(service) => write!(f, "{} Error", service),
            ErrorKind::RateLimit => write!(f, "Rate Limit Error"),
            ErrorKind::Unexpected => write!(f, "Unexpected Error"),
        }
    }
}

/// Core error type for the Phoenix ORCH AGI system
///
/// Note: `Clone` is implemented manually so that cloned errors intentionally
/// drop the underlying `cause` and `backtrace`. This keeps clones cheap and
/// serialization-friendly while preserving all structured metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    /// A unique identifier for this error instance
    pub id: Uuid,
    /// The kind of error that occurred
    pub kind: ErrorKind,
    /// Detailed error message
    pub message: String,
    /// The time when the error occurred
    pub timestamp: DateTime<Utc>,
    /// Error severity level
    pub severity: Severity,
    /// The service where the error originated
    pub service: Option<String>,
    /// Correlation ID for request tracing
    pub correlation_id: Option<String>,
    /// Error code for categorization and documentation
    pub code: Option<String>,
    /// User-facing message (sanitized)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_message: Option<String>,
    /// Additional context as key-value pairs
    #[serde(default)]
    pub context: serde_json::Map<String, serde_json::Value>,
    /// Chain of causes (not serialized)
    #[serde(skip)]
    pub cause: Option<Box<dyn StdError + Send + Sync>>,
    /// Backtrace (not serialized)
    #[serde(skip)]
    pub backtrace: Option<Backtrace>,
    /// Flag indicating if this error has been reported
    #[serde(skip)]
    pub reported: bool,
    /// Flag indicating if this is a transient error that might succeed on retry
    pub transient: bool,
}

impl Clone for Error {
    /// Cloning an `Error` preserves all structured metadata (kind, message,
    /// codes, context, etc.) but intentionally drops the opaque `cause` and
    /// `backtrace` fields, which are typically not serializable and are only
    /// meaningful at the original creation site.
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            kind: self.kind.clone(),
            message: self.message.clone(),
            timestamp: self.timestamp.clone(),
            severity: self.severity,
            service: self.service.clone(),
            correlation_id: self.correlation_id.clone(),
            code: self.code.clone(),
            user_message: self.user_message.clone(),
            context: self.context.clone(),
            cause: None,
            backtrace: None,
            reported: self.reported,
            transient: self.transient,
        }
    }
}

impl Error {
    /// Creates a new error with the specified kind and message
    pub fn new<S: Into<String>>(kind: ErrorKind, message: S) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            message: message.into(),
            timestamp: Utc::now(),
            severity: Severity::default(),
            service: None,
            correlation_id: crate::logging::current_correlation_id(),
            code: None,
            user_message: None,
            context: serde_json::Map::new(),
            cause: None,
            backtrace: Some(Backtrace::capture()),
            reported: false,
            transient: false,
        }
    }

    /// Sets the error severity
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Sets the service name
    pub fn service<S: Into<String>>(mut self, service: S) -> Self {
        self.service = Some(service.into());
        self
    }

    /// Sets the error code
    pub fn code<S: Into<String>>(mut self, code: S) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Sets a user-friendly message
    pub fn user_message<S: Into<String>>(mut self, message: S) -> Self {
        self.user_message = Some(message.into());
        self
    }

    /// Adds context information to the error
    pub fn context<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Serialize,
    {
        if let Ok(value) = serde_json::to_value(value) {
            self.context.insert(key.into(), value);
        }
        self
    }

    /// Chains this error with its cause
    pub fn cause<E>(mut self, cause: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.cause = Some(Box::new(cause));
        self
    }

    /// Marks this error as transient (can be retried)
    pub fn transient(mut self) -> Self {
        self.transient = true;
        self
    }

    /// Marks this error as reported to avoid duplicate reporting
    pub fn mark_reported(&mut self) {
        self.reported = true;
    }

    /// Returns true if this error is transient and might succeed on retry
    pub fn is_transient(&self) -> bool {
        self.transient
    }
    
    /// Converts the error to a sanitized version suitable for external responses
    pub fn sanitize(&self) -> Self {
        crate::sanitization::sanitize_error(self.clone())
    }

    /// Reports the error to the error reporting system if not already reported
    pub fn report(&mut self) -> Result<()> {
        if !self.reported {
            crate::reporting::report_error(self)?;
            self.mark_reported();
        }
        Ok(())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.kind, self.message)?;
        
        if let Some(code) = &self.code {
            write!(f, " (Code: {})", code)?;
        }
        
        if let Some(service) = &self.service {
            write!(f, " [Service: {}]", service)?;
        }
        
        if let Some(correlation_id) = &self.correlation_id {
            write!(f, " [CorrelationID: {}]", correlation_id)?;
        }

        Ok(())
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.cause.as_ref().map(|e| e.as_ref() as &(dyn StdError + 'static))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::new(ErrorKind::IO, err.to_string()).cause(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        let kind = if err.is_timeout() {
            ErrorKind::Timeout
        } else if err.is_connect() {
            ErrorKind::Communication
        } else {
            ErrorKind::External
        };
        
        let transient = err.is_timeout() || 
                        err.is_connect() || 
                        err.status().map_or(false, |s| s.as_u16() >= 500);
        
        let mut error = Self::new(kind, format!("HTTP request error: {}", err)).cause(err);
        if transient {
            error = error.transient();
        }
        error
    }
}

/// A trait for creating service-specific error types
pub trait ServiceError: StdError + Send + Sync + 'static {
    /// Converts the service error to the standard Error type
    fn to_error(&self) -> Error;
    
    /// Returns true if this error is transient and might succeed on retry
    fn is_transient(&self) -> bool;
    
    /// Returns the error's severity level
    fn severity(&self) -> Severity;
}

// Additional From implementations for common error types
impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::new(ErrorKind::Processing, format!("JSON error: {}", err)).cause(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = Error::new(ErrorKind::Validation, "Invalid input format")
            .service("test-service")
            .code("VAL-001")
            .context("field", "username")
            .severity(Severity::Minor);

        assert_eq!(err.kind, ErrorKind::Validation);
        assert_eq!(err.message, "Invalid input format");
        assert_eq!(err.service, Some("test-service".to_string()));
        assert_eq!(err.code, Some("VAL-001".to_string()));
        assert_eq!(err.severity, Severity::Minor);
    }

    #[test]
    fn test_error_display() {
        let err = Error::new(ErrorKind::Validation, "Invalid input")
            .service("test-service")
            .code("VAL-001");

        let display = format!("{}", err);
        assert!(display.contains("MAJOR"));
        assert!(display.contains("Validation Error"));
        assert!(display.contains("Invalid input"));
        assert!(display.contains("Code: VAL-001"));
        assert!(display.contains("Service: test-service"));
    }
}