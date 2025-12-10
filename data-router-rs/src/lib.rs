//! Data Router Library
//! Provides core routing and service communication functionality

mod circuit_breaker;
mod language_detector;
mod router;

pub use circuit_breaker::{CircuitBreaker, CircuitState, ProtectedServiceClient};
pub use language_detector::{detect_language, is_language, LanguageInfo};
pub use router::{AgentScopeManager, ScopeVerificationResult};

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};
use std::time::Instant;
use once_cell::sync::Lazy;

// Track service start time for uptime reporting
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

/// Core Data Router implementation
#[derive(Debug)]
pub struct DataRouter {
    // Enhanced Circuit Breaker for resilience
    circuit_breaker: Arc<CircuitBreaker>,
    // Service health state tracking
    service_health: Arc<RwLock<HashMap<String, bool>>>,
    // Agent scope manager for isolation enforcement
    agent_scope_manager: Arc<router::AgentScopeManager>,
}

impl DataRouter {
    /// Create a new DataRouter instance
    pub fn new() -> Self {
        // Create an enhanced circuit breaker with advanced features
        let circuit_breaker = Arc::new(CircuitBreaker::new());
        
        // Initialize service health tracking
        let service_health = Arc::new(RwLock::new(HashMap::new()));
        
        // Initialize agent scope manager for isolation
        let agent_scope_manager = Arc::new(router::AgentScopeManager::new());
        
        // Initialize known services as healthy
        let service_names = ["llm", "tools", "safety", "logging",
                         "mind-kb", "body-kb", "heart-kb", "social-kb", "soul-kb",
                         "context-manager"];
        for &service in &service_names {
            service_health.write().unwrap().insert(service.to_string(), true);
        }
        
        Self {
            circuit_breaker,
            service_health,
            agent_scope_manager,
        }
    }

    /// Get the circuit breaker instance
    pub fn circuit_breaker(&self) -> Arc<CircuitBreaker> {
        self.circuit_breaker.clone()
    }

    /// Get the agent scope manager instance
    pub fn agent_scope_manager(&self) -> Arc<AgentScopeManager> {
        self.agent_scope_manager.clone()
    }

    /// Get service health status
    pub async fn get_service_health(&self, service_name: &str) -> bool {
        let health_guard = self.service_health.read().await;
        *health_guard.get(service_name).unwrap_or(&false)
    }

    /// Update service health status
    pub async fn update_service_health(&self, service_name: &str, is_healthy: bool) {
        let mut health_guard = self.service_health.write().await;
        health_guard.insert(service_name.to_string(), is_healthy);
    }

    /// Get uptime in seconds
    pub fn get_uptime(&self) -> i64 {
        START_TIME.elapsed().as_secs() as i64
    }

    /// Helper method to get a client by service name
    pub fn get_service_name(&self, service_name: &str) -> Result<String, String> {
        match service_name {
            "llm-service" | "llm" => Ok("llm".to_string()),
            "tools-service" | "tools" => Ok("tools".to_string()),
            "safety-service" | "safety" => Ok("safety".to_string()),
            "logging-service" | "logging" => Ok("logging".to_string()),
            "mind-kb" | "mind" => Ok("mind-kb".to_string()),
            "body-kb" | "body" => Ok("body-kb".to_string()),
            "heart-kb" | "heart" => Ok("heart-kb".to_string()),
            "social-kb" | "social" => Ok("social-kb".to_string()),
            "soul-kb" | "soul" => Ok("soul-kb".to_string()),
            "context-manager" | "context" => Ok("context-manager".to_string()),
            _ => Err(format!("Unknown service: {}", service_name)),
        }
    }
}

impl Default for DataRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_health_tracking() {
        let router = DataRouter::new();
        
        // Test initial health state
        assert!(router.get_service_health("llm").await);
        
        // Test health update
        router.update_service_health("llm", false).await;
        assert!(!router.get_service_health("llm").await);
    }

    #[test]
    fn test_service_name_normalization() {
        let router = DataRouter::new();
        
        assert_eq!(router.get_service_name("llm-service").unwrap(), "llm");
        assert_eq!(router.get_service_name("mind-kb").unwrap(), "mind-kb");
        assert!(router.get_service_name("invalid-service").is_err());
    }
}