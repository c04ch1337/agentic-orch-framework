#![allow(unused_imports)]
#![allow(dead_code)]

//! # Enhanced Monitoring and Observability System
//!
//! This module provides comprehensive monitoring capabilities including:
//! - Performance metrics collection
//! - Resource utilization tracking
//! - Execution statistics
//! - Health monitoring
//! - Distributed tracing integration
//! - Alerting and notifications

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn, span, Level};
use metrics::{counter, gauge, histogram, increment_counter, decrement_gauge};
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};
use config_management_rs::ConfigChange;

/// Global monitoring state
#[derive(Debug, Clone)]
pub struct MonitoringState {
    execution_stats: Arc<RwLock<ExecutionStats>>,
    resource_metrics: Arc<RwLock<ResourceMetrics>>,
    health_status: Arc<RwLock<HealthStatus>>,
    alert_rules: Arc<RwLock<Vec<AlertRule>>>,
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub timeout_executions: u64,
    pub resource_limit_executions: u64,
    pub average_execution_time_ms: f64,
    pub max_execution_time_ms: f64,
    pub min_execution_time_ms: f64,
}

/// Resource metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceMetrics {
    pub current_memory_usage_mb: f64,
    pub peak_memory_usage_mb: f64,
    pub current_cpu_usage_percent: f64,
    pub peak_cpu_usage_percent: f64,
    pub active_processes: u32,
    pub peak_processes: u32,
    pub total_processes_created: u64,
    pub total_processes_terminated: u64,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthStatus {
    pub overall_health: HealthLevel,
    pub memory_health: HealthLevel,
    pub cpu_health: HealthLevel,
    pub process_health: HealthLevel,
    pub configuration_health: HealthLevel,
    pub last_health_check: Option<Instant>,
    pub last_issue_detected: Option<Instant>,
}

/// Health levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthLevel {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub metric: String,
    pub threshold: f64,
    pub comparison: ComparisonOperator,
    pub severity: AlertSeverity,
    pub cooldown_seconds: u64,
    pub last_triggered: Option<Instant>,
}

/// Comparison operators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    EqualTo,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Alert severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Execution metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetric {
    pub command: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub exit_code: Option<i32>,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub status: ExecutionStatus,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Timeout,
    ResourceLimit,
    Terminated,
}

/// Global monitoring instance
static GLOBAL_MONITORING: Lazy<Arc<MonitoringState>> = Lazy::new(|| {
    Arc::new(MonitoringState {
        execution_stats: Arc::new(RwLock::new(ExecutionStats::default())),
        resource_metrics: Arc::new(RwLock::new(ResourceMetrics::default())),
        health_status: Arc::new(RwLock::new(HealthStatus::default())),
        alert_rules: Arc::new(RwLock::new(vec![
            create_default_alert_rules()
        ])),
    })
});

/// Initialize monitoring system
pub fn init_monitoring() {
    info!("Initializing enhanced monitoring system");

    // Set up metrics collection
    setup_metrics_collection();

    // Set up health monitoring
    setup_health_monitoring();

    // Set up alerting
    setup_alerting();

    info!("Monitoring system initialized");
}

/// Get global monitoring instance
pub fn get_monitoring() -> Arc<MonitoringState> {
    GLOBAL_MONITORING.clone()
}

/// Create default alert rules
fn create_default_alert_rules() -> Vec<AlertRule> {
    vec![
        AlertRule {
            name: "HighMemoryUsage".to_string(),
            metric: "memory_usage_mb".to_string(),
            threshold: 400.0,
            comparison: ComparisonOperator::GreaterThan,
            severity: AlertSeverity::Warning,
            cooldown_seconds: 300,
            last_triggered: None,
        },
        AlertRule {
            name: "CriticalMemoryUsage".to_string(),
            metric: "memory_usage_mb".to_string(),
            threshold: 450.0,
            comparison: ComparisonOperator::GreaterThan,
            severity: AlertSeverity::Critical,
            cooldown_seconds: 60,
            last_triggered: None,
        },
        AlertRule {
            name: "HighCpuUsage".to_string(),
            metric: "cpu_usage_percent".to_string(),
            threshold: 80.0,
            comparison: ComparisonOperator::GreaterThan,
            severity: AlertSeverity::Warning,
            cooldown_seconds: 60,
            last_triggered: None,
        },
        AlertRule {
            name: "CriticalCpuUsage".to_string(),
            metric: "cpu_usage_percent".to_string(),
            threshold: 90.0,
            comparison: ComparisonOperator::GreaterThan,
            severity: AlertSeverity::Critical,
            cooldown_seconds: 30,
            last_triggered: None,
        },
        AlertRule {
            name: "HighFailureRate".to_string(),
            metric: "failure_rate".to_string(),
            threshold: 0.1, // 10% failure rate
            comparison: ComparisonOperator::GreaterThan,
            severity: AlertSeverity::Warning,
            cooldown_seconds: 120,
            last_triggered: None,
        },
    ]
}

/// Set up metrics collection
fn setup_metrics_collection() {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(5));

        loop {
            interval.tick().await;
            collect_metrics().await;
        }
    });
}

/// Collect system metrics
async fn collect_metrics() {
    let monitoring = get_monitoring();

    // Collect execution metrics
    let stats = monitoring.execution_stats.read().await;
    gauge!("executor.total_executions", stats.total_executions as f64);
    gauge!("executor.successful_executions", stats.successful_executions as f64);
    gauge!("executor.failed_executions", stats.failed_executions as f64);
    gauge!("executor.average_execution_time_ms", stats.average_execution_time_ms);
    gauge!("executor.max_execution_time_ms", stats.max_execution_time_ms);

    // Collect resource metrics
    let resources = monitoring.resource_metrics.read().await;
    gauge!("executor.memory_usage_mb", resources.current_memory_usage_mb);
    gauge!("executor.peak_memory_usage_mb", resources.peak_memory_usage_mb);
    gauge!("executor.cpu_usage_percent", resources.current_cpu_usage_percent);
    gauge!("executor.peak_cpu_usage_percent", resources.peak_cpu_usage_percent);
    gauge!("executor.active_processes", resources.active_processes as f64);
    gauge!("executor.peak_processes", resources.peak_processes as f64);

    debug!("Metrics collected successfully");
}

/// Set up health monitoring
fn setup_health_monitoring() {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(10));

        loop {
            interval.tick().await;
            check_health().await;
        }
    });
}

/// Check system health
async fn check_health() {
    let monitoring = get_monitoring();
    let mut health = monitoring.health_status.write().await;

    // Check memory health
    let resources = monitoring.resource_metrics.read().await;
    health.memory_health = if resources.current_memory_usage_mb > 450.0 {
        HealthLevel::Critical
    } else if resources.current_memory_usage_mb > 400.0 {
        HealthLevel::Warning
    } else {
        HealthLevel::Healthy
    };

    // Check CPU health
    health.cpu_health = if resources.current_cpu_usage_percent > 90.0 {
        HealthLevel::Critical
    } else if resources.current_cpu_usage_percent > 80.0 {
        HealthLevel::Warning
    } else {
        HealthLevel::Healthy
    };

    // Check process health
    health.process_health = if resources.active_processes > 15 {
        HealthLevel::Critical
    } else if resources.active_processes > 10 {
        HealthLevel::Warning
    } else {
        HealthLevel::Healthy
    };

    // Determine overall health
    health.overall_health = if health.memory_health == HealthLevel::Critical ||
        health.cpu_health == HealthLevel::Critical ||
        health.process_health == HealthLevel::Critical {
        HealthLevel::Critical
    } else if health.memory_health == HealthLevel::Warning ||
        health.cpu_health == HealthLevel::Warning ||
        health.process_health == HealthLevel::Warning {
        HealthLevel::Warning
    } else {
        HealthLevel::Healthy
    };

    health.last_health_check = Some(Instant::now());

    // Log health status
    match health.overall_health {
        HealthLevel::Healthy => debug!("System health: Healthy"),
        HealthLevel::Warning => warn!("System health: Warning"),
        HealthLevel::Critical => error!("System health: Critical"),
        HealthLevel::Unknown => warn!("System health: Unknown"),
    }
}

/// Set up alerting system
fn setup_alerting() {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(5));

        loop {
            interval.tick().await;
            check_alerts().await;
        }
    });
}

/// Check alert conditions
async fn check_alerts() {
    let monitoring = get_monitoring();
    let alert_rules = monitoring.alert_rules.read().await;
    let resources = monitoring.resource_metrics.read().await;
    let stats = monitoring.execution_stats.read().await;

    for rule in alert_rules.iter() {
        if should_trigger_alert(rule, &resources, &stats).await {
            trigger_alert(rule).await;
        }
    }
}

/// Determine if alert should be triggered
async fn should_trigger_alert(
    rule: &AlertRule,
    resources: &ResourceMetrics,
    stats: &ExecutionStats,
) -> bool {
    // Check cooldown period
    if let Some(last_triggered) = rule.last_triggered {
        if last_triggered.elapsed().as_secs() < rule.cooldown_seconds {
            return false;
        }
    }

    // Check alert condition
    let value = match rule.metric.as_str() {
        "memory_usage_mb" => resources.current_memory_usage_mb,
        "cpu_usage_percent" => resources.current_cpu_usage_percent,
        "failure_rate" => {
            if stats.total_executions > 0 {
                stats.failed_executions as f64 / stats.total_executions as f64
            } else {
                0.0
            }
        }
        _ => 0.0,
    };

    match rule.comparison {
        ComparisonOperator::GreaterThan => value > rule.threshold,
        ComparisonOperator::LessThan => value < rule.threshold,
        ComparisonOperator::EqualTo => (value - rule.threshold).abs() < f64::EPSILON,
        ComparisonOperator::GreaterThanOrEqual => value >= rule.threshold,
        ComparisonOperator::LessThanOrEqual => value <= rule.threshold,
    }
}

/// Trigger an alert
async fn trigger_alert(rule: &AlertRule) {
    let monitoring = get_monitoring();
    let mut health = monitoring.health_status.write().await;

    match rule.severity {
        AlertSeverity::Info => info!("ALERT: {} - {} {} {}", rule.name, rule.metric, get_comparison_symbol(&rule.comparison), rule.threshold),
        AlertSeverity::Warning => warn!("ALERT: {} - {} {} {}", rule.name, rule.metric, get_comparison_symbol(&rule.comparison), rule.threshold),
        AlertSeverity::Critical => error!("ALERT: {} - {} {} {}", rule.name, rule.metric, get_comparison_symbol(&rule.comparison), rule.threshold),
        AlertSeverity::Emergency => {
            error!("EMERGENCY ALERT: {} - {} {} {}", rule.name, rule.metric, get_comparison_symbol(&rule.comparison), rule.threshold);
            health.last_issue_detected = Some(Instant::now());
        }
    }

    // Update last triggered time
    let mut alert_rules = monitoring.alert_rules.write().await;
    for stored_rule in alert_rules.iter_mut() {
        if stored_rule.name == rule.name {
            stored_rule.last_triggered = Some(Instant::now());
            break;
        }
    }
}

/// Get comparison symbol for display
fn get_comparison_symbol(op: &ComparisonOperator) -> &'static str {
    match op {
        ComparisonOperator::GreaterThan => ">",
        ComparisonOperator::LessThan => "<",
        ComparisonOperator::EqualTo => "=",
        ComparisonOperator::GreaterThanOrEqual => ">=",
        ComparisonOperator::LessThanOrEqual => "<=",
    }
}

/// Record execution start
pub async fn record_execution_start(command: &str) {
    let monitoring = get_monitoring();
    let mut stats = monitoring.execution_stats.write().await;

    stats.total_executions += 1;
    increment_counter!("executor.executions_started", "command" => command.to_string());

    debug!("Execution started: {}", command);
}

/// Record execution completion
pub async fn record_execution_completion(
    command: &str,
    start_time: Instant,
    exit_code: i32,
    memory_usage_mb: f64,
    cpu_usage_percent: f64,
    status: ExecutionStatus,
) {
    let monitoring = get_monitoring();
    let mut stats = monitoring.execution_stats.write().await;
    let mut resources = monitoring.resource_metrics.write().await;

    let execution_time_ms = start_time.elapsed().as_millis() as f64;

    // Update execution statistics
    match status {
        ExecutionStatus::Completed if exit_code == 0 => {
            stats.successful_executions += 1;
            increment_counter!("executor.executions_succeeded", "command" => command.to_string());
        }
        ExecutionStatus::Completed => {
            stats.failed_executions += 1;
            increment_counter!("executor.executions_failed", "command" => command.to_string());
        }
        ExecutionStatus::Failed => {
            stats.failed_executions += 1;
            increment_counter!("executor.executions_failed", "command" => command.to_string());
        }
        ExecutionStatus::Timeout => {
            stats.timeout_executions += 1;
            increment_counter!("executor.executions_timeout", "command" => command.to_string());
        }
        ExecutionStatus::ResourceLimit => {
            stats.resource_limit_executions += 1;
            increment_counter!("executor.executions_resource_limit", "command" => command.to_string());
        }
        ExecutionStatus::Terminated => {
            stats.failed_executions += 1;
            increment_counter!("executor.executions_terminated", "command" => command.to_string());
        }
        ExecutionStatus::Running => {
            // Should not happen for completion
        }
    }

    // Update execution time statistics
    stats.average_execution_time_ms = if stats.total_executions > 0 {
        (stats.average_execution_time_ms * (stats.total_executions - 1) as f64 + execution_time_ms) / stats.total_executions as f64
    } else {
        execution_time_ms
    };

    if execution_time_ms > stats.max_execution_time_ms {
        stats.max_execution_time_ms = execution_time_ms;
    }

    if stats.min_execution_time_ms == 0.0 || execution_time_ms < stats.min_execution_time_ms {
        stats.min_execution_time_ms = execution_time_ms;
    }

    // Update resource metrics
    resources.current_memory_usage_mb = memory_usage_mb;
    if memory_usage_mb > resources.peak_memory_usage_mb {
        resources.peak_memory_usage_mb = memory_usage_mb;
    }

    resources.current_cpu_usage_percent = cpu_usage_percent;
    if cpu_usage_percent > resources.peak_cpu_usage_percent {
        resources.peak_cpu_usage_percent = cpu_usage_percent;
    }

    // Log execution details
    match status {
        ExecutionStatus::Completed if exit_code == 0 => {
            debug!(
                "Execution completed: {} (time: {:.2}ms, memory: {:.2}MB, cpu: {:.2}%)",
                command, execution_time_ms, memory_usage_mb, cpu_usage_percent
            );
        }
        ExecutionStatus::Completed => {
            warn!(
                "Execution completed with errors: {} (exit: {}, time: {:.2}ms, memory: {:.2}MB, cpu: {:.2}%)",
                command, exit_code, execution_time_ms, memory_usage_mb, cpu_usage_percent
            );
        }
        ExecutionStatus::Failed => {
            error!(
                "Execution failed: {} (time: {:.2}ms, memory: {:.2}MB, cpu: {:.2}%)",
                command, execution_time_ms, memory_usage_mb, cpu_usage_percent
            );
        }
        ExecutionStatus::Timeout => {
            warn!(
                "Execution timeout: {} (time: {:.2}ms, memory: {:.2}MB, cpu: {:.2}%)",
                command, execution_time_ms, memory_usage_mb, cpu_usage_percent
            );
        }
        ExecutionStatus::ResourceLimit => {
            warn!(
                "Execution resource limit: {} (time: {:.2}ms, memory: {:.2}MB, cpu: {:.2}%)",
                command, execution_time_ms, memory_usage_mb, cpu_usage_percent
            );
        }
        ExecutionStatus::Terminated => {
            warn!(
                "Execution terminated: {} (time: {:.2}ms, memory: {:.2}MB, cpu: {:.2}%)",
                command, execution_time_ms, memory_usage_mb, cpu_usage_percent
            );
        }
        ExecutionStatus::Running => {
            // Should not happen
        }
    }
}

/// Update resource metrics
pub async fn update_resource_metrics(
    memory_usage_mb: f64,
    cpu_usage_percent: f64,
    active_processes: u32,
) {
    let monitoring = get_monitoring();
    let mut resources = monitoring.resource_metrics.write().await;

    resources.current_memory_usage_mb = memory_usage_mb;
    if memory_usage_mb > resources.peak_memory_usage_mb {
        resources.peak_memory_usage_mb = memory_usage_mb;
    }

    resources.current_cpu_usage_percent = cpu_usage_percent;
    if cpu_usage_percent > resources.peak_cpu_usage_percent {
        resources.peak_cpu_usage_percent = cpu_usage_percent;
    }

    resources.active_processes = active_processes;
    if active_processes > resources.peak_processes {
        resources.peak_processes = active_processes;
    }
}

/// Get current monitoring statistics
pub async fn get_monitoring_stats() -> MonitoringStats {
    let monitoring = get_monitoring();

    let stats = monitoring.execution_stats.read().await.clone();
    let resources = monitoring.resource_metrics.read().await.clone();
    let health = monitoring.health_status.read().await.clone();

    MonitoringStats {
        execution_stats: stats,
        resource_metrics: resources,
        health_status: health,
    }
}

/// Get current health status
pub async fn get_health_status() -> HealthStatus {
    let monitoring = get_monitoring();
    monitoring.health_status.read().await.clone()
}

/// Monitoring statistics response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringStats {
    pub execution_stats: ExecutionStats,
    pub resource_metrics: ResourceMetrics,
    pub health_status: HealthStatus,
}

/// Configuration change handler for monitoring
pub async fn handle_config_change_for_monitoring(change: config_management::ConfigChange<crate::config::ExecutorConfig>) {
    info!("Configuration changed, updating monitoring thresholds");

    let monitoring = get_monitoring();
    let mut alert_rules = monitoring.alert_rules.write().await;

    // Update alert rules based on new configuration
    let new_config = change.new_config;

    // Update memory alert thresholds
    for rule in alert_rules.iter_mut() {
        if rule.metric == "memory_usage_mb" {
            if rule.name == "HighMemoryUsage" {
                rule.threshold = (new_config.max_memory_mb as f64 * 0.8).min(400.0);
            } else if rule.name == "CriticalMemoryUsage" {
                rule.threshold = (new_config.max_memory_mb as f64 * 0.9).min(450.0);
            }
        }
    }

    info!("Monitoring thresholds updated based on new configuration");
}

/// Start monitoring for a specific execution
pub async fn start_execution_monitoring(command: &str) -> ExecutionMonitor {
    let start_time = Instant::now();

    record_execution_start(command).await;

    ExecutionMonitor {
        command: command.to_string(),
        start_time,
        completed: false,
    }
}

/// Execution monitor handle
#[derive(Debug)]
pub struct ExecutionMonitor {
    command: String,
    start_time: Instant,
    completed: bool,
}

impl ExecutionMonitor {
    /// Complete the execution monitoring
    pub async fn complete(
        &mut self,
        exit_code: i32,
        memory_usage_mb: f64,
        cpu_usage_percent: f64,
        status: ExecutionStatus,
    ) {
        if !self.completed {
            record_execution_completion(
                &self.command,
                self.start_time,
                exit_code,
                memory_usage_mb,
                cpu_usage_percent,
                status,
            ).await;
            self.completed = true;
        }
    }
}

/// Monitoring utilities
pub mod utils {
    use super::*;

    /// Format health status for display
    pub fn format_health_status(health: &HealthStatus) -> String {
        format!(
            "Overall: {:?}, Memory: {:?}, CPU: {:?}, Process: {:?}, Config: {:?}",
            health.overall_health,
            health.memory_health,
            health.cpu_health,
            health.process_health,
            health.configuration_health
        )
    }

    /// Get health level color for display
    pub fn get_health_color(health: &HealthLevel) -> &'static str {
        match health {
            HealthLevel::Healthy => "green",
            HealthLevel::Warning => "yellow",
            HealthLevel::Critical => "red",
            HealthLevel::Unknown => "gray",
        }
    }

    /// Calculate failure rate
    pub fn calculate_failure_rate(stats: &ExecutionStats) -> f64 {
        if stats.total_executions > 0 {
            stats.failed_executions as f64 / stats.total_executions as f64
        } else {
            0.0
        }
    }
}

/// Monitoring testing utilities
#[cfg(test)]
pub mod test_utils {
    use super::*;

    /// Create test monitoring state
    pub fn create_test_monitoring() -> Arc<MonitoringState> {
        Arc::new(MonitoringState {
            execution_stats: Arc::new(RwLock::new(ExecutionStats::default())),
            resource_metrics: Arc::new(RwLock::new(ResourceMetrics::default())),
            health_status: Arc::new(RwLock::new(HealthStatus::default())),
            alert_rules: Arc::new(RwLock::new(create_default_alert_rules())),
        })
    }

    /// Simulate execution for testing
    pub async fn simulate_execution(monitoring: &Arc<MonitoringState>, command: &str, success: bool) {
        let mut stats = monitoring.execution_stats.write().await;
        stats.total_executions += 1;

        if success {
            stats.successful_executions += 1;
        } else {
            stats.failed_executions += 1;
        }
    }
}

/// Monitoring examples
pub mod examples {
    use super::*;

    /// Example monitoring configuration
    pub fn example_monitoring_config() -> Vec<AlertRule> {
        vec![
            AlertRule {
                name: "HighMemoryUsage".to_string(),
                metric: "memory_usage_mb".to_string(),
                threshold: 400.0,
                comparison: ComparisonOperator::GreaterThan,
                severity: AlertSeverity::Warning,
                cooldown_seconds: 300,
                last_triggered: None,
            },
            AlertRule {
                name: "HighCpuUsage".to_string(),
                metric: "cpu_usage_percent".to_string(),
                threshold: 80.0,
                comparison: ComparisonOperator::GreaterThan,
                severity: AlertSeverity::Warning,
                cooldown_seconds: 60,
                last_triggered: None,
            },
        ]
    }
}

/// Monitoring macros
#[macro_export]
macro_rules! monitor_execution {
    ($command:expr) => {{
        let monitor = $crate::monitoring::start_execution_monitoring($command).await;
        monitor
    }};
}

#[macro_export]
macro_rules! complete_monitoring {
    ($monitor:expr, $exit_code:expr, $memory:expr, $cpu:expr, $status:expr) => {{
        $monitor.complete($exit_code, $memory, $cpu, $status).await;
    }};
}