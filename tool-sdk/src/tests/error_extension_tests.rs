//! Extended tests for error handling functionality
//!
//! These tests verify advanced error scenarios and service-specific error mapping.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use reqwest::StatusCode;
    
    use crate::error::{ServiceError, ErrorContext, Result, mapping};
    use crate::services::openai;
    use crate::services::serpapi;
    
    #[test]
    fn test_error_propagation_chain() {
        // Test error propagation through a chain of functions
        fn inner_function() -> Result<()> {
            Err(ServiceError::timeout("Inner timeout"))
        }
        
        fn middle_function() -> Result<()> {
            inner_function().map_err(|e| {
                e.with_context_value("middle_layer", true)
            })
        }
        
        fn outer_function() -> Result<()> {
            middle_function().map_err(|e| {
                e.with_context_value("outer_layer", "reached")
            })
        }
        
        let error = outer_function().unwrap_err();
        
        // Error should maintain the original message
        assert!(error.to_string().contains("Inner timeout"));
        
        // Error should have all context from the chain
        let context = error.context();
        assert!(context.contains_key("middle_layer"));
        assert!(context.contains_key("outer_layer"));
    }
    
    #[test]
    fn test_complex_error_context_enrichment() {
        // Create an initial error
        let mut error = ServiceError::network("Connection failed");
        
        // Add context at multiple points
        error = error
            .with_context_value("attempt", 1)
            .with_context_value("endpoint", "/api/data");
        
        // Create a new context and apply it
        let context = ErrorContext::for_service("test_service")
            .status_code(500)
            .request_id("req-abc-123")
            .with("timeout_ms", 5000)
            .with("host", "api.example.com");
        
        error = error.with_context(context);
        
        // Add more context after applying a full context object
        error = error.with_context_value("retry", true);
        
        // Verify all context is present
        let error_context = error.context();
        assert_eq!(error_context.get("service_name").unwrap(), "test_service");
        assert_eq!(error_context.get("status_code").unwrap(), "500");
        assert_eq!(error_context.get("request_id").unwrap(), "req-abc-123");
        assert_eq!(error_context.get("timeout_ms").unwrap(), "5000");
        assert_eq!(error_context.get("host").unwrap(), "api.example.com");
        assert_eq!(error_context.get("attempt").unwrap(), "1");
        assert_eq!(error_context.get("endpoint").unwrap(), "/api/data");
        assert_eq!(error_context.get("retry").unwrap(), "true");
    }
    
    #[test]
    fn test_openai_error_mapping_comprehensive() {
        // Test various OpenAI API error types
        
        // 1. Rate limit error
        let rate_limit_json = serde_json::json!({
            "error": {
                "message": "Rate limit reached for gpt-3.5-turbo",
                "type": "rate_limit_error",
                "param": null,
                "code": "rate_limit_exceeded"
            }
        });
        
        let mut context = ErrorContext::for_service("openai");
        let error = mapping::map_openai_error(
            StatusCode::TOO_MANY_REQUESTS,
            &rate_limit_json,
            &mut context
        );
        
        assert!(matches!(error, ServiceError::RateLimit(_)));
        assert!(error.is_retryable());
        assert_eq!(context.get("error_type").unwrap(), "rate_limit_error");
        assert_eq!(context.get("error_code").unwrap(), "rate_limit_exceeded");
        
        // 2. Authentication error
        let auth_error_json = serde_json::json!({
            "error": {
                "message": "Incorrect API key provided",
                "type": "authentication_error",
                "param": null,
                "code": "invalid_api_key"
            }
        });
        
        let mut context = ErrorContext::for_service("openai");
        let error = mapping::map_openai_error(
            StatusCode::UNAUTHORIZED,
            &auth_error_json,
            &mut context
        );
        
        assert!(matches!(error, ServiceError::Authentication(_)));
        assert!(!error.is_retryable());
        assert_eq!(context.get("error_type").unwrap(), "authentication_error");
        
        // 3. Invalid request error
        let invalid_request_json = serde_json::json!({
            "error": {
                "message": "This model's maximum context length is 4097 tokens",
                "type": "invalid_request_error",
                "param": "messages",
                "code": "context_length_exceeded"
            }
        });
        
        let mut context = ErrorContext::for_service("openai");
        let error = mapping::map_openai_error(
            StatusCode::BAD_REQUEST,
            &invalid_request_json,
            &mut context
        );
        
        assert!(matches!(error, ServiceError::Validation(_)));
        assert!(!error.is_retryable());
        assert_eq!(context.get("error_type").unwrap(), "invalid_request_error");
        assert_eq!(context.get("error_param").unwrap(), "messages");
        
        // 4. Server error
        let server_error_json = serde_json::json!({
            "error": {
                "message": "The server is overloaded",
                "type": "server_error",
                "param": null,
                "code": null
            }
        });
        
        let mut context = ErrorContext::for_service("openai");
        let error = mapping::map_openai_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &server_error_json,
            &mut context
        );
        
        assert!(matches!(error, ServiceError::Service(_)));
        assert!(error.is_retryable());
        assert_eq!(context.get("error_type").unwrap(), "server_error");
    }
    
    #[test]
    fn test_serpapi_error_mapping_comprehensive() {
        // Test various SerpAPI error types
        
        // 1. Authentication error
        let auth_error_json = serde_json::json!({
            "error": "Invalid API key"
        });
        
        let mut context = ErrorContext::for_service("serpapi");
        let error = mapping::map_serpapi_error(
            StatusCode::FORBIDDEN,
            &auth_error_json,
            &mut context
        );
        
        assert!(matches!(error, ServiceError::Authentication(_)));
        assert!(!error.is_retryable());
        
        // 2. Rate limit error
        let rate_limit_json = serde_json::json!({
            "error": "You have exceeded the maximum number of searches per month"
        });
        
        let mut context = ErrorContext::for_service("serpapi");
        let error = mapping::map_serpapi_error(
            StatusCode::TOO_MANY_REQUESTS,
            &rate_limit_json,
            &mut context
        );
        
        assert!(matches!(error, ServiceError::RateLimit(_)));
        assert!(error.is_retryable());
        
        // 3. Validation error
        let validation_json = serde_json::json!({
            "error": "Parameter 'q' is required"
        });
        
        let mut context = ErrorContext::for_service("serpapi");
        let error = mapping::map_serpapi_error(
            StatusCode::BAD_REQUEST,
            &validation_json,
            &mut context
        );
        
        assert!(matches!(error, ServiceError::Validation(_)));
        assert!(!error.is_retryable());
        
        // 4. Generic error with no specific message
        let generic_json = serde_json::json!({});
        
        let mut context = ErrorContext::for_service("serpapi");
        let error = mapping::map_serpapi_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &generic_json,
            &mut context
        );
        
        assert!(matches!(error, ServiceError::Service(_)));
        assert!(error.is_retryable());
    }
    
    #[test]
    fn test_error_from_string_message() {
        // Test creating errors from string messages
        let error = ServiceError::network(format!("Failed to connect to {}", "api.example.com"));
        assert!(error.to_string().contains("api.example.com"));
        
        // Test with &str and String interoperability
        let message: String = "Authentication failed".to_string();
        let error = ServiceError::authentication(&message);
        assert!(error.to_string().contains(&message));
    }
    
    #[test]
    fn test_downcasting_errors() {
        // Test that we can downcast errors for specialized handling
        fn attempt_operation() -> Result<()> {
            Err(ServiceError::rate_limit("Service rate limited"))
        }
        
        let result = attempt_operation();
        assert!(result.is_err());
        
        match result {
            Ok(_) => panic!("Expected error"),
            Err(e) => {
                // Check error variant
                match e {
                    ServiceError::RateLimit(_) => {
                        // This is the expected path
                        assert!(e.to_string().contains("rate limited"));
                    }
                    _ => panic!("Expected RateLimit error variant"),
                }
                
                // Ensure the error is retryable
                assert!(e.is_retryable());
            }
        }
    }
    
    #[tokio::test]
    async fn test_async_error_propagation() {
        // Test error propagation in async context
        async fn async_inner() -> Result<()> {
            Err(ServiceError::timeout("Async operation timed out"))
        }
        
        async fn async_wrapper() -> Result<()> {
            async_inner().await?; // Use ? operator for propagation
            Ok(())
        }
        
        let error = async_wrapper().await.unwrap_err();
        assert!(error.to_string().contains("timed out"));
    }
    
    #[test]
    fn test_error_mapping_with_headers() {
        // Create a mock headers structure
        let mut headers = HashMap::new();
        headers.insert("retry-after".to_string(), "30".to_string());
        headers.insert("x-request-id".to_string(), "abc123".to_string());
        
        // Test extracting information from headers into error context
        let status = StatusCode::TOO_MANY_REQUESTS;
        let mut context = ErrorContext::new();
        
        // Add headers to context
        for (key, value) in headers {
            context.with(&key, value);
        }
        
        let error = ServiceError::from_status(status, "Rate limit exceeded")
            .with_context(context);
        
        // Verify headers were incorporated into context
        assert_eq!(error.context().get("retry-after").unwrap(), "30");
        assert_eq!(error.context().get("x-request-id").unwrap(), "abc123");
        
        // Verify the error is correctly classified
        assert!(matches!(error, ServiceError::RateLimit(_)));
        assert!(error.is_retryable());
    }
}