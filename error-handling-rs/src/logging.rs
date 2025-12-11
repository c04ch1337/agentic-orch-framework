//! # Structured Logging
//! 
//! This module provides structured logging with correlation ID tracking
//! across service boundaries.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tracing::{Subscriber, Level};
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, Registry};
use tracing_appender::rolling::RollingFileAppender;
use tracing_appender::non_blocking::NonBlocking;
use uuid::Uuid;
use crate::types::{Result, Error, ErrorKind};

// Thread-local storage for the current correlation ID
thread_local! {
    static CORRELATION_ID: RwLock<Option<String>> = RwLock::new(None);
}

// Flag to track if logging has been initialized
static LOGGING_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Configuration for the logging system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// The log level to use (trace, debug, info, warn, error)
    pub level: String,
    /// The service name for identification
    pub service_name: String,
    /// Whether to output logs to a file
    pub file_output: bool,
    /// The directory to store log files in
    pub log_dir: Option<String>,
    /// Whether to use JSON formatting
    pub json_format: bool,
    /// Whether to include source code information
    pub include_source_code: bool,
    /// Custom fields to add to every log
    pub custom_fields: HashMap<String, serde_json::Value>,
    /// Whether to capture spans for request tracing
    pub enable_tracing: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            service_name: "unknown-service".to_string(),
            file_output: false,
            log_dir: None,
            json_format: true,
            include_source_code: true,
            custom_fields: HashMap::new(),
            enable_tracing: true,
        }
    }
}

/// Initializes the structured logging system
pub fn init_logging(config: Option<LoggingConfig>) -> Result<()> {
    // Don't re-initialize if already done
    if LOGGING_INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }

    let config = config.unwrap_or_default();
    
    // Parse log level
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new(format!("{},warn", config.level))
        });
    
    // Create subscriber with multiple layers
    let subscriber = Registry::default().with(filter);
    
    // Attach an appropriate formatting layer based on configuration. We build
    // distinct layers for JSON vs text output rather than trying to store them
    // behind a single concrete type.
    let subscriber = if config.json_format {
        let json_layer = fmt::layer()
            .json()
            .flatten_event(true)
            .with_current_span(true)
            .with_target(true)
            .with_span_list(true);
    
        // Note: additional structured fields such as service name and correlation
        // ID are attached at call sites (e.g. in `log_structured_error`) rather
        // than via a custom `with_context` hook on the subscriber layer, which
        // is not supported by `tracing-subscriber`'s `fmt::Layer`.
        subscriber.with(json_layer)
    } else {
        let text_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true);
    
        subscriber.with(text_layer)
    };
    
    // Add file output if configured
    let subscriber = if config.file_output {
        if let Some(log_dir) = config.log_dir {
            let file_appender = RollingFileAppender::new(
                tracing_appender::rolling::Rotation::DAILY,
                log_dir,
                format!("{}.log", config.service_name),
            );
            
            let (non_blocking, _guard) = NonBlocking::new(file_appender);
            
            // Keep the guard alive for the lifetime of the program
            // This is important to ensure logs are written properly
            Box::leak(Box::new(_guard));
            
            let file_layer = fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false);
                
            subscriber.with(file_layer)
        } else {
            subscriber
        }
    } else {
        subscriber
    };
    
    // Set the subscriber as the global default
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| Error::new(ErrorKind::Initialization, format!("Failed to set global subscriber: {}", e)))?;
        
    // Mark logging as initialized
    LOGGING_INITIALIZED.store(true, Ordering::SeqCst);
    
    // Log initialization success
    tracing::info!(
        service = %config.service_name,
        level = %config.level,
        json = %config.json_format,
        "Structured logging initialized"
    );
    
    Ok(())
}

/// Sets the correlation ID for the current thread
pub fn set_correlation_id<S: Into<String>>(correlation_id: S) {
    CORRELATION_ID.with(|id| {
        *id.write().unwrap() = Some(correlation_id.into());
    });
}

/// Generates and sets a new correlation ID
pub fn generate_correlation_id() -> String {
    let id = Uuid::new_v4().to_string();
    set_correlation_id(id.clone());
    id
}

/// Retrieves the current correlation ID
pub fn current_correlation_id() -> Option<String> {
    CORRELATION_ID.with(|id| id.read().unwrap().clone())
}

/// Clears the correlation ID for the current thread
pub fn clear_correlation_id() {
    CORRELATION_ID.with(|id| {
        *id.write().unwrap() = None;
    });
}

/// Executes a function with a specific correlation ID
pub fn with_correlation_id<F, R, S>(correlation_id: S, f: F) -> R
where
    F: FnOnce() -> R,
    S: Into<String>,
{
    // Save the current correlation ID
    let previous = current_correlation_id();
    
    // Set the new correlation ID
    set_correlation_id(correlation_id);
    
    // Execute the function
    let result = f();
    
    // Restore the previous correlation ID
    match previous {
        Some(id) => set_correlation_id(id),
        None => clear_correlation_id(),
    }
    
    result
}

/// Convenience macro for logging an error with context
#[macro_export]
macro_rules! log_error {
    ($err:expr) => {
        {
            use tracing::error;
            use $crate::logging::current_correlation_id;
            
            let err = $err;
            let correlation_id = current_correlation_id().unwrap_or_else(|| "unknown".to_string());
            
            error!(
                error_id = %err.id,
                error_kind = %err.kind,
                correlation_id = %correlation_id,
                service = %err.service.as_deref().unwrap_or("unknown"),
                message = %err.message,
                code = ?err.code,
                severity = %err.severity,
                "Error occurred"
            );
            
            err
        }
    };
}

/// Logs an error at the appropriate level based on its severity
pub fn log_structured_error(error: &Error) {
    use tracing::{error, warn, info, debug};
    use crate::types::Severity;
    
    let correlation_id = error.correlation_id.clone().unwrap_or_else(|| "unknown".to_string());
    let service = error.service.as_deref().unwrap_or("unknown");
    
    // Log with different levels based on severity
    match error.severity {
        Severity::Fatal | Severity::Critical => {
            error!(
                error_id = %error.id,
                error_kind = %error.kind,
                correlation_id = %correlation_id,
                service = %service,
                message = %error.message,
                code = ?error.code,
                severity = %error.severity,
                transient = %error.transient,
                timestamp = %error.timestamp,
                context = ?error.context,
                "Critical error occurred"
            );
        },
        Severity::Major => {
            error!(
                error_id = %error.id,
                error_kind = %error.kind,
                correlation_id = %correlation_id,
                service = %service,
                message = %error.message,
                code = ?error.code,
                severity = %error.severity,
                "Error occurred"
            );
        },
        Severity::Minor => {
            warn!(
                error_id = %error.id,
                error_kind = %error.kind,
                correlation_id = %correlation_id,
                service = %service,
                message = %error.message,
                code = ?error.code,
                severity = %error.severity,
                "Warning occurred"
            );
        },
        Severity::Info => {
            info!(
                error_id = %error.id,
                error_kind = %error.kind,
                correlation_id = %correlation_id,
                service = %service,
                message = %error.message,
                code = ?error.code,
                severity = %error.severity,
                "Info occurred"
            );
        },
    }
}
impl TryFrom&lt;config::Config&gt; for LoggingConfig {
    type Error = config::ConfigError;

    fn try_from(cfg: config::Config) -&gt; std::result::Result&lt;Self, Self::Error&gt; {
        // Start from defaults and selectively override from the provided config.
        let mut base = LoggingConfig::default();

        if let Ok(level) = cfg.get::&lt;String&gt;("logging.level") {
            base.level = level;
        }
        if let Ok(service_name) = cfg.get::&lt;String&gt;("logging.service_name") {
            base.service_name = service_name;
        }
        if let Ok(file_output) = cfg.get::&lt;bool&gt;("logging.file_output") {
            base.file_output = file_output;
        }
        if let Ok(log_dir) = cfg.get::&lt;String&gt;("logging.log_dir") {
            base.log_dir = Some(log_dir);
        }
        if let Ok(json_format) = cfg.get::&lt;bool&gt;("logging.json_format") {
            base.json_format = json_format;
        }
        if let Ok(include_source_code) = cfg.get::&lt;bool&gt;("logging.include_source_code") {
            base.include_source_code = include_source_code;
        }
        if let Ok(enable_tracing) = cfg.get::&lt;bool&gt;("logging.enable_tracing") {
            base.enable_tracing = enable_tracing;
        }
        if let Ok(custom) = cfg.get::&lt;serde_json::Value&gt;("logging.custom_fields") {
            if let serde_json::Value::Object(map) = custom {
                base.custom_fields = map;
            }
        }

        Ok(base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Error, ErrorKind, Severity};
    
    #[test]
    fn test_correlation_id() {
        assert!(current_correlation_id().is_none());
        
        let id = "test-correlation-id";
        set_correlation_id(id);
        
        assert_eq!(current_correlation_id(), Some(id.to_string()));
        
        clear_correlation_id();
        assert!(current_correlation_id().is_none());
    }
    
    #[test]
    fn test_with_correlation_id() {
        assert!(current_correlation_id().is_none());
        
        let result = with_correlation_id("nested-id", || {
            assert_eq!(current_correlation_id(), Some("nested-id".to_string()));
            "test-result"
        });
        
        assert_eq!(result, "test-result");
        assert!(current_correlation_id().is_none());
        
        // Test nesting
        set_correlation_id("outer-id");
        let result = with_correlation_id("inner-id", || {
            assert_eq!(current_correlation_id(), Some("inner-id".to_string()));
            "nested-test"
        });
        
        assert_eq!(result, "nested-test");
        assert_eq!(current_correlation_id(), Some("outer-id".to_string()));
    }
    
    #[test]
    fn test_generate_correlation_id() {
        assert!(current_correlation_id().is_none());
        
        let id = generate_correlation_id();
        assert!(!id.is_empty());
        
        assert_eq!(current_correlation_id(), Some(id));
    }
}