//! Validation builder pattern
//!
//! This module provides a fluent API for building and composing validation rules.

use crate::errors::{ValidationError, ValidationResult};

/// Builder for chaining validation rules
#[derive(Debug, Clone)]
pub struct ValidationBuilder<T> {
    /// The value being validated
    value: T,
    /// Any previous error encountered during validation
    error: Option<ValidationError>,
}

impl<T> ValidationBuilder<T> {
    /// Create a new validation builder
    pub fn new(value: T) -> Self {
        Self { value, error: None }
    }

    /// Apply a validation function and return a new builder with the result
    pub fn validate<F>(mut self, validator: F) -> Self
    where
        F: FnOnce(&T) -> ValidationResult<()>,
    {
        // Only apply validation if there's no previous error
        if self.error.is_none() {
            if let Err(err) = validator(&self.value) {
                self.error = Some(err);
            }
        }
        self
    }

    /// Apply a validation function that transforms the value
    pub fn map<U, F>(self, mapper: F) -> ValidationBuilder<U>
    where
        F: FnOnce(T) -> ValidationResult<U>,
    {
        match self.error {
            Some(err) => ValidationBuilder {
                value: unsafe { std::mem::zeroed() }, // This is safe as we'll never access value when error is Some
                error: Some(err),
            },
            None => match mapper(self.value) {
                Ok(new_value) => ValidationBuilder::new(new_value),
                Err(err) => ValidationBuilder {
                    value: unsafe { std::mem::zeroed() }, // This is safe as we'll never access value when error is Some
                    error: Some(err),
                },
            },
        }
    }

    /// Apply a custom validation function with a custom error message
    pub fn custom<F>(self, f: F, message: &str) -> Self
    where
        F: FnOnce(&T) -> bool,
    {
        self.validate(|value| {
            if f(value) {
                Ok(())
            } else {
                Err(ValidationError::new(message))
            }
        })
    }

    /// Finish validation and return the result
    pub fn finish(self) -> ValidationResult<T> {
        match self.error {
            Some(err) => Err(err),
            None => Ok(self.value),
        }
    }

    /// Get a reference to the value (if validation succeeded so far)
    pub fn value(&self) -> Option<&T> {
        if self.error.is_none() {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Get the error (if any)
    pub fn error(&self) -> Option<&ValidationError> {
        self.error.as_ref()
    }
}

// String-specific validation extension
impl ValidationBuilder<String> {
    /// Validate that a string is not empty
    pub fn not_empty(self) -> Self {
        self.validate(|s| {
            if s.is_empty() {
                Err(ValidationError::TooShort(
                    "String must not be empty".to_string(),
                ))
            } else {
                Ok(())
            }
        })
    }

    /// Validate that a string has a minimum length
    pub fn min_length(self, min: usize) -> Self {
        self.validate(|s| {
            if s.len() < min {
                Err(ValidationError::TooShort(format!(
                    "String length ({}) is less than minimum length ({})",
                    s.len(),
                    min
                )))
            } else {
                Ok(())
            }
        })
    }

    /// Validate that a string does not exceed maximum length
    pub fn max_length(self, max: usize) -> Self {
        self.validate(|s| {
            if s.len() > max {
                Err(ValidationError::TooLong(format!(
                    "String length ({}) exceeds maximum length ({})",
                    s.len(),
                    max
                )))
            } else {
                Ok(())
            }
        })
    }

    /// Validate that a string matches a regex pattern
    pub fn matches(self, pattern: &str) -> Self {
        match regex::Regex::new(pattern) {
            Ok(re) => self.validate(|s| {
                if re.is_match(s) {
                    Ok(())
                } else {
                    Err(ValidationError::PatternMismatch(format!(
                        "String does not match pattern: {}",
                        pattern
                    )))
                }
            }),
            Err(e) => ValidationBuilder {
                value: self.value,
                error: Some(ValidationError::Generic(format!(
                    "Invalid regex pattern: {}",
                    e
                ))),
            },
        }
    }

    /// Validate string contains only allowed characters
    pub fn allowed_chars(self, allowed: &str) -> Self {
        let allowed: std::collections::HashSet<char> = allowed.chars().collect();
        self.validate(|s| {
            for c in s.chars() {
                if !allowed.contains(&c) {
                    return Err(ValidationError::InvalidCharacters(format!(
                        "String contains invalid character: '{}'",
                        c
                    )));
                }
            }
            Ok(())
        })
    }
}

// Numeric validation extension for any type that implements PartialOrd
impl<T: PartialOrd + std::fmt::Debug> ValidationBuilder<T> {
    /// Validate that a value is at least the minimum value
    pub fn min(self, min: T) -> Self {
        self.validate(|value| {
            if value < &min {
                Err(ValidationError::OutOfRange(format!(
                    "Value {:?} is less than minimum {:?}",
                    value, min
                )))
            } else {
                Ok(())
            }
        })
    }

    /// Validate that a value is at most the maximum value
    pub fn max(self, max: T) -> Self {
        self.validate(|value| {
            if value > &max {
                Err(ValidationError::OutOfRange(format!(
                    "Value {:?} exceeds maximum {:?}",
                    value, max
                )))
            } else {
                Ok(())
            }
        })
    }

    /// Validate that a value is within a range (inclusive)
    pub fn between(self, min: T, max: T) -> Self {
        self.validate(|value| {
            if value < &min || value > &max {
                Err(ValidationError::OutOfRange(format!(
                    "Value {:?} is outside range [{:?}, {:?}]",
                    value, min, max
                )))
            } else {
                Ok(())
            }
        })
    }
}

// Validation extensions for Options
impl<T> ValidationBuilder<Option<T>> {
    /// Validate that an Option is Some
    pub fn required(self) -> Self {
        self.validate(|opt| {
            if opt.is_none() {
                Err(ValidationError::MissingFields(
                    "Required value is missing".to_string(),
                ))
            } else {
                Ok(())
            }
        })
    }

    /// Transform Option<T> to ValidationBuilder<T> if Some, or return an error
    pub fn unwrap_or_err(self, message: &str) -> ValidationBuilder<T> {
        match self.error {
            Some(err) => ValidationBuilder {
                value: unsafe { std::mem::zeroed() }, // Safe because we never access value when error is Some
                error: Some(err),
            },
            None => match self.value {
                Some(value) => ValidationBuilder::new(value),
                None => ValidationBuilder {
                    value: unsafe { std::mem::zeroed() }, // Safe because we never access value when error is Some
                    error: Some(ValidationError::new(message)),
                },
            },
        }
    }

    /// Apply validator to the inner value if Some
    pub fn and_then<F>(self, validator: F) -> Self
    where
        F: FnOnce(T) -> ValidationResult<T>,
    {
        match self.error {
            Some(err) => ValidationBuilder {
                value: None,
                error: Some(err),
            },
            None => match self.value {
                Some(value) => match validator(value) {
                    Ok(new_value) => ValidationBuilder::new(Some(new_value)),
                    Err(err) => ValidationBuilder {
                        value: None,
                        error: Some(err),
                    },
                },
                None => self,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_validation() {
        // Test successful validation
        let result = ValidationBuilder::new("hello".to_string())
            .not_empty()
            .min_length(3)
            .max_length(10)
            .finish();

        assert!(result.is_ok());

        // Test failed validation
        let result = ValidationBuilder::new("".to_string()).not_empty().finish();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::TooShort(_)));

        // Test chained validation that fails at a later step
        let result = ValidationBuilder::new("hello".to_string())
            .not_empty()
            .max_length(3) // Should fail here
            .finish();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::TooLong(_)));
    }

    #[test]
    fn test_numeric_validation() {
        // Test successful validation
        let result = ValidationBuilder::new(5)
            .min(0)
            .max(10)
            .between(1, 8)
            .finish();

        assert!(result.is_ok());

        // Test failed validation - below minimum
        let result = ValidationBuilder::new(5).min(10).finish();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::OutOfRange(_)
        ));

        // Test failed validation - outside range
        let result = ValidationBuilder::new(5).between(10, 20).finish();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::OutOfRange(_)
        ));
    }

    #[test]
    fn test_option_validation() {
        // Test Some value
        let result = ValidationBuilder::new(Some(5)).required().finish();

        assert!(result.is_ok());

        // Test None value with required
        let result = ValidationBuilder::new(Option::<i32>::None)
            .required()
            .finish();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ValidationError::MissingFields(_)
        ));

        // Test unwrap_or_err with Some
        let result = ValidationBuilder::new(Some(5))
            .unwrap_or_err("Missing value")
            .finish();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);

        // Test unwrap_or_err with None
        let result = ValidationBuilder::new(Option::<i32>::None)
            .unwrap_or_err("Missing value")
            .finish();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::Generic(_)));
    }
}
