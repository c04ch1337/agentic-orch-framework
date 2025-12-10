// Phoenix API Key Authentication Module
// Implements secure API key validation with file-based storage

use axum::{
    http::{Request, StatusCode, HeaderMap},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
    body::Body,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;

// API Key storage
static API_KEYS: Lazy<Arc<RwLock<HashSet<String>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashSet::new()))
});

// Rate limit tracking per API key
static RATE_LIMITER: Lazy<Arc<RwLock<HashMap<String, RateLimitInfo>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

#[derive(Clone)]
struct RateLimitInfo {
    count: u32,
    window_start: std::time::Instant,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyError {
    pub error: String,
    pub code: u16,
}

// Phoenix API Key header constant
const PHOENIX_API_KEY_HEADER: &str = "X-PHOENIX-API-KEY";

/// Load API keys from a secure file
pub async fn load_api_keys_from_file(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(file_path);
    
    if !path.exists() {
        // Create default API keys file if it doesn't exist
        let default_keys = vec![
            "phoenix-default-key-2024",
            "phoenix-admin-key-secure",
            "phoenix-service-key-internal"
        ];
        
        let content = default_keys.join("\n");
        fs::write(path, content)?;
        log::info!("Created default API keys file at: {}", file_path);
    }
    
    // Read API keys from file
    let content = fs::read_to_string(path)?;
    let mut keys = API_KEYS.write().await;
    keys.clear();
    
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            keys.insert(trimmed.to_string());
        }
    }
    
    log::info!("Loaded {} API keys from {}", keys.len(), file_path);
    Ok(())
}

/// Add an API key programmatically
pub async fn add_api_key(key: String) {
    let mut keys = API_KEYS.write().await;
    keys.insert(key);
}

/// Remove an API key programmatically
pub async fn remove_api_key(key: &str) {
    let mut keys = API_KEYS.write().await;
    keys.remove(key);
}

/// Check if an API key is valid
async fn is_valid_api_key(key: &str) -> bool {
    let keys = API_KEYS.read().await;
    keys.contains(key)
}

/// Extract API key from headers
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get(PHOENIX_API_KEY_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Phoenix API Key authentication interceptor
pub async fn phoenix_auth_interceptor(
    req: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let path = req.uri().path();
    
    // Skip auth for health and root endpoints
    if path == "/" || path == "/health" {
        return Ok(next.run(req).await);
    }
    
    // Extract API key from headers
    let api_key = match extract_api_key(req.headers()) {
        Some(key) => key,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiKeyError {
                    error: format!("Missing {} header", PHOENIX_API_KEY_HEADER),
                    code: 401,
                }),
            ));
        }
    };
    
    // Validate API key
    if !is_valid_api_key(&api_key).await {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiKeyError {
                error: "Invalid API key".to_string(),
                code: 401,
            }),
        ));
    }
    
    // API key is valid, store it in request extensions for rate limiting
    let mut req = req;
    req.extensions_mut().insert(api_key);
    
    Ok(next.run(req).await)
}

/// Simple sliding window rate limiter
pub async fn rate_limit_interceptor(
    req: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let path = req.uri().path();
    
    // Skip rate limiting for health and root endpoints
    if path == "/" || path == "/health" {
        return Ok(next.run(req).await);
    }
    
    // Get API key from extensions (set by phoenix_auth_interceptor)
    let api_key = match req.extensions().get::<String>() {
        Some(key) => key.clone(),
        None => {
            // No API key, skip rate limiting (auth will handle it)
            return Ok(next.run(req).await);
        }
    };
    
    // Rate limit parameters
    const REQUESTS_PER_MINUTE: u32 = 100;
    const WINDOW_DURATION_SECS: u64 = 60;
    
    let now = std::time::Instant::now();
    let mut rate_limiter = RATE_LIMITER.write().await;
    
    let rate_info = rate_limiter.entry(api_key.clone()).or_insert_with(|| {
        RateLimitInfo {
            count: 0,
            window_start: now,
        }
    });
    
    // Check if we need to reset the window
    if now.duration_since(rate_info.window_start).as_secs() >= WINDOW_DURATION_SECS {
        rate_info.count = 0;
        rate_info.window_start = now;
    }
    
    // Check rate limit
    if rate_info.count >= REQUESTS_PER_MINUTE {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiKeyError {
                error: format!("Rate limit exceeded: {} requests per minute", REQUESTS_PER_MINUTE),
                code: 429,
            }),
        ));
    }
    
    // Increment counter
    rate_info.count += 1;
    
    // Continue to next middleware
    drop(rate_limiter); // Release lock before proceeding
    Ok(next.run(req).await)
}