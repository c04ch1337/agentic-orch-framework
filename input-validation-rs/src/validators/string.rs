//! String validators
//!
//! This module provides validators for string inputs.

use crate::errors::{ValidationError, ValidationResult};
use unicode_normalization::UnicodeNormalization;
use regex::RegexBuilder;
use std::collections::HashSet;
use super::utils::get_regex;

/// Validate that a string is not empty
pub fn not_empty(s: &str) -> ValidationResult<()> {
    if s.is_empty() {
        Err(ValidationError::TooShort("String must not be empty".to_string()))
    } else {
        Ok(())
    }
}

/// Validate that a string meets a minimum length requirement
pub fn min_length(s: &str, min: usize) -> ValidationResult<()> {
    if s.len() < min {
        Err(ValidationError::TooShort(format!(
            "String length ({}) is less than minimum length ({})",
            s.len(),
            min
        )))
    } else {
        Ok(())
    }
}

/// Validate that a string does not exceed a maximum length
pub fn max_length(s: &str, max: usize) -> ValidationResult<()> {
    if s.len() > max {
        Err(ValidationError::TooLong(format!(
            "String length ({}) exceeds maximum length ({})",
            s.len(),
            max
        )))
    } else {
        Ok(())
    }
}

/// Validate that a string matches a pattern
pub fn matches_pattern(s: &str, pattern: &str) -> ValidationResult<()> {
    match get_regex(pattern) {
        Ok(re) => {
            if re.is_match(s) {
                Ok(())
            } else {
                Err(ValidationError::PatternMismatch(format!(
                    "String does not match pattern: {}", 
                    pattern
                )))
            }
        },
        Err(e) => {
            Err(ValidationError::Generic(format!("Invalid regex pattern: {}", e)))
        }
    }
}

/// Validate that a string matches a pattern with specific regex options
pub fn matches_pattern_with_options(
    s: &str, 
    pattern: &str,
    case_insensitive: bool,
    multi_line: bool,
    dot_matches_new_line: bool
) -> ValidationResult<()> {
    match RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .multi_line(multi_line)
        .dot_matches_new_line(dot_matches_new_line)
        .build()
    {
        Ok(re) => {
            if re.is_match(s) {
                Ok(())
            } else {
                Err(ValidationError::PatternMismatch(format!(
                    "String does not match pattern: {}", 
                    pattern
                )))
            }
        },
        Err(e) => {
            Err(ValidationError::Generic(format!("Invalid regex pattern: {}", e)))
        }
    }
}

/// Validate that a string contains only allowed characters
pub fn allowed_chars(s: &str, allowed: &str) -> ValidationResult<()> {
    let allowed_chars: HashSet<char> = allowed.chars().collect();
    
    for c in s.chars() {
        if !allowed_chars.contains(&c) {
            return Err(ValidationError::InvalidCharacters(format!(
                "String contains invalid character: '{}'",
                c
            )));
        }
    }
    
    Ok(())
}

/// Validate that a string does not contain any denied characters
pub fn denied_chars(s: &str, denied: &str) -> ValidationResult<()> {
    let denied_chars: HashSet<char> = denied.chars().collect();

    for c in s.chars() {
        if denied_chars.contains(&c) {
            return Err(ValidationError::InvalidCharacters(format!(
                "String contains forbidden character: '{}'",
                c
            )));
        }
    }

    Ok(())
}

/// Validate email address format
pub fn is_email(s: &str) -> ValidationResult<()> {
    // Based on the HTML5 spec's "valid email address" definition but stricter
    const EMAIL_PATTERN: &str = r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$";
    
    matches_pattern(s, EMAIL_PATTERN)
}

/// Validate username format (alphanumeric, underscore, hyphen)
pub fn is_username(s: &str) -> ValidationResult<()> {
    // Allow alphanumeric, underscore and hyphen, 3-30 characters
    const USERNAME_PATTERN: &str = r"^[a-zA-Z0-9_-]{3,30}$";
    
    matches_pattern(s, USERNAME_PATTERN)
}

/// Validate that a string is a valid UUID
pub fn is_uuid(s: &str) -> ValidationResult<()> {
    const UUID_PATTERN: &str = r"^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$";
    
    matches_pattern_with_options(s, UUID_PATTERN, true, false, false)
}

/// Validate that a string contains only alphanumeric characters
pub fn is_alphanumeric(s: &str) -> ValidationResult<()> {
    for c in s.chars() {
        if !c.is_alphanumeric() {
            return Err(ValidationError::InvalidCharacters(format!(
                "String contains non-alphanumeric character: '{}'",
                c
            )));
        }
    }
    
    Ok(())
}

/// Validate that a string is properly normalized Unicode text
pub fn normalized_unicode(s: &str) -> ValidationResult<()> {
    let normalized = s.nfc().collect::<String>();
    if normalized != s {
        Err(ValidationError::InvalidEncoding(
            "String is not in normalized Unicode form (NFC)".to_string()
        ))
    } else {
        Ok(())
    }
}

/// Validate a string is valid JSON
pub fn is_json(s: &str) -> ValidationResult<()> {
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(_) => Ok(()),
        Err(e) => Err(ValidationError::InvalidFormat(format!(
            "Invalid JSON: {}", e
        ))),
    }
}

/// Validate a string is a valid hostname
pub fn is_hostname(s: &str) -> ValidationResult<()> {
    // Based on RFC 1123 hostname rules
    const HOSTNAME_PATTERN: &str = r"^(([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9\-]*[a-zA-Z0-9])\.)*([A-Za-z0-9]|[A-Za-z0-9][A-Za-z0-9\-]*[A-Za-z0-9])$";
    
    // Check length
    if s.len() > 253 {
        return Err(ValidationError::TooLong(
            "Hostname exceeds maximum length (253 characters)".to_string()
        ));
    }
    
    // Check pattern
    matches_pattern(s, HOSTNAME_PATTERN)
}

/// Validate a string is a valid IPv4 address
pub fn is_ipv4(s: &str) -> ValidationResult<()> {
    const IPV4_PATTERN: &str = r"^((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$";
    
    matches_pattern(s, IPV4_PATTERN)
}

/// Validate a string is a valid IPv6 address
pub fn is_ipv6(s: &str) -> ValidationResult<()> {
    // This is a simplified pattern for IPv6 - a full compliant one would be much more complex
    const IPV6_PATTERN: &str = r"^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))$";
    
    matches_pattern(s, IPV6_PATTERN)
}

/// Validate a string is a valid IP address (IPv4 or IPv6)
pub fn is_ip(s: &str) -> ValidationResult<()> {
    if is_ipv4(s).is_ok() || is_ipv6(s).is_ok() {
        Ok(())
    } else {
        Err(ValidationError::InvalidFormat(
            "Not a valid IP address (neither IPv4 nor IPv6)".to_string()
        ))
    }
}

/// Validate a string contains no control characters
pub fn no_control_chars(s: &str) -> ValidationResult<()> {
    for c in s.chars() {
        if c.is_control() {
            return Err(ValidationError::InvalidCharacters(format!(
                "String contains control character (code point U+{:04X})",
                c as u32
            )));
        }
    }
    
    Ok(())
}

/// Validate a string contains only valid UTF-8
pub fn is_valid_utf8(s: &str) -> ValidationResult<()> {
    // In Rust, all strings are already valid UTF-8, so this check is redundant
    // But we include this for API completeness and to handle potential future changes
    match std::str::from_utf8(s.as_bytes()) {
        Ok(_) => Ok(()),
        Err(_) => Err(ValidationError::InvalidEncoding(
            "Invalid UTF-8 sequence".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_empty() {
        assert!(not_empty("hello").is_ok());
        assert!(not_empty("").is_err());
    }

    #[test]
    fn test_min_length() {
        assert!(min_length("hello", 3).is_ok());
        assert!(min_length("hi", 3).is_err());
    }

    #[test]
    fn test_max_length() {
        assert!(max_length("hello", 10).is_ok());
        assert!(max_length("hello world", 10).is_err());
    }

    #[test]
    fn test_matches_pattern() {
        assert!(matches_pattern("abc123", r"^[a-z0-9]+$").is_ok());
        assert!(matches_pattern("ABC", r"^[a-z0-9]+$").is_err());
    }

    #[test]
    fn test_allowed_chars() {
        assert!(allowed_chars("abc123", "abcdefghijklmnopqrstuvwxyz0123456789").is_ok());
        assert!(allowed_chars("abc@123", "abcdefghijklmnopqrstuvwxyz0123456789").is_err());
    }

    #[test]
    fn test_denied_chars() {
        assert!(denied_chars("abc123", "!@#$%^&*()").is_ok());
        assert!(denied_chars("abc@123", "!@#$%^&*()").is_err());
    }

    #[test]
    fn test_is_email() {
        assert!(is_email("user@example.com").is_ok());
        assert!(is_email("invalid-email").is_err());
        assert!(is_email("@example.com").is_err());
        assert!(is_email("user@").is_err());
    }

    #[test]
    fn test_is_username() {
        assert!(is_username("valid_user123").is_ok());
        assert!(is_username("us").is_err()); // Too short
        assert!(is_username("invalid@username").is_err()); // Invalid character
    }

    #[test]
    fn test_is_uuid() {
        assert!(is_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(is_uuid("not-a-uuid").is_err());
    }

    #[test]
    fn test_is_alphanumeric() {
        assert!(is_alphanumeric("abc123").is_ok());
        assert!(is_alphanumeric("abc_123").is_err());
    }

    #[test]
    fn test_is_json() {
        assert!(is_json(r#"{"name": "value"}"#).is_ok());
        assert!(is_json(r#"{invalid json}"#).is_err());
    }

    #[test]
    fn test_is_hostname() {
        assert!(is_hostname("example.com").is_ok());
        assert!(is_hostname("sub.example.com").is_ok());
        assert!(is_hostname("example..com").is_err());
        assert!(is_hostname("-example.com").is_err());
    }

    #[test]
    fn test_is_ip() {
        assert!(is_ip("192.168.1.1").is_ok());
        assert!(is_ip("2001:0db8:85a3:0000:0000:8a2e:0370:7334").is_ok());
        assert!(is_ip("not-an-ip").is_err());
    }

    #[test]
    fn test_no_control_chars() {
        assert!(no_control_chars("Normal text").is_ok());
        assert!(no_control_chars("Text with \x00 null byte").is_err());
        assert!(no_control_chars("Text with \x07 bell").is_err());
    }
}