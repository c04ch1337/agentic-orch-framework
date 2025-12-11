//! # Centralized Error Reporting
//!
//! This module provides centralized error reporting and monitoring functionality
//! for collecting errors across services and aggregating them for analysis.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, Mutex};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time;
use metrics::{counter, gauge};
use crate::types::{Error, ErrorKind, Result, Severity};

// Static flag to track if error reporting has been initialized
static REPORTING_INITIALIZED: AtomicBool = AtomicBool::new(false);

// Global error reporter instance
static mut ERROR_REPORTER: Option<Arc<ErrorReporter>> = None;

/// Configuration for error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReporterConfig {
    /// The endpoint to send error reports to (if any)
    pub report_endpoint: Option<String>,
    /// Service name identifier
    pub service_name: String,
    /// Environment (production, staging, development)
    pub environment: String,
    /// Maximum errors to batch before sending
    pub batch_size: usize,
    /// Maximum time to wait before sending a partial batch
    pub flush_interval_secs: u64,
    /// Whether to record error metrics
    pub record_metrics: bool,
    /// Number of recent errors to keep in memory
    pub in_memory_limit: usize,
    /// Maximum number of errors to send per second (rate limiting)
    pub rate_limit: usize,
    /// Authentication token for the reporting endpoint
    pub auth_token: Option<String>,
    /// Custom tags to add to all error reports
    pub tags: std::collections::HashMap<String, String>,
}

impl Default for ReporterConfig {
    fn default() -> Self {
        Self {
            report_endpoint: None,
            service_name: "unknown-service".to_string(),
            environment: "development".to_string(),
            batch_size: 10,
            flush_interval_secs: 5,
            record_metrics: true,
            in_memory_limit: 100,
            rate_limit: 50,
            auth_token: None,
            tags: std::collections::HashMap::new(),
            }
        }
    
    impl TryFrom&lt;config::Config&gt; for ReporterConfig {
    type Error = config::ConfigError;

    fn try_from(cfg: config::Config) -&gt; std::result::Result&lt;Self, Self::Error&gt; {
        // Start with defaults and override from config where present.
        let mut base = ReporterConfig::default();

        if let Ok(endpoint) = cfg.get::&lt;String&gt;("error_reporting.report_endpoint") {
            base.report_endpoint = Some(endpoint);
        }
        if let Ok(service_name) = cfg.get::&lt;String&gt;("error_reporting.service_name") {
            base.service_name = service_name;
        }
        if let Ok(environment) = cfg.get::&lt;String&gt;("error_reporting.environment") {
            base.environment = environment;
        }
        if let Ok(batch_size) = cfg.get::&lt;usize&gt;("error_reporting.batch_size") {
            base.batch_size = batch_size;
        }
        if let Ok(flush_interval) = cfg.get::&lt;u64&gt;("error_reporting.flush_interval_secs") {
            base.flush_interval_secs = flush_interval;
        }
        if let Ok(record_metrics) = cfg.get::&lt;bool&gt;("error_reporting.record_metrics") {
            base.record_metrics = record_metrics;
        }
        if let Ok(in_memory_limit) = cfg.get::&lt;usize&gt;("error_reporting.in_memory_limit") {
            base.in_memory_limit = in_memory_limit;
        }
        if let Ok(rate_limit) = cfg.get::&lt;usize&gt;("error_reporting.rate_limit") {
            base.rate_limit = rate_limit;
        }
        if let Ok(token) = cfg.get::&lt;String&gt;("error_reporting.auth_token") {
            base.auth_token = Some(token);
        }
        if let Ok(tags) = cfg.get::&lt;std::collections::HashMap&lt;String, String&gt;&gt;("error_reporting.tags") {
            base.tags = tags;
        }

        Ok(base)
    }
}

/// Central error reporter for collecting and sending error reports
#[derive(Debug)]
pub struct ErrorReporter {
    /// Configuration for the reporter
    config: ReporterConfig,
    /// Queue of errors waiting to be sent
    queue: Mutex<VecDeque<Error>>,
    /// Recent errors kept in memory for inspection
    recent_errors: RwLock<VecDeque<Error>>,
    /// HTTP client for sending reports
    client: reqwest::Client,
    /// Channel for sending errors to background worker
    sender: mpsc::Sender<Error>,
    /// Handle to the background worker task
    #[allow(dead_code)]
    worker_handle: Mutex<Option<JoinHandle<()>>>,
    /// Last flush time
    last_flush: Mutex<Instant>,
    /// Count of errors since last flush (for rate limiting)
    error_count: std::sync::atomic::AtomicUsize,
}

impl ErrorReporter {
    /// Creates a new error reporter with the given configuration
    pub fn new(config: ReporterConfig) -> Self {
        // Create a channel for sending errors to background worker
        let (sender, receiver) = mpsc::channel(100);
        
        // Create the error reporter
        let reporter = Self {
            config: config.clone(),
            queue: Mutex::new(VecDeque::with_capacity(config.batch_size)),
            recent_errors: RwLock::new(VecDeque::with_capacity(config.in_memory_limit)),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            sender,
            worker_handle: Mutex::new(None),
            last_flush: Mutex::new(Instant::now()),
            error_count: std::sync::atomic::AtomicUsize::new(0),
        };
        
        // Start the background worker only if we have an endpoint
        if config.report_endpoint.is_some() {
            let reporter_clone = Arc::new(reporter.clone());
            let handle = tokio::spawn(Self::run_background_worker(receiver, reporter_clone));
            
            // Store the handle
            let mut worker_handle = reporter.worker_handle.lock().unwrap();
            *worker_handle = Some(handle);
        }
        
        reporter
    }
    
    /// Runs the background worker that processes errors
    async fn run_background_worker(
        mut receiver: mpsc::Receiver<Error>,
        reporter: Arc<Self>,
    ) {
        let mut interval = time::interval(Duration::from_secs(reporter.config.flush_interval_secs));
        
        loop {
            tokio::select! {
                // Process incoming errors
                Some(error) = receiver.recv() => {
                    // Add to queue
                    let mut queue = reporter.queue.lock().unwrap();
                    queue.push_back(error);
                    
                    // Flush if batch size reached
                    if queue.len() >= reporter.config.batch_size {
                        let errors = queue.drain(..).collect::<Vec<_>>();
                        drop(queue); // Release lock before async call
                        
                        // Send batch asynchronously
                        let reporter_clone = Arc::clone(&reporter);
                        tokio::spawn(async move {
                            if let Err(e) = reporter_clone.send_batch(errors).await {
                                tracing::error!("Failed to send error batch: {}", e);
                            }
                        });
                        
                        // Update last flush time
                        *reporter.last_flush.lock().unwrap() = Instant::now();
                    }
                }
                
                // Periodic flush check
                _ = interval.tick() => {
                    let last_flush = *reporter.last_flush.lock().unwrap();
                    let elapsed = last_flush.elapsed().as_secs();
                    
                    // Check if we need to flush
                    let should_flush = {
                        let queue = reporter.queue.lock().unwrap();
                        !queue.is_empty() && elapsed >= reporter.config.flush_interval_secs
                    };
                    
                    if should_flush {
                        // Extract errors from queue
                        let errors = {
                            let mut queue = reporter.queue.lock().unwrap();
                            queue.drain(..).collect::<Vec<_>>()
                        };
                        
                        // Send batch asynchronously
                        let reporter_clone = Arc::clone(&reporter);
                        tokio::spawn(async move {
                            if let Err(e) = reporter_clone.send_batch(errors).await {
                                tracing::error!("Failed to flush error batch: {}", e);
                            }
                        });
                        
                        // Update last flush time
                        *reporter.last_flush.lock().unwrap() = Instant::now();
                    }
                    
                    // Reset error count every 30 seconds for rate limiting
                    if elapsed >= 30 {
                        reporter.error_count.store(0, Ordering::SeqCst);
                    }
                }
            }
        }
    }
    
    /// Sends a batch of errors to the reporting endpoint
    async fn send_batch(&self, errors: Vec<Error>) -> Result<()> {
        // Don't send if no endpoint configured
        let endpoint = match &self.config.report_endpoint {
            Some(url) => url,
            None => return Ok(()),
        };
        
        // Check rate limit
        let count = self.error_count.fetch_add(errors.len(), Ordering::SeqCst);
        if count >= self.config.rate_limit {
            tracing::warn!("Error reporting rate limit exceeded, dropping {} errors", errors.len());
            return Ok(());
        }
        
        // Prepare the payload
        let payload = serde_json::json!({
            "service": self.config.service_name,
            "environment": self.config.environment,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "tags": self.config.tags,
            "errors": errors,
        });
        
        // Build the request
        let mut request = self.client.post(endpoint)
            .json(&payload)
            .header("Content-Type", "application/json");
        
        // Add authentication if configured
        if let Some(token) = &self.config.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        // Send the request
        let response = request.send().await.map_err(|e| {
            Error::new(
                ErrorKind::Communication, 
                format!("Failed to send error report: {}", e)
            ).cause(e)
        })?;
        
        // Check response status
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "Error reading response".to_string());
            
            return Err(Error::new(
                ErrorKind::External,
                format!("Error reporting failed: HTTP {} - {}", status, body)
            ));
        }
        
        Ok(())
    }
     
    /// Reports an error to the centralized system
    pub fn report(&self, error: &Error) -> Result<()> {
        // Skip if already reported
        if error.reported {
            return Ok(());
        }
        
        // Record metrics if enabled
        if self.config.record_metrics {
            record_error_metrics(error);
        }
        
        // Add to recent errors list
        {
            let mut recent = self.recent_errors.write().unwrap();
            recent.push_back(error.clone());
            
            // Trim if over limit
            while recent.len() > self.config.in_memory_limit {
                recent.pop_front();
            }
        }
        
        // Try to send over channel (non-blocking)
        if let Some(_) = &self.config.report_endpoint {
            if let Err(_) = self.sender.try_send(error.clone()) {
                tracing::warn!("Error reporting channel full, dropping error");
            }
        }
        
        Ok(())
    }
    
    /// Gets recent errors from memory
    pub fn get_recent_errors(&self) -> Vec<Error> {
        let recent = self.recent_errors.read().unwrap();
        recent.iter().cloned().collect()
    }
    
    /// Gets errors by severity
    pub fn get_errors_by_severity(&self, severity: Severity) -> Vec<Error> {
        let recent = self.recent_errors.read().unwrap();
        recent.iter()
            .filter(|e| e.severity == severity)
            .cloned()
            .collect()
    }
    
    /// Gets errors by service
    pub fn get_errors_by_service(&self, service: &str) -> Vec<Error> {
        let recent = self.recent_errors.read().unwrap();
        recent.iter()
            .filter(|e| e.service.as_deref() == Some(service))
            .cloned()
            .collect()
    }

    /// Gets errors by kind
    pub fn get_errors_by_kind(&self, kind: &ErrorKind) -> Vec<Error> {
        let recent = self.recent_errors.read().unwrap();
        recent.iter()
            .filter(|e| e.kind == *kind)
            .cloned()
            .collect()
    }
}

// Clone implementation for the error reporter
impl Clone for ErrorReporter {
    fn clone(&self) -> Self {
        // Create a clone with an independent queue/recent-error buffer and a
        // fresh channel/sender. This is sufficient for the background worker
        // use-case and avoids cloning the internal RwLock directly.
        let (sender, _) = mpsc::channel(100);

        Self {
            config: self.config.clone(),
            queue: Mutex::new(VecDeque::with_capacity(self.config.batch_size)),
            recent_errors: RwLock::new(VecDeque::with_capacity(self.config.in_memory_limit)),
            client: self.client.clone(),
            sender,
            worker_handle: Mutex::new(None),
            last_flush: Mutex::new(Instant::now()),
            error_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

/// Initializes the error reporting system
pub fn init_reporter(config: Option<ReporterConfig>) -> Result<()> {
    // Don't re-initialize if already done
    if REPORTING_INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }
    
    let config = config.unwrap_or_default();
    
    // Create the error reporter
    let reporter = Arc::new(ErrorReporter::new(config.clone()));
    
    // Store globally
    unsafe {
        ERROR_REPORTER = Some(reporter);
    }
    
    // Mark as initialized
    REPORTING_INITIALIZED.store(true, Ordering::SeqCst);
    
    tracing::info!(
        service = %config.service_name,
        environment = %config.environment,
        "Error reporting initialized"
    );
    
    Ok(())
}

/// Reports an error to the centralized system
pub fn report_error(error: &Error) -> Result<()> {
    // Get the reporter
    let reporter = unsafe {
        match &ERROR_REPORTER {
            Some(reporter) => reporter.clone(),
            None => {
                // Auto-initialize with defaults if not done
                init_reporter(None)?;
                ERROR_REPORTER.as_ref().unwrap().clone()
            }
        }
    };
    
    // Report the error
    reporter.report(error)
}

/// Gets the global error reporter instance
pub fn get_reporter() -> Arc<ErrorReporter> {
    unsafe {
        match &ERROR_REPORTER {
            Some(reporter) => reporter.clone(),
            None => {
                // Auto-initialize with defaults if not done
                init_reporter(None).expect("Failed to initialize error reporter");
                ERROR_REPORTER.as_ref().unwrap().clone()
            }
        }
    }
}

/// Records metrics for an error
fn record_error_metrics(error: &Error) {
    // Counter for total errors
    counter!("errors.total", 1);
    
    // Counter by severity
    match error.severity {
        Severity::Fatal => counter!("errors.severity.fatal", 1),
        Severity::Critical => counter!("errors.severity.critical", 1),
        Severity::Major => counter!("errors.severity.major", 1),
        Severity::Minor => counter!("errors.severity.minor", 1),
        Severity::Info => counter!("errors.severity.info", 1),
    }
    
    // Counter by error kind
    match error.kind {
        ErrorKind::Validation => counter!("errors.kind.validation", 1),
        ErrorKind::Authentication => counter!("errors.kind.authentication", 1),
        ErrorKind::Communication => counter!("errors.kind.communication", 1),
        ErrorKind::External => counter!("errors.kind.external", 1),
        ErrorKind::Internal => counter!("errors.kind.internal", 1),
        ErrorKind::Timeout => counter!("errors.kind.timeout", 1),
        ErrorKind::Unavailable => counter!("errors.kind.unavailable", 1),
        ErrorKind::RateLimit => counter!("errors.kind.ratelimit", 1),
        _ => counter!("errors.kind.other", 1),
    }
    
    // Counter by service
    if let Some(service) = &error.service {
        let service_label = format!("errors.service.{}", service);
        counter!(service_label, 1);
    } else {
        counter!("errors.service.unknown", 1);
    }
    
    // Counter for transient errors
    if error.transient {
        counter!("errors.transient", 1);
    }
    
    // Gauge for recent errors
    gauge!("errors.recent", get_reporter().get_recent_errors().len() as f64);
}

/// Alert levels for monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    /// Informational alert
    Info,
    /// Warning alert
    Warning,
    /// Critical alert
    Critical,
    /// Emerge alert requiring immediate attention
    Emergency,
}

/// Sends an alert about errors or issues
pub fn send_alert(
    level: AlertLevel, 
    message: &str, 
    error: Option<&Error>,
    context: Option<serde_json::Value>,
) -> Result<()> {
    // Get the reporter
    let reporter = get_reporter();
    
    // Prepare the alert payload
    let mut payload = serde_json::json!({
        "level": format!("{:?}", level),
        "message": message,
        "service": reporter.config.service_name,
        "environment": reporter.config.environment,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    
    // Add error details if provided
    if let Some(err) = error {
        payload["error"] = serde_json::json!({
            "id": err.id.to_string(),
            "kind": format!("{}", err.kind),
            "message": err.message,
            "severity": format!("{}", err.severity),
        });
    }
    
    // Add context if provided
    if let Some(ctx) = context {
        payload["context"] = ctx;
    }
    
    // Log locally
    match level {
        AlertLevel::Info => tracing::info!(alert = true, %message, "Alert triggered"),
        AlertLevel::Warning => tracing::warn!(alert = true, %message, "Alert triggered"),
        AlertLevel::Critical => tracing::error!(alert = true, %message, "Alert triggered"),
        AlertLevel::Emergency => tracing::error!(alert = true, emergency = true, %message, "EMERGENCY ALERT TRIGGERED"),
    }
    
    // Send to reporting endpoint if configured (using alert-specific endpoint)
    if let Some(endpoint) = &reporter.config.report_endpoint {
        let alert_endpoint = format!("{}/alerts", endpoint);
        
        let mut request = reporter.client.post(&alert_endpoint)
            .json(&payload)
            .header("Content-Type", "application/json");
            
        if let Some(token) = &reporter.config.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        // Send asynchronously
        tokio::spawn(async move {
            if let Err(e) = request.send().await {
                tracing::error!("Failed to send alert: {}", e);
            }
        });
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Error, ErrorKind, Severity};
    
    #[tokio::test]
    async fn test_error_reporting() {
        // Initialize with a test config
        let config = ReporterConfig {
            service_name: "test-service".to_string(),
            environment: "test".to_string(),
            report_endpoint: None, // Don't actually send reports in tests
            in_memory_limit: 5,
            ..Default::default()
        };
        
        init_reporter(Some(config)).unwrap();
        
        // Report some errors
        let error1 = Error::new(ErrorKind::Validation, "Test error 1")
            .severity(Severity::Minor);
            
        let error2 = Error::new(ErrorKind::Timeout, "Test error 2")
            .severity(Severity::Critical);
            
        report_error(&error1).unwrap();
        report_error(&error2).unwrap();
        
        // Check recent errors
        let reporter = get_reporter();
        let recent = reporter.get_recent_errors();
        
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].message, "Test error 1");
        assert_eq!(recent[1].message, "Test error 2");
        
        // Check filtering by severity
        let critical = reporter.get_errors_by_severity(Severity::Critical);
        assert_eq!(critical.len(), 1);
        assert_eq!(critical[0].message, "Test error 2");
    }
}