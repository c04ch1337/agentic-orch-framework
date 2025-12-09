//! Error handling for the validation library
//!
//! This module provides a comprehensive error handling system for validation,
//! with structured errors and detailed error messages.

use std::fmt;
use thiserror::Error;

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Enum representing different validation error types
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Input is too long (e.g., string, array)
    #[error("Input exceeds maximum length: {0}")]
    TooLong(String),

    /// Input is too short (e.g., string, array)
    #[error("Input is shorter than minimum length: {0}")]
    TooShort(String),

    /// Input is outside numeric range
    #[error("Value is outside allowed range: {0}")]
    OutOfRange(String),

    /// Input format is invalid
    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    /// Input contains invalid characters
    #[error("Contains invalid characters: {0}")]
    InvalidCharacters(String),

    /// Input is missing required fields
    #[error("Missing required fields: {0}")]
    MissingFields(String),

    /// Input contains unexpected fields
    #[error("Contains unexpected fields: {0}")]
    ExtraFields(String),

    /// Input type is incorrect
    #[error("Invalid type: {0}")]
    InvalidType(String),

    /// Input contains potential security threat
    #[error("Security threat detected: {0}")]
    SecurityThreat(String),

    /// Payload size exceeds maximum
    #[error("Payload too large: {0}")]
    PayloadTooLarge(String),

    /// Input validation failed for regex pattern
    #[error("Pattern match failed: {0}")]
    PatternMismatch(String),

    /// Input contains invalid URL
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Input contains invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Invalid encoding (e.g., UTF-8)
    #[error("Invalid encoding: {0}")]
    InvalidEncoding(String),

    /// Nested objects exceed maximum depth
    #[error("Exceeded maximum nesting depth: {0}")]
    ExceededMaxDepth(String),

    /// Invalid content type
    #[error("Invalid content type: {0}")]
    InvalidContentType(String),

    /// Schema validation error
    #[error("Schema validation failed: {0}")]
    SchemaError(String),
    
    /// Composite validation error (multiple errors)
    #[error("{0} validation errors occurred")]
    Composite(CompositeError),
    
    /// Generic validation error
    #[error("{0}")]
    Generic(String),
}

/// Container for multiple validation errors
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeError {
    /// Collection of validation errors
    pub errors: Vec<ValidationError>,
    /// Field path information
    pub path: Option<String>,
}

impl fmt::Display for CompositeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} validation errors:", self.errors.len())?;
        
        for (idx, err) in self.errors.iter().enumerate() {
            if let Some(path) = &self.path {
                writeln!(f, "  {}. At {}: {}", idx + 1, path, err)?;
            } else {
                writeln!(f, "  {}. {}", idx + 1, err)?;
            }
        }
        
        Ok(())
    }
}

impl ValidationError {
    /// Create a new generic validation error with a message
    pub fn new<S: Into<String>>(message: S) -> Self {
        ValidationError::Generic(message.into())
    }

    /// Create a new composite validation error from a collection of errors
    pub fn composite<I>(errors: I) -> Self 
    where
        I: IntoIterator<Item = ValidationError>
    {
        let errors: Vec<ValidationError> = errors.into_iter().collect();
        if errors.len() == 1 {
            // If there's only one error, return that directly
            errors.into_iter().next().unwrap()
        } else {
            ValidationError::Composite(CompositeError {
                errors,
                path: None,
            })
        }
    }

    /// Create a composite error with a specific field path
    pub fn composite_at<I, S>(errors: I, path: S) -> Self
    where
        I: IntoIterator<Item = ValidationError>,
        S: Into<String>,
    {
        let errors: Vec<ValidationError> = errors.into_iter().collect();
        if errors.len() == 1 {
            // If there's only one error, return that directly
            errors.into_iter().next().unwrap()
        } else {
            ValidationError::Composite(CompositeError {
                errors,
                path: Some(path.into()),
            })
        }
    }
    
    /// Returns true if this is a security-related error
    pub fn is_security_threat(&self) -> bool {
        matches!(self, ValidationError::SecurityThreat(_))
    }
}

/// Contextual validation error that includes field information
#[derive(Debug, Clone)]
pub struct ContextualError {
    /// The field or context where the error occurred
    pub field: String,
    /// The validation error
    pub error: ValidationError,
}

impl ContextualError {
    /// Create a new contextual error
    pub fn new<S: Into<String>>(field: S, error: ValidationError) -> Self {
        Self {
            field: field.into(),
            error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_creation() {
        let err = ValidationError::new("Test error");
        assert!(matches!(err, ValidationError::Generic(_)));
        
        if let ValidationError::Generic(msg) = err {
            assert_eq!(msg, "Test error");
        }
    }
    
    #[test]
    fn test_composite_error() {
        let errs = vec![
            ValidationError::TooShort("String too short".to_string()),
            ValidationError::InvalidFormat("Invalid email".to_string()),
        ];
        
        let composite = ValidationError::composite(errs);
        
        if let ValidationError::Composite(comp) = composite {
            assert_eq!(comp.errors.len(), 2);
        } else {
            panic!("Expected composite error");
        }

        // Test with a single error
        let single_err = ValidationError::InvalidUrl("Bad URL".to_string());
        let composite_single = ValidationError::composite(vec![single_err.clone()]);
        
        // Should unwrap to the original error
        assert_eq!(composite_single, single_err);
    }
}