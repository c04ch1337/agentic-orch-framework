//! Error mapping for service-specific APIs
//!
//! This module provides mapping functions to convert service-specific
//! error responses to our normalized ServiceError type.

use reqwest::StatusCode;
use serde_json::Value;

use super::{ErrorContext, ServiceError};

/// Map an OpenAI API error to a ServiceError
pub fn map_openai_error(
    status: StatusCode,
    json: &Value,
    context: &mut ErrorContext,
) -> ServiceError {
    // Set the service name
    context.service = "openai".to_string();
    
    // Extract OpenAI-specific error information
    if let Some(error) = json.get("error") {
        // Extract error type
        if let Some(error_type) = error.get("type").and_then(|t| t.as_str()) {
            context.add("error_type", error_type);
        }
        
        // Extract error code
        if let Some(code) = error.get("code").and_then(|c| c.as_str()) {
            context.error_code = Some(code.to_string());
        }
        
        // Extract error message
        let message = error.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown OpenAI error");
        
        // Map to appropriate error based on status code and error type
        return match status {
            StatusCode::UNAUTHORIZED => ServiceError::authentication(message),
            StatusCode::FORBIDDEN => ServiceError::authorization(message),
            StatusCode::TOO_MANY_REQUESTS => ServiceError::rate_limit(message),
            StatusCode::BAD_REQUEST => ServiceError::validation(message),
            StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE => ServiceError::service(message),
            _ => ServiceError::service(message),
        };
    } else {
        // Fallback if we can't parse the error structure
        let message = json.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        
        return match status {
            StatusCode::UNAUTHORIZED => ServiceError::authentication(message),
            StatusCode::FORBIDDEN => ServiceError::authorization(message),
            StatusCode::TOO_MANY_REQUESTS => ServiceError::rate_limit(message),
            StatusCode::BAD_REQUEST => ServiceError::validation(message),
            StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE => ServiceError::service(message),
            _ => ServiceError::service(message),
        };
    }
}

/// Map a SerpAPI error to a ServiceError
pub fn map_serpapi_error(
    status: StatusCode,
    json: &Value,
    context: &mut ErrorContext,
) -> ServiceError {
    // Set the service name
    context.service = "serpapi".to_string();
    
    // Extract SerpAPI-specific error information
    let error_message = json.get("error")
        .and_then(|e| e.as_str())
        .unwrap_or("Unknown SerpAPI error");
    
    // Map to appropriate error based on status code and message
    return match status {
        StatusCode::UNAUTHORIZED => ServiceError::authentication(error_message),
        StatusCode::PAYMENT_REQUIRED => ServiceError::authorization("Account credits exhausted")
            .with_context_value("details", error_message),
        StatusCode::TOO_MANY_REQUESTS => ServiceError::rate_limit(error_message),
        StatusCode::BAD_REQUEST => ServiceError::validation(error_message),
        _ => ServiceError::service(error_message),
    };
}

/// Map a generic HTTP error to a ServiceError
pub fn map_http_error(
    status: StatusCode,
    body: &str,
    context: &mut ErrorContext,
) -> ServiceError {
    // Try to parse as JSON first
    if let Ok(json) = serde_json::from_str::<Value>(body) {
        match context.service.as_str() {
            "openai" => return map_openai_error(status, &json, context),
            "serpapi" => return map_serpapi_error(status, &json, context),
            _ => {
                // Generic JSON error handling
                let message = json.get("message")
                    .or_else(|| json.get("error"))
                    .and_then(|m| m.as_str())
                    .unwrap_or(body);
                
                return match status {
                    StatusCode::UNAUTHORIZED => ServiceError::authentication(message),
                    StatusCode::FORBIDDEN => ServiceError::authorization(message),
                    StatusCode::TOO_MANY_REQUESTS => ServiceError::rate_limit(message),
                    StatusCode::BAD_REQUEST => ServiceError::validation(message),
                    StatusCode::NOT_FOUND => ServiceError::service(format!("Resource not found: {}", message)),
                    StatusCode::INTERNAL_SERVER_ERROR
                    | StatusCode::BAD_GATEWAY
                    | StatusCode::SERVICE_UNAVAILABLE => ServiceError::service(message),
                    _ => ServiceError::service(message),
                };
            }
        }
    }
    
    // Fallback to status-based mapping
    let message = if body.is_empty() {
        status.to_string()
    } else if body.len() > 100 {
        format!("{}: {:.100}...", status, body)
    } else {
        format!("{}: {}", status, body)
    };
    
    return match status {
        StatusCode::UNAUTHORIZED => ServiceError::authentication(message),
        StatusCode::FORBIDDEN => ServiceError::authorization(message),
        StatusCode::TOO_MANY_REQUESTS => ServiceError::rate_limit(message),
        StatusCode::BAD_REQUEST => ServiceError::validation(message),
        StatusCode::NOT_FOUND => ServiceError::service(format!("Resource not found: {}", status)),
        StatusCode::INTERNAL_SERVER_ERROR
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE => ServiceError::service(message),
        _ => ServiceError::service(message),
    };
}

/// Helper function to classify HTTP errors by category
pub fn classify_http_error(status: StatusCode) -> &'static str {
    match status.as_u16() {
        400 => "validation",
        401 => "authentication",
        403 => "authorization",
        404 => "not_found",
        408 => "timeout",
        429 => "rate_limit",
        500..=599 => "server",
        _ => "unknown",
    }
}

/// Determine if an HTTP status code indicates a retryable error
pub fn is_retryable_status(status: StatusCode) -> bool {
    match status.as_u16() {
        408 | 429 | 500 | 502 | 503 | 504 => true,
        _ => false,
    }
}