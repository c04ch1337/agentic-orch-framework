// agent-registry-rs/src/main.rs
// Agent Registry Service - Agent management and lookup
// Port 50070

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response, Status};
use tonic_health::pb::health_client::HealthClient;
use tonic_health::pb::HealthCheckRequest;

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    agent_registry_service_server::{AgentRegistryService, AgentRegistryServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    GetAgentRequest, GetAgentResponse, GetAvailableCapabilitiesRequest,
    GetAvailableCapabilitiesResponse, HealthRequest, HealthResponse, ListAgentsRequest,
    ListAgentsResponse, RegisterAgentRequest, RegisterAgentResponse,
};

//// Configuration file structure for agent_registry.toml
#[derive(Debug, Deserialize)]
struct AgentConfig {
    /// List of statically configured agents. Defaults to empty so the
    /// registry can start cleanly when no agents are defined.
    #[serde(default)]
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
pub struct AgentEntry {
    agent_id: String,
    name: String,
    port: i32,
    role: String,
    capabilities: Vec<String>,
    status: String,
    metadata: HashMap<String, String>,
    verified: bool, // Track whether agent health has been verified
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
    /// Verify the health of an agent using the gRPC Health Checking Protocol
    /// Returns true if the agent is healthy (status is SERVING), false otherwise
    async fn verify_agent_health(
        &self,
        endpoint: &str,
        service_name: &str,
        timeout_seconds: u64,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Create health client with timeout
        let timeout = std::time::Duration::from_secs(timeout_seconds);
        let channel = tonic::transport::Channel::from_shared(format!("http://{}", endpoint))?
            .timeout(timeout)
            .connect()
            .await?;

        let mut client = HealthClient::new(channel);

        // Make health check request
        let request = tonic::Request::new(HealthCheckRequest {
            service: service_name.to_string(),
        });

        // Check status
        match client.check(request).await {
            Ok(response) => {
                let status = response.into_inner().status;
                // Health check response status 1 = SERVING
                Ok(status == 1)
            }
            Err(e) => {
                log::warn!("Health check failed for {}: {}", endpoint, e);
                Ok(false)
            }
        }
    }

    /// Load agents from config/agent_registry.toml without verification
    pub async fn load_agents_from_config(&self) -> Vec<AgentEntry> {
        let config_path = env::var("AGENT_REGISTRY_CONFIG")
            .unwrap_or_else(|_| "../config/agent_registry.toml".to_string());

        let path = Path::new(&config_path);

        if !path.exists() {
            log::warn!(
                "Agent config file not found at {}, starting with empty registry",
                config_path
            );
            return Vec::new();
        }

        let content = match tokio::fs::read_to_string(path).await {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to read config file: {}", e);
                return Vec::new();
            }
        };

        let config: AgentConfig = match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                log::error!("Failed to parse config file: {}", e);
                return Vec::new();
            }
        };

        let mut agents = Vec::new();

        for def in config.agent {
            let agent_id = uuid::Uuid::new_v4().to_string();
            log::info!(
                "Loading agent from config: {} (port {})",
                def.name,
                def.port
            );

            let entry = AgentEntry {
                agent_id: agent_id.clone(),
                name: def.name.clone(),
                port: def.port,
                role: def.role,
                capabilities: def.capabilities,
                status: "OFFLINE".to_string(), // Start as offline until health check
                metadata: HashMap::new(),
                verified: false, // Start as unverified
            };

            agents.push(entry);
        }

        log::info!("Loaded {} agents from config", agents.len());
        agents
    }

    /// Load agents from config and verify their health
    pub async fn load_and_verify_agents(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Load from config
        let agents = self.load_agents_from_config().await;

        if agents.is_empty() {
            log::warn!("No agents loaded from configuration");
            return Ok(());
        }

        let mut verified_count = 0;
        let mut unverified_count = 0;
        let mut verified_agents = self.agents.write().await;

        for agent in agents {
            let endpoint = format!("localhost:{}", agent.port);

            // Perform health check verification
            match self.verify_agent_health(&endpoint, &agent.name, 5).await {
                Ok(true) => {
                    // Agent is healthy and verified
                    let mut verified_agent = agent.clone();
                    verified_agent.status = "ONLINE".to_string();
                    verified_agent.verified = true;
                    verified_agents.insert(agent.name.clone(), verified_agent);
                    log::info!("Agent {} verified and marked as ONLINE", agent.name);
                    verified_count += 1;
                }
                _ => {
                    // Store in registry but marked as unverified
                    let unverified_agent = agent.clone();
                    verified_agents.insert(agent.name.clone(), unverified_agent);
                    log::warn!(
                        "Agent {} failed health verification, marked as unverified",
                        agent.name
                    );
                    unverified_count += 1;
                }
            }
        }

        log::info!(
            "Agent verification complete: {} verified, {} unverified",
            verified_count,
            unverified_count
        );

        Ok(())
    }
}

#[tonic::async_trait]
impl AgentRegistryService for AgentRegistryServer {
    async fn get_available_capabilities(
        &self,
        request: Request<GetAvailableCapabilitiesRequest>,
    ) -> Result<Response<GetAvailableCapabilitiesResponse>, Status> {
        log::debug!(
            "GetAvailableCapabilities request received from {:?}",
            request.remote_addr()
        );

        let mut capabilities = HashSet::new();
        let mut metadata = HashMap::new();

        // Get all agents with timeout protection
        let agents =
            match tokio::time::timeout(std::time::Duration::from_secs(5), self.agents.read()).await
            {
                Ok(guard) => guard,
                Err(_) => {
                    log::error!("Timeout while acquiring agents read lock");
                    return Err(Status::internal("Failed to access agent registry"));
                }
            };

        let verified_count = agents.values().filter(|a| a.verified).count();
        log::debug!("Processing {} verified agents", verified_count);

        // Only include capabilities from verified agents
        for agent in agents.values() {
            if !agent.verified {
                log::trace!("Skipping unverified agent: {}", agent.name);
                continue;
            }

            log::trace!("Processing capabilities for agent: {}", agent.name);
            for cap in &agent.capabilities {
                capabilities.insert(cap.clone());

                // Add metadata if available
                if let Some(meta) = agent.metadata.get(cap) {
                    let mut required_params = Vec::new();
                    if let Some(params) = agent.metadata.get(&format!("{}_params", cap)) {
                        required_params = params
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }

                    metadata.insert(
                        cap.clone(),
                        agi_core::CapabilityMetadata {
                            description: meta.clone(),
                            required_params,
                            provider_agent: agent.name.clone(),
                        },
                    );
                    log::trace!("Added metadata for capability: {}", cap);
                }
            }
        }

        let total_capabilities = capabilities.len();
        let total_with_metadata = metadata.len();

        log::info!("GetAvailableCapabilities: found {} capabilities ({} with metadata) from {} verified agents",
                  total_capabilities, total_with_metadata, verified_count);

        if total_capabilities == 0 {
            log::warn!("No capabilities found from verified agents");
        }

        Ok(Response::new(GetAvailableCapabilitiesResponse {
            capabilities: capabilities.into_iter().collect(),
            metadata,
        }))
    }

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
            verified: true, // Directly registered agents are considered verified
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
                // Only return verified agents
                if entry.verified {
                    return Ok(Response::new(GetAgentResponse {
                        found: true,
                        agent: Some(agi_core::AgentInfo {
                            agent_id: entry.agent_id.clone(),
                            name: entry.name.clone(),
                            port: entry.port,
                            role: entry.role.clone(),
                            capabilities: entry.capabilities.clone(),
                            status: entry.status.clone(),
                            metadata: entry.metadata.clone(),
                        }),
                    }));
                } else {
                    log::debug!("Agent {} found but not verified", req.name);
                }
            }
        }

        // Lookup by capability
        if !req.capability.is_empty() {
            for entry in agents.values() {
                // Only consider verified agents
                if entry.verified && entry.capabilities.contains(&req.capability) {
                    return Ok(Response::new(GetAgentResponse {
                        found: true,
                        agent: Some(agi_core::AgentInfo {
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

        let result: Vec<agi_core::AgentInfo> = agents
            .values()
            .filter(|a| {
                // Only verified agents should be returned
                if !a.verified {
                    return false;
                }

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
            .map(|a| agi_core::AgentInfo {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[tokio::test]
    async fn test_get_available_capabilities() {
        let service = AgentRegistryServer::default();

        // Add a verified agent with capabilities and metadata
        let verified_agent = AgentEntry {
            agent_id: "test1".to_string(),
            name: "test_agent1".to_string(),
            port: 8001,
            role: "worker".to_string(),
            capabilities: vec!["cap1".to_string(), "cap2".to_string()],
            status: "ONLINE".to_string(),
            metadata: {
                let mut map = HashMap::new();
                map.insert("cap1".to_string(), "Description for cap1".to_string());
                map.insert("cap1_params".to_string(), "param1,param2".to_string());
                map.insert("cap2".to_string(), "Description for cap2".to_string());
                map
            },
            verified: true,
        };

        // Add an unverified agent - its capabilities should not appear
        let unverified_agent = AgentEntry {
            agent_id: "test2".to_string(),
            name: "test_agent2".to_string(),
            port: 8002,
            role: "worker".to_string(),
            capabilities: vec!["cap3".to_string()],
            status: "ONLINE".to_string(),
            metadata: HashMap::new(),
            verified: false,
        };

        // Insert test agents
        {
            let mut agents = service.agents.write().await;
            agents.insert(verified_agent.name.clone(), verified_agent);
            agents.insert(unverified_agent.name.clone(), unverified_agent);
        }

        // Test the GetAvailableCapabilities RPC
        let request = Request::new(GetAvailableCapabilitiesRequest {});
        let response = service.get_available_capabilities(request).await.unwrap();
        let result = response.into_inner();

        // Verify capabilities list
        let capabilities: HashSet<String> = result.capabilities.into_iter().collect();
        assert_eq!(capabilities.len(), 2);
        assert!(capabilities.contains("cap1"));
        assert!(capabilities.contains("cap2"));
        assert!(!capabilities.contains("cap3")); // Unverified agent's capability

        // Verify metadata
        assert_eq!(result.metadata.len(), 2);

        // Check cap1 metadata
        let cap1_meta = result.metadata.get("cap1").expect("cap1 metadata missing");
        assert_eq!(cap1_meta.description, "Description for cap1");
        assert_eq!(cap1_meta.required_params, vec!["param1", "param2"]);
        assert_eq!(cap1_meta.provider_agent, "test_agent1");

        // Check cap2 metadata
        let cap2_meta = result.metadata.get("cap2").expect("cap2 metadata missing");
        assert_eq!(cap2_meta.description, "Description for cap2");
        assert!(cap2_meta.required_params.is_empty());
        assert_eq!(cap2_meta.provider_agent, "test_agent1");
    }

    #[tokio::test]
    async fn test_get_available_capabilities_empty_registry() {
        let service = AgentRegistryServer::default();
        let request = Request::new(GetAvailableCapabilitiesRequest {});
        let response = service.get_available_capabilities(request).await.unwrap();
        let result = response.into_inner();

        assert!(result.capabilities.is_empty());
        assert!(result.metadata.is_empty());
    }

    #[tokio::test]
    async fn test_get_available_capabilities_timeout() {
        let service = AgentRegistryServer::default();

        // Simulate a slow lock by holding write lock
        let _write_lock = service.agents.write().await;

        // Try to get capabilities (should timeout)
        let request = Request::new(GetAvailableCapabilitiesRequest {});
        let result = service.get_available_capabilities(request).await;

        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::Internal);
            assert!(status.message().contains("Failed to access agent registry"));
        }
    }
}

#[tonic::async_trait]
impl HealthService for AgentRegistryServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
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

    let addr_str = env::var("AGENT_REGISTRY_ADDR").unwrap_or_else(|_| "0.0.0.0:50070".to_string());

    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str
            .strip_prefix("http://")
            .unwrap_or(&addr_str)
            .parse()?
    } else {
        addr_str.parse()?
    };

    let _ = *START_TIME;

    let registry_server = Arc::new(AgentRegistryServer::default());

    // Load and verify agents from config file
    if let Err(e) = registry_server.load_and_verify_agents().await {
        log::warn!(
            "Failed to load and verify agents: {}. Starting with empty registry.",
            e
        );
    }

    // Periodically verify agents health (could be done in a separate task)
    let verify_server = registry_server.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            log::debug!("Running periodic agent health verification...");
            if let Err(e) = verify_server.load_and_verify_agents().await {
                log::error!("Agent verification failed: {}", e);
            }
        }
    });

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
