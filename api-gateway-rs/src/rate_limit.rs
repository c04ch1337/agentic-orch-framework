// Rate Limiting Module using tower-governor
// Implements per-API-key sliding window rate limiting

use axum::{
    extract::Request,
    http::{StatusCode, HeaderMap},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
    body::Body,
};
use governor::{
    clock::{QuantaInstant, QuantaClock},
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use serde::Serialize;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use tower_governor::{governor::GovernorConfigBuilder, key_extractor::{KeyExtractor}};

// Custom key extractor for API keys
#[derive(Clone)]
pub struct ApiKeyExtractor;

impl KeyExtractor for ApiKeyExtractor {
    type Key = String;

    fn extract(&self, req: &Request<Body>) -> Result<Self::Key, tower_governor::errors::GovernorError> {
        // Try to get API key from X-PHOENIX-API-KEY header
        if let Some(api_key) = req.headers()
            .get("X-PHOENIX-API-KEY")
            .and_then(|v| v.to_str().ok()) {
            return Ok(api_key.to_string());
        }
        
        // Fallback to getting from request extensions (set by auth interceptor)
        if let Some(api_key) = req.extensions().get::<String>() {
            return Ok(api_key.clone());
        }
        
        // No API key found, use a default key for unauthenticated requests
        Ok("anonymous".to_string())
    }
}

// Per-key rate limiters
static RATE_LIMITERS: Lazy<Arc<RwLock<HashMap<String, Arc<RateLimiter<String, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>>>>>> 
    = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

#[derive(Debug, Serialize)]
pub struct RateLimitError {
    pub error: String,
    pub code: u16,
    pub retry_after_seconds: u64,
}

/// Create a rate limiter for a specific API key
async fn get_or_create_rate_limiter(api_key: &str) -> Arc<RateLimiter<String, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>> {
    let mut limiters = RATE_LIMITERS.write().await;
    
    if let Some(limiter) = limiters.get(api_key) {
        return limiter.clone();
    }
    
    // Create new rate limiter with 100 requests per minute
    let quota = Quota::per_minute(NonZeroU32::new(100).unwrap());
    let limiter = Arc::new(RateLimiter::keyed(quota));
    
    limiters.insert(api_key.to_string(), limiter.clone());
    limiter
}

/// Extract API key from headers
fn extract_api_key_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-PHOENIX-API-KEY")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Tower-governor based rate limiting middleware
pub async fn tower_governor_rate_limiter(
    req: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let path = req.uri().path();
    
    // Skip rate limiting for health and root endpoints
    if path == "/" || path == "/health" {
        return Ok(next.run(req).await);
    }
    
    // Extract API key
    let api_key = extract_api_key_from_headers(req.headers())
        .or_else(|| req.extensions().get::<String>().cloned())
        .unwrap_or_else(|| "anonymous".to_string());
    
    // Get or create rate limiter for this API key
    let limiter = get_or_create_rate_limiter(&api_key).await;
    
    // Check rate limit
    match limiter.check_key(&api_key) {
        Ok(_) => {
            // Request is within rate limit
            Ok(next.run(req).await)
        }
        Err(_) => {
            // Rate limit exceeded
            let retry_after = 60; // Suggest retry after 60 seconds
            
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(RateLimitError {
                    error: "Rate limit exceeded: Maximum 100 requests per minute per API key".to_string(),
                    code: 429,
                    retry_after_seconds: retry_after,
                }),
            ))
        }
    }
}

/// Update rate limit for a specific API key
pub async fn update_rate_limit(api_key: &str, requests_per_minute: u32) {
    let mut limiters = RATE_LIMITERS.write().await;
    
    // Remove old limiter
    limiters.remove(api_key);
    
    // Create new limiter with updated quota
    if let Some(rpm) = NonZeroU32::new(requests_per_minute) {
        let quota = Quota::per_minute(rpm);
        let limiter = Arc::new(RateLimiter::keyed(quota));
        limiters.insert(api_key.to_string(), limiter);
    }
}

/// Get current rate limit stats for an API key
pub async fn get_rate_limit_stats(api_key: &str) -> Option<(u32, Duration)> {
    let limiters = RATE_LIMITERS.read().await;
    
    if let Some(limiter) = limiters.get(api_key) {
        // This is a simplified version - in production you'd want more detailed stats
        Some((100, Duration::from_secs(60))) // 100 requests per 60 seconds
    } else {
        None
    }
}