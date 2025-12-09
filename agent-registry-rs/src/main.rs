// agent-registry-rs/src/main.rs
// Agent Registry Service - Agent management and lookup
// Port 50067

use tonic::{transport::Server, Request, Response, Status};
use std::sync::Arc;
use std::time::Instant;
use std::net::SocketAddr;
use std::env;
use std::collections::HashMap;
use std::path::Path;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use serde::Deserialize;

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    agent_registry_service_server::{AgentRegistryService, AgentRegistryServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    RegisterAgentRequest,
    RegisterAgentResponse,
    GetAgentRequest,
    GetAgentResponse,
    ListAgentsRequest,
    ListAgentsResponse,
    AgentInfo,
    HealthRequest,
    HealthResponse,
};

/// Configuration file structure for agent_registry.toml
#[derive(Debug, Deserialize)]
struct AgentConfig {
    agent: Vec<AgentDefinition>,
}

#[derive(Debug, Deserialize, Clone)]
struct AgentDefinition {
    name: String,
    port: i32,
    role: String,
    capabilities: Vec<String>,
}

/// Internal agent entry with runtime status
#[derive(Debug, Clone)]
struct AgentEntry {
    agent_id: String,
    name: String,
    port: i32,
    role: String,
    capabilities: Vec<String>,
    status: String,
    metadata: HashMap<String, String>,
}

#[derive(Debug)]
pub struct AgentRegistryServer {
    agents: Arc<RwLock<HashMap<String, AgentEntry>>>,
}

impl Default for AgentRegistryServer {
    fn default() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl AgentRegistryServer {
    /// Load agents from config/agent_registry.toml
    pub async fn load_from_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = env::var("AGENT_REGISTRY_CONFIG")
            .unwrap_or_else(|_| "../config/agent_registry.toml".to_string());
        
        let path = Path::new(&config_path);
        
        if !path.exists() {
            log::warn!("Agent config file not found at {}, starting with empty registry", config_path);
            return Ok(());
        }
        
        let content = tokio::fs::read_to_string(path).await?;
        let config: AgentConfig = toml::from_str(&content)?;
        
        let mut agents = self.agents.write().await;
        
        for def in config.agent {
            let agent_id = uuid::Uuid::new_v4().to_string();
            log::info!("Loading agent from config: {} (port {})", def.name, def.port);
            
            let entry = AgentEntry {
                agent_id: agent_id.clone(),
                name: def.name.clone(),
                port: def.port,
                role: def.role,
                capabilities: def.capabilities,
                status: "OFFLINE".to_string(), // Start as offline until health check
                metadata: HashMap::new(),
            };
            
            agents.insert(def.name, entry);
        }
        
        log::info!("Loaded {} agents from config", agents.len());
        Ok(())
    }
}

#[tonic::async_trait]
impl AgentRegistryService for AgentRegistryServer {
    async fn register_agent(
        &self,
        request: Request<RegisterAgentRequest>,
    ) -> Result<Response<RegisterAgentResponse>, Status> {
        let req = request.into_inner();
        
        log::info!("RegisterAgent: name='{}', port={}", req.name, req.port);
        
        let agent_id = uuid::Uuid::new_v4().to_string();
        
        let entry = AgentEntry {
            agent_id: agent_id.clone(),
            name: req.name.clone(),
            port: req.port,
            role: req.role,
            capabilities: req.capabilities,
            status: "ONLINE".to_string(),
            metadata: req.metadata,
        };
        
        let mut agents = self.agents.write().await;
        agents.insert(req.name.clone(), entry);
        
        log::info!("Agent registered: {} -> {}", req.name, agent_id);
        
        Ok(Response::new(RegisterAgentResponse {
            success: true,
            agent_id,
            error: String::new(),
        }))
    }
    
    async fn get_agent(
        &self,
        request: Request<GetAgentRequest>,
    ) -> Result<Response<GetAgentResponse>, Status> {
        let req = request.into_inner();
        let agents = self.agents.read().await;
        
        // Lookup by name first
        if !req.name.is_empty() {
            if let Some(entry) = agents.get(&req.name) {
                return Ok(Response::new(GetAgentResponse {
                    found: true,
                    agent: Some(AgentInfo {
                        agent_id: entry.agent_id.clone(),
                        name: entry.name.clone(),
                        port: entry.port,
                        role: entry.role.clone(),
                        capabilities: entry.capabilities.clone(),
                        status: entry.status.clone(),
                        metadata: entry.metadata.clone(),
                    }),
                }));
            }
        }
        
        // Lookup by capability
        if !req.capability.is_empty() {
            for entry in agents.values() {
                if entry.capabilities.contains(&req.capability) {
                    return Ok(Response::new(GetAgentResponse {
                        found: true,
                        agent: Some(AgentInfo {
                            agent_id: entry.agent_id.clone(),
                            name: entry.name.clone(),
                            port: entry.port,
                            role: entry.role.clone(),
                            capabilities: entry.capabilities.clone(),
                            status: entry.status.clone(),
                            metadata: entry.metadata.clone(),
                        }),
                    }));
                }
            }
        }
        
        Ok(Response::new(GetAgentResponse {
            found: false,
            agent: None,
        }))
    }
    
    async fn list_agents(
        &self,
        request: Request<ListAgentsRequest>,
    ) -> Result<Response<ListAgentsResponse>, Status> {
        let req = request.into_inner();
        let agents = self.agents.read().await;
        
        let result: Vec<AgentInfo> = agents
            .values()
            .filter(|a| {
                // Apply capability filter
                if !req.capability_filter.is_empty() {
                    if !a.capabilities.contains(&req.capability_filter) {
                        return false;
                    }
                }
                // Apply status filter
                if !req.status_filter.is_empty() {
                    if a.status != req.status_filter {
                        return false;
                    }
                }
                true
            })
            .map(|a| AgentInfo {
                agent_id: a.agent_id.clone(),
                name: a.name.clone(),
                port: a.port,
                role: a.role.clone(),
                capabilities: a.capabilities.clone(),
                status: a.status.clone(),
                metadata: a.metadata.clone(),
            })
            .collect();
        
        let total = result.len() as i32;
        
        Ok(Response::new(ListAgentsResponse {
            agents: result,
            total_count: total,
        }))
    }
}

#[tonic::async_trait]
impl HealthService for AgentRegistryServer {
    async fn get_health(&self, _request: Request<HealthRequest>) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let agents = self.agents.read().await;
        let online_count = agents.values().filter(|a| a.status == "ONLINE").count();
        
        let mut dependencies = HashMap::new();
        dependencies.insert("registry_engine".to_string(), "ACTIVE".to_string());
        dependencies.insert("registered_agents".to_string(), agents.len().to_string());
        dependencies.insert("online_agents".to_string(), online_count.to_string());
        
        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "agent-registry-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let addr_str = env::var("AGENT_REGISTRY_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50067".to_string());
    
    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str.strip_prefix("http://").unwrap_or(&addr_str).parse()?
    } else {
        addr_str.parse()?
    };

    let _ = *START_TIME;

    let registry_server = Arc::new(AgentRegistryServer::default());
    
    // Load agents from config file
    if let Err(e) = registry_server.load_from_config().await {
        log::warn!("Failed to load agent config: {}. Starting with empty registry.", e);
    }
    
    let reg_for_health = registry_server.clone();

    log::info!("Agent Registry Service starting on {}", addr);
    println!("Agent Registry Service listening on {}", addr);

    Server::builder()
        .add_service(AgentRegistryServiceServer::from_arc(registry_server))
        .add_service(HealthServiceServer::from_arc(reg_for_health))
        .serve(addr)
        .await?;

    Ok(())
}
