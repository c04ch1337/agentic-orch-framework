use std::sync::Arc;
use std::collections::HashMap;
use std::path::Path;
use std::env;
use std::time::Instant;
use tokio::sync::RwLock;
use serde::Deserialize;
use once_cell::sync::Lazy;
use uuid::Uuid;

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

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
    verified: bool,  // Track whether agent health has been verified
}

#[derive(Debug)]
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<String, AgentEntry>>>,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load agents from config/agent_registry.toml without verification
    pub async fn load_agents_from_config(&self) -> Vec<AgentEntry> {
        let config_path = env::var("AGENT_REGISTRY_CONFIG")
            .unwrap_or_else(|_| "../config/agent_registry.toml".to_string());
        
        let path = Path::new(&config_path);
        
        if !path.exists() {
            tracing::warn!("Agent config file not found at {}, starting with empty registry", config_path);
            return Vec::new();
        }
        
        let content = match tokio::fs::read_to_string(path).await {
            Ok(content) => content,
            Err(e) => {
                tracing::error!("Failed to read config file: {}", e);
                return Vec::new();
            }
        };
        
        let config: AgentConfig = match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                tracing::error!("Failed to parse config file: {}", e);
                return Vec::new();
            }
        };
        
        let mut agents = Vec::new();
        
        for def in config.agent {
            let agent_id = Uuid::new_v4().to_string();
            tracing::info!("Loading agent from config: {} (port {})", def.name, def.port);
            
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
        
        tracing::info!("Loaded {} agents from config", agents.len());
        agents
    }

    pub async fn register_agent(
        &self,
        name: String,
        port: i32,
        role: String,
        capabilities: Vec<String>,
        metadata: HashMap<String, String>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let agent_id = Uuid::new_v4().to_string();
        
        let entry = AgentEntry {
            agent_id: agent_id.clone(),
            name: name.clone(),
            port,
            role,
            capabilities,
            status: "ONLINE".to_string(),
            metadata,
            verified: true,  // Directly registered agents are considered verified
        };
        
        let mut agents = self.agents.write().await;
        agents.insert(name.clone(), entry);
        
        tracing::info!("Agent registered: {} -> {}", name, agent_id);
        
        Ok(agent_id)
    }

    pub async fn get_agent_by_name(&self, name: &str) -> Option<AgentEntry> {
        let agents = self.agents.read().await;
        agents.get(name).cloned()
    }

    pub async fn get_agent_by_capability(&self, capability: &str) -> Option<AgentEntry> {
        let agents = self.agents.read().await;
        agents.values()
            .find(|a| a.verified && a.capabilities.contains(&capability.to_string()))
            .cloned()
    }

    pub async fn list_agents(
        &self,
        capability_filter: Option<String>,
        status_filter: Option<String>,
    ) -> Vec<AgentEntry> {
        let agents = self.agents.read().await;
        
        agents.values()
            .filter(|a| {
                // Only verified agents should be returned
                if !a.verified {
                    return false;
                }
                
                // Apply capability filter
                if let Some(cap) = &capability_filter {
                    if !a.capabilities.contains(cap) {
                        return false;
                    }
                }
                
                // Apply status filter
                if let Some(status) = &status_filter {
                    if &a.status != status {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect()
    }

    pub async fn get_available_capabilities(&self) -> (Vec<String>, HashMap<String, String>) {
        let agents = self.agents.read().await;
        let mut capabilities = Vec::new();
        let mut metadata = HashMap::new();
        
        for agent in agents.values() {
            if !agent.verified {
                continue;
            }
            
            for cap in &agent.capabilities {
                if !capabilities.contains(cap) {
                    capabilities.push(cap.clone());
                    
                    // Add metadata if available
                    if let Some(meta) = agent.metadata.get(cap) {
                        metadata.insert(cap.clone(), meta.clone());
                    }
                }
            }
        }
        
        (capabilities, metadata)
    }

    pub async fn get_health(&self) -> (bool, i64, HashMap<String, String>) {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let agents = self.agents.read().await;
        let online_count = agents.values().filter(|a| a.status == "ONLINE").count();
        
        let mut dependencies = HashMap::new();
        dependencies.insert("registry_engine".to_string(), "ACTIVE".to_string());
        dependencies.insert("registered_agents".to_string(), agents.len().to_string());
        dependencies.insert("online_agents".to_string(), online_count.to_string());
        
        (true, uptime, dependencies)
    }
}