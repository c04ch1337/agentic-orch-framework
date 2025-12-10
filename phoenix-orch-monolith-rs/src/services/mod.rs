use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use agent_registry_rs::AgentRegistry;
use api_gateway_rs::ApiGateway;
use auth_service_rs::AuthService;
use body_kb_rs::BodyKnowledgeBase;
use context_manager_rs::ContextManager;
use curiosity_engine_rs::CuriosityEngine;
use data_router_rs::DataRouter;
use executor_rs::Executor;

#[derive(Debug)]
pub struct ServiceRegistry {
    agent_registry: Option<Arc<AgentRegistry>>,
    api_gateway: Option<Arc<ApiGateway>>,
    auth_service: Option<Arc<AuthService>>,
    body_kb: Option<Arc<BodyKnowledgeBase>>,
    context_manager: Option<Arc<ContextManager>>,
    curiosity_engine: Option<Arc<CuriosityEngine>>,
    data_router: Option<Arc<DataRouter>>,
    executor: Option<Arc<Executor>>,
    service_states: HashMap<Uuid, ServiceState>,
}

#[derive(Debug)]
struct ServiceState {
    name: String,
    status: ServiceStatus,
    last_health_check: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ServiceStatus {
    Starting,
    Running,
    Degraded,
    Failed,
    Stopping,
    Stopped,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            agent_registry: None,
            api_gateway: None,
            auth_service: None,
            body_kb: None,
            context_manager: None,
            curiosity_engine: None,
            data_router: None,
            executor: None,
            service_states: HashMap::new(),
        }
    }

    pub async fn initialize(&mut self, config: &config_rs::Config) -> crate::Result<()> {
        // Initialize core services
        self.init_agent_registry(config).await?;
        self.init_api_gateway(config).await?;
        self.init_auth_service(config).await?;
        self.init_body_kb(config).await?;
        self.init_context_manager(config).await?;
        self.init_curiosity_engine(config).await?;
        self.init_data_router(config).await?;
        self.init_executor(config).await?;

        Ok(())
    }

    async fn init_agent_registry(&mut self, config: &config_rs::Config) -> crate::Result<()> {
        let service = AgentRegistry::new(config).await?;
        let id = Uuid::new_v4();
        self.service_states.insert(id, ServiceState {
            name: "AgentRegistry".to_string(),
            status: ServiceStatus::Starting,
            last_health_check: chrono::Utc::now(),
        });
        self.agent_registry = Some(Arc::new(service));
        self.update_service_status(id, ServiceStatus::Running);
        Ok(())
    }

    async fn init_api_gateway(&mut self, config: &config_rs::Config) -> crate::Result<()> {
        let service = ApiGateway::new(config).await?;
        let id = Uuid::new_v4();
        self.service_states.insert(id, ServiceState {
            name: "ApiGateway".to_string(),
            status: ServiceStatus::Starting,
            last_health_check: chrono::Utc::now(),
        });
        self.api_gateway = Some(Arc::new(service));
        self.update_service_status(id, ServiceStatus::Running);
        Ok(())
    }

    // Similar init methods for other services...

    fn update_service_status(&mut self, id: Uuid, status: ServiceStatus) {
        if let Some(state) = self.service_states.get_mut(&id) {
            state.status = status;
            state.last_health_check = chrono::Utc::now();
        }
    }

    pub async fn health_check(&mut self) -> Vec<(String, ServiceStatus)> {
        let mut statuses = Vec::new();
        
        for (id, state) in &mut self.service_states {
            // Perform health check for each service
            let status = match state.name.as_str() {
                "AgentRegistry" => self.check_agent_registry().await,
                "ApiGateway" => self.check_api_gateway().await,
                // Add other services...
                _ => ServiceStatus::Unknown,
            };
            
            self.update_service_status(*id, status);
            statuses.push((state.name.clone(), status));
        }
        
        statuses
    }

    async fn check_agent_registry(&self) -> ServiceStatus {
        if let Some(service) = &self.agent_registry {
            // Perform actual health check
            ServiceStatus::Running
        } else {
            ServiceStatus::Failed
        }
    }

    async fn check_api_gateway(&self) -> ServiceStatus {
        if let Some(service) = &self.api_gateway {
            // Perform actual health check
            ServiceStatus::Running
        } else {
            ServiceStatus::Failed
        }
    }

    // Similar health check methods for other services...
}

// Add ServiceStatus::Unknown variant
impl ServiceStatus {
    const Unknown: ServiceStatus = ServiceStatus::Failed;
}