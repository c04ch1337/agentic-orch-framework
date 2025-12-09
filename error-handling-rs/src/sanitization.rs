//! # Error Sanitization
//!
//! This module provides functionality to sanitize errors before they are exposed
//! to external systems or end users, preventing sensitive information leakage.

use std::collections::HashSet;
use once_cell::sync::Lazy;
use regex::Regex;
use crate::types::{Error, ErrorKind, Severity};

// Static regex patterns for sensitive information detection
static SENSITIVE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // API keys, tokens, credentials
        Regex::new(r"(?i)(api[_-]?key|token|secret|password|credential)s?[\"']?\s*[=:]\s*[\"']?([^\"'\s]+)").unwrap(),
        
        // OAuth 2.0 tokens
        Regex::new(r"bearer\s+([a-zA-Z0-9\._\-]+)").unwrap(),
        
        // JWTs
        Regex::new(r"eyJ[a-zA-Z0-9\-_]+\.eyJ[a-zA-Z0-9\-_]+\.[a-zA-Z0-9\-_]+").unwrap(),
        
        // Email addresses
        Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
        
        // IP addresses
        Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(),
        
        // Phone numbers
        Regex::new(r"(\+\d{1,3}[\s-])?\(?\d{3}\)?[\s.-]?\d{3}[\s.-]?\d{4}").unwrap(),
        
        // Social security numbers
        Regex::new(r"\b\d{3}[-]?\d{2}[-]?\d{4}\b").unwrap(),
        
        // Credit cards
        Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b|\b\d{13,16}\b").unwrap(),
    ]
});

// List of context keys that should be redacted
static SENSITIVE_KEYS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let keys = [
        "password", "secret", "token", "key", "credential", "auth",
        "ssn", "social_security", "credit_card", "cc_number", "cvv",
        "private_key", "certificate", "api_key", "access_token",
        "refresh_token", "session_id", "cookie", "hash", "salt",
    ];
    HashSet::from_iter(keys.iter().copied())
});

/// Sanitizes an error by removing sensitive information
pub fn sanitize_error(mut error: Error) -> Error {
    // Create a new error with the same ID but sanitized information
    let mut sanitized = Error::new(error.kind.clone(), sanitize_message(&error.message))
        .severity(error.severity)
        .code(error.code.clone().unwrap_or_else(|| "UNKNOWN".to_string()));

    // Preserve the error ID for traceability
    sanitized.id = error.id;
    sanitized.timestamp = error.timestamp;
    sanitized.correlation_id = error.correlation_id.clone();
    sanitized.service = error.service.clone();
    sanitized.transient = error.transient;

    // Use the user_message if available, otherwise create a generic one
    if let Some(user_msg) = error.user_message {
        sanitized.user_message = Some(user_msg);
    } else {
        sanitized.user_message = Some(create_user_message(&error));
    }

    // Sanitize context data
    if !error.context.is_empty() {
        for (key, value) in error.context.iter() {
            // Skip sensitive keys entirely
            if is_sensitive_key(key) {
                continue;
            }

            // Add sanitized values for non-sensitive keys
            if let Some(value_str) = value.as_str() {
                let sanitized_value = sanitize_value(value_str);
                sanitized = sanitized.context(key, sanitized_value);
            } else {
                // For non-string values, just copy them over
                sanitized = sanitized.context(key, value.clone());
            }
        }
    }

    sanitized
}

/// Determines if a context key is sensitive
fn is_sensitive_key(key: &str) -> bool {
    let key_lower = key.to_lowercase();
    
    // Check direct matches
    if SENSITIVE_KEYS.contains(key_lower.as_str()) {
        return true;
    }
    
    // Check partial matches
    for sensitive_key in SENSITIVE_KEYS.iter() {
        if key_lower.contains(*sensitive_key) {
            return true;
        }
    }
    
    false
}

/// Sanitizes a message by removing sensitive information
fn sanitize_message(message: &str) -> String {
    let mut sanitized = message.to_string();
    
    // Replace sensitive patterns
    for pattern in SENSITIVE_PATTERNS.iter() {
        sanitized = pattern.replace_all(&sanitized, |caps: &regex::Captures| {
            if caps.len() > 1 {
                // Keep the key name but redact the value
                format!("{}=[REDACTED]", &caps[1])
            } else {
                // Redact the entire match
                "[REDACTED]".to_string()
            }
        }).to_string();
    }
    
    sanitized
}

/// Sanitizes a string value
fn sanitize_value(value: &str) -> String {
    // Check if the value matches any sensitive patterns
    for pattern in SENSITIVE_PATTERNS.iter() {
        if pattern.is_match(value) {
            return "[REDACTED]".to_string();
        }
    }
    
    value.to_string()
}

/// Creates a user-friendly message for an error
fn create_user_message(error: &Error) -> String {
    match error.kind {
        ErrorKind::Authentication => "Authentication failed. Please check your credentials and try again.".to_string(),
        ErrorKind::Validation => "The provided data is invalid. Please check your input and try again.".to_string(),
        ErrorKind::RateLimit => "Too many requests. Please try again later.".to_string(),
        ErrorKind::Timeout => "The operation timed out. Please try again later.".to_string(),
        ErrorKind::Unavailable => "The service is currently unavailable. Please try again later.".to_string(),
        ErrorKind::External => "An error occurred while communicating with an external service.".to_string(),
        ErrorKind::Security => "A security issue was detected. Our team has been notified.".to_string(),
        _ => match error.severity {
            Severity::Critical | Severity::Fatal => 
                "A critical error occurred. Our team has been notified of the issue.".to_string(),
            _ => 
                "An unexpected error occurred. Please try again later.".to_string(),
        }
    }
}

/// Determines if an error is safe to expose externally
pub fn is_safe_for_external(error: &Error) -> bool {
    match error.kind {
        ErrorKind::Validation |
        ErrorKind::Authentication |
        ErrorKind::RateLimit |
        ErrorKind::Timeout |
        ErrorKind::Unavailable => true,
        _ => false
    }
}

/// Creates an error response suitable for external APIs
pub fn create_external_error_response(error: &Error) -> serde_json::Value {
    let error_sanitized = sanitize_error(error.clone());
    
    let status_code = match error.kind {
        ErrorKind::Authentication => 401,
        ErrorKind::Validation => 400,
        ErrorKind::RateLimit => 429,
        ErrorKind::Timeout => 408,
        ErrorKind::Unavailable => 503,
        _ => 500,
    };
    
    let error_code = error_sanitized.code.unwrap_or_else(|| "UNKNOWN".to_string());
    
    // Create the error response
    serde_json::json!({
        "error": {
            "code": error_code,
            "message": error_sanitized.user_message.unwrap_or_else(|| "An error occurred".to_string()),
            "status": status_code,
            "id": error_sanitized.id.to_string(),
            "type": error_sanitized.kind.to_string(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Error, ErrorKind, Severity};

    #[test]
    fn test_sanitize_message() {
        let sensitive = "Failed to connect with API key=abc123xyz";
        let sanitized = sanitize_message(sensitive);
        assert_eq!(sanitized, "Failed to connect with API key=[REDACTED]");
        
        let jwt = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let sanitized_jwt = sanitize_message(jwt);
        assert_eq!(sanitized_jwt, "Bearer [REDACTED]");
    }

    #[test]
    fn test_is_sensitive_key() {
        assert!(is_sensitive_key("password"));
        assert!(is_sensitive_key("user_password"));
        assert!(is_sensitive_key("api_key"));
        assert!(is_sensitive_key("access_token"));
        
        assert!(!is_sensitive_key("username"));
        assert!(!is_sensitive_key("timestamp"));
        assert!(!is_sensitive_key("count"));
    }

    #[test]
    fn test_sanitize_error() {
        let error = Error::new(ErrorKind::Authentication, "Failed to authenticate with token abc123")
            .context("username", "test_user")
            .context("password", "secret123")
            .context("request_path", "/api/auth");

        let sanitized = sanitize_error(error);
        
        // ID and kind should be preserved
        assert_eq!(sanitized.kind, ErrorKind::Authentication);
        
        // Message should be sanitized
        assert!(sanitized.message.contains("[REDACTED]"));
        assert!(!sanitized.message.contains("abc123"));
        
        // Sensitive contexts should be removed
        assert!(!sanitized.context.contains_key("password"));
        
        // Non-sensitive contexts should be preserved
        assert!(sanitized.context.contains_key("username"));
        assert!(sanitized.context.contains_key("request_path"));
    }

    #[test]
    fn test_external_error_response() {
        let error = Error::new(ErrorKind::Validation, "Invalid input: password too short")
            .code("VAL_001");
            
        let response = create_external_error_response(&error);
        
        assert_eq!(response["error"]["code"], "VAL_001");
        assert_eq!(response["error"]["status"], 400);
        assert!(!response["error"]["message"].as_str().unwrap().contains("password"));
    }
}