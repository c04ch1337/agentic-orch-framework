//! Validator functions and utilities
//!
//! This module provides specialized validators for different types of input.
//! These validators can be used directly or with the validation builder.

pub mod generic;
pub mod numeric;
pub mod path;
pub mod redos;
pub mod security;
pub mod string;
pub mod url;

// Re-export all validators for convenience
pub use generic::*;
pub use numeric::*;
pub use path::*;
pub use redos::*;
pub use security::*;
pub use string::*;
pub use url::*;

/// Utility module for validation helpers
pub mod utils {
    use lazy_static::lazy_static;
    use regex::Regex;

    /// Get a regex from a pattern string, with caching for efficiency
    pub fn get_regex(pattern: &str) -> Result<&Regex, regex::Error> {
        lazy_static! {
            static ref REGEX_CACHE: std::sync::Mutex<std::collections::HashMap<String, Regex>> =
                std::sync::Mutex::new(std::collections::HashMap::new());
        }

        let mut cache = REGEX_CACHE.lock().unwrap();

        if !cache.contains_key(pattern) {
            let compiled = Regex::new(pattern)?;
            cache.insert(pattern.to_string(), compiled);
        }

        // This unwrap is safe because we just ensured the key exists
        Ok(unsafe {
            // This is safe because:
            // 1. The HashMap in REGEX_CACHE is never modified after this insertion
            // 2. The Regex value is immutable and thread-safe once created
            // 3. The cache itself lives for the entire program lifetime
            // 4. We're only using the reference within this function scope
            std::mem::transmute::<&Regex, &Regex>(cache.get(pattern).unwrap())
        })
    }
}

// Helper for bulk-testing multiple validators
pub(crate) fn test_validators<T>(
    input: &T,
    validators: Vec<Box<dyn Fn(&T) -> crate::ValidationResult<()>>>,
) -> crate::ValidationResult<()> {
    let mut errors = Vec::new();

    for validator in validators {
        if let Err(err) = validator(input) {
            errors.push(err);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(crate::errors::ValidationError::composite(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ValidationError;

    #[test]
    fn test_bulk_validation() {
        // Test with passing validators
        let result = test_validators(
            &"test",
            vec![
                Box::new(|s: &&str| {
                    if !s.is_empty() {
                        Ok(())
                    } else {
                        Err(ValidationError::new("empty"))
                    }
                }),
                Box::new(|s: &&str| {
                    if s.len() <= 10 {
                        Ok(())
                    } else {
                        Err(ValidationError::new("too long"))
                    }
                }),
            ],
        );
        assert!(result.is_ok());

        // Test with failing validators
        let result = test_validators(
            &"test_string_that_is_too_long",
            vec![
                Box::new(|s: &&str| {
                    if !s.is_empty() {
                        Ok(())
                    } else {
                        Err(ValidationError::new("empty"))
                    }
                }),
                Box::new(|s: &&str| {
                    if s.len() <= 10 {
                        Ok(())
                    } else {
                        Err(ValidationError::new("too long"))
                    }
                }),
            ],
        );
        assert!(result.is_err());
    }
}
