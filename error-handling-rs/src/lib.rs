//! # Error Handling Framework
//! 
//! A comprehensive error handling framework for the Phoenix ORCH AGI system
//! with standardized error types, context-preserving error propagation,
//! structured logging, sanitization, and centralized reporting.
//!
//! ## Features
//!
//! - Standardized error types with service-specific extensions
//! - Context-preserving error propagation
//! - Structured logging with correlation IDs
//! - Error sanitization to prevent information leakage
//! - Centralized error reporting and monitoring
//! - Retry mechanisms with exponential backoff
//! - Circuit breaker implementation
//! - Fallback strategies for critical operations
//!

pub mod types;
pub mod context;
pub mod logging;
pub mod sanitization;
pub mod reporting;
pub mod retry;
pub mod circuit_breaker;
pub mod fallback;
pub mod monitoring;
pub mod supervisor;

// Re-export commonly used types
pub use types::{Error, Result, ErrorKind, ServiceError};
pub use context::{Context, ErrorContext, WithContext};
pub use logging::{init_logging, set_correlation_id, current_correlation_id};
pub use reporting::{report_error, ErrorReporter};
pub use retry::{retry, RetryPolicy, RetryableError};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use fallback::{with_fallback, FallbackStrategy};
pub use sanitization::sanitize_error;

/// Initializes the error handling framework with default settings
pub fn init() -> Result<()> {
    init_logging(None)?;
    reporting::init_reporter(None)?;
    Ok(())
}

/// Initializes the error handling framework with custom settings
pub fn init_with_config(config: config::Config) -> Result<()> {
    let log_config = config.clone().try_into().ok();
    let reporter_config = config.clone().try_into().ok();
    
    init_logging(log_config)?;
    reporting::init_reporter(reporter_config)?;
    Ok(())
}