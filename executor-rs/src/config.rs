#![allow(unused_imports)]
#![allow(dead_code)]

//! # Executor Service Configuration with Hot-Reloading
//!
//! This module provides comprehensive configuration management for the executor service
//! with support for hot-reloading, validation, and environment-based overrides.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use config_management_rs::{
    ConfigManager, ConfigChange, ConfigError, ConfigSource, ConfigValidator, ConfigBuilder,
    examples::example_executor_config,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Executor service configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutorConfig {
    /// Sandbox directory path
    pub sandbox_dir: String,

    /// Maximum memory usage in MB
    pub max_memory_mb: u64,

    /// Maximum CPU usage percentage
    pub max_cpu_percent: u32,

    /// Execution timeout in seconds
    pub execution_timeout_seconds: u64,

    /// Maximum concurrent processes
    pub max_processes: u32,

    /// Allowed commands list
    pub allowed_commands: Vec<String>,

    /// Resource monitoring interval in milliseconds
    pub resource_monitoring_interval_ms: u64,

    /// Enable low integrity level sandboxing
    pub enable_low_integrity: bool,

    /// Enable process watchdog
    pub enable_watchdog: bool,

    /// Enable detailed resource logging
    pub enable_resource_logging: bool,
}

/// Default configuration implementation
impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            sandbox_dir: r"C:\phoenix_sandbox".to_string(),
            max_memory_mb: 512,
            max_cpu_percent: 50,
            execution_timeout_seconds: 10,
            max_processes: 5,
            allowed_commands: vec![
                "python".to_string(),
                "python3".to_string(),
                "cmd".to_string(),
                "powershell".to_string(),
            ],
            resource_monitoring_interval_ms: 100,
            enable_low_integrity: true,
            enable_watchdog: true,
            enable_resource_logging: true,
        }
    }
}

/// Configuration validator implementation
impl ConfigValidator for ExecutorConfig {
    fn validate(&self) -> Result<(), String> {
        // Validate sandbox directory
        if self.sandbox_dir.is_empty() {
            return Err("Sandbox directory cannot be empty".to_string());
        }

        // Validate memory limits
        if self.max_memory_mb < 10 || self.max_memory_mb > 4096 {
            return Err("Memory limit must be between 10MB and 4096MB".to_string());
        }

        // Validate CPU limits
        if self.max_cpu_percent < 10 || self.max_cpu_percent > 100 {
            return Err("CPU limit must be between 10% and 100%".to_string());
        }

        // Validate timeout
        if self.execution_timeout_seconds < 1 || self.execution_timeout_seconds > 300 {
            return Err("Timeout must be between 1 and 300 seconds".to_string());
        }

        // Validate process count
        if self.max_processes < 1 || self.max_processes > 20 {
            return Err("Process count must be between 1 and 20".to_string());
        }

        // Validate allowed commands
        if self.allowed_commands.is_empty() {
            return Err("At least one command must be allowed".to_string());
        }

        Ok(())
    }
}

/// Global configuration manager
#[derive(Debug, Clone)]
pub struct GlobalConfigManager {
    inner: Arc<Mutex<ConfigManager<ExecutorConfig>>>,
}

/// Initialize the global configuration manager
pub async fn init_config_manager() -> Result<GlobalConfigManager, ConfigError> {
    info!("Initializing executor configuration manager");

    // Determine configuration file path
    let config_path = get_config_path();

    // Create default configuration
    let default_config = ExecutorConfig::default();

    // Create configuration manager
    let config_manager = ConfigManager::new(config_path, default_config).await?;

    // Start watching for changes
    config_manager.start_watching()?;

    info!("Executor configuration manager initialized successfully");

    Ok(GlobalConfigManager {
        inner: Arc::new(Mutex::new(config_manager)),
    })
}

/// Get the configuration file path
fn get_config_path() -> PathBuf {
    // Try to get from environment variable first
    if let Ok(path) = std::env::var("EXECUTOR_CONFIG_PATH") {
        return PathBuf::from(path);
    }

    // Fallback to default location
    PathBuf::from("config/executor.json")
}

/// Get the global configuration manager instance
pub fn get_config_manager() -> Option<GlobalConfigManager> {
    static mut GLOBAL_CONFIG: Option<GlobalConfigManager> = None;
    static INIT: std::sync::Once = std::sync::Once::new();

    unsafe {
        INIT.call_once(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            GLOBAL_CONFIG = Some(runtime.block_on(init_config_manager()).expect("Failed to initialize config manager"));
        });

        GLOBAL_CONFIG.clone()
    }
}

/// Get current configuration
pub async fn get_config() -> ExecutorConfig {
    if let Some(manager) = get_config_manager() {
        manager.get_config().await
    } else {
        // Fallback to default if manager not initialized
        warn!("Configuration manager not initialized, using defaults");
        ExecutorConfig::default()
    }
}

/// Update configuration
pub async fn update_config(config: ExecutorConfig) -> Result<(), ConfigError> {
    if let Some(manager) = get_config_manager() {
        manager.update_config(config, ConfigSource::Environment).await
    } else {
        Err(ConfigError::AccessError("Configuration manager not initialized".to_string()))
    }
}

/// Subscribe to configuration changes
pub fn subscribe_to_changes() -> tokio::sync::broadcast::Receiver<ConfigChange<ExecutorConfig>> {
    if let Some(manager) = get_config_manager() {
        manager.subscribe_to_changes()
    } else {
        // Create a dummy receiver if manager not initialized
        let (_, receiver) = tokio::sync::broadcast::channel(1);
        receiver
    }
}

/// Configuration change handler
pub async fn handle_config_change(change: ConfigChange<ExecutorConfig>) -> Result<(), ConfigError> {
    info!(
        "Executor configuration changed from {:?} to {:?}",
        change.source, change.new_config
    );

    // Apply the new configuration
    apply_config(&change.new_config).await?;

    Ok(())
}

/// Apply configuration changes
pub async fn apply_config(config: &ExecutorConfig) -> Result<(), ConfigError> {
    info!("Applying new executor configuration");

    // Update environment variables
    std::env::set_var("EXECUTOR_SANDBOX_DIR", &config.sandbox_dir);
    std::env::set_var("EXECUTOR_MAX_MEMORY_MB", config.max_memory_mb.to_string());
    std::env::set_var("EXECUTOR_MAX_CPU_PERCENT", config.max_cpu_percent.to_string());
    std::env::set_var("EXECUTOR_TIMEOUT_SECONDS", config.execution_timeout_seconds.to_string());
    std::env::set_var("EXECUTOR_MAX_PROCESSES", config.max_processes.to_string());

    // Log the allowed commands
    debug!("Allowed commands: {:?}", config.allowed_commands);

    info!("Configuration applied successfully");
    Ok(())
}

/// Load configuration from environment variables
pub fn load_env_config() -> ExecutorConfig {
    let mut config = ExecutorConfig::default();

    // Override from environment variables
    if let Ok(dir) = std::env::var("EXECUTOR_SANDBOX_DIR") {
        config.sandbox_dir = dir;
    }

    if let Ok(mem) = std::env::var("EXECUTOR_MAX_MEMORY_MB") {
        if let Ok(mem_val) = mem.parse() {
            config.max_memory_mb = mem_val;
        }
    }

    if let Ok(cpu) = std::env::var("EXECUTOR_MAX_CPU_PERCENT") {
        if let Ok(cpu_val) = cpu.parse() {
            config.max_cpu_percent = cpu_val;
        }
    }

    if let Ok(timeout) = std::env::var("EXECUTOR_TIMEOUT_SECONDS") {
        if let Ok(timeout_val) = timeout.parse() {
            config.execution_timeout_seconds = timeout_val;
        }
    }

    if let Ok(processes) = std::env::var("EXECUTOR_MAX_PROCESSES") {
        if let Ok(processes_val) = processes.parse() {
            config.max_processes = processes_val;
        }
    }

    if let Ok(commands) = std::env::var("EXECUTOR_ALLOWED_COMMANDS") {
        config.allowed_commands = commands
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    config
}

/// Save configuration to file
pub async fn save_config(config: &ExecutorConfig) -> Result<(), ConfigError> {
    let config_manager = if let Some(manager) = get_config_manager() {
        manager
    } else {
        return Err(ConfigError::AccessError("Configuration manager not initialized".to_string()));
    };

    config_manager.save_config(config).await
}

/// Configuration health check
pub async fn check_config_health() -> bool {
    match get_config().await.validate() {
        Ok(_) => true,
        Err(e) => {
            error!("Configuration health check failed: {}", e);
            false
        }
    }
}

/// Configuration migration utilities
pub mod migration {
    use super::*;

    /// Migrate from old configuration format
    pub async fn migrate_old_config(old_config: ExecutorConfig) -> ExecutorConfig {
        // Add any missing fields with default values
        ExecutorConfig {
            resource_monitoring_interval_ms: old_config.resource_monitoring_interval_ms,
            enable_low_integrity: old_config.enable_low_integrity,
            enable_watchdog: old_config.enable_watchdog,
            enable_resource_logging: old_config.enable_resource_logging,
            ..old_config
        }
    }
}

/// Configuration monitoring
pub async fn start_config_monitoring() {
    let mut receiver = subscribe_to_changes();

    tokio::spawn(async move {
        while let Ok(change) = receiver.recv().await {
            if let Err(e) = handle_config_change(change).await {
                error!("Failed to handle config change: {}", e);
            }
        }
    });
}

/// Configuration utilities
pub mod utils {
    use super::*;

    /// Get sandbox directory from configuration
    pub async fn get_sandbox_dir() -> String {
        get_config().await.sandbox_dir
    }

    /// Get maximum memory limit
    pub async fn get_max_memory() -> u64 {
        get_config().await.max_memory_mb
    }

    /// Get maximum CPU limit
    pub async fn get_max_cpu() -> u32 {
        get_config().await.max_cpu_percent
    }

    /// Get execution timeout
    pub async fn get_timeout() -> u64 {
        get_config().await.execution_timeout_seconds
    }

    /// Check if command is allowed
    pub async fn is_command_allowed(command: &str) -> bool {
        let config = get_config().await;
        config.allowed_commands.iter().any(|cmd| cmd == command)
    }
}

/// Configuration testing utilities
#[cfg(test)]
pub mod test_utils {
    use super::*;
    use config_management_rs::test_utils::create_test_config;

    /// Create a test configuration
    pub async fn create_test_config_manager() -> ConfigManager<ExecutorConfig> {
        let config = ExecutorConfig::default();
        create_test_config(&config).await.0
    }

    /// Create a test configuration with custom values
    pub async fn create_custom_test_config(
        sandbox_dir: &str,
        max_memory: u64,
    ) -> ConfigManager<ExecutorConfig> {
        let mut config = ExecutorConfig::default();
        config.sandbox_dir = sandbox_dir.to_string();
        config.max_memory_mb = max_memory;
        create_test_config(&config).await.0
    }
}

/// Configuration examples
pub mod examples {
    use super::*;

    /// Production configuration example
    pub fn production_config() -> ExecutorConfig {
        ExecutorConfig {
            sandbox_dir: r"C:\phoenix_sandbox".to_string(),
            max_memory_mb: 1024,
            max_cpu_percent: 75,
            execution_timeout_seconds: 30,
            max_processes: 10,
            allowed_commands: vec![
                "python".to_string(),
                "python3".to_string(),
                "cmd".to_string(),
                "powershell".to_string(),
                "git".to_string(),
            ],
            resource_monitoring_interval_ms: 50,
            enable_low_integrity: true,
            enable_watchdog: true,
            enable_resource_logging: true,
        }
    }

    /// Development configuration example
    pub fn development_config() -> ExecutorConfig {
        ExecutorConfig {
            sandbox_dir: r"C:\phoenix_sandbox_dev".to_string(),
            max_memory_mb: 256,
            max_cpu_percent: 30,
            execution_timeout_seconds: 5,
            max_processes: 3,
            allowed_commands: vec![
                "python".to_string(),
                "python3".to_string(),
                "cmd".to_string(),
            ],
            resource_monitoring_interval_ms: 200,
            enable_low_integrity: false,
            enable_watchdog: true,
            enable_resource_logging: true,
        }
    }
}

/// Configuration macros
#[macro_export]
macro_rules! executor_config {
    () => {
        $crate::config::get_config()
    };
}

#[macro_export]
macro_rules! sandbox_dir {
    () => {
        $crate::config::utils::get_sandbox_dir()
    };
}