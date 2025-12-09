//! # Input Validation Library
//!
//! A comprehensive input validation and sanitization library for the Phoenix ORCH AGI system.
//! This library provides reusable validation and sanitization utilities that can be used across
//! different services to ensure consistent input handling and security.
//!
//! ## Features
//!
//! - Validators for common input types (strings, numbers, URLs, file paths)
//! - Sanitization for potentially dangerous inputs
//! - Schema-based validation
//! - Composable validation rules
//! - Protection against common security vulnerabilities
//! - Comprehensive error handling

mod errors;
mod schema;
mod builder;
pub mod validators;
pub mod sanitizers;

pub use builder::ValidationBuilder;
pub use errors::{ValidationError, ValidationResult};
pub use schema::Schema;

/// Re-export commonly used validators for convenience
pub mod prelude {
    pub use crate::builder::ValidationBuilder;
    pub use crate::errors::{ValidationError, ValidationResult};
    pub use crate::sanitizers;
    pub use crate::schema::Schema;
    pub use crate::validators;
}

/// Version of the validation library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Limit for max request payload size (10 MB)
pub const DEFAULT_MAX_PAYLOAD_SIZE: usize = 10 * 1024 * 1024;

/// Default maximum allowed length for strings
pub const DEFAULT_MAX_STRING_LENGTH: usize = 32_768;

/// Default minimum allowed length for strings
pub const DEFAULT_MIN_STRING_LENGTH: usize = 1;

/// Default maximum allowed array length
pub const DEFAULT_MAX_ARRAY_LENGTH: usize = 1_000;

/// Default maximum depth for nested objects
pub const DEFAULT_MAX_DEPTH: usize = 10;

/// Configuration for the validation library
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum payload size in bytes
    pub max_payload_size: usize,
    /// Maximum allowed string length
    pub max_string_length: usize,
    /// Minimum allowed string length
    pub min_string_length: usize,
    /// Maximum allowed array length
    pub max_array_length: usize,
    /// Maximum depth for nested objects
    pub max_depth: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
            max_string_length: DEFAULT_MAX_STRING_LENGTH,
            min_string_length: DEFAULT_MIN_STRING_LENGTH,
            max_array_length: DEFAULT_MAX_ARRAY_LENGTH,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

/// Get a new default configuration
pub fn default_config() -> ValidationConfig {
    ValidationConfig::default()
}

/// Validate input with default settings
pub fn validate<T>(input: T) -> ValidationBuilder<T> {
    ValidationBuilder::new(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = default_config();
        assert_eq!(config.max_payload_size, DEFAULT_MAX_PAYLOAD_SIZE);
        assert_eq!(config.max_string_length, DEFAULT_MAX_STRING_LENGTH);
        assert_eq!(config.min_string_length, DEFAULT_MIN_STRING_LENGTH);
        assert_eq!(config.max_array_length, DEFAULT_MAX_ARRAY_LENGTH);
        assert_eq!(config.max_depth, DEFAULT_MAX_DEPTH);
    }
}