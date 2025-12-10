// data-router-rs/src/router.rs
// Implementation of the Data Router with agent scope isolation
// Ensures that agents can only access data from their own scope or shared data

use std::sync::Arc;
use std::collections::HashMap;
use prost::Message;
use tokio::sync::{Mutex, RwLock};
use tonic::{Request, Response, Status};
use std::error::Error;

// Import from KB module to check knowledge metadata
pub mod kb_clients;
use kb_clients::QueryMetadata;

use crate::agi_core::{
    QueryRequest,
    QueryResponse,
    StoreRequest,
    StoreResponse,
    RetrieveRequest,
    RetrieveResponse,
};

// Constants for agent scope properties
const SCOPE_METADATA_FIELD: &str = "scope";
const DEFAULT_SCOPE: &str = "PUBLIC";
const SYSTEM_SCOPE: &str = "SYSTEM";

// Enum to represent Agent Scope verification result
#[derive(Debug, Clone, PartialEq)]
pub enum ScopeVerificationResult {
    Allowed,
    Denied { reason: String },
    Warning { message: String },
}

// Struct to manage agent scope validation
#[derive(Debug, Clone)]
pub struct AgentScopeManager {
    // Map of agent scopes - each agent can have multiple scopes it belongs to
    agent_scopes: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl AgentScopeManager {
    pub fn new() -> Self {
        let mut agent_scopes = HashMap::new();
        
        // Initialize with default agent scopes
        // RED-TEAM agents
        agent_scopes.insert("RED-TEAM-SHADOW".to_string(), 
            vec!["RED_TEAM".to_string(), "SHADOW_AGENTS".to_string()]);
            
        // BLUE-TEAM agents
        agent_scopes.insert("BLUE-TEAM-SENTINEL".to_string(), 
            vec!["BLUE_TEAM".to_string(), "SENTINEL_AGENTS".to_string()]);
            
        // System agents (can access all data)
        agent_scopes.insert("SYSTEM-ADMIN".to_string(), 
            vec![SYSTEM_SCOPE.to_string()]);
            
        Self {
            agent_scopes: Arc::new(RwLock::new(agent_scopes)),
        }
    }
    
    // Check if one agent can access data from another agent's scope
    pub async fn can_access_scope(&self, agent_id: &str, target_scope: &str) -> bool {
        // System scope can access everything
        if agent_id == SYSTEM_SCOPE {
            return true;
        }

        // Public data can be accessed by any agent
        if target_scope == DEFAULT_SCOPE {
            return true;
        }
        
        let scopes_map = self.agent_scopes.read().await;
        
        // Get agent's scopes (if agent not found, use empty vec)
        let agent_scopes = scopes_map.get(agent_id).cloned().unwrap_or_default();
        
        // Check if agent belongs to the target scope or has system access
        agent_scopes.contains(&target_scope.to_string()) || 
            agent_scopes.contains(&SYSTEM_SCOPE.to_string())
    }
    
    // Validate a query request against agent scopes
    pub async fn validate_query(&self, agent_id: &str, query: &QueryRequest) 
        -> ScopeVerificationResult {
        
        // If no agent ID provided, use default scope
        let agent_id = if agent_id.is_empty() { DEFAULT_SCOPE } else { agent_id };
        
        // System scope can access everything
        if agent_id == SYSTEM_SCOPE {
            return ScopeVerificationResult::Allowed;
        }
        
        // Check if this is a query that should be scope-filtered
        if query.kb_type.to_lowercase() == "mind" {
            if !query.metadata.contains_key("agent_id") {
                return ScopeVerificationResult::Warning { 
                    message: "No agent_id provided in query metadata, limiting to public scope".to_string() 
                };
            }
            
            // All queries are allowed but will be filtered by scope
            return ScopeVerificationResult::Allowed;
        }
        
        // For other KB types, allow without restriction for now
        ScopeVerificationResult::Allowed
    }
    
    // Helper method to get scopes that an agent can access
    pub async fn get_accessible_scopes(&self, agent_id: &str) -> Vec<String> {
        let mut accessible_scopes = vec![DEFAULT_SCOPE.to_string()];
        
        let scopes_map = self.agent_scopes.read().await;
        
        if let Some(agent_scopes) = scopes_map.get(agent_id) {
            accessible_scopes.extend(agent_scopes.clone());
            
            // If agent has system scope, they can access everything
            if agent_scopes.contains(&SYSTEM_SCOPE.to_string()) {
                // Add all unique scopes from the map
                for scopes in scopes_map.values() {
                    for scope in scopes {
                        if !accessible_scopes.contains(scope) {
                            accessible_scopes.push(scope.clone());
                        }
                    }
                }
            }
        }
        
        accessible_scopes
    }
    
    // Apply scope filtering to a query and return the modified request
    pub async fn apply_scope_filter(&self, agent_id: &str, mut query: QueryRequest) -> QueryRequest {
        // If no agent ID provided, use default scope
        let agent_id = if agent_id.is_empty() { DEFAULT_SCOPE } else { agent_id };
        
        // Get the scopes this agent can access
        let accessible_scopes = self.get_accessible_scopes(agent_id).await;
        
        // Add scope filter to query
        let mut filter = query.filter.unwrap_or_default();
        
        // Convert the scopes to a query filter
        if !accessible_scopes.contains(&SYSTEM_SCOPE.to_string()) {
            // Only add scope filter if the agent doesn't have system access
            let scope_filter = format!(
                "scope:(\"{}\") OR !scope:*", 
                accessible_scopes.join("\" OR scope:\"")
            );
            
            // Append to existing filter or create new one
            if filter.is_empty() {
                filter = scope_filter;
            } else {
                filter = format!("({}) AND ({})", filter, scope_filter);
            }
        }
        
        query.filter = Some(filter);
        query
    }
    
    // Register a new agent with specific scopes
    pub async fn register_agent(&self, agent_id: &str, scopes: Vec<String>) -> Result<(), Box<dyn Error>> {
        let mut scopes_map = self.agent_scopes.write().await;
        scopes_map.insert(agent_id.to_string(), scopes);
        Ok(())
    }
}

impl Default for AgentScopeManager {
    fn default() -> Self {
        Self::new()
    }
}

// Function to create a Status error for scope violations
pub fn create_scope_violation_error(agent_id: &str, target_scope: &str) -> Status {
    Status::permission_denied(format!(
        "Agent '{}' does not have permission to access data from scope '{}'", 
        agent_id, target_scope
    ))
}

// Function to enrich query response with scope information for debugging
pub fn add_scope_metadata_to_response(mut response: QueryResponse, 
                                      agent_id: &str, 
                                      filtered_scopes: &[String]) -> QueryResponse {
    let mut metadata = response.metadata.unwrap_or_default();
    metadata.insert("querying_agent".to_string(), agent_id.to_string());
    metadata.insert("accessible_scopes".to_string(), filtered_scopes.join(","));
    response.metadata = Some(metadata);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scope_validation() {
        let scope_manager = AgentScopeManager::new();
        
        // Test basic access patterns
        assert!(scope_manager.can_access_scope("RED-TEAM-SHADOW", "RED_TEAM").await);
        assert!(scope_manager.can_access_scope("BLUE-TEAM-SENTINEL", "BLUE_TEAM").await);
        
        // Test cross-team access (should be denied)
        assert!(!scope_manager.can_access_scope("RED-TEAM-SHADOW", "BLUE_TEAM").await);
        assert!(!scope_manager.can_access_scope("BLUE-TEAM-SENTINEL", "RED_TEAM").await);
        
        // Test public scope access (all should be allowed)
        assert!(scope_manager.can_access_scope("RED-TEAM-SHADOW", "PUBLIC").await);
        assert!(scope_manager.can_access_scope("BLUE-TEAM-SENTINEL", "PUBLIC").await);
        
        // Test system access
        assert!(scope_manager.can_access_scope("SYSTEM-ADMIN", "RED_TEAM").await);
        assert!(scope_manager.can_access_scope("SYSTEM-ADMIN", "BLUE_TEAM").await);
    }
    
    #[tokio::test]
    async fn test_scope_filter_application() {
        let scope_manager = AgentScopeManager::new();
        
        // Create a test query
        let query = QueryRequest {
            kb_type: "mind".to_string(),
            query: "test query".to_string(),
            limit: 10,
            threshold: 0.7,
            filter: Some("original_filter".to_string()),
            metadata: HashMap::new(),
        };
        
        // Apply scope filter for RED-TEAM agent
        let filtered_query = scope_manager.apply_scope_filter("RED-TEAM-SHADOW", query.clone()).await;
        
        // Verify filter contains both original filter and scope filter
        let filter = filtered_query.filter.unwrap_or_default();
        assert!(filter.contains("original_filter"));
        assert!(filter.contains("scope:(\"PUBLIC\" OR scope:\"RED_TEAM\""));
        
        // Verify SYSTEM-ADMIN gets unmodified filter
        let system_query = scope_manager.apply_scope_filter("SYSTEM-ADMIN", query.clone()).await;
        let system_filter = system_query.filter.unwrap_or_default();
        assert_eq!(system_filter, "original_filter");
    }
}