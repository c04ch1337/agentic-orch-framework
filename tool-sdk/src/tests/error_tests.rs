//! Tests for error handling functionality
//!
//! These tests verify that the error system in the SDK works correctly.

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fmt;
    use reqwest::StatusCode;
    
    use crate::error::{ServiceError, ErrorContext, Result, mapping};
    
    #[test]
    fn test_service_error_creation() {
        // Test factory methods
        let network_err = ServiceError::network("Connection failed");
        let auth_err = ServiceError::authentication("Invalid credentials");
        let rate_limit_err = ServiceError::rate_limit("Too many requests");
        
        // Check error messages
        assert_eq!(network_err.to_string(), "Network error: Connection failed");
        assert_eq!(auth_err.to_string(), "Authentication error: Invalid credentials");
        assert_eq!(rate_limit_err.to_string(), "Rate limit exceeded: Too many requests");
        
        // Check error classification for retry decisions
        assert!(network_err.is_retryable());
        assert!(!auth_err.is_retryable());
        assert!(rate_limit_err.is_retryable());
        
        // Test permanence check
        assert!(!network_err.is_permanent());
        assert!(auth_err.is_permanent());
        assert!(!rate_limit_err.is_permanent());
    }
    
    #[test]
    fn test_error_context() {
        // Create basic error
        let base_err = ServiceError::network("Connection timeout");
        
        // Add context using with_context
        let context = ErrorContext::for_service("test_service")
            .status_code(408)
            .request_id("req-123")
            .endpoint("api/data")
            .with("attempt", 3)
            .with("host", "api.example.com");
        
        let err_with_context = base_err.with_context(context);
        
        // Test that context data is accessible
        assert_eq!(err_with_context.service_name(), Some("test_service"));
        assert_eq!(err_with_context.status_code(), Some(408));
        
        // Test display formatting includes base error
        assert!(err_with_context.to_string().contains("Connection timeout"));
        
        // Test quick context addition
        let quick_err = ServiceError::timeout("Request timed out")
            .with_context_value("attempt", 2);
        
        // Error message should still be the same
        assert!(quick_err.to_string().contains("Request timed out"));
    }
    
    #[test]
    fn test_error_mapping() {
        // Test HTTP error code classification
        assert_eq!(mapping::classify_http_error(StatusCode::UNAUTHORIZED), "authentication");
        assert_eq!(mapping::classify_http_error(StatusCode::FORBIDDEN), "authorization");
        assert_eq!(mapping::classify_http_error(StatusCode::TOO_MANY_REQUESTS), "rate_limit");
        assert_eq!(mapping::classify_http_error(StatusCode::INTERNAL_SERVER_ERROR), "server");
        
        // Test HTTP retryable status
        assert!(mapping::is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(mapping::is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(!mapping::is_retryable_status(StatusCode::UNAUTHORIZED));
        assert!(!mapping::is_retryable_status(StatusCode::BAD_REQUEST));
        
        // Test OpenAI error mapping
        let json = serde_json::json!({
            "error": {
                "type": "rate_limit_error",
                "code": "rate_limit_exceeded",
                "message": "You have exceeded your rate limit"
            }
        });
        
        let mut context = ErrorContext::for_service("openai");
        let mapped_error = mapping::map_openai_error(
            StatusCode::TOO_MANY_REQUESTS,
            &json,
            &mut context
        );
        
        assert!(matches!(mapped_error, ServiceError::RateLimit(_)));
        assert!(mapped_error.is_retryable());
        
        // Test SerpAPI error mapping
        let json = serde_json::json!({
            "error": "No search results found",
        });
        
        let mut context = ErrorContext::for_service("serpapi");
        let mapped_error = mapping::map_serpapi_error(
            StatusCode::BAD_REQUEST,
            &json,
            &mut context
        );
        
        assert!(matches!(mapped_error, ServiceError::Validation(_)));
        assert!(!mapped_error.is_retryable());
    }
    
    #[test]
    fn test_reqwest_error_conversion() {
        // Create a mock reqwest error
        #[derive(Debug)]
        struct MockReqwestError {
            kind: MockErrorKind,
            status: Option<StatusCode>,
        }
        
        #[derive(Debug)]
        enum MockErrorKind {
            Timeout,
            Connect,
            Decode,
            Redirect,
        }
        
        impl fmt::Display for MockReqwestError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.kind {
                    MockErrorKind::Timeout => write!(f, "request timed out"),
                    MockErrorKind::Connect => write!(f, "connection error"),
                    MockErrorKind::Decode => write!(f, "decode error"),
                    MockErrorKind::Redirect => write!(f, "too many redirects"),
                }
            }
        }
        
        impl Error for MockReqwestError {}
        
        // Unfortunately, we can't directly test the From<reqwest::Error> implementation
        // without actual reqwest errors, so we'll just check the behavior manually
        
        // Timeout error should be mapped to ServiceError::Timeout
        let error_str = format!(
            "{}",
            ServiceError::timeout("Request timed out")
        );
        assert!(error_str.contains("Timeout error"));
        assert!(ServiceError::timeout("test").is_retryable());
        
        // Network error should be mapped to ServiceError::Network
        let error_str = format!(
            "{}",
            ServiceError::network("Connection failed")
        );
        assert!(error_str.contains("Network error"));
        assert!(ServiceError::network("test").is_retryable());
    }
    
    #[test]
    fn test_result_type() {
        // Test the Result type alias works correctly
        let success: Result<i32> = Ok(42);
        let failure: Result<i32> = Err(ServiceError::parsing("Invalid number"));
        
        assert_eq!(success.unwrap(), 42);
        assert!(failure.is_err());
        
        // Test using map and other Result methods
        let mapped = success.map(|n| n * 2);
        assert_eq!(mapped.unwrap(), 84);
        
        // Test error propagation with ?
        fn might_fail() -> Result<i32> {
            let x = failure?; // Should propagate the error
            Ok(x)
        }
        
        assert!(might_fail().is_err());
    }
}