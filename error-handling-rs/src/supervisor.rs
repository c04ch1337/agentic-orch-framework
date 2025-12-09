//! # System Supervisor
//!
//! This module provides system-wide resilience features including:
//! - Graceful shutdown hooks for proper termination
//! - Process supervision with automatic restart
//! - Health check management and propagation
//! - Resource isolation
//! - System lifecycle management

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::{Duration, Instant};
use std::future::Future;
use std::pin::Pin;

use serde::{Serialize, Deserialize};
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::{timeout, interval, sleep};
use tracing::{trace, debug, info, warn, error};
use metrics::{counter, gauge, histogram};

use crate::types::{Error, Result, ErrorKind, Severity};
use crate::fallback::{Bulkhead, DegradedMode, DegradedSeverity};
use crate::circuit_breaker::{CircuitBreaker, CircuitState};
use crate::reporting::{send_alert, AlertLevel};

/// Health status of a system component or service
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Service is healthy and fully operational
    Healthy,
    /// Service is degraded but operational
    Degraded,
    /// Service is unhealthy but still responding
    Unhealthy,
    /// Service is completely unavailable
    Unavailable,
    /// Service is starting up, not ready yet
    Starting,
    /// Service is shutting down
    ShuttingDown,
    /// Service status is unknown
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "HEALTHY"),
            HealthStatus::Degraded => write!(f, "DEGRADED"),
            HealthStatus::Unhealthy => write!(f, "UNHEALTHY"),
            HealthStatus::Unavailable => write!(f, "UNAVAILABLE"),
            HealthStatus::Starting => write!(f, "STARTING"),
            HealthStatus::ShuttingDown => write!(f, "SHUTTING_DOWN"),
            HealthStatus::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Health check details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthInfo {
    /// Overall status
    pub status: HealthStatus,
    /// Service name
    pub service: String,
    /// Version information
    pub version: Option<String>,
    /// Timestamp when health was checked
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Memory usage in bytes
    pub memory_usage: Option<u64>,
    /// CPU usage percentage
    pub cpu_usage: Option<f64>,
    /// Any operational message
    pub message: Option<String>,
    /// Detailed checks
    pub checks: HashMap<String, SubHealthCheck>,
    /// Dependency health
    pub dependencies: HashMap<String, DependencyHealth>,
    /// Last error encountered
    pub last_error: Option<String>,
    /// Readiness status for serving requests
    pub ready: bool,
}

impl Default for HealthInfo {
    fn default() -> Self {
        Self {
            status: HealthStatus::Unknown,
            service: "unknown".to_string(),
            version: None,
            timestamp: chrono::Utc::now(),
            uptime_seconds: 0,
            memory_usage: None,
            cpu_usage: None,
            message: None,
            checks: HashMap::new(),
            dependencies: HashMap::new(),
            last_error: None,
            ready: false,
        }
    }
}

/// Result of an individual health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubHealthCheck {
    /// Name of the check
    pub name: String,
    /// Status of the check
    pub status: HealthStatus,
    /// Optional message
    pub message: Option<String>,
    /// Timestamp of the check
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Last successful check
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    /// Performance metrics
    pub metrics: Option<HashMap<String, serde_json::Value>>,
}

/// Health of a dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyHealth {
    /// Name of the dependency
    pub name: String,
    /// Status of the dependency
    pub status: HealthStatus,
    /// Whether the dependency is required
    pub required: bool,
    /// Whether a circuit breaker is open
    pub circuit_open: bool,
    /// Last successful connection
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    /// Last connection attempt
    pub last_attempt: Option<chrono::DateTime<chrono::Utc>>,
}

/// Health check function signature
pub type HealthCheckFn = Box<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<SubHealthCheck>> + Send>> + Send + Sync,
>;

/// Configuration for health check behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Interval between health checks
    pub check_interval: Duration,
    /// Timeout for health checks
    pub check_timeout: Duration,
    /// How many failures before marking unhealthy
    pub failure_threshold: usize,
    /// Backoff multiplier for failing checks
    pub backoff_multiplier: f64,
    /// Maximum backoff time
    pub max_backoff: Duration,
    /// Whether to degrade automatically on health failures
    pub auto_degrade: bool,
    /// Whether to allow manual status override
    pub allow_override: bool,
    /// Path for writing health status file
    pub status_file_path: Option<String>,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            check_timeout: Duration::from_secs(5),
            failure_threshold: 3,
            backoff_multiplier: 2.0,
            max_backoff: Duration::from_secs(300),
            auto_degrade: true,
            allow_override: true,
            status_file_path: None,
        }
    }
}

/// Health check manager
#[derive(Debug)]
pub struct HealthManager {
    /// Service name
    service_name: String,
    /// Version information
    version: Option<String>,
    /// Start time
    start_time: Instant,
    /// Current health info
    health: Arc<RwLock<HealthInfo>>,
    /// Registered health checks
    checks: Arc<RwLock<HashMap<String, HealthCheckFn>>>,
    /// Health check configuration
    config: HealthCheckConfig,
    /// Status change publisher
    status_change_tx: watch::Sender<HealthStatus>,
    /// Status change receiver
    status_change_rx: watch::Receiver<HealthStatus>,
    /// Manual override status
    override_status: Arc<Mutex<Option<HealthStatus>>>,
    /// Consecutive failures by check
    failures: Arc<RwLock<HashMap<String, usize>>>,
    /// Health check worker handle
    worker_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Shutdown signal
    shutdown_tx: broadcast::Sender<()>,
    /// Degraded mode manager
    degraded_mode: Arc<DegradedMode>,
}

impl HealthManager {
    /// Creates a new health manager
    pub fn new<S: Into<String>>(service_name: S, version: Option<String>, config: Option<HealthCheckConfig>) -> Self {
        let config = config.unwrap_or_default();
        let service_name = service_name.into();
        
        // Create channels
        let (status_tx, status_rx) = watch::channel(HealthStatus::Starting);
        let (shutdown_tx, _) = broadcast::channel(1);
        
        // Create initial health info
        let health_info = HealthInfo {
            status: HealthStatus::Starting,
            service: service_name.clone(),
            version: version.clone(),
            timestamp: chrono::Utc::now(),
            uptime_seconds: 0,
            checks: HashMap::new(),
            dependencies: HashMap::new(),
            ready: false,
            ..Default::default()
        };
        
        Self {
            service_name,
            version,
            start_time: Instant::now(),
            health: Arc::new(RwLock::new(health_info)),
            checks: Arc::new(RwLock::new(HashMap::new())),
            config,
            status_change_tx: status_tx,
            status_change_rx: status_rx,
            override_status: Arc::new(Mutex::new(None)),
            failures: Arc::new(RwLock::new(HashMap::new())),
            worker_handle: Arc::new(Mutex::new(None)),
            shutdown_tx,
            degraded_mode: Arc::new(DegradedMode::new()),
        }
    }
    
    /// Starts the health check monitoring
    pub fn start(&self) -> Result<()> {
        let mut worker_handle = self.worker_handle.lock().unwrap();
        
        if worker_handle.is_some() {
            return Err(Error::new(
                ErrorKind::Initialization,
                "Health check worker is already running".to_string()
            ));
        }
        
        // Clone references for the worker
        let health = self.health.clone();
        let checks = self.checks.clone();
        let config = self.config.clone();
        let service_name = self.service_name.clone();
        let version = self.version.clone();
        let start_time = self.start_time;
        let failures = self.failures.clone();
        let status_tx = self.status_change_tx.clone();
        let override_status = self.override_status.clone();
        let degraded_mode = self.degraded_mode.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        
        // Start the worker
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.check_interval);
            
            // Set initial status
            let mut current_status = HealthStatus::Starting;
            let startup_grace = Duration::from_secs(5);
            let startup_deadline = Instant::now() + startup_grace;
            
            // Update health info with starting status
            {
                let mut health_info = health.write().unwrap();
                health_info.status = HealthStatus::Starting;
                health_info.timestamp = chrono::Utc::now();
                health_info.uptime_seconds = start_time.elapsed().as_secs();
                health_info.ready = false;
                
                if let Some(path) = &config.status_file_path {
                    // Write health status to file
                    if let Ok(json) = serde_json::to_string(&health_info) {
                        let _ = std::fs::write(path, json);
                    }
                }
            }
            
            loop {
                tokio::select! {
                    // Handle shutdown signal
                    _ = shutdown_rx.recv() => {
                        info!(service = %service_name, "Health check worker shutting down");
                        
                        // Set shutting down status
                        let mut health_info = health.write().unwrap();
                        health_info.status = HealthStatus::ShuttingDown;
                        health_info.timestamp = chrono::Utc::now();
                        health_info.ready = false;
                        
                        // Notify status change
                        let _ = status_tx.send(HealthStatus::ShuttingDown);
                        
                        // Write final status file
                        if let Some(path) = &config.status_file_path {
                            if let Ok(json) = serde_json::to_string(&health_info) {
                                let _ = std::fs::write(path, json);
                            }
                        }
                        
                        break;
                    }
                    
                    // Handle interval tick
                    _ = interval.tick() => {
                        // Check for manual override
                        let override_status = {
                            let override_guard = override_status.lock().unwrap();
                            *override_guard
                        };
                        
                        if let Some(status) = override_status {
                            // Use manual override status
                            if current_status != status {
                                info!(
                                    service = %service_name,
                                    previous = %current_status,
                                    current = %status,
                                    "Health status manually overridden"
                                );
                                
                                current_status = status;
                                let _ = status_tx.send(status);
                            }
                            
                            // Update health info with override status
                            let mut health_info = health.write().unwrap();
                            health_info.status = status;
                            health_info.timestamp = chrono::Utc::now();
                            health_info.uptime_seconds = start_time.elapsed().as_secs();
                            health_info.message = Some("Status manually overridden".to_string());
                            
                            if status != HealthStatus::Starting && status != HealthStatus::ShuttingDown {
                                health_info.ready = status == HealthStatus::Healthy || status == HealthStatus::Degraded;
                            } else {
                                health_info.ready = false;
                            }
                            
                            // Write health status to file
                            if let Some(path) = &config.status_file_path {
                                if let Ok(json) = serde_json::to_string(&health_info) {
                                    let _ = std::fs::write(path, json);
                                }
                            }
                            
                            continue;
                        }
                        
                        // Check startup grace period
                        if current_status == HealthStatus::Starting && Instant::now() > startup_deadline {
                            debug!(service = %service_name, "Startup grace period ended, running health checks");
                        }
                        
                        // Run all health checks
                        let checks_snapshot = {
                            let checks_guard = checks.read().unwrap();
                            checks_guard.clone()
                        };
                        
                        if checks_snapshot.is_empty() {
                            debug!(service = %service_name, "No health checks registered");
                            
                            // If no checks and past startup grace, assume healthy
                            if current_status == HealthStatus::Starting && Instant::now() > startup_deadline {
                                current_status = HealthStatus::Healthy;
                                let _ = status_tx.send(HealthStatus::Healthy);
                                
                                info!(service = %service_name, "Assuming healthy status (no checks registered)");
                            }
                            
                            // Update basic health info
                            let mut health_info = health.write().unwrap();
                            health_info.status = current_status;
                            health_info.timestamp = chrono::Utc::now();
                            health_info.uptime_seconds = start_time.elapsed().as_secs();
                            
                            if current_status != HealthStatus::Starting && current_status != HealthStatus::ShuttingDown {
                                health_info.ready = current_status == HealthStatus::Healthy ||
                                                  current_status == HealthStatus::Degraded;
                            }
                            
                            continue;
                        }
                        
                        let mut check_results = HashMap::new();
                        let mut overall_status = HealthStatus::Healthy;
                        let mut had_error = false;
                        
                        // Execute all health checks
                        for (name, check_fn) in checks_snapshot.iter() {
                            debug!(service = %service_name, check = %name, "Running health check");
                            
                            let start = Instant::now();
                            let check_future = check_fn();
                            
                            // Run with timeout
                            match timeout(config.check_timeout, check_future).await {
                                Ok(result) => match result {
                                    Ok(check) => {
                                        // Update failures count (reset to 0 on success)
                                        {
                                            let mut failures_guard = failures.write().unwrap();
                                            failures_guard.insert(name.clone(), 0);
                                        }
                                        
                                        // Record successful check
                                        counter!(&format!("health.{}.check.{}.success", service_name, name), 1);
                                        histogram!(
                                            &format!("health.{}.check.{}.duration_ms", service_name, name),
                                            start.elapsed().as_millis() as f64
                                        );
                                        
                                        // Update overall status based on check
                                        if check.status == HealthStatus::Unhealthy || check.status == HealthStatus::Unavailable {
                                            // Any unhealthy check makes system unhealthy
                                            if overall_status == HealthStatus::Healthy || overall_status == HealthStatus::Degraded {
                                                overall_status = HealthStatus::Unhealthy;
                                            }
                                            
                                            warn!(
                                                service = %service_name,
                                                check = %name,
                                                status = %check.status,
                                                message = ?check.message,
                                                "Unhealthy check detected"
                                            );
                                            
                                            had_error = true;
                                        } else if check.status == HealthStatus::Degraded && overall_status == HealthStatus::Healthy {
                                            // Any degraded check makes system degraded (unless already unhealthy)
                                            overall_status = HealthStatus::Degraded;
                                            
                                            info!(
                                                service = %service_name,
                                                check = %name,
                                                message = ?check.message,
                                                "Degraded check detected"
                                            );
                                        }
                                        
                                        check_results.insert(name.clone(), check);
                                    },
                                    Err(e) => {
                                        // Update failures count
                                        let failure_count = {
                                            let mut failures_guard = failures.write().unwrap();
                                            let count = failures_guard.entry(name.clone()).or_insert(0);
                                            *count += 1;
                                            *count
                                        };
                                        
                                        // Record failed check
                                        counter!(&format!("health.{}.check.{}.failure", service_name, name), 1);
                                        
                                        // Check if we've reached failure threshold
                                        if failure_count >= config.failure_threshold {
                                            // This check is considered failed
                                            if overall_status == HealthStatus::Healthy || overall_status == HealthStatus::Degraded {
                                                overall_status = HealthStatus::Unhealthy;
                                            }
                                            
                                            error!(
                                                service = %service_name,
                                                check = %name,
                                                failures = %failure_count,
                                                threshold = %config.failure_threshold,
                                                error = %e,
                                                "Health check failed too many times"
                                            );
                                            
                                            had_error = true;
                                        } else {
                                            // Not yet at failure threshold
                                            warn!(
                                                service = %service_name,
                                                check = %name,
                                                failures = %failure_count,
                                                threshold = %config.failure_threshold,
                                                error = %e,
                                                "Health check failed"
                                            );
                                            
                                            // Still consider system degraded if any check is failing
                                            if overall_status == HealthStatus::Healthy {
                                                overall_status = HealthStatus::Degraded;
                                            }
                                        }
                                        
                                        // Create failed check
                                        let failed_check = SubHealthCheck {
                                            name: name.clone(),
                                            status: if failure_count >= config.failure_threshold {
                                                HealthStatus::Unhealthy
                                            } else {
                                                HealthStatus::Degraded
                                            },
                                            message: Some(format!("Check failed: {}", e)),
                                            timestamp: chrono::Utc::now(),
                                            last_success: None,
                                            metrics: None,
                                        };
                                        
                                        check_results.insert(name.clone(), failed_check);
                                    }
                                },
                                Err(_) => {
                                    // Timeout occurred
                                    counter!(&format!("health.{}.check.{}.timeout", service_name, name), 1);
                                    
                                    // Update failures count
                                    let failure_count = {
                                        let mut failures_guard = failures.write().unwrap();
                                        let count = failures_guard.entry(name.clone()).or_insert(0);
                                        *count += 1;
                                        *count
                                    };
                                    
                                    // Timeout is treated more severely
                                    if failure_count >= config.failure_threshold {
                                        // Mark as unavailable on timeouts
                                        overall_status = HealthStatus::Unavailable;
                                        
                                        error!(
                                            service = %service_name,
                                            check = %name,
                                            failures = %failure_count,
                                            timeout_ms = %config.check_timeout.as_millis(),
                                            "Health check timed out too many times"
                                        );
                                    } else {
                                        // Not yet at threshold
                                        warn!(
                                            service = %service_name,
                                            check = %name,
                                            failures = %failure_count,
                                            timeout_ms = %config.check_timeout.as_millis(),
                                            "Health check timed out"
                                        );
                                        
                                        if overall_status == HealthStatus::Healthy {
                                            overall_status = HealthStatus::Degraded;
                                        }
                                    }
                                    
                                    had_error = true;
                                    
                                    // Create timeout check
                                    let timeout_check = SubHealthCheck {
                                        name: name.clone(),
                                        status: if failure_count >= config.failure_threshold {
                                            HealthStatus::Unavailable
                                        } else {
                                            HealthStatus::Degraded
                                        },
                                        message: Some(format!("Check timed out after {}ms", config.check_timeout.as_millis())),
                                        timestamp: chrono::Utc::now(),
                                        last_success: None,
                                        metrics: None,
                                    };
                                    
                                    check_results.insert(name.clone(), timeout_check);
                                }
                            }
                        }
                        
                        // Handle status change
                        if current_status != overall_status {
                            // Special case for startup
                            if current_status == HealthStatus::Starting {
                                if Instant::now() > startup_deadline {
                                    info!(
                                        service = %service_name,
                                        status = %overall_status,
                                        "Service health initialized"
                                    );
                                    
                                    current_status = overall_status;
                                    let _ = status_tx.send(overall_status);
                                }
                            } else {
                                // Normal status change
                                info!(
                                    service = %service_name,
                                    previous = %current_status,
                                    current = %overall_status,
                                    "Health status changed"
                                );
                                
                                // Handle entering degraded mode
                                if config.auto_degrade && 
                                   overall_status == HealthStatus::Degraded && 
                                   current_status == HealthStatus::Healthy {
                                    degraded_mode.activate(
                                        format!("{}_health", service_name),
                                        "Health checks degraded",
                                        DegradedSeverity::Minor
                                    );
                                } 
                                // Handle entering unhealthy mode
                                else if config.auto_degrade && 
                                        overall_status == HealthStatus::Unhealthy && 
                                        (current_status == HealthStatus::Healthy || current_status == HealthStatus::Degraded) {
                                    degraded_mode.activate(
                                        format!("{}_health", service_name),
                                        "Health checks unhealthy",
                                        DegradedSeverity::Severe
                                    );
                                    
                                    // Send alert for unhealthy status
                                    let _ = send_alert(
                                        AlertLevel::Critical, 
                                        &format!("Service {} health status is UNHEALTHY", service_name),
                                        None,
                                        None
                                    );
                                }
                                // Handle entering unavailable mode
                                else if config.auto_degrade && 
                                        overall_status == HealthStatus::Unavailable {
                                    degraded_mode.activate(
                                        format!("{}_health", service_name),
                                        "Service unavailable",
                                        DegradedSeverity::Critical
                                    );
                                    
                                    // Send alert for unavailable status
                                    let _ = send_alert(
                                        AlertLevel::Emergency, 
                                        &format!("Service {} health status is UNAVAILABLE", service_name),
                                        None,
                                        None
                                    );
                                }
                                // Handle recovery
                                else if config.auto_degrade && 
                                        overall_status == HealthStatus::Healthy && 
                                        (current_status == HealthStatus::Degraded || 
                                         current_status == HealthStatus::Unhealthy ||
                                         current_status == HealthStatus::Unavailable) {
                                    degraded_mode.deactivate(format!("{}_health", service_name));
                                    
                                    info!(
                                        service = %service_name,
                                        "Service health recovered, deactivated degraded mode"
                                    );
                                }
                                
                                current_status = overall_status;
                                let _ = status_tx.send(overall_status);
                            }
                        }
                        
                        // Update health info with check results
                        let mut health_info = health.write().unwrap();
                        health_info.status = current_status;
                        health_info.timestamp = chrono::Utc::now();
                        health_info.uptime_seconds = start_time.elapsed().as_secs();
                        health_info.checks = check_results;
                        health_info.last_error = if had_error {
                            Some("One or more health checks failed".to_string())
                        } else {
                            None
                        };
                        
                        if current_status != HealthStatus::Starting && current_status != HealthStatus::ShuttingDown {
                            health_info.ready = current_status == HealthStatus::Healthy || 
                                               current_status == HealthStatus::Degraded;
                        } else {
                            health_info.ready = false;
                        }
                        
                        // Update metrics
                        gauge!(&format!("health.{}.status", service_name), match current_status {
                            HealthStatus::Healthy => 0.0,
                            HealthStatus::Degraded => 1.0,
                            HealthStatus::Unhealthy => 2.0,
                            HealthStatus::Unavailable => 3.0,
                            HealthStatus::Starting => 4.0,
                            HealthStatus::ShuttingDown => 5.0,
                            HealthStatus::Unknown => 6.0,
                        });
                        
                        gauge!(&format!("health.{}.uptime", service_name), start_time.elapsed().as_secs() as f64);
                        gauge!(&format!("health.{}.ready", service_name), if health_info.ready { 1.0 } else { 0.0 });
                        
                        // Write health status to file
                        if let Some(path) = &config.status_file_path {
                            if let Ok(json) = serde_json::to_string(&health_info) {
                                let _ = std::fs::write(path, json);
                            }
                        }
                    }
                }
            }
        });
        
        // Store the handle
        *worker_handle = Some(handle);
        
        // Log startup
        info!(
            service = %self.service_name,
            version = ?self.version,
            check_interval_secs = %self.config.check_interval.as_secs(),
            "Health check monitoring started"
        );
        
        Ok(())
    }
    
    /// Stop the health check monitoring
    pub fn stop(&self) {
        // Send shutdown signal
        let _ = self.shutdown_tx.send(());
        
        // Log shutdown
        info!(
            service = %self.service_name,
            "Health check monitoring stopped"
        );
    }
    
    /// Register a health check function
    pub fn register<S, F, Fut>(&self, name: S, check: F) -> Result<()>
    where
        S: Into<String>,
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<SubHealthCheck>> + Send + 'static,
    {
        let name = name.into();
        let mut checks = self.checks.write().unwrap();
        
        if checks.contains_key(&name) {
            return Err(Error::new(
                ErrorKind::Validation,
                format!("Health check '{}' already registered", name)
            ));
        }
        
        // Wrap the check function
        let check_fn = Box::new(move || Box::pin(check()) as Pin<Box<dyn Future<Output = Result<SubHealthCheck>> + Send>>);
        
        // Add to registry
        checks.insert(name.clone(), check_fn);
        
        // Log registration
        debug!(
            service = %self.service_name,
            check = %name,
            total_checks = %checks.len(),
            "Health check registered"
        );
        
        Ok(())
    }
    
    /// Add a dependency health status
    pub fn add_dependency<S1, S2>(&self, name: S1, status: HealthStatus, required: bool, circuit_open: bool, host: Option<S2>) -> Result<()>
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        let name = name.into();
        let mut health = self.health.write().unwrap();
        
        let dependency = DependencyHealth {
            name: name.clone(),
            status,
            required,
            circuit_open,
            last_success: None,
            last_attempt: Some(chrono::Utc::now()),
        };
        
        health.dependencies.insert(name.clone(), dependency);
        
        // If a required dependency is unhealthy or unavailable, mark the service as degraded
        if required && (status == HealthStatus::Unhealthy || status == HealthStatus::Unavailable) {
            if health.status == HealthStatus::Healthy {
                health.status = HealthStatus::Degraded;
                let _ = self.status_change_tx.send(HealthStatus::Degraded);
                
                warn!(
                    service = %self.service_name,
                    dependency = %name,
                    status = %status,
                    "Service degraded due to dependency health"
                );
                
                // Activate degraded mode
                if self.config.auto_degrade {
                    self.degraded_mode.activate(
                        format!("{}_dependency_{}", self.service_name, name),
                        format!("Dependency {} is {}", name, status),
                        DegradedSeverity::Moderate
                    );
                }
            }
        }
        
        Ok(())
    }
    
    /// Update a dependency health status
    pub fn update_dependency<S>(&self, name: S, status: HealthStatus, circuit_open: bool, success: bool) -> Result<()>
    where
        S: Into<String>,
    {
        let name = name.into();
        let mut health = self.health.write().unwrap();
        
        if let Some(dep) = health.dependencies.get_mut(&name) {
            dep.status = status;
            dep.circuit_open = circuit_open;
            dep.last_attempt = Some(chrono::Utc::now());
            
            if success {
                dep.last_success = Some(chrono::Utc::now());
            }
            
            // Recalculate overall status based on dependencies
            let required_deps_healthy = health.dependencies.values()
                .filter(|d| d.required)
                .all(|d| d.status == HealthStatus::Healthy || d.status == HealthStatus::Degraded);
            
            // If all required dependencies are healthy but we're degraded due to dependencies
            if required_deps_healthy && 
               health.status == HealthStatus::Degraded && 
               self.degraded_mode.is_active(format!("{}_dependency_{}", self.service_name, name)) {
                // Deactivate dependency degraded mode
                self.degraded_mode.deactivate(format!("{}_dependency_{}", self.service_name, name));
                
                // Check if all dependency degraded modes are inactive
                let prefix = format!("{}_dependency_", self.service_name);
                let any_active_dep = self.degraded_mode.active_modes()
                    .iter()
                    .any(|mode| mode.starts_with(&prefix));
                
                if !any_active_dep {
                    // If no dependency degraded modes are active, check if health degraded mode is active
                    if !self.degraded_mode.is_active(format!("{}_health", self.service_name)) {
                        // If health is also not degraded, we can set status to healthy
                        health.status = HealthStatus::Healthy;
                        let _ = self.status_change_tx.send(HealthStatus::Healthy);
                        
                        info!(
                            service = %self.service_name,
                            dependency = %name,
                            "Service health recovered due to dependency recovery"
                        );
                    }
                }
            } 
            // If a required dependency is unhealthy and we're healthy
            else if dep.required && 
                    (status == HealthStatus::Unhealthy || status == HealthStatus::Unavailable) &&
                    health.status == HealthStatus::Healthy {
                // Degrade service
                health.status = HealthStatus::Degraded;
                let _ = self.status_change_tx.send(HealthStatus::Degraded);
                
                warn!(
                    service = %self.service_name,
                    dependency = %name,
                    status = %status,
                    "Service degraded due to dependency health"
                );
                
                // Activate degraded mode
                if self.config.auto_degrade {
                    self.degraded_mode.activate(
                        format!("{}_dependency_{}", self.service_name, name),
                        format!("Dependency {} is {}", name, status),
                        DegradedSeverity::Moderate
                    );
                }
            }
        } else {
            return Err(Error::new(
                ErrorKind::Validation,
                format!("Dependency '{}' not found", name)
            ));
        }
        
        Ok(())
    }
    
    /// Set readiness status
    pub fn set_ready(&self, ready: bool) {
        let mut health = self.health.write().unwrap();
        health.ready = ready;
        
        info!(
            service = %self.service_name,
            ready = %ready,
            "Service readiness status set"
        );
    }
    
    /// Override health status manually
    pub fn override_status(&self, status: HealthStatus, reason: &str) -> Result<()> {
        if !self.config.allow_override {
            return Err(Error::new(
                ErrorKind::Validation,
                "Manual health status override is disabled".to_string()
            ));
        }
        
        let mut override_status = self.override_status.lock().unwrap();
        *override_status = Some(status);
        
        warn!(
            service = %self.service_name,
            status = %status,
            reason = %reason,
            "Health status manually overridden"
        );
        
        Ok(())
    }
    
    /// Clear manual health status override
    pub fn clear_override(&self) {
        let mut override_status = self.override_status.lock().unwrap();
        *override_status = None;
        
        info!(
            service = %self.service_name,
            "Health status manual override cleared"
        );
    }
    
    /// Get current health status
    pub fn get_status(&self) -> HealthStatus {
        // Check for override first
        let override_status = {
            let override_guard = self.override_status.lock().unwrap();
            *override_guard
        };
        
        if let Some(status) = override_status {
            return status;
        }
        
        // Get actual status
        let health = self.health.read().unwrap();
        health.status
    }
    
    /// Get current health info
    pub fn get_health(&self) -> HealthInfo {
        let health = self.health.read().unwrap();
        health.clone()
    }
    
    /// Get a watch receiver for status changes
    pub fn subscribe(&self) -> watch::Receiver<HealthStatus> {
        self.status_change_rx.clone()
    }
    
    /// Get the degraded mode manager
    pub fn degraded_mode(&self) -> Arc<DegradedMode> {
        self.degraded_mode.clone()
    }
}

/// A handle for graceful service shutdown
#[derive(Debug)]
pub struct ShutdownHandle {
    /// Service name
    service_name: String,
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
    /// Done signal channel
    done_tx: Option<oneshot::Sender<()>>,
    /// Shutdown timeout
    timeout: Duration,
    /// Tasks to wait for
    tasks: Vec<JoinHandle<()>>,
    /// Shutdown hooks
    hooks: Vec<Box<dyn FnOnce() + Send>>,
    /// Health manager
    health_manager: Option<Arc<HealthManager>>,
}

impl ShutdownHandle {
    /// Creates a new shutdown handle
    pub fn new<S: Into<String>>(service_name: S) -> Self {
        let service_name = service_name.into();
        let (shutdown_tx, _) = broadcast::channel(1);
        
        Self {
            service_name,
            shutdown_tx,
            done_tx: None,
            timeout: Duration::from_secs(30),
            tasks: Vec::new(),
            hooks: Vec::new(),
            health_manager: None,
        }
    }
    
    /// Set the maximum time to wait for graceful shutdown
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// Set a done signaling channel
    pub fn with_done_signal(mut self, done_tx: oneshot::Sender<()>) -> Self {
        self.done_tx = Some(done_tx);
        self
    }
    
    /// Add a task to wait for during shutdown
    pub fn add_task(&mut self, task: JoinHandle<()>) {
        self.tasks.push(task);
    }
    
    /// Add a health manager for status updates during shutdown
    pub fn with_health_manager(mut self, health_manager: Arc<HealthManager>) -> Self {
        self.health_manager = Some(health_manager);
        self
    }
    
    /// Add a shutdown hook to be called during shutdown
    pub fn add_shutdown_hook<F>(&mut self, hook: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.hooks.push(Box::new(hook));
    }
    
    /// Get a shutdown signal receiver
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }
    
    /// Trigger shutdown
    pub async fn shutdown(mut self) {
        info!(service = %self.service_name, "Starting graceful shutdown");
        
        // Update health status if available
        if let Some(health) = &self.health_manager {
            if let Ok(_) = health.override_status(HealthStatus::ShuttingDown, "Service shutting down") {
                info!(service = %self.service_name, "Health status set to SHUTTING_DOWN");
            }
        }
        
        // Send shutdown signal
        let _ = self.shutdown_tx.send(());
        
        // Execute shutdown hooks
        for hook in self.hooks.drain(..) {
            hook();
        }
        
        // Wait for tasks with timeout
        if !self.tasks.is_empty() {
            info!(service = %self.service_name, task_count = %self.tasks.len(), "Waiting for tasks to complete");
            
            match timeout(self.timeout, futures::future::join_all(self.tasks)).await {
                Ok(_) => {
                    info!(service = %self.service_name, "All tasks completed gracefully");
                }
                Err(_) => {
                    warn!(
                        service = %self.service_name, 
                        timeout_secs = %self.timeout.as_secs(),
                        "Shutdown timed out waiting for tasks"
                    );
                }
            }
        }
        
        // Stop health manager if available
        if let Some(health) = &self.health_manager {
            health.stop();
        }
        
        // Send done signal if available
        if let Some(done_tx) = self.done_tx {
            let _ = done_tx.send(());
        }
        
        info!(service = %self.service_name, "Shutdown complete");
    }
}

/// Process supervisor for automatic restart
#[derive(Debug)]
pub struct ProcessSupervisor {
    /// Process name
    name: String,
    /// Process configuration
    config: SupervisorConfig,
    /// Restart history
    restarts: Arc<Mutex<Vec<Instant>>>,
    /// Worker handle
    worker_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Shutdown signal
    shutdown_tx: broadcast::Sender<()>,
    /// Process status
    status_tx: watch::Sender<ProcessStatus>,
    /// Process status receiver
    status_rx: watch::Receiver<ProcessStatus>,
    /// Last error
    last_error: Arc<Mutex<Option<String>>>,
}

/// Process status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    /// Not started yet
    Idle,
    /// Running
    Running,
    /// Restarting after failure
    Restarting,
    /// Failed completely
    Failed,
    /// Shutting down
    ShuttingDown,
    /// Shut down
    Terminated,
}

impl std::fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessStatus::Idle => write!(f, "IDLE"),
            ProcessStatus::Running => write!(f, "RUNNING"),
            ProcessStatus::Restarting => write!(f, "RESTARTING"),
            ProcessStatus::Failed => write!(f, "FAILED"),
            ProcessStatus::ShuttingDown => write!(f, "SHUTTING_DOWN"),
            ProcessStatus::Terminated => write!(f, "TERMINATED"),
        }
    }
}

/// Process supervisor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorConfig {
    /// Maximum number of restarts in the period
    pub max_restarts: usize,
    /// Period to track restarts in
    pub restart_period: Duration,
    /// Delay before restarting
    pub restart_delay: Duration,
    /// Whether to use exponential backoff for restart delays
    pub use_backoff: bool,
    /// Maximum restart delay with backoff
    pub max_restart_delay: Duration,
    /// Whether to apply jitter to restart delays
    pub use_jitter: bool,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            max_restarts: 5,
            restart_period: Duration::from_secs(60),
            restart_delay: Duration::from_secs(1),
            use_backoff: true,
            max_restart_delay: Duration::from_secs(60),
            use_jitter: true,
        }
    }
}

impl ProcessSupervisor {
    /// Creates a new process supervisor
    pub fn new<S: Into<String>>(name: S, config: Option<SupervisorConfig>) -> Self {
        let name = name.into();
        let config = config.unwrap_or_default();
        
        // Create channels
        let (shutdown_tx, _) = broadcast::channel(1);
        let (status_tx, status_rx) = watch::channel(ProcessStatus::Idle);
        
        Self {
            name,
            config,
            restarts: Arc::new(Mutex::new(Vec::new())),
            worker_handle: Arc::new(Mutex::new(None)),
            shutdown_tx,
            status_tx,
            status_rx,
            last_error: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Start supervising a process
    pub async fn start<F, Fut>(&self, process_factory: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let mut worker_handle = self.worker_handle.lock().unwrap();
        
        if worker_handle.is_some() {
            return Err(Error::new(
                ErrorKind::Initialization,
                format!("Supervisor for {} is already running", self.name)
            ));
        }
        
        // Set status
        let _ = self.status_tx.send(ProcessStatus::Running);
        
        // Clone references for the worker
        let name = self.name.clone();
        let config = self.config.clone();
        let restarts = self.restarts.clone();
        let status_tx = self.status_tx.clone();
        let shutdown_rx = self.shutdown_tx.subscribe();
        let last_error = self.last_error.clone();
        
        // Start the worker
        let handle = tokio::spawn(Self::run_supervised(
            name,
            config,
            restarts,
            status_tx,
            shutdown_rx,
            process_factory,
            last_error,
        ));
        
        // Store the handle
        *worker_handle = Some(handle);
        
        Ok(())
    }
    
    /// Supervise a process with automatic restart
    async fn run_supervised<F, Fut>(
        name: String,
        config: SupervisorConfig,
        restarts: Arc<Mutex<Vec<Instant>>>,
        status_tx: watch::Sender<ProcessStatus>,
        mut shutdown_rx: broadcast::Receiver<()>,
        process_factory: F,
        last_error: Arc<Mutex<Option<String>>>,
    )
    where
        F: Fn() -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let mut restart_count = 0;
        let mut restart_delay = config.restart_delay;
        
        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = shutdown_rx.recv() => {
                    info!(process = %name, "Supervisor shutting down");
                    let _ = status_tx.send(ProcessStatus::ShuttingDown);
                    break;
                }
                
                // Run the process
                result = process_factory() => {
                    match result {
                        Ok(()) => {
                            // Normal completion
                            info!(process = %name, "Process completed normally");
                            let _ = status_tx.send(ProcessStatus::Terminated);
                            break;
                        }
                        Err(e) => {
                            // Process failed
                            error!(
                                process = %name,
                                error = %e,
                                "Process failed, preparing to restart"
                            );
                            
                            // Update last error
                            {
                                let mut err = last_error.lock().unwrap();
                                *err = Some(e.to_string());
                            }
                            
                            // Update restart history
                            let restart_allowed = {
                                let mut restart_history = restarts.lock().unwrap();
                                
                                // Add current restart
                                restart_history.push(Instant::now());
                                
                                // Prune old restarts outside the tracking period
                                let cutoff = Instant::now() - config.restart_period;
                                restart_history.retain(|time| *time > cutoff);
                                
                                // Check if we're allowing more restarts
                                restart_history.len() <= config.max_restarts
                            };
                            
                            if restart_allowed {
                                // Set status to restarting
                                let _ = status_tx.send(ProcessStatus::Restarting);
                                
                                // Increment restart counter
                                restart_count += 1;
                                
                                // Calculate delay with exponential backoff if enabled
                                if config.use_backoff {
                                    let backoff_factor = (1.5_f64).powi(restart_count.min(10) as i32);
                                    let base_delay = config.restart_delay.as_millis() as f64;
                                    let delay_ms = (base_delay * backoff_factor) as u64;
                                    restart_delay = Duration::from_millis(
                                        delay_ms.min(config.max_restart_delay.as_millis() as u64)
                                    );
                                }
                                
                                // Add jitter if enabled
                                let actual_delay = if config.use_jitter {
                                    let jitter_factor = rand::random::<f64>() * 0.2 - 0.1; // -10% to +10%
                                    let base_ms = restart_delay.as_millis() as f64;
                                    let jittered_ms = base_ms * (1.0 + jitter_factor);
                                    Duration::from_millis(jittered_ms as u64)
                                } else {
                                    restart_delay
                                };
                                
                                // Record metrics
                                counter!(&format!("supervisor.{}.restarts", name), 1);
                                gauge!(
                                    &format!("supervisor.{}.restart_delay_ms", name),
                                    actual_delay.as_millis() as f64
                                );
                                
                                info!(
                                    process = %name,
                                    restart_count = %restart_count,
                                    delay_ms = %actual_delay.as_millis(),
                                    "Restarting process after delay"
                                );
                                
                                // Wait before restarting
                                tokio::time::sleep(actual_delay).await;
                                
                                // Now set status back to running
                                let _ = status_tx.send(ProcessStatus::Running);
                                
                                // Continue loop, which will restart the process
                            } else {
                                // Too many restarts
                                error!(
                                    process = %name,
                                    max_restarts = %config.max_restarts,
                                    period_secs = %config.restart_period.as_secs(),
                                    "Too many restarts, giving up"
                                );
                                
                                // Set status to failed
                                let _ = status_tx.send(ProcessStatus::Failed);
                                
                                // Report the failure
                                let _ = send_alert(
                                    AlertLevel::Emergency,
                                    &format!("Process {} failed after {} restart attempts", name, restart_count),
                                    None,
                                    None
                                );
                                
                                // Exit loop
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Stop the supervisor
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(());
        
        info!(process = %self.name, "Supervisor stopped");
    }
    
    /// Get the current status
    pub fn status(&self) -> ProcessStatus {
        *self.status_rx.borrow()
    }
    
    /// Get a status change receiver
    pub fn subscribe(&self) -> watch::Receiver<ProcessStatus> {
        self.status_rx.clone()
    }
    
    /// Get the last error
    pub fn last_error(&self) -> Option<String> {
        let error = self.last_error.lock().unwrap();
        error.clone()
    }
    
    /// Get restart count in the current period
    pub fn restart_count(&self) -> usize {
        let restarts = self.restarts.lock().unwrap();
        
        // Count restarts in the period
        let cutoff = Instant::now() - self.config.restart_period;
        restarts.iter().filter(|time| **time > cutoff).count()
    }
}

/// Creates a standard health check for an HTTP endpoint
pub fn create_http_health_check(
    name: String, 
    url: String,
    timeout: Option<Duration>,
    circuit_breaker: Option<Arc<CircuitBreaker>>,
) -> HealthCheckFn {
    let timeout = timeout.unwrap_or_else(|| Duration::from_secs(5));
    
    Box::new(move || {
        let name = name.clone();
        let url = url.clone();
        let circuit_breaker_clone = circuit_breaker.clone();
        
        Box::pin(async move {
            // Check if circuit breaker is open
            if let Some(cb) = &circuit_breaker_clone {
                if !cb.is_allowed(&name) {
                    return Ok(SubHealthCheck {
                        name: name.clone(),
                        status: HealthStatus::Degraded,
                        message: Some("Circuit breaker is open".to_string()),
                        timestamp: chrono::Utc::now(),
                        last_success: None,
                        metrics: None,
                    });
                }
            }
            
            // Create HTTP client
            let client = reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .map_err(|e| Error::new(ErrorKind::Initialization, format!("Failed to create HTTP client: {}", e)))?;
            
            // Send request with timeout
            let start = Instant::now();
            let response = client.get(&url).send().await.map_err(|e| {
                // Record failure in circuit breaker if configured
                if let Some(cb) = &circuit_breaker_clone {
                    cb.record_failure(&name);
                }
                
                Error::new(
                    if e.is_timeout() { ErrorKind::Timeout } else { ErrorKind::Communication },
                    format!("HTTP health check failed: {}", e)
                )
            })?;
            
            let status_code = response.status();
            let duration = start.elapsed();
            
            // Check response status
            if status_code.is_success() {
                // Record success in circuit breaker if configured
                if let Some(cb) = &circuit_breaker_clone {
                    cb.record_success(&name);
                }
                
                // Success
                Ok(SubHealthCheck {
                    name: name.clone(),
                    status: HealthStatus::Healthy,
                    message: Some(format!("HTTP {} - {}ms", status_code, duration.as_millis())),
                    timestamp: chrono::Utc::now(),
                    last_success: Some(chrono::Utc::now()),
                    metrics: Some(HashMap::from([
                        ("status_code".to_string(), serde_json::json!(status_code.as_u16())),
                        ("latency_ms".to_string(), serde_json::json!(duration.as_millis())),
                    ])),
                })
            } else if status_code.as_u16() >= 500 {
                // Record failure in circuit breaker if configured
                if let Some(cb) = &circuit_breaker_clone {
                    cb.record_failure(&name);
                }
                
                // Server error - mark as unhealthy
                Ok(SubHealthCheck {
                    name: name.clone(),
                    status: HealthStatus::Unhealthy,
                    message: Some(format!("HTTP {} - {}ms", status_code, duration.as_millis())),
                    timestamp: chrono::Utc::now(),
                    last_success: None,
                    metrics: Some(HashMap::from([
                        ("status_code".to_string(), serde_json::json!(status_code.as_u16())),
                        ("latency_ms".to_string(), serde_json::json!(duration.as_millis())),
                    ])),
                })
            } else {
                // Client error - mark as degraded
                Ok(SubHealthCheck {
                    name: name.clone(),
                    status: HealthStatus::Degraded,
                    message: Some(format!("HTTP {} - {}ms", status_code, duration.as_millis())),
                    timestamp: chrono::Utc::now(),
                    last_success: None,
                    metrics: Some(HashMap::from([
                        ("status_code".to_string(), serde_json::json!(status_code.as_u16())),
                        ("latency_ms".to_string(), serde_json::json!(duration.as_millis())),
                    ])),
                })
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::mpsc;
    
    #[tokio::test]
    async fn test_health_manager_basic() {
        let health = HealthManager::new("test-service", Some("1.0.0".to_string()), None);
        
        // Start health check worker
        health.start().unwrap();
        
        // Register a simple health check
        health.register("test", || async {
            Ok(SubHealthCheck {
                name: "test".to_string(),
                status: HealthStatus::Healthy,
                message: Some("Test check".to_string()),
                timestamp: chrono::Utc::now(),
                last_success: Some(chrono::Utc::now()),
                metrics: None,
            })
        }).unwrap();
        
        // Give it time to run checks
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Check status
        assert_eq!(health.get_status(), HealthStatus::Healthy);
        
        // Stop health check worker
        health.stop();
    }
    
    #[tokio::test]
    async fn test_shutdown_handle() {
        let (tx, mut rx) = mpsc::channel(1);
        
        // Create shutdown handle
        let mut shutdown = ShutdownHandle::new("test-service")
            .with_timeout(Duration::from_millis(100));
        
        // Add a task
        let task = tokio::spawn(async move {
            // Signal that we're running
            tx.send(()).await.unwrap();
            
            // Wait indefinitely (will be aborted by shutdown)
            tokio::time::sleep(Duration::from_secs(60)).await;
        });
        
        shutdown.add_task(task);
        
        // Wait for task to start
        rx.recv().await.unwrap();
        
        // Initiate shutdown
        shutdown.shutdown().await;
    }
    
    #[tokio::test]
    async fn test_process_supervisor() {
        let supervisor = ProcessSupervisor::new("test-process", None);
        
        // Create a counter for tracking restarts
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        // Start supervising a process that fails twice then succeeds
        supervisor.start(move || {
            let counter = counter_clone.clone();
            
            async move {
                // Increment counter
                let count = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                
                if count <= 2 {
                    // Fail the first two times
                    Err(Error::new(ErrorKind::Internal, format!("Test failure {}", count)))
                } else {
                    // Succeed the third time
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    Ok(())
                }
            }
        }).await.unwrap();
        
        // Give it time to restart a few times
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        // Check restart count
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
        
        // Check status
        assert_eq!(supervisor.status(), ProcessStatus::Terminated);
        
        // Stop supervisor
        supervisor.stop();
    }
}