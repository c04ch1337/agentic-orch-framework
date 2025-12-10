// data-router-rs/src/tests/scope_isolation_tests.rs
// Integration tests for agent scope isolation features
// Tests scope filtering, validation, and error handling

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::agi_core::{
    QueryRequest,
    QueryResponse,
    StoreRequest,
    RetrieveRequest,
};
use crate::router::{AgentScopeManager, ScopeVerificationResult};
use crate::kb_clients::{MindKbClient, QueryMetadata};

// Mock client for testing Mind KB calls
struct MockMindKbServiceClient;

// Helper to create a mock KB client for testing
fn create_test_kb_client() -> MindKbClient {
    // Create a mock client
    let mock_client = Arc::new(Mutex::new(None::<tonic::transport::Channel>));
    let scope_manager = Arc::new(AgentScopeManager::new());
    
    MindKbClient::new(mock_client, scope_manager)
}

#[tokio::test]
async fn test_agent_scope_validation() {
    let scope_manager = AgentScopeManager::new();
    
    // Test case 1: RED-TEAM agent access
    let red_agent = "RED-TEAM-SHADOW";
    assert!(scope_manager.can_access_scope(red_agent, "RED_TEAM").await);
    assert!(scope_manager.can_access_scope(red_agent, "PUBLIC").await);
    assert!(!scope_manager.can_access_scope(red_agent, "BLUE_TEAM").await);
    
    // Test case 2: BLUE-TEAM agent access
    let blue_agent = "BLUE-TEAM-SENTINEL";
    assert!(scope_manager.can_access_scope(blue_agent, "BLUE_TEAM").await);
    assert!(scope_manager.can_access_scope(blue_agent, "PUBLIC").await);
    assert!(!scope_manager.can_access_scope(blue_agent, "RED_TEAM").await);
    
    // Test case 3: SYSTEM agent access (can access all)
    let system_agent = "SYSTEM-ADMIN";
    assert!(scope_manager.can_access_scope(system_agent, "RED_TEAM").await);
    assert!(scope_manager.can_access_scope(system_agent, "BLUE_TEAM").await);
    assert!(scope_manager.can_access_scope(system_agent, "PUBLIC").await);
    assert!(scope_manager.can_access_scope(system_agent, "SENTINEL_AGENTS").await);
}

#[tokio::test]
async fn test_scope_filter_application() {
    let scope_manager = Arc::new(AgentScopeManager::new());
    
    // Create a test query with no filter
    let query = QueryRequest {
        kb_type: "mind".to_string(),
        query: "test query".to_string(),
        limit: 10,
        threshold: 0.7,
        filter: None,
        metadata: HashMap::new(),
    };
    
    // Test case 1: RED-TEAM query filtering
    let red_agent = "RED-TEAM-SHADOW";
    let filtered_query = scope_manager.apply_scope_filter(red_agent, query.clone()).await;
    
    // Verify the filter contains RED_TEAM scope
    let filter = filtered_query.filter.unwrap_or_default();
    assert!(filter.contains("RED_TEAM"));
    assert!(filter.contains("PUBLIC"));
    assert!(!filter.contains("BLUE_TEAM"));
    
    // Test case 2: BLUE-TEAM query filtering
    let blue_agent = "BLUE-TEAM-SENTINEL";
    let filtered_query = scope_manager.apply_scope_filter(blue_agent, query.clone()).await;
    
    // Verify the filter contains BLUE_TEAM scope
    let filter = filtered_query.filter.unwrap_or_default();
    assert!(filter.contains("BLUE_TEAM"));
    assert!(filter.contains("PUBLIC"));
    assert!(!filter.contains("RED_TEAM"));
    
    // Test case 3: SYSTEM query filtering (should have no scope filter)
    let system_agent = "SYSTEM-ADMIN";
    let filtered_query = scope_manager.apply_scope_filter(system_agent, query.clone()).await;
    
    // System agents don't have scope filters applied
    assert_eq!(filtered_query.filter, None);
}

#[tokio::test]
async fn test_scope_validation_results() {
    let scope_manager = Arc::new(AgentScopeManager::new());
    
    // Create a test query
    let mut query = QueryRequest {
        kb_type: "mind".to_string(),
        query: "test query".to_string(),
        limit: 10,
        threshold: 0.7,
        filter: None,
        metadata: HashMap::new(),
    };
    
    // Test case 1: Query with no agent_id metadata
    let validation_result = scope_manager.validate_query("RED-TEAM-SHADOW", &query).await;
    
    // Should return a warning because no agent_id in metadata
    assert!(matches!(validation_result, ScopeVerificationResult::Warning { .. }));
    
    // Test case 2: Add agent_id metadata
    query.metadata.insert("agent_id".to_string(), "RED-TEAM-SHADOW".to_string());
    let validation_result = scope_manager.validate_query("RED-TEAM-SHADOW", &query).await;
    
    // Should be allowed now
    assert!(matches!(validation_result, ScopeVerificationResult::Allowed));
}

#[tokio::test]
async fn test_accessible_scopes() {
    let scope_manager = Arc::new(AgentScopeManager::new());
    
    // Test case 1: RED-TEAM agent accessible scopes
    let red_agent = "RED-TEAM-SHADOW";
    let red_scopes = scope_manager.get_accessible_scopes(red_agent).await;
    
    // Should have PUBLIC and RED_TEAM scopes
    assert!(red_scopes.contains(&"PUBLIC".to_string()));
    assert!(red_scopes.contains(&"RED_TEAM".to_string()));
    assert!(red_scopes.contains(&"SHADOW_AGENTS".to_string()));
    assert!(!red_scopes.contains(&"BLUE_TEAM".to_string()));
    
    // Test case 2: SYSTEM agent accessible scopes
    let system_agent = "SYSTEM-ADMIN";
    let system_scopes = scope_manager.get_accessible_scopes(system_agent).await;
    
    // System agent should have access to all scopes
    assert!(system_scopes.contains(&"PUBLIC".to_string()));
    assert!(system_scopes.contains(&"RED_TEAM".to_string()));
    assert!(system_scopes.contains(&"BLUE_TEAM".to_string()));
}

#[tokio::test]
async fn test_agent_registration() {
    let scope_manager = Arc::new(AgentScopeManager::new());
    
    // Register a new agent with custom scopes
    let new_agent = "PURPLE-TEAM-SCOUT";
    let scopes = vec!["PURPLE_TEAM".to_string(), "SCOUT_AGENTS".to_string()];
    let register_result = scope_manager.register_agent(new_agent, scopes.clone()).await;
    
    // Registration should succeed
    assert!(register_result.is_ok());
    
    // Verify the agent has access to its scopes
    assert!(scope_manager.can_access_scope(new_agent, "PURPLE_TEAM").await);
    assert!(scope_manager.can_access_scope(new_agent, "SCOUT_AGENTS").await);
    
    // Verify the agent doesn't have access to other scopes
    assert!(!scope_manager.can_access_scope(new_agent, "RED_TEAM").await);
    assert!(!scope_manager.can_access_scope(new_agent, "BLUE_TEAM").await);
    
    // But can still access PUBLIC scope
    assert!(scope_manager.can_access_scope(new_agent, "PUBLIC").await);
}

#[tokio::test]
async fn test_complex_filter_merging() {
    let scope_manager = Arc::new(AgentScopeManager::new());
    
    // Create a query with an existing filter
    let query = QueryRequest {
        kb_type: "mind".to_string(),
        query: "test query".to_string(),
        limit: 10,
        threshold: 0.7,
        filter: Some("importance:HIGH AND category:CRITICAL".to_string()),
        metadata: HashMap::new(),
    };
    
    // Apply scope filter
    let red_agent = "RED-TEAM-SHADOW";
    let filtered_query = scope_manager.apply_scope_filter(red_agent, query.clone()).await;
    
    // Verify the combined filter has both the original and scope conditions
    let filter = filtered_query.filter.unwrap_or_default();
    assert!(filter.contains("importance:HIGH AND category:CRITICAL"));
    assert!(filter.contains("scope:(\"PUBLIC\" OR scope:\"RED_TEAM\""));
    assert!(filter.contains("AND")); // Should combine with AND
}

// This would be extended in a real implementation to test the MindKbClient with mock responses
// Here's a sketch of what that would look like
#[tokio::test]
#[ignore] // Marked as ignored since it requires mock clients
async fn test_kb_client_scope_filtering() {
    // This test would verify that the MindKbClient correctly applies scope filtering
    // by using mock gRPC responses
    
    // In a real implementation, we would:
    // 1. Create mock responses for different agent scopes
    // 2. Verify that query results only include items from the correct scopes
    // 3. Test that store operations correctly tag data with scopes
    // 4. Test that retrieve operations enforce scope checks
}