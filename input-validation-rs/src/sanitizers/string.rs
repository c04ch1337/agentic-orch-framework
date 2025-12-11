//! String sanitization utilities
//!
//! This module provides sanitizers for string inputs to remove or escape
//! potentially dangerous characters and sequences.

use super::SanitizeResult;
use lazy_static::lazy_static;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;

lazy_static! {
    static ref CONTROL_CHARS_REGEX: Regex =
        Regex::new(r"[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]").unwrap();
    static ref INVALID_UTF8_REGEX: Regex = Regex::new(r"[\xF0-\xFF][\x80-\xBF]{0,3}").unwrap();
}

/// Remove control characters from a string
pub fn remove_control_chars(input: &str) -> SanitizeResult<String> {
    let sanitized = CONTROL_CHARS_REGEX.replace_all(input, "").to_string();

    if sanitized == input {
        SanitizeResult::unmodified(input.to_string())
    } else {
        SanitizeResult::modified(sanitized, Some("Removed control characters".to_string()))
    }
}

/// Limit string length to a maximum value
pub fn limit_length(input: &str, max_length: usize) -> SanitizeResult<String> {
    if input.len() <= max_length {
        SanitizeResult::unmodified(input.to_string())
    } else {
        let truncated = input.chars().take(max_length).collect::<String>();
        SanitizeResult::modified(
            truncated,
            Some(format!(
                "Truncated string from {} to {} characters",
                input.len(),
                max_length
            )),
        )
    }
}

/// Normalize Unicode text (NFC form)
pub fn normalize_unicode(input: &str) -> SanitizeResult<String> {
    let normalized = input.nfc().collect::<String>();

    if normalized == input {
        SanitizeResult::unmodified(input.to_string())
    } else {
        SanitizeResult::modified(
            normalized,
            Some("Normalized Unicode characters".to_string()),
        )
    }
}

/// Trim whitespace from beginning and end
pub fn trim_whitespace(input: &str) -> SanitizeResult<String> {
    let trimmed = input.trim().to_string();

    if trimmed == input {
        SanitizeResult::unmodified(input.to_string())
    } else {
        SanitizeResult::modified(trimmed, Some("Trimmed whitespace".to_string()))
    }
}

/// Escape special regex metacharacters for safe regex pattern use
pub fn escape_regex_special_chars(input: &str) -> SanitizeResult<String> {
    let escaped = regex::escape(input);

    if escaped == input {
        SanitizeResult::unmodified(input.to_string())
    } else {
        SanitizeResult::modified(
            escaped,
            Some("Escaped regex special characters".to_string()),
        )
    }
}

/// Remove any characters not in the allowed set
pub fn keep_allowed_chars(input: &str, allowed: &str) -> SanitizeResult<String> {
    let allowed_chars: std::collections::HashSet<char> = allowed.chars().collect();
    let result: String = input
        .chars()
        .filter(|c| allowed_chars.contains(c))
        .collect();

    if result == input {
        SanitizeResult::unmodified(input.to_string())
    } else {
        SanitizeResult::modified(result, Some("Removed disallowed characters".to_string()))
    }
}

/// Remove any characters in the disallowed set
pub fn remove_disallowed_chars(input: &str, disallowed: &str) -> SanitizeResult<String> {
    let disallowed_chars: std::collections::HashSet<char> = disallowed.chars().collect();
    let result: String = input
        .chars()
        .filter(|c| !disallowed_chars.contains(c))
        .collect();

    if result == input {
        SanitizeResult::unmodified(input.to_string())
    } else {
        SanitizeResult::modified(result, Some("Removed disallowed characters".to_string()))
    }
}

/// Replace characters using a mapping
pub fn replace_chars(input: &str, replacements: &[(char, &str)]) -> SanitizeResult<String> {
    let mut result = input.to_string();
    let mut was_modified = false;

    for (from, to) in replacements {
        let original_len = result.len();
        result = result.replace(*from, to);
        if result.len() != original_len {
            was_modified = true;
        }
    }

    if was_modified {
        SanitizeResult::modified(result, Some("Replaced characters".to_string()))
    } else {
        SanitizeResult::unmodified(input.to_string())
    }
}

/// Normalize line endings to LF only
pub fn normalize_line_endings(input: &str) -> SanitizeResult<String> {
    // First convert all CR+LF to LF, then convert any remaining CR to LF
    let result = input.replace("\r\n", "\n").replace('\r', "\n");

    if result == input {
        SanitizeResult::unmodified(input.to_string())
    } else {
        SanitizeResult::modified(result, Some("Normalized line endings".to_string()))
    }
}

/// Fix potentially malformed UTF-8 sequences
pub fn sanitize_utf8(input: &str) -> SanitizeResult<String> {
    let bytes = input.as_bytes();
    let mut sanitized = String::with_capacity(input.len());
    let mut i = 0;

    let mut was_modified = false;

    while i < bytes.len() {
        match std::str::from_utf8(&bytes[i..=i]) {
            Ok(c) => {
                sanitized.push_str(c);
                i += 1;
            }
            Err(_) => {
                // Try to find the next valid UTF-8 sequence
                was_modified = true;

                // Skip invalid byte
                i += 1;

                // Find next valid UTF-8 sequence start
                while i < bytes.len() && (bytes[i] & 0xC0) == 0x80 {
                    i += 1;
                }
            }
        }
    }

    if was_modified {
        SanitizeResult::modified(
            sanitized,
            Some("Fixed malformed UTF-8 sequences".to_string()),
        )
    } else {
        SanitizeResult::unmodified(input.to_string())
    }
}

/// Collapse multiple whitespace characters into a single space
pub fn collapse_whitespace(input: &str) -> SanitizeResult<String> {
    lazy_static! {
        static ref WHITESPACE_REGEX: Regex = Regex::new(r"\s+").unwrap();
    }

    let result = WHITESPACE_REGEX.replace_all(input, " ").to_string();

    if result == input {
        SanitizeResult::unmodified(input.to_string())
    } else {
        SanitizeResult::modified(result, Some("Collapsed whitespace".to_string()))
    }
}

/// Convert string to lowercase
pub fn to_lowercase(input: &str) -> SanitizeResult<String> {
    let result = input.to_lowercase();

    if result == input {
        SanitizeResult::unmodified(input.to_string())
    } else {
        SanitizeResult::modified(result, Some("Converted to lowercase".to_string()))
    }
}

/// Apply standard string sanitization (combination of common sanitizers)
pub fn standard_string_sanitize(input: &str) -> SanitizeResult<String> {
    use super::chain_sanitizers;

    let sanitizers: Vec<Box<dyn Fn(String) -> SanitizeResult<String>>> = vec![
        Box::new(|s| remove_control_chars(&s)),
        Box::new(|s| normalize_unicode(&s)),
        Box::new(|s| trim_whitespace(&s)),
        Box::new(|s| normalize_line_endings(&s)),
    ];

    chain_sanitizers(input.to_string(), sanitizers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_control_chars() {
        let input = "Hello\u{0000}World";
        let result = remove_control_chars(input);

        assert!(result.was_modified);
        assert_eq!(result.sanitized, "HelloWorld");

        // Test with input that doesn't need sanitization
        let clean = "Hello World";
        let result = remove_control_chars(clean);

        assert!(!result.was_modified);
        assert_eq!(result.sanitized, clean);
    }

    #[test]
    fn test_limit_length() {
        let input = "This is a very long string that should be truncated";
        let max_length = 20;
        let result = limit_length(input, max_length);

        assert!(result.was_modified);
        assert_eq!(result.sanitized.len(), max_length);
        assert_eq!(result.sanitized, "This is a very long ");

        // Test with input that doesn't need truncation
        let short = "Short string";
        let result = limit_length(short, max_length);

        assert!(!result.was_modified);
        assert_eq!(result.sanitized, short);
    }

    #[test]
    fn test_normalize_unicode() {
        // Test combining character normalization
        // "é" can be represented as a single code point (U+00E9) or
        // as an "e" followed by combining acute accent (U+0065 U+0301)
        let input = "cafe\u{0301}"; // "café" with combining accent
        let result = normalize_unicode(input);

        assert!(result.was_modified);
        assert_eq!(result.sanitized, "café"); // Single code point

        // Test with already normalized input
        let normalized = "café"; // Already using single code point
        let result = normalize_unicode(normalized);

        assert!(!result.was_modified);
        assert_eq!(result.sanitized, normalized);
    }

    #[test]
    fn test_trim_whitespace() {
        let input = "  Hello World  ";
        let result = trim_whitespace(input);

        assert!(result.was_modified);
        assert_eq!(result.sanitized, "Hello World");

        // Test with input that doesn't need trimming
        let clean = "Hello World";
        let result = trim_whitespace(clean);

        assert!(!result.was_modified);
        assert_eq!(result.sanitized, clean);
    }

    #[test]
    fn test_keep_allowed_chars() {
        let input = "Hello123!@#";
        let allowed = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let result = keep_allowed_chars(input, allowed);

        assert!(result.was_modified);
        assert_eq!(result.sanitized, "Hello");

        // Test with input that only has allowed chars
        let clean = "HelloWorld";
        let result = keep_allowed_chars(clean, allowed);

        assert!(!result.was_modified);
        assert_eq!(result.sanitized, clean);
    }

    #[test]
    fn test_normalize_line_endings() {
        let input = "Line1\r\nLine2\rLine3";
        let result = normalize_line_endings(input);

        assert!(result.was_modified);
        assert_eq!(result.sanitized, "Line1\nLine2\nLine3");

        // Test with input that already has normalized line endings
        let clean = "Line1\nLine2\nLine3";
        let result = normalize_line_endings(clean);

        assert!(!result.was_modified);
        assert_eq!(result.sanitized, clean);
    }

    #[test]
    fn test_collapse_whitespace() {
        let input = "Hello    World   !";
        let result = collapse_whitespace(input);

        assert!(result.was_modified);
        assert_eq!(result.sanitized, "Hello World !");

        // Test with input that already has single spaces
        let clean = "Hello World !";
        let result = collapse_whitespace(clean);

        assert!(!result.was_modified);
        assert_eq!(result.sanitized, clean);
    }

    #[test]
    fn test_standard_string_sanitize() {
        let input = "  Hello\u{0000}World\r\n  ";
        let result = standard_string_sanitize(input);

        assert!(result.was_modified);
        assert_eq!(result.sanitized, "HelloWorld");
    }
}
