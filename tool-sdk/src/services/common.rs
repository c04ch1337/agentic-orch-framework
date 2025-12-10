//! Common utilities for service clients
//!
//! This module provides shared functionality for all service clients.

use std::fmt;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;
use reqwest::{header, Client};
use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::error::{Result, ServiceError, ErrorContext};

/// Default user agent string
const DEFAULT_USER_AGENT: &str = "Phoenix-ORCH/0.1.0 (tool-sdk) rust/1.69.0";

/// UserAgent structure for identifying the client to upstream services
#[derive(Debug, Clone)]
pub struct UserAgent {
    /// Application name
    pub app_name: String,
    
    /// Version string
    pub version: String,
    
    /// Optional extra info
    pub extra: Option<String>,
}

impl Default for UserAgent {
    fn default() -> Self {
        Self {
            app_name: "Phoenix-ORCH".to_string(),
            version: "0.1.0".to_string(),
            extra: Some("tool-sdk".to_string()),
        }
    }
}

impl fmt::Display for UserAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.app_name, self.version)?;
        
        if let Some(ref extra) = self.extra {
            write!(f, " ({})", extra)?;
        }
        
        // Add Rust version
        write!(f, " rust/{}", env!("CARGO_PKG_RUST_VERSION", "1.69.0"))
    }
}

/// Shared metrics collection for service clients
#[derive(Debug, Default)]
struct ClientMetrics {
    /// Total requests made
    request_count: AtomicU64,
    
    /// Total successful responses
    success_count: AtomicU64,
    
    /// Total errors
    error_count: AtomicU64,
    
    /// Total bytes sent
    bytes_sent: AtomicU64,
    
    /// Total bytes received
    bytes_received: AtomicU64,
    
    /// Service-specific metrics
    service_metrics: Mutex<HashMap<String, HashMap<String, String>>>,
}

impl ClientMetrics {
    /// Record a request
    fn record_request(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record a successful response
    fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record an error
    fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record bytes sent
    fn record_bytes_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }
    
    /// Record bytes received
    fn record_bytes_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }
    
    /// Get metrics for a specific service
    fn get_service_metrics(&self, service: &str) -> HashMap<String, String> {
        let metrics = self.service_metrics.lock().unwrap();
        metrics.get(service)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Record a service-specific metric
    fn record_service_metric(&self, service: &str, key: &str, value: String) {
        let mut metrics = self.service_metrics.lock().unwrap();
        let service_metrics = metrics.entry(service.to_string()).or_insert_with(HashMap::new);
        service_metrics.insert(key.to_string(), value);
    }
    
    /// Get all metrics as a map
    fn as_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        
        map.insert("request_count".to_string(), self.request_count.load(Ordering::Relaxed).to_string());
        map.insert("success_count".to_string(), self.success_count.load(Ordering::Relaxed).to_string());
        map.insert("error_count".to_string(), self.error_count.load(Ordering::Relaxed).to_string());
        map.insert("bytes_sent".to_string(), self.bytes_sent.load(Ordering::Relaxed).to_string());
        map.insert("bytes_received".to_string(), self.bytes_received.load(Ordering::Relaxed).to_string());
        
        map
    }
}

/// Global client metrics for all services
static GLOBAL_METRICS: Lazy<ClientMetrics> = Lazy::new(ClientMetrics::default);

/// Build a standard HTTP client with default settings
pub fn build_http_client(
    user_agent: Option<UserAgent>,
    timeout: Option<Duration>,
) -> Result<Client> {
    let mut headers = header::HeaderMap::new();
    let ua = user_agent.unwrap_or_default().to_string();
    
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_str(&ua).map_err(|e| {
            ServiceError::configuration(format!("Invalid user agent: {}", e))
        })?,
    );
    
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(timeout.unwrap_or_else(|| Duration::from_secs(30)))
        .gzip(true)
        .build()
        .map_err(|e| {
            ServiceError::configuration(format!("Failed to build HTTP client: {}", e))
        })?;
    
    Ok(client)
}

/// Create error context for HTTP requests
pub fn create_error_context(
    service_name: &str,
    status: Option<reqwest::StatusCode>,
) -> ErrorContext {
    let mut context = ErrorContext::for_service(service_name);
    
    if let Some(status_code) = status {
        context = context.status_code(status_code.as_u16());
    }
    
    context
}

/// Parse error response from HTTP response
pub async fn parse_error_response(
    service_name: &str,
    response: reqwest::Response,
) -> ServiceError {
    let status = response.status();
    let status_code = status.as_u16();
    let mut context = create_error_context(service_name, Some(status));
    
    // Try to get the response body
    let body = match response.text().await {
        Ok(body) => body,
        Err(e) => format!("Failed to read error response: {}", e),
    };
    
    // Map to appropriate error type based on status
    crate::error::mapping::map_http_error(status, &body, &mut context)
        .with_context(context)
}

/// Record client metrics for a request
pub fn record_request_metrics(
    service: &str,
    endpoint: &str,
    start_time: Instant,
    status: u16,
    is_success: bool,
    bytes_sent: Option<u64>,
    bytes_received: Option<u64>,
) {
    GLOBAL_METRICS.record_request();
    
    if is_success {
        GLOBAL_METRICS.record_success();
    } else {
        GLOBAL_METRICS.record_error();
    }
    
    if let Some(bytes) = bytes_sent {
        GLOBAL_METRICS.record_bytes_sent(bytes);
    }
    
    if let Some(bytes) = bytes_received {
        GLOBAL_METRICS.record_bytes_received(bytes);
    }
    
    // Record latency
    let duration = start_time.elapsed();
    GLOBAL_METRICS.record_service_metric(
        service,
        &format!("latency_{}", endpoint),
        format!("{:.2}ms", duration.as_secs_f64() * 1000.0),
    );
    
    // Record status code distribution
    GLOBAL_METRICS.record_service_metric(
        service,
        &format!("status_{}_count", status),
        (1).to_string(),
    );
}

/// Get metrics for all clients
pub fn get_global_metrics() -> HashMap<String, String> {
    GLOBAL_METRICS.as_map()
}

/// Get metrics for a specific service
pub fn get_service_metrics(service: &str) -> HashMap<String, String> {
    GLOBAL_METRICS.get_service_metrics(service)
}

/// Reset all metrics
pub fn reset_metrics() {
    GLOBAL_METRICS.request_count.store(0, Ordering::Relaxed);
    GLOBAL_METRICS.success_count.store(0, Ordering::Relaxed);
    GLOBAL_METRICS.error_count.store(0, Ordering::Relaxed);
    GLOBAL_METRICS.bytes_sent.store(0, Ordering::Relaxed);
    GLOBAL_METRICS.bytes_received.store(0, Ordering::Relaxed);
    
    let mut metrics = GLOBAL_METRICS.service_metrics.lock().unwrap();
    metrics.clear();
}