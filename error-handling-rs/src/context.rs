//! # Error Context Handling
//! 
//! This module provides functionality for adding and preserving context
//! when propagating errors through the system.

use std::fmt;
use std::error::Error as StdError;
use serde::{Serialize, Deserialize};
use crate::types::{Error, ErrorKind, Result};

/// Represents context information to be attached to an error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// The operation being performed when the error occurred
    pub operation: String,
    /// Additional context keys and values
    pub data: serde_json::Map<String, serde_json::Value>,
}

impl Context {
    /// Creates a new error context for the specified operation
    pub fn new<S: Into<String>>(operation: S) -> Self {
        Self {
            operation: operation.into(),
            data: serde_json::Map::new(),
        }
    }

    /// Adds a key-value pair to the context
    pub fn add<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Serialize,
    {
        if let Ok(value) = serde_json::to_value(value) {
            self.data.insert(key.into(), value);
        }
        self
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "during operation: {}", self.operation)?;
        
        if !self.data.is_empty() {
            write!(f, " [")?;
            let mut first = true;
            for (k, v) in &self.data {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "{}: {}", k, v)?;
                first = false;
            }
            write!(f, "]")?;
        }
        
        Ok(())
    }
}

/// Error with added context information 
#[derive(Debug)]
pub struct ErrorContext<E> {
    /// The original error
    pub error: E,
    /// The context information
    pub context: Context,
}

impl<E: StdError> fmt::Display for ErrorContext<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.error, self.context)
    }
}

impl<E: StdError + 'static> StdError for ErrorContext<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}

/// A trait for adding context to errors
pub trait WithContext<T, E> {
    /// Adds context to an error result
    fn with_context<C, F>(self, context_fn: F) -> Result<T>
    where
        C: Into<Context>,
        F: FnOnce() -> C;

    /// Adds operation context to an error result
    fn with_operation<S>(self, operation: S) -> Result<T>
    where
        S: Into<String>;
}

impl<T, E> WithContext<T, E> for std::result::Result<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn with_context<C, F>(self, context_fn: F) -> Result<T>
    where
        C: Into<Context>,
        F: FnOnce() -> C,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                let context = context_fn().into();
                let error_message = format!("{} ({})", error, context);
                
                // If the error is already our Error type, add the context to it
                if let Some(mut our_error) = error.downcast_ref::<Error>().cloned() {
                    // Add all context data to the error
                    let mut new_error = our_error.clone();
                    for (k, v) in context.data {
                        new_error = new_error.context(k, v);
                    }
                    Err(new_error)
                } else {
                    // Convert other error types to our Error type
                    Err(Error::new(ErrorKind::Internal, error_message).cause(error))
                }
            }
        }
    }

    fn with_operation<S>(self, operation: S) -> Result<T>
    where
        S: Into<String>,
    {
        self.with_context(|| Context::new(operation))
    }
}

/// Extension trait for Error to add context directly
pub trait ErrorExt {
    /// Adds a context key-value pair directly to the error
    fn add_context<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<String>,
        V: Serialize;

    /// Sets the operation context directly
    fn set_operation<S>(&mut self, operation: S) -> &mut Self
    where
        S: Into<String>;
}

impl ErrorExt for Error {
    fn add_context<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<String>,
        V: Serialize,
    {
        if let Ok(value) = serde_json::to_value(value) {
            self.context.insert(key.into(), value);
        }
        self
    }

    fn set_operation<S>(&mut self, operation: S) -> &mut Self
    where
        S: Into<String>,
    {
        self.context.insert("operation".into(), serde_json::Value::String(operation.into()));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Error, ErrorKind};
    use std::io;

    #[test]
    fn test_context_creation() {
        let ctx = Context::new("save_user")
            .add("user_id", 123)
            .add("email", "test@example.com");

        assert_eq!(ctx.operation, "save_user");
        assert_eq!(ctx.data.len(), 2);
    }

    #[test]
    fn test_with_context() {
        let res: Result<(), io::Error> = Err(io::Error::new(io::ErrorKind::NotFound, "File not found"));
        
        let err = res.with_context(|| {
            Context::new("read_config")
                .add("path", "/etc/config.json")
        }).unwrap_err();

        assert_eq!(err.kind, ErrorKind::Internal);
        assert!(err.message.contains("File not found"));
        assert!(err.message.contains("read_config"));
    }

    #[test]
    fn test_with_operation() {
        let res: Result<(), Error> = Err(Error::new(ErrorKind::Validation, "Invalid input"));
        
        let err = res.with_operation("validate_user").unwrap_err();
        
        assert_eq!(err.kind, ErrorKind::Validation);
        assert_eq!(err.message, "Invalid input");
        assert!(err.context.get("operation").is_some());
    }
}