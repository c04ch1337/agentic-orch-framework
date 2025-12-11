// Rate Limiting Module using simple in-memory sliding window
// Replaces previous tower-governor based implementation with a
// dependency-free approach while preserving the public API.

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// Per-key rate limit tracking
#[derive(Clone)]
struct RateLimitInfo {
    count: u32,
    window_start: Instant,
}

// Global map of API key -> rate info
static RATE_LIMITERS: Lazy<Arc<RwLock<HashMap<String, RateLimitInfo>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

#[derive(Debug, Serialize)]
pub struct RateLimitError {
    pub error: String,
    pub code: u16,
    pub retry_after_seconds: u64,
}

// Extract API key from headers (same header as before)
fn extract_api_key_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-PHOENIX-API-KEY")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

// Simple sliding window based rate limiter
async fn check_and_update_rate_limit(api_key: &str) -> Result<(), RateLimitError> {
    const REQUESTS_PER_MINUTE: u32 = 100;
    const WINDOW_DURATION_SECS: u64 = 60;

    let now = Instant::now();
    let mut map = RATE_LIMITERS.write().await;

    let entry = map
        .entry(api_key.to_string())
        .or_insert_with(|| RateLimitInfo {
            count: 0,
            window_start: now,
        });

    // Reset window if expired
    if now.duration_since(entry.window_start).as_secs() >= WINDOW_DURATION_SECS {
        entry.count = 0;
        entry.window_start = now;
    }

    if entry.count >= REQUESTS_PER_MINUTE {
        return Err(RateLimitError {
            error: format!(
                "Rate limit exceeded: {} requests per minute per API key",
                REQUESTS_PER_MINUTE
            ),
            code: StatusCode::TOO_MANY_REQUESTS.as_u16(),
            retry_after_seconds: WINDOW_DURATION_SECS,
        });
    }

    entry.count += 1;
    Ok(())
}

// Public middleware function used by lib.rs
pub async fn tower_governor_rate_limiter(
    req: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let path = req.uri().path().to_string();

    // Skip rate limiting for health and root endpoints
    if path == "/" || path == "/health" {
        return Ok(next.run(req).await);
    }

    let api_key = extract_api_key_from_headers(req.headers())
        .or_else(|| req.extensions().get::<String>().cloned())
        .unwrap_or_else(|| "anonymous".to_string());

    if let Err(err) = check_and_update_rate_limit(&api_key).await {
        return Err((StatusCode::TOO_MANY_REQUESTS, Json(err)));
    }

    Ok(next.run(req).await)
}

// Update rate limit for a specific API key (for future extension; currently just resets)
pub async fn update_rate_limit(api_key: &str, _requests_per_minute: u32) {
    let mut map = RATE_LIMITERS.write().await;
    if let Some(info) = map.get_mut(api_key) {
        info.count = 0;
        info.window_start = Instant::now();
    }
}

// Get current rate limit stats for an API key
pub async fn get_rate_limit_stats(api_key: &str) -> Option<(u32, Duration)> {
    const WINDOW_DURATION_SECS: u64 = 60;
    let map = RATE_LIMITERS.read().await;
    map.get(api_key)
        .map(|info| (info.count, Duration::from_secs(WINDOW_DURATION_SECS)))
}
