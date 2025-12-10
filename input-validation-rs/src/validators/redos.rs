//! ReDoS (Regular Expression Denial of Service) protection
//!
//! This module provides utilities for preventing ReDoS attacks by using
//! safer regex implementations and timeout mechanisms.

use crate::errors::{ValidationError, ValidationResult};
use regex_automata::meta::Regex as DfaRegex;
use std::time::{Duration, Instant};

/// Maximum time allowed for regex matching (milliseconds)
const DEFAULT_REGEX_TIMEOUT_MS: u64 = 100;

/// Safe pattern match using regex-automata's meta Regex implementation
/// which is less vulnerable to ReDoS attacks than the standard regex crate
pub fn safe_pattern_match(input: &str, pattern: &str) -> ValidationResult<bool> {
    match DfaRegex::new(pattern) {
        Ok(re) => {
            let start = Instant::now();
            let matches = re.is_match(input);
            
            // Check if the match took too long (potential ReDoS attempt)
            if start.elapsed() > Duration::from_millis(DEFAULT_REGEX_TIMEOUT_MS) {
                Err(ValidationError::SecurityThreat(format!(
                    "Regex matching timed out (potential ReDoS attack) for pattern: {}", pattern
                )))
            } else {
                Ok(matches)
            }
        },
        Err(e) => Err(ValidationError::Generic(format!(
            "Invalid regex pattern: {}", e
        ))),
    }
}

/// Safe pattern match with custom timeout
pub fn safe_pattern_match_with_timeout(
    input: &str, 
    pattern: &str, 
    timeout_ms: u64
) -> ValidationResult<bool> {
    match DfaRegex::new(pattern) {
        Ok(re) => {
            let start = Instant::now();
            let matches = re.is_match(input);
            
            if start.elapsed() > Duration::from_millis(timeout_ms) {
                Err(ValidationError::SecurityThreat(format!(
                    "Regex matching timed out (potential ReDoS attack) for pattern: {}", pattern
                )))
            } else {
                Ok(matches)
            }
        },
        Err(e) => Err(ValidationError::Generic(format!(
            "Invalid regex pattern: {}", e
        ))),
    }
}

/// Validate a regex pattern for safety (checks for patterns that could lead to ReDoS)
pub fn is_safe_regex(pattern: &str) -> ValidationResult<()> {
    // Check for common patterns that could lead to ReDoS
    let unsafe_constructs = [
        // Nested repetition
        (r"\(.*\*.*\)+", "nested repetition with * inside capturing group with +"),
        (r"\(.*\+.*\)+", "nested repetition with + inside capturing group with +"),
        (r"\(.*\*.*\)*", "nested repetition with * inside capturing group with *"),
        (r"\(.*\+.*\)*", "nested repetition with + inside capturing group with *"),
        
        // Catastrophic backtracking patterns
        (r"\(.*\)\*", "capturing group with .* followed by *"),
        (r"\(.*\)\+", "capturing group with .* followed by +"),
        
        // Overlapping repetitions
        (r".*.*", "consecutive .* patterns"),
        (r".+.+", "consecutive .+ patterns"),
        
        // Complex lookaheads/lookbehinds
        (r"\(\?=.*\).*", "complex lookahead with .*"),
        (r"\(\?!.*\).*", "complex negative lookahead with .*"),
        
        // Unbounded repetition of optional groups
        (r"\(\w*\)+", "unbounded repetition of optional group"),
    ];
    
    for (bad_pattern, description) in unsafe_constructs.iter() {
        match safe_pattern_match(pattern, bad_pattern) {
            Ok(true) => {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potentially unsafe regex pattern detected: {}", description
                )));
            }
            Ok(false) => {
                // Pattern is safe (for this check)
            }
            Err(e) => {
                // Error checking the pattern
                return Err(e);
            }
        }
    }
    
    // Additional check: try to compile the pattern with regex-automata
    // to ensure it's a valid regex
    match DfaRegex::new(pattern) {
        Ok(_) => Ok(()),
        Err(e) => Err(ValidationError::Generic(format!(
            "Invalid regex pattern: {}", e
        ))),
    }
}

/// Token-based pattern matching as an alternative to regex for simple cases
/// This is much less vulnerable to ReDoS attacks
pub fn token_based_contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

/// Token-based pattern matching that ignores case
pub fn token_based_contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// Safe starts_with check
pub fn token_based_starts_with(haystack: &str, prefix: &str) -> bool {
    haystack.starts_with(prefix)
}

/// Safe ends_with check
pub fn token_based_ends_with(haystack: &str, suffix: &str) -> bool {
    haystack.ends_with(suffix)
}

/// Limit input length before regex processing to prevent ReDoS
pub fn validate_with_length_limit(
    input: &str, 
    pattern: &str, 
    max_length: usize
) -> ValidationResult<bool> {
    if input.len() > max_length {
        return Err(ValidationError::TooLong(format!(
            "Input length ({}) exceeds maximum allowed for regex processing ({})",
            input.len(), max_length
        )));
    }
    
    safe_pattern_match(input, pattern)
}

/// Validate an input with a simplified token-based approach instead of regex
/// This is much safer than regex for many common use cases
pub fn validate_tokens(
    input: &str,
    tokens: &[&str],
    require_all: bool
) -> ValidationResult<()> {
    let matches: Vec<&str> = tokens
        .iter()
        .filter(|&&token| input.contains(token))
        .copied()
        .collect();
    
    if (require_all && matches.len() == tokens.len()) ||
       (!require_all && !matches.is_empty()) {
        Ok(())
    } else if require_all {
        Err(ValidationError::PatternMismatch(format!(
            "Input doesn't contain all required tokens: {:?}", 
            tokens.iter().filter(|&&t| !matches.contains(&t)).collect::<Vec<_>>()
        )))
    } else {
        Err(ValidationError::PatternMismatch(format!(
            "Input doesn't contain any of the tokens: {:?}", tokens
        )))
    }
}

/// Process pattern matching with a timeout
/// Returns an error if matching takes too long (potential ReDoS)
pub fn match_with_timeout<F>(
    matcher: F, 
    timeout_ms: u64
) -> ValidationResult<bool>
where
    F: FnOnce() -> bool,
{
    let start = Instant::now();
    let result = matcher();
    
    if start.elapsed() > Duration::from_millis(timeout_ms) {
        Err(ValidationError::SecurityThreat(
            "Pattern matching timed out (potential ReDoS attack)".to_string()
        ))
    } else {
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_pattern_match() {
        // Test with safe pattern
        assert!(safe_pattern_match("abc123", r"^[a-z0-9]+$").unwrap());
        assert!(!safe_pattern_match("ABC", r"^[a-z0-9]+$").unwrap());
        
        // Test with invalid pattern
        assert!(safe_pattern_match("test", r"[unclosed").is_err());
    }
    
    #[test]
    fn test_is_safe_regex() {
        // Safe patterns
        assert!(is_safe_regex(r"^[a-z0-9]+$").is_ok());
        assert!(is_safe_regex(r"hello|world").is_ok());
        
        // Unsafe patterns that could lead to ReDoS
        assert!(is_safe_regex(r"(a*)*").is_err());
        assert!(is_safe_regex(r"(a+)+").is_err());
    }
    
    #[test]
    fn test_token_based_matching() {
        assert!(token_based_contains("hello world", "world"));
        assert!(!token_based_contains("hello world", "earth"));
        
        assert!(token_based_contains_ignore_case("Hello World", "world"));
        assert!(!token_based_contains_ignore_case("Hello World", "earth"));
    }
    
    #[test]
    fn test_validate_tokens() {
        // Test require_all=true
        assert!(validate_tokens("hello world", &["hello", "world"], true).is_ok());
        assert!(validate_tokens("hello world", &["hello", "earth"], true).is_err());
        
        // Test require_all=false
        assert!(validate_tokens("hello world", &["hello", "earth"], false).is_ok());
        assert!(validate_tokens("hello world", &["earth", "moon"], false).is_err());
    }
}