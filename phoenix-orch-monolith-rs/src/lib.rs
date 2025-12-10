//! Phoenix ORCH Monolithic Service
//! Integrates all microservices with enhanced resilience features

use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::path::PathBuf;
use anyhow::Result;
use metrics::{counter, gauge};

// Re-export service modules
pub use agent_registry;
pub use api_gateway;
pub use auth_service;
pub use body_kb;
pub use context_manager;
pub use curiosity_engine;
pub use data_router;
pub use executor;

mod resilience;
pub use resilience::{
    ProcessWatchdog,
    DataIntegrityManager,
    SecretManager,
    ResourceMonitor,
    FailureLogger,
    CriticalEvent,
};

/// Core service state and coordination
pub struct PhoenixOrch {
    // Core services
    agent_registry: Arc<agent_registry::AgentRegistry>,
    api_gateway: Arc<api_gateway::ApiGateway>,
    auth_service: Arc<auth_service::AuthService>,
    body_kb: Arc<body_kb::BodyKb>,
    context_manager: Arc<context_manager::ContextManager>,
    curiosity_engine: Arc<curiosity_engine::CuriosityEngine>,
    data_router: Arc<data_router::DataRouter>,
    executor: Arc<executor::Executor>,

    // Emergency resilience components
    process_watchdog: Arc<ProcessWatchdog>,
    data_integrity: Arc<DataIntegrityManager>,
    secret_manager: Arc<SecretManager>,
    resource_monitor: Arc<ResourceMonitor>,
    failure_logger: Arc<FailureLogger>,

    // Service health tracking
    service_health: Arc<RwLock<HashMap<String, bool>>>,
}

impl PhoenixOrch {
    /// Create new Phoenix ORCH instance
    pub async fn new() -> Result<Self> {
        // Initialize emergency resilience components first
        let process_watchdog = Arc::new(ProcessWatchdog::new());
        let data_integrity = Arc::new(DataIntegrityManager::new()?);
        let secret_manager = Arc::new(SecretManager::new().await?);
        let resource_monitor = Arc::new(ResourceMonitor::new());
        let failure_logger = Arc::new(FailureLogger::new()?);

        // Initialize core services with resilience wrappers
        let agent_registry = Arc::new(agent_registry::AgentRegistry::new());
        let api_gateway = Arc::new(api_gateway::ApiGateway::new());
        let auth_service = Arc::new(auth_service::AuthService::new());
        let body_kb = Arc::new(body_kb::BodyKb::new());
        let context_manager = Arc::new(context_manager::ContextManager::new());
        let curiosity_engine = Arc::new(curiosity_engine::CuriosityEngine::new());
        let data_router = Arc::new(data_router::DataRouter::new());
        let executor = Arc::new(executor::Executor::new());

        // Initialize service health tracking
        let service_health = Arc::new(RwLock::new(HashMap::new()));
        {
            let mut health = service_health.write().await;
            for service in ["agent_registry", "api_gateway", "auth_service", "body_kb",
                          "context_manager", "curiosity_engine", "data_router", "executor"] {
                health.insert(service.to_string(), true);
            }
        }

        Ok(Self {
            agent_registry,
            api_gateway,
            auth_service,
            body_kb,
            context_manager,
            curiosity_engine,
            data_router,
            executor,
            process_watchdog,
            data_integrity,
            secret_manager,
            resource_monitor,
            failure_logger,
            service_health,
        })
    }

    /// Start all services with emergency resilience monitoring
    pub async fn start(&self) -> Result<()> {
        log::info!("Starting Phoenix ORCH with emergency resilience features");

        // Start resource monitoring
        self.resource_monitor.start();

        // Create initial data snapshot
        self.data_integrity.create_snapshot().await?;

        // Start process watchdog
        self.process_watchdog.start_monitoring();

        // Initialize secret rotation
        self.secret_manager.start_rotation_schedule().await?;

        Ok(())
    }

    /// Stop all services gracefully
    pub async fn stop(&self) -> Result<()> {
        log::info!("Stopping Phoenix ORCH services");

        // Create final snapshot before shutdown
        self.data_integrity.create_snapshot().await?;

        // Stop monitoring
        self.resource_monitor.stop();
        self.process_watchdog.stop_monitoring();

        Ok(())
    }

    /// Handle critical failure with rollback
    pub async fn handle_critical_failure(&self, event: CriticalEvent) -> Result<()> {
        log::error!("Handling critical failure: {:?}", event);

        // Log the critical event
        self.failure_logger.log_critical_event(&event).await?;

        // Stop affected processes
        self.process_watchdog.emergency_terminate().await?;

        // Rollback to last known good state
        self.data_integrity.rollback_to_last_snapshot().await?;

        // Rotate compromised secrets if needed
        if event.requires_secret_rotation() {
            self.secret_manager.emergency_rotation().await?;
        }

        Ok(())
    }

    /// Get service health status
    pub async fn get_service_health(&self) -> HashMap<String, bool> {
        self.service_health.read().await.clone()
    }

    /// Update service health status
    pub async fn update_service_health(&self, service: &str, is_healthy: bool) {
        let mut health = self.service_health.write().await;
        health.insert(service.to_string(), is_healthy);

        // Update metrics
        gauge!("service_health", if is_healthy { 1.0 } else { 0.0 }, "service" => service.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_initialization() {
        let orch = PhoenixOrch::new().await.unwrap();
        let health = orch.get_service_health().await;
        assert!(health.values().all(|&healthy| healthy));
    }

    #[tokio::test]
    async fn test_critical_failure_handling() {
        let orch = PhoenixOrch::new().await.unwrap();
        
        let event = CriticalEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            severity: resilience::Severity::Critical,
            category: resilience::FailureCategory::ResourceExhaustion,
            description: "Memory usage exceeded 50%".to_string(),
            affected_services: vec!["executor".to_string()],
        };

        orch.handle_critical_failure(event).await.unwrap();
        
        // Verify executor service was marked unhealthy
        let health = orch.get_service_health().await;
        assert!(!health.get("executor").unwrap());
    }
}