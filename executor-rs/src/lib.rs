//! Executor Library
//! Provides Windows native execution control and process management functionality

mod execution_logic;
mod windows_executor;
mod config;
mod monitoring;
mod performance;
mod security;

pub use execution_logic::{
    execute_python_sandboxed, execute_shell_command, get_execution_stats, simulate_input,
};

pub use windows_executor::{execute_with_windows_control, validate_path, JobObjectManager, check_sandbox_integrity};
pub use config::{get_config, update_config, subscribe_to_changes, check_config_health};
pub use monitoring::{init_monitoring, get_monitoring_stats, get_health_status, start_execution_monitoring};
pub use performance::{init_performance_optimizer, validate_performance, get_performance_stats};
pub use security::{init_security_manager, validate_command_security, get_security_stats};

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

// Track service start time for uptime reporting
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

/// Core Executor implementation
#[derive(Debug, Default)]
pub struct Executor {
    // Service health state tracking
    service_health: Arc<tokio::sync::RwLock<HashMap<String, bool>>>,
}

impl Executor {
    /// Create a new Executor instance
    pub fn new() -> Self {
        // Create service health tracking
        let service_health = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        let service_health_clone = Arc::clone(&service_health);

        // Initialize known services as healthy
        let service_names = ["shell", "python", "input_simulation", "process_watchdog"];
        tokio::spawn(async move {
            let mut health = service_health_clone.write().await;
            for &service in &service_names {
                health.insert(service.to_string(), true);
            }
        });

        Self { service_health }
    }

    /// Execute a shell command with Windows native control
    pub async fn execute_command(
        &self,
        command: &str,
        args: &[String],
        env_vars: &HashMap<String, String>,
    ) -> Result<(String, String, i32), String> {
        execute_shell_command(command, args, env_vars).await
    }

    /// Execute Python code in a sandboxed environment
    pub async fn execute_python(
        &self,
        code: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<(String, String, i32), String> {
        execute_python_sandboxed(code, env_vars).await
    }

    /// Simulate input (mouse/keyboard) with security boundaries
    pub fn simulate_input(
        &self,
        input_type: &str,
        params: &HashMap<String, String>,
    ) -> Result<(), String> {
        simulate_input(input_type, params)
    }

    /// Get service health status
    pub async fn get_service_health(&self, service_name: &str) -> bool {
        let health = self.service_health.read().await;
        *health.get(service_name).unwrap_or(&false)
    }

    /// Update service health status
    pub async fn update_service_health(&self, service_name: &str, is_healthy: bool) {
        let mut health = self.service_health.write().await;
        health.insert(service_name.to_string(), is_healthy);
    }

    /// Get uptime in seconds
    pub fn get_uptime(&self) -> i64 {
        START_TIME.elapsed().as_secs() as i64
    }

    /// Get execution statistics and configuration
    pub fn get_stats(&self) -> HashMap<String, String> {
        get_execution_stats()
    }

    /// Get service dependencies status
    pub fn get_dependencies(&self) -> HashMap<String, String> {
        let mut dependencies = HashMap::new();

        #[cfg(target_os = "windows")]
        {
            dependencies.insert("windows_job_object".to_string(), "AVAILABLE".to_string());
            dependencies.insert("process_watchdog".to_string(), "AVAILABLE".to_string());
            dependencies.insert("sandbox_directory".to_string(), "CONFIGURED".to_string());
        }

        dependencies.insert("shell".to_string(), "AVAILABLE".to_string());
        dependencies.insert("input_simulation".to_string(), "AVAILABLE".to_string());

        dependencies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_health_tracking() {
        let executor = Executor::new();

        // Test initial health state
        assert!(executor.get_service_health("shell").await);

        // Test health update
        executor.update_service_health("shell", false).await;
        assert!(!executor.get_service_health("shell").await);
    }

    #[tokio::test]
    async fn test_execute_command() {
        let executor = Executor::new();
        let result = executor
            .execute_command("echo", &["Hello".to_string()], &HashMap::new())
            .await;

        assert!(result.is_ok());
        let (stdout, stderr, exit_code) = result.unwrap();
        assert!(stdout.contains("Hello"));
        assert!(stderr.is_empty());
        assert_eq!(exit_code, 0);
    }
}
