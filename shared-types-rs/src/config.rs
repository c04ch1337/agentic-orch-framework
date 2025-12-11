// shared-types-rs/src/config.rs
// Centralized configuration loader for Phoenix ORCH

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

static PHOENIX_CONFIG: OnceCell<Arc<PhoenixConfig>> = OnceCell::new();

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    #[error("Failed to parse configuration: {0}")]
    ParseError(String),

    #[error("Configuration not initialized")]
    NotInitialized,

    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PhoenixConfig {
    pub system: SystemConfig,
    pub vault: VaultConfig,
    pub services: ServicesConfig,
    pub llm: LlmConfig,
    pub agent: AgentConfig,
    pub storage: StorageConfig,
    pub mind_kb: MindKbConfig,
    pub body_kb: BodyKbConfig,
    pub heart_kb: HeartKbConfig,
    pub social_kb: SocialKbConfig,
    pub soul_kb: SoulKbConfig,
    pub persistence_kb: PersistenceKbConfig,
    pub safety: SafetyConfig,
    pub executor: ExecutorConfig,
    pub context_manager: ContextManagerConfig,
    pub reflection: ReflectionConfig,
    pub scheduler: SchedulerConfig,
    pub api_gateway: ApiGatewayConfig,
    pub auth: AuthConfig,
    pub monitoring: MonitoringConfig,
    pub features: FeaturesConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SystemConfig {
    pub environment: String,
    pub log_level: String,
    pub service_host: String,
    pub enable_tracing: bool,
    pub enable_metrics: bool,
    pub enable_recovery: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VaultConfig {
    pub address: String,
    pub secret_mount: String,
    pub token_ttl: u64,
    pub auto_rotate_secrets: bool,
    pub rotation_interval: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServicesConfig {
    pub orchestrator: u16,
    pub data_router: u16,
    pub llm: u16,
    pub tools: u16,
    pub executor: u16,
    pub safety: u16,
    pub secrets: u16,
    pub auth: u16,
    pub logging: u16,
    pub mind_kb: u16,
    pub body_kb: u16,
    pub heart_kb: u16,
    pub social_kb: u16,
    pub soul_kb: u16,
    pub persistence_kb: u16,
    pub context_manager: u16,
    pub reflection: u16,
    pub curiosity_engine: u16,
    pub scheduler: u16,
    pub agent_registry: u16,
    pub red_team: u16,
    pub blue_team: u16,
    pub api_gateway: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    pub provider: String,
    pub api_url: String,
    pub model: String,
    pub max_retries: u32,
    pub initial_retry_delay_ms: u64,
    pub max_retry_delay_ms: u64,
    pub request_timeout_secs: u64,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    pub name: String,
    pub purpose: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    pub qdrant: QdrantConfig,
    pub postgres: PostgresConfig,
    pub redis: RedisConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QdrantConfig {
    pub url: String,
    pub collection: String,
    pub vector_size: usize,
    pub distance_metric: String,
    pub auto_create_collection: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub max_connections: u32,
    pub connection_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub database: u8,
    pub connection_timeout_secs: u64,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MindKbConfig {
    pub decay_half_life_days: u32,
    pub decay_interval_secs: u64,
    pub max_retrieval_count: usize,
    pub min_relevance_score: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BodyKbConfig {
    pub retention_days: u32,
    pub polling_interval_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeartKbConfig {
    pub sentiment_model: String,
    pub emotion_window_hours: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SocialKbConfig {
    pub retention_days: u32,
    pub max_interactions_per_session: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SoulKbConfig {
    pub ethics_model_version: String,
    pub alignment_check_frequency_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PersistenceKbConfig {
    pub snapshot_interval_secs: u64,
    pub max_snapshots: usize,
    pub enable_compression: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SafetyConfig {
    pub enabled: bool,
    pub threat_sensitivity: String,
    pub additional_blocked_keywords: Vec<String>,
    pub additional_blocked_operations: Vec<String>,
    pub max_request_size_bytes: usize,
    pub rate_limit_requests_per_minute: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutorConfig {
    pub enable_job_objects: bool,
    pub max_cpu_percent: u32,
    pub max_memory_mb: u32,
    pub max_processes: u32,
    pub max_execution_time_secs: u64,
    pub python_executable: String,
    pub enable_python_sandbox: bool,
    pub allowed_shell_commands: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextManagerConfig {
    pub prompts: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReflectionConfig {
    pub enable_metacognition: bool,
    pub reflection_interval_secs: u64,
    pub max_reflection_depth: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchedulerConfig {
    pub enabled: bool,
    pub max_concurrent_tasks: u32,
    pub task_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiGatewayConfig {
    pub enable_tls: bool,
    pub tls_cert_path: String,
    pub tls_key_path: String,
    pub enable_mtls: bool,
    pub enable_cors: bool,
    pub cors_allowed_origins: Vec<String>,
    pub rate_limit_requests_per_minute: u32,
    pub rate_limit_burst: u32,
    pub enable_request_validation: bool,
    pub max_request_body_size_bytes: usize,
    pub api_keys_file: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub jwt_issuer: String,
    pub jwt_expiration_secs: u64,
    pub enable_token_refresh: bool,
    pub refresh_token_expiration_secs: u64,
    pub session_timeout_secs: u64,
    pub max_sessions_per_user: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    pub metrics_port: u16,
    pub metrics_path: String,
    pub health_check_interval_secs: u64,
    pub health_check_timeout_secs: u64,
    pub tracing: TracingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TracingConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub service_name: String,
    pub sample_rate: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeaturesConfig {
    pub enable_experimental: bool,
    pub enable_curiosity: bool,
    pub enable_multi_agent: bool,
    pub enable_emergency_resilience: bool,
    pub enable_auto_recovery: bool,
}

impl PhoenixConfig {
    /// Load configuration from file
    pub fn load() -> Result<Arc<PhoenixConfig>, ConfigError> {
        // Check if already loaded
        if let Some(config) = PHOENIX_CONFIG.get() {
            return Ok(Arc::clone(config));
        }

        // Get config file path from environment or use default
        let config_path =
            env::var("PHOENIX_CONFIG_PATH").unwrap_or_else(|_| "./config/phoenix.toml".to_string());

        let path = PathBuf::from(&config_path);

        if !path.exists() {
            return Err(ConfigError::FileNotFound(config_path));
        }

        // Read and parse config file
        let contents = fs::read_to_string(&path)?;
        let config: PhoenixConfig =
            toml::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        // Store in global static
        let config_arc = Arc::new(config);
        PHOENIX_CONFIG
            .set(Arc::clone(&config_arc))
            .map_err(|_| ConfigError::InvalidValue("Config already initialized".to_string()))?;

        Ok(config_arc)
    }

    /// Get the global configuration instance
    pub fn get() -> Result<Arc<PhoenixConfig>, ConfigError> {
        PHOENIX_CONFIG
            .get()
            .map(Arc::clone)
            .ok_or(ConfigError::NotInitialized)
    }

    /// Get service address for a given service
    pub fn get_service_address(&self, service_name: &str) -> String {
        let port = match service_name.to_lowercase().as_str() {
            "orchestrator" => self.services.orchestrator,
            "data_router" => self.services.data_router,
            "llm" => self.services.llm,
            "tools" => self.services.tools,
            "executor" => self.services.executor,
            "safety" => self.services.safety,
            "secrets" => self.services.secrets,
            "auth" => self.services.auth,
            "logging" => self.services.logging,
            "mind_kb" => self.services.mind_kb,
            "body_kb" => self.services.body_kb,
            "heart_kb" => self.services.heart_kb,
            "social_kb" => self.services.social_kb,
            "soul_kb" => self.services.soul_kb,
            "persistence_kb" => self.services.persistence_kb,
            "context_manager" => self.services.context_manager,
            "reflection" => self.services.reflection,
            "curiosity_engine" => self.services.curiosity_engine,
            "scheduler" => self.services.scheduler,
            "agent_registry" => self.services.agent_registry,
            "red_team" => self.services.red_team,
            "blue_team" => self.services.blue_team,
            "api_gateway" => self.services.api_gateway,
            _ => 50100, // Default port for unknown services
        };

        // Check for environment variable override
        let env_var = format!("{}_SERVICE_ADDR", service_name.to_uppercase());
        env::var(&env_var)
            .unwrap_or_else(|_| format!("http://{}:{}", self.system.service_host, port))
    }

    /// Get bind address for a service
    pub fn get_bind_address(&self, service_name: &str) -> String {
        let port = match service_name.to_lowercase().as_str() {
            "orchestrator" => self.services.orchestrator,
            "data_router" => self.services.data_router,
            "llm" => self.services.llm,
            "tools" => self.services.tools,
            "executor" => self.services.executor,
            "safety" => self.services.safety,
            "secrets" => self.services.secrets,
            "auth" => self.services.auth,
            "logging" => self.services.logging,
            "mind_kb" => self.services.mind_kb,
            "body_kb" => self.services.body_kb,
            "heart_kb" => self.services.heart_kb,
            "social_kb" => self.services.social_kb,
            "soul_kb" => self.services.soul_kb,
            "persistence_kb" => self.services.persistence_kb,
            "context_manager" => self.services.context_manager,
            "reflection" => self.services.reflection,
            "curiosity_engine" => self.services.curiosity_engine,
            "scheduler" => self.services.scheduler,
            "agent_registry" => self.services.agent_registry,
            "red_team" => self.services.red_team,
            "blue_team" => self.services.blue_team,
            "api_gateway" => self.services.api_gateway,
            _ => 50100,
        };

        format!("0.0.0.0:{}", port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_loading() {
        // This test requires the phoenix.toml file to exist
        // In a real scenario, you'd use a test fixture
        let result = PhoenixConfig::load();
        assert!(result.is_ok() || matches!(result, Err(ConfigError::FileNotFound(_))));
    }
}
