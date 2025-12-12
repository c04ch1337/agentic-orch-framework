//! Utility module for common functionality
//!
//! This module provides common utility functions used across the Tool SDK.

use std::time::{Duration, Instant};

/// Measure the execution time of a closure
pub fn measure_time<F, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    (result, duration)
}

/// Async version of measure_time
pub async fn measure_time_async<F, T, Fut>(f: F) -> (T, Duration)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = f().await;
    let duration = start.elapsed();
    (result, duration)
}

/// Truncate a string to a maximum length, adding ellipsis if truncated
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s[..max_len].to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Sanitize a string for logging (remove sensitive data patterns)
pub fn sanitize_for_logging(s: &str) -> String {
    // Replace common sensitive patterns
    let patterns = [
        (r"Bearer [A-Za-z0-9\-_]+", "Bearer [REDACTED]"),
        (r"api[_-]?key[=:]\s*[A-Za-z0-9\-_]+", "api_key=[REDACTED]"),
        (r"password[=:]\s*[^\s&]+", "password=[REDACTED]"),
        (r"secret[=:]\s*[^\s&]+", "secret=[REDACTED]"),
    ];
    
    let mut result = s.to_string();
    for (pattern, replacement) in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            result = re.replace_all(&result, replacement).to_string();
        }
    }
    result
}

/// Generate a unique request ID
pub fn generate_request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Parse a duration from a string (e.g., "30s", "5m", "1h")
pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim().to_lowercase();
    
    if s.ends_with("ms") {
        s[..s.len()-2].parse::<u64>().ok().map(Duration::from_millis)
    } else if s.ends_with('s') {
        s[..s.len()-1].parse::<u64>().ok().map(Duration::from_secs)
    } else if s.ends_with('m') {
        s[..s.len()-1].parse::<u64>().ok().map(|m| Duration::from_secs(m * 60))
    } else if s.ends_with('h') {
        s[..s.len()-1].parse::<u64>().ok().map(|h| Duration::from_secs(h * 3600))
    } else {
        // Try parsing as seconds
        s.parse::<u64>().ok().map(Duration::from_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("hi", 2), "hi");
    }
    
    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s"), Some(Duration::from_secs(30)));
        assert_eq!(parse_duration("5m"), Some(Duration::from_secs(300)));
        assert_eq!(parse_duration("1h"), Some(Duration::from_secs(3600)));
        assert_eq!(parse_duration("100ms"), Some(Duration::from_millis(100)));
        assert_eq!(parse_duration("60"), Some(Duration::from_secs(60)));
    }
    
    #[test]
    fn test_sanitize_for_logging() {
        let input = "Authorization: Bearer abc123xyz";
        let output = sanitize_for_logging(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("abc123xyz"));
    }
}
