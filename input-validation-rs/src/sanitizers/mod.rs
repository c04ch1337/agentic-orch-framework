//! Input sanitization utilities
//!
//! This module provides sanitization functions to clean potentially dangerous inputs.
//! These sanitizers can be used alongside validators to ensure input safety.

pub mod string;
pub mod html;
pub mod command;
pub mod path;

// Re-export all sanitizers for convenience
pub use string::*;
pub use html::*;
pub use command::*;
pub use path::*;

/// Sanitization result containing the sanitized content and information
/// about whether changes were made during sanitization
#[derive(Debug, Clone, PartialEq)]
pub struct SanitizeResult<T> {
    /// Sanitized content
    pub sanitized: T,
    /// Whether any changes were made during sanitization
    pub was_modified: bool,
    /// Optional details about what was modified
    pub details: Option<String>,
}

impl<T> SanitizeResult<T> {
    /// Create a new sanitization result
    pub fn new(sanitized: T, was_modified: bool, details: Option<String>) -> Self {
        Self {
            sanitized,
            was_modified,
            details,
        }
    }

    /// Create a result with unmodified content
    pub fn unmodified(content: T) -> Self {
        Self {
            sanitized: content,
            was_modified: false,
            details: None,
        }
    }

    /// Create a result with modified content
    pub fn modified(content: T, details: Option<String>) -> Self {
        Self {
            sanitized: content,
            was_modified: true,
            details,
        }
    }

    /// Map the sanitized content
    pub fn map<U, F>(self, f: F) -> SanitizeResult<U>
    where
        F: FnOnce(T) -> U,
    {
        SanitizeResult {
            sanitized: f(self.sanitized),
            was_modified: self.was_modified,
            details: self.details,
        }
    }
}

/// Run multiple sanitizers in sequence
pub fn chain_sanitizers<T, F>(input: T, sanitizers: Vec<F>) -> SanitizeResult<T>
where
    F: FnOnce(T) -> SanitizeResult<T>,
{
    let mut result = SanitizeResult::unmodified(input);
    let mut all_details = Vec::new();

    for sanitizer in sanitizers {
        let current_result = sanitizer(result.sanitized);
        
        result.sanitized = current_result.sanitized;
        
        if current_result.was_modified {
            result.was_modified = true;
            if let Some(details) = current_result.details {
                all_details.push(details);
            }
        }
    }

    if !all_details.is_empty() {
        result.details = Some(all_details.join("; "));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_result() {
        let unmodified = SanitizeResult::unmodified("test");
        assert_eq!(unmodified.was_modified, false);
        assert_eq!(unmodified.sanitized, "test");
        assert_eq!(unmodified.details, None);

        let modified = SanitizeResult::modified("test_sanitized", Some("removed unsafe chars".to_string()));
        assert_eq!(modified.was_modified, true);
        assert_eq!(modified.sanitized, "test_sanitized");
        assert_eq!(modified.details, Some("removed unsafe chars".to_string()));
    }

    #[test]
    fn test_chain_sanitizers() {
        // Define some test sanitizers
        let sanitizer1 = |s: String| -> SanitizeResult<String> {
            if s.contains("<") {
                SanitizeResult::modified(
                    s.replace("<", "&lt;"),
                    Some("Replaced < with &lt;".to_string()),
                )
            } else {
                SanitizeResult::unmodified(s)
            }
        };

        let sanitizer2 = |s: String| -> SanitizeResult<String> {
            if s.contains(">") {
                SanitizeResult::modified(
                    s.replace(">", "&gt;"),
                    Some("Replaced > with &gt;".to_string()),
                )
            } else {
                SanitizeResult::unmodified(s)
            }
        };

        // Test with input that needs both sanitizers
        let input = "Test <script>alert('XSS')</script>".to_string();
        let result = chain_sanitizers(input, vec![sanitizer1, sanitizer2]);

        assert_eq!(result.was_modified, true);
        assert_eq!(result.sanitized, "Test &lt;script&gt;alert('XSS')&lt;/script&gt;");
        assert!(result.details.unwrap().contains("Replaced <"));
        assert!(result.details.unwrap().contains("Replaced >"));

        // Test with input that doesn't need sanitization
        let input = "Clean text".to_string();
        let result = chain_sanitizers(input.clone(), vec![sanitizer1, sanitizer2]);

        assert_eq!(result.was_modified, false);
        assert_eq!(result.sanitized, input);
        assert_eq!(result.details, None);
    }

    #[test]
    fn test_map() {
        let result = SanitizeResult::modified("42", Some("Changed".to_string()));
        let mapped = result.map(|s| s.parse::<i32>().unwrap());
        
        assert_eq!(mapped.sanitized, 42);
        assert_eq!(mapped.was_modified, true);
        assert_eq!(mapped.details, Some("Changed".to_string()));
    }
}