//! Safety Service Enhanced Validation
//!
//! This module provides improved validation capabilities for the Safety Service,
//! leveraging the input-validation-rs library for robust validation and
//! protection against regex denial of service (ReDoS).

use input_validation_rs::prelude::*;
use input_validation_rs::validators::{redos, security};
use lazy_static::lazy_static;
use regex_automata::dfa::regex::Regex as DfaRegex;
use std::time::{Duration, Instant};

/// Timeout for regex operations (milliseconds)
const REGEX_TIMEOUT_MS: u64 = 100;

/// Result of enhanced pattern matching
pub struct PatternMatchResult {
    /// Whether the pattern matched
    pub matched: bool,
    /// The pattern that matched (if any)
    pub pattern: Option<String>,
    /// Category of the matched pattern
    pub category: Option<String>,
    /// Severity of the match (1-5, with 5 being most severe)
    pub severity: u8,
}

/// Validate text using token-based pattern matching for improved security and performance
pub fn validate_with_tokens(
    text: &str,
    patterns: &[&str],
    category: &str,
    severity: u8,
) -> PatternMatchResult {
    for pattern in patterns {
        let start_time = Instant::now();

        // Use token-based contains for simple patterns (faster and safer)
        let found = redos::token_based_contains(text, pattern);

        // Check for timeout (possible DoS attempt)
        if start_time.elapsed() > Duration::from_millis(REGEX_TIMEOUT_MS) {
            return PatternMatchResult {
                matched: true, // Treat timeout as a match for safety
                pattern: Some(format!("timeout_checking_{}", pattern)),
                category: Some(format!("{}_timeout", category)),
                severity: 5, // Maximum severity for timeouts
            };
        }

        if found {
            return PatternMatchResult {
                matched: true,
                pattern: Some(pattern.to_string()),
                category: Some(category.to_string()),
                severity,
            };
        }
    }

    PatternMatchResult {
        matched: false,
        pattern: None,
        category: None,
        severity: 0,
    }
}

/// Validate text using safe regex matching with timeout protection
pub fn validate_with_safe_regex(
    text: &str,
    regex_patterns: &[&str],
    category: &str,
    severity: u8,
) -> PatternMatchResult {
    for pattern in regex_patterns {
        // Use safe DFA-based regex with timeout protection
        match redos::safe_pattern_match(text, pattern) {
            Ok(matched) => {
                if matched {
                    return PatternMatchResult {
                        matched: true,
                        pattern: Some(pattern.to_string()),
                        category: Some(category.to_string()),
                        severity,
                    };
                }
            }
            Err(e) => {
                // If we got an error (likely timeout), treat as a match for safety
                if let ValidationError::SecurityThreat(_) = e {
                    return PatternMatchResult {
                        matched: true,
                        pattern: Some(pattern.to_string()),
                        category: Some(format!("{}_security_threat", category)),
                        severity: 5, // Maximum severity for security threats
                    };
                }

                // Other errors (like invalid regex) should be logged but not matched
                log::warn!("Error in regex matching: {}", e);
            }
        }
    }

    PatternMatchResult {
        matched: false,
        pattern: None,
        category: None,
        severity: 0,
    }
}

/// Enhanced threat detection that's protected against ReDoS
pub fn detect_threats(content: &str) -> Option<(String, String, u8)> {
    // Normalize content and convert to lowercase for case-insensitive matching
    let normalized = content.to_lowercase();

    // First check with the security module from input-validation-rs
    if let Err(err) = security::default_security_scan(&normalized) {
        if let ValidationError::SecurityThreat(msg) = err {
            return Some(("SECURITY_THREAT".to_string(), msg, 5));
        }
    }

    // Check for SQL injection with token-based matching first (faster)
    lazy_static! {
        static ref SQL_TOKENS: Vec<&'static str> = vec![
            "select", "from", "where", "union", "insert", "update", "delete", "drop", "alter",
            "create", "--", "/*", "*/", ";", "' or", "\" or", "or 1=1", "admin'--"
        ];
    }
    let sql_result = validate_with_tokens(&normalized, &SQL_TOKENS, "SQL_INJECTION", 5);
    if sql_result.matched {
        return Some((
            "SQL_INJECTION".to_string(),
            format!(
                "Matched pattern: {}",
                sql_result.pattern.unwrap_or_default()
            ),
            sql_result.severity,
        ));
    }

    // Check for shell injection with token-based matching
    lazy_static! {
        static ref SHELL_TOKENS: Vec<&'static str> = vec![
            "rm -rf",
            "/bin/sh",
            "wget ",
            "curl ",
            "&& ",
            "|| ",
            "; ",
            "eval(",
            "`",
            "$(",
            "> /dev/null",
            "2>&1",
            "/etc/passwd"
        ];
    }
    let shell_result = validate_with_tokens(&normalized, &SHELL_TOKENS, "SHELL_INJECTION", 5);
    if shell_result.matched {
        return Some((
            "SHELL_INJECTION".to_string(),
            format!(
                "Matched pattern: {}",
                shell_result.pattern.unwrap_or_default()
            ),
            shell_result.severity,
        ));
    }

    // Check for XSS with token-based matching
    lazy_static! {
        static ref XSS_TOKENS: Vec<&'static str> = vec![
            "<script",
            "</script>",
            "javascript:",
            "onerror=",
            "onload=",
            "onclick=",
            "onmouseover=",
            "document.cookie",
            "eval(",
            "fromcharcode"
        ];
    }
    let xss_result = validate_with_tokens(&normalized, &XSS_TOKENS, "XSS", 4);
    if xss_result.matched {
        return Some((
            "XSS".to_string(),
            format!(
                "Matched pattern: {}",
                xss_result.pattern.unwrap_or_default()
            ),
            xss_result.severity,
        ));
    }

    // Path traversal with token-based matching
    lazy_static! {
        static ref PATH_TOKENS: Vec<&'static str> = vec![
            "../",
            "..",
            "/..",
            "../etc/passwd",
            "..\\windows\\",
            "/etc/shadow",
            "\\system32\\",
            "..\\..\\"
        ];
    }
    let path_result = validate_with_tokens(&normalized, &PATH_TOKENS, "PATH_TRAVERSAL", 4);
    if path_result.matched {
        return Some((
            "PATH_TRAVERSAL".to_string(),
            format!(
                "Matched pattern: {}",
                path_result.pattern.unwrap_or_default()
            ),
            path_result.severity,
        ));
    }

    // For more complex patterns, use safe regex matching as a fallback
    lazy_static! {
        static ref COMPLEX_SQL_PATTERNS: Vec<&'static str> = vec![
            r"(?i)(\bSELECT\b.*\bFROM\b)",
            r"(?i)(\bUNION\b.*\bSELECT\b)",
            r"(?i)(--\s*$|;\s*--\s*)"
        ];
    }
    let complex_sql_result = validate_with_safe_regex(
        &normalized,
        &COMPLEX_SQL_PATTERNS,
        "COMPLEX_SQL_INJECTION",
        5,
    );
    if complex_sql_result.matched {
        return Some((
            "COMPLEX_SQL_INJECTION".to_string(),
            format!(
                "Matched regex: {}",
                complex_sql_result.pattern.unwrap_or_default()
            ),
            complex_sql_result.severity,
        ));
    }

    // No threats detected
    None
}

/// Comprehensive content validation
pub fn validate_content(content: &str, operation: Option<&str>) -> ValidationResult<()> {
    // Apply multiple validators in sequence for thorough validation
    let validators: Vec<Box<dyn Fn(&str) -> ValidationResult<()>>> = vec![
        // Check for malformed UTF-8 sequences
        Box::new(|s| input_validation_rs::validators::string::is_valid_utf8(s)),
        // Check for security threats
        Box::new(|s| security::default_security_scan(s)),
        // Check for control characters
        Box::new(|s| input_validation_rs::validators::string::no_control_chars(s)),
    ];

    // Run all validators
    let mut errors = Vec::new();
    for validator in validators {
        if let Err(err) = validator(content) {
            errors.push(err);
        }
    }

    // Check operation if provided
    if let Some(op) = operation {
        let op_validator = |s: &str| {
            if s.contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '.') {
                Err(ValidationError::InvalidCharacters(format!(
                    "Operation contains invalid characters: {}",
                    s
                )))
            } else {
                Ok(())
            }
        };

        if let Err(err) = op_validator(op) {
            errors.push(err);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::composite(errors))
    }
}

/// Sanitize input to make it safer
pub fn sanitize_input(content: &str) -> String {
    let result = input_validation_rs::sanitizers::string::standard_string_sanitize(content);
    result.sanitized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_threats() {
        // SQL Injection
        let sql_attack = "SELECT * FROM users WHERE username='admin'";
        assert!(detect_threats(sql_attack).is_some());

        // Shell Injection
        let shell_attack = "cat /etc/passwd; rm -rf /";
        assert!(detect_threats(shell_attack).is_some());

        // XSS Attack
        let xss_attack = "<script>alert('XSS');</script>";
        assert!(detect_threats(xss_attack).is_some());

        // Path Traversal
        let path_attack = "../../etc/passwd";
        assert!(detect_threats(path_attack).is_some());

        // Safe Input
        let safe_input = "Hello, this is a normal message!";
        assert!(detect_threats(safe_input).is_none());
    }

    #[test]
    fn test_validate_content() {
        // Valid content
        let valid = "This is valid content.";
        assert!(validate_content(valid, Some("valid_operation")).is_ok());

        // Content with control characters
        let invalid = "Hello\u{0000}World";
        assert!(validate_content(invalid, None).is_err());

        // Invalid operation
        assert!(validate_content("Valid content", Some("invalid;operation")).is_err());
    }

    #[test]
    fn test_sanitize_input() {
        // Input with control characters
        let input = "Hello\u{0003}World\u{0000}!";
        let sanitized = sanitize_input(input);
        assert_eq!(sanitized, "HelloWorld!");
    }
}
