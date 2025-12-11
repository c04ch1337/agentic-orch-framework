// data-router-rs/src/kb_clients.rs
// Implementation of Knowledge Base clients with scope filtering support
// This file extends the basic KB client functionality with agent scope isolation

use prost::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::agi_core::{
    QueryRequest, QueryResponse, RetrieveRequest, RetrieveResponse, StoreRequest, StoreResponse,
    mind_kb_service_client::MindKbServiceClient,
};

use crate::router::{
    AgentScopeManager, add_scope_metadata_to_response, create_scope_violation_error,
};

// Structure to track request metadata for Knowledge Base operations
#[derive(Debug, Clone)]
pub struct QueryMetadata {
    pub agent_id: String,
    pub target_kb: String,
    pub operation: String,
}

impl QueryMetadata {
    pub fn new(agent_id: &str, target_kb: &str, operation: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            target_kb: target_kb.to_string(),
            operation: operation.to_string(),
        }
    }
}

// Structure to handle Mind KB client calls with scope isolation
pub struct MindKbClient {
    client: Arc<tokio::sync::Mutex<Option<MindKbServiceClient<tonic::transport::Channel>>>>,
    scope_manager: Arc<AgentScopeManager>,
}

impl MindKbClient {
    pub fn new(
        client: Arc<tokio::sync::Mutex<Option<MindKbServiceClient<tonic::transport::Channel>>>>,
        scope_manager: Arc<AgentScopeManager>,
    ) -> Self {
        Self {
            client,
            scope_manager,
        }
    }

    // Main query method with scope filtering
    pub async fn query_kb(
        &self,
        mut query_req: QueryRequest,
        query_meta: QueryMetadata,
    ) -> Result<Response<QueryResponse>, Status> {
        // First validate if the agent can make this query
        let validation_result = self
            .scope_manager
            .validate_query(&query_meta.agent_id, &query_req)
            .await;

        match validation_result {
            crate::router::ScopeVerificationResult::Allowed => {
                // Apply scope filter to limit results to accessible scopes
                query_req = self
                    .scope_manager
                    .apply_scope_filter(&query_meta.agent_id, query_req)
                    .await;

                // Get the accessible scopes for debugging/audit
                let accessible_scopes = self
                    .scope_manager
                    .get_accessible_scopes(&query_meta.agent_id)
                    .await;

                // Log the scope-filtered query
                log::info!(
                    "Scope-filtered query for agent '{}': Filter: '{}', Accessible scopes: '{}'",
                    query_meta.agent_id,
                    query_req.filter.as_deref().unwrap_or(""),
                    accessible_scopes.join(", ")
                );

                // Get client and execute
                let client_guard = self.client.lock().await;
                let client = match client_guard.as_ref() {
                    Some(c) => c.clone(),
                    None => return Err(Status::unavailable("Mind-KB client not initialized")),
                };
                drop(client_guard);

                let mut client_req = tonic::Request::new(query_req);

                // Add agent ID to request metadata for audit trail
                client_req.metadata_mut().insert(
                    "agent_id",
                    tonic::metadata::MetadataValue::from_str(&query_meta.agent_id)?,
                );

                // Execute the query
                let response = client.query_kb(client_req).await?;
                let inner_response = response.into_inner();

                // Add scope metadata to response for debugging/audit
                let enhanced_response = add_scope_metadata_to_response(
                    inner_response,
                    &query_meta.agent_id,
                    &accessible_scopes,
                );

                Ok(Response::new(enhanced_response))
            }
            crate::router::ScopeVerificationResult::Denied { reason } => {
                Err(Status::permission_denied(format!(
                    "Agent '{}' denied access to Mind KB: {}",
                    query_meta.agent_id, reason
                )))
            }
            crate::router::ScopeVerificationResult::Warning { message } => {
                // Add warning to logs but continue with filtered query
                log::warn!(
                    "Warning for agent '{}' querying Mind KB: {}",
                    query_meta.agent_id,
                    message
                );

                // Apply scope filter and continue
                query_req = self
                    .scope_manager
                    .apply_scope_filter(&query_meta.agent_id, query_req)
                    .await;

                // Get the accessible scopes for debugging/audit
                let accessible_scopes = self
                    .scope_manager
                    .get_accessible_scopes(&query_meta.agent_id)
                    .await;

                // Get client and execute
                let client_guard = self.client.lock().await;
                let client = match client_guard.as_ref() {
                    Some(c) => c.clone(),
                    None => return Err(Status::unavailable("Mind-KB client not initialized")),
                };
                drop(client_guard);

                let mut client_req = tonic::Request::new(query_req);

                // Add agent ID to request metadata for audit trail
                client_req.metadata_mut().insert(
                    "agent_id",
                    tonic::metadata::MetadataValue::from_str(&query_meta.agent_id)?,
                );

                // Extra metadata for warnings
                client_req.metadata_mut().insert(
                    "warning",
                    tonic::metadata::MetadataValue::from_str(&message)?,
                );

                // Execute the query
                let response = client.query_kb(client_req).await?;
                let inner_response = response.into_inner();

                // Add scope metadata to response for debugging/audit
                let enhanced_response = add_scope_metadata_to_response(
                    inner_response,
                    &query_meta.agent_id,
                    &accessible_scopes,
                );

                Ok(Response::new(enhanced_response))
            }
        }
    }

    // Store fact with scope validation
    pub async fn store_fact(
        &self,
        mut store_req: StoreRequest,
        query_meta: QueryMetadata,
    ) -> Result<Response<StoreResponse>, Status> {
        // Ensure scope is set in metadata - if not, default to agent's primary scope
        let mut metadata = store_req.metadata.clone().unwrap_or_default();

        if !metadata.contains_key("scope") {
            // Get agent's primary scope - first in the list
            let accessible_scopes = self
                .scope_manager
                .get_accessible_scopes(&query_meta.agent_id)
                .await;
            if !accessible_scopes.is_empty() {
                // Use first non-PUBLIC scope if available
                let default_scope = accessible_scopes
                    .iter()
                    .find(|s| s != &"PUBLIC")
                    .unwrap_or(&accessible_scopes[0])
                    .clone();

                metadata.insert("scope".to_string(), default_scope);
            } else {
                // Fallback to PUBLIC
                metadata.insert("scope".to_string(), "PUBLIC".to_string());
            }
        }

        // Check if agent can write to the requested scope
        let target_scope = metadata.get("scope").unwrap().clone();
        if !self
            .scope_manager
            .can_access_scope(&query_meta.agent_id, &target_scope)
            .await
        {
            return Err(create_scope_violation_error(
                &query_meta.agent_id,
                &target_scope,
            ));
        }

        // Update metadata in request
        store_req.metadata = Some(metadata);

        // Get client and execute
        let client_guard = self.client.lock().await;
        let client = match client_guard.as_ref() {
            Some(c) => c.clone(),
            None => return Err(Status::unavailable("Mind-KB client not initialized")),
        };
        drop(client_guard);

        let mut client_req = tonic::Request::new(store_req);

        // Add agent ID to request metadata for audit trail
        client_req.metadata_mut().insert(
            "agent_id",
            tonic::metadata::MetadataValue::from_str(&query_meta.agent_id)?,
        );

        // Execute the store operation
        let response = client.store_fact(client_req).await?;
        Ok(response)
    }

    // Retrieve fact with scope validation
    pub async fn retrieve(
        &self,
        retrieve_req: RetrieveRequest,
        query_meta: QueryMetadata,
    ) -> Result<Response<RetrieveResponse>, Status> {
        // For retrieval by ID, we need to check scope after retrieval
        let client_guard = self.client.lock().await;
        let client = match client_guard.as_ref() {
            Some(c) => c.clone(),
            None => return Err(Status::unavailable("Mind-KB client not initialized")),
        };
        drop(client_guard);

        let mut client_req = tonic::Request::new(retrieve_req);

        // Add agent ID to request metadata for audit trail
        client_req.metadata_mut().insert(
            "agent_id",
            tonic::metadata::MetadataValue::from_str(&query_meta.agent_id)?,
        );

        // Execute the retrieve operation
        let response = client.retrieve(client_req).await?;
        let result = response.into_inner();

        // If we got results, check if the agent can access them based on scope
        if let Some(fact) = &result.fact {
            if let Some(metadata) = &fact.metadata {
                if let Some(scope) = metadata.get("scope") {
                    if !self
                        .scope_manager
                        .can_access_scope(&query_meta.agent_id, scope)
                        .await
                    {
                        // Don't reveal that the item exists, return not found
                        return Err(Status::not_found(format!(
                            "Fact with ID {} not found",
                            retrieve_req.id
                        )));
                    }
                }
            }
        }

        Ok(Response::new(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Mutex;

    // Mock MindKbServiceClient for testing
    struct MockMindKbClient {}

    #[tokio::test]
    async fn test_scope_filtering() {
        // Initialize scope manager
        let scope_manager = Arc::new(AgentScopeManager::new());

        // Create a test query
        let query = QueryRequest {
            kb_type: "mind".to_string(),
            query: "test query".to_string(),
            limit: 10,
            threshold: 0.7,
            filter: None,
            metadata: HashMap::new(),
        };

        // Apply scope filter for RED-TEAM agent
        let filtered_query = scope_manager
            .apply_scope_filter("RED-TEAM-SHADOW", query)
            .await;

        // Verify filter contains scope filter
        let filter = filtered_query.filter.unwrap_or_default();
        assert!(filter.contains("scope:(\"PUBLIC\" OR scope:\"RED_TEAM\""));

        // Ensure BLUE_TEAM is excluded for RED-TEAM agent
        assert!(!filter.contains("BLUE_TEAM"));
    }
}
