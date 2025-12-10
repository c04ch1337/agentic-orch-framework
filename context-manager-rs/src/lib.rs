use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use chrono::Utc;
use anyhow::Result;

// Track service start time for uptime reporting
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

// Re-export proto types
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    data_router_service_client::DataRouterServiceClient,
    llm_service_client::LLMServiceClient,
    ContextEntry,
    ContextSummarySchema,
    RawContextData,
    AgiState,
    UserIdentity,
    GetStateRequest,
    GetUserRequest,
    Request as ProtoRequest,
};

/// Core Context Manager implementation
pub struct ContextManager {
    // Recent context cache for compaction
    context_cache: Arc<RwLock<Vec<ContextEntry>>>,
    // Client for Data Router to access KBs
    data_router_client: Arc<RwLock<Option<DataRouterServiceClient<tonic::transport::Channel>>>>,
    // Client for LLM Service to compile context
    llm_client: Arc<RwLock<Option<LLMServiceClient<tonic::transport::Channel>>>>,
    // Default context summary schema
    context_schema: Arc<RwLock<Option<ContextSummarySchema>>>,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            context_cache: Arc::new(RwLock::new(Vec::new())),
            data_router_client: Arc::new(RwLock::new(None)),
            llm_client: Arc::new(RwLock::new(None)),
            context_schema: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the default context schema
    pub fn init_context_schema(&self) {
        let schema = ContextSummarySchema {
            schema_id: "context-summary-v1".to_string(),
            field_definitions: vec![
                "last_action: string".to_string(),
                "relevant_facts: [string]".to_string(),
                "tool_definitions: [string]".to_string(),
                "key_entities: [string]".to_string(),
                "user_intent: string".to_string(),
            ],
            schema_description: "Structured context summary for AGI system prompt".to_string(),
        };

        let mut guard = self.context_schema.write().unwrap();
        *guard = Some(schema);
        tracing::info!("Initialized default context summary schema");
    }

    /// Initialize the LLM Service client
    pub async fn init_llm_client(&self, llm_addr: String) -> Result<()> {
        tracing::info!("Connecting to LLM Service at {}", llm_addr);
        let client = LLMServiceClient::connect(llm_addr).await?;
        let mut guard = self.llm_client.write().await;
        *guard = Some(client);
        tracing::info!("Connected to LLM Service");
        Ok(())
    }

    /// Initialize the Data Router Service client
    pub async fn init_data_router_client(&self, router_addr: String) -> Result<()> {
        tracing::info!("Connecting to Data Router Service at {}", router_addr);
        let client = DataRouterServiceClient::connect(router_addr).await?;
        let mut guard = self.data_router_client.write().await;
        *guard = Some(client);
        tracing::info!("Connected to Data Router Service");
        Ok(())
    }

    // Helper to get LLM client
    async fn get_llm_client(&self) -> Option<LLMServiceClient<tonic::transport::Channel>> {
        self.llm_client.read().await.clone()
    }

    // Helper to get Data Router client
    async fn get_data_router_client(&self) -> Option<DataRouterServiceClient<tonic::transport::Channel>> {
        self.data_router_client.read().await.clone()
    }

    /// Get user sentiment from Heart-KB
    pub async fn get_user_sentiment(&self, user_id: &str) -> Option<AgiState> {
        let mut client = self.get_data_router_client().await?;
        
        let req = GetStateRequest { source_id: user_id.to_string() };
        let payload = prost::Message::encode_to_vec(&req);
        
        let route_req = agi_core::RouteRequest {
            target_service: "heart-kb".to_string(),
            request: Some(ProtoRequest {
                id: uuid::Uuid::new_v4().to_string(),
                service: "heart-kb".to_string(),
                method: "QueryState".to_string(),
                payload,
                metadata: HashMap::new(),
            }),
        };

        match client.route(tonic::Request::new(route_req)).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                if let Some(response) = inner.response {
                     if response.status_code == 200 {
                         return AgiState::decode(response.payload.as_slice()).ok();
                     }
                }
                None
            }
            Err(e) => {
                tracing::warn!("Failed to fetch sentiment: {}", e);
                None
            }
        }
    }

    /// Get user identity from Social-KB
    pub async fn get_user_identity(&self, user_id: &str) -> Option<UserIdentity> {
        let mut client = self.get_data_router_client().await?;
        
        let req = GetUserRequest { user_id: user_id.to_string() };
        let payload = prost::Message::encode_to_vec(&req);
        
        let route_req = agi_core::RouteRequest {
            target_service: "social-kb".to_string(),
            request: Some(ProtoRequest {
                id: uuid::Uuid::new_v4().to_string(),
                service: "social-kb".to_string(),
                method: "GetUser".to_string(),
                payload,
                metadata: HashMap::new(),
            }),
        };

        match client.route(tonic::Request::new(route_req)).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                if let Some(response) = inner.response {
                     if response.status_code == 200 {
                         if let Ok(user_resp) = agi_core::GetUserResponse::decode(response.payload.as_slice()) {
                             return user_resp.identity;
                         }
                     }
                }
                None
            }
            Err(e) => {
                tracing::warn!("Failed to fetch identity: {}", e);
                None
            }
        }
    }

    /// Query a specific KB for relevant context
    pub async fn query_kb(&self, kb_name: &str, query: &str) -> Vec<ContextEntry> {
        // Return placeholder context based on KB type
        let placeholder_content = match kb_name {
            "mind" => "Previous conversation context available.",
            "soul" => "Core values: integrity, security, efficiency.",
            "body" => "System status: operational.",
            "heart" => "Emotional context: neutral.",
            "social" => "No recent social interactions.",
            _ => return Vec::new(),
        };

        vec![ContextEntry {
            source_kb: kb_name.to_string(),
            content: placeholder_content.to_string(),
            relevance_score: 0.8,
            timestamp: Utc::now().timestamp(),
        }]
    }

    /// Build system prompt based on agent type and compiled context
    pub fn build_system_prompt(
        &self,
        agent_type: &str,
        compiled_context_json: &str,
        sentiment_info: &str,
        identity_info: &str
    ) -> String {
        // Get the base prompt based on agent type
        let base_prompt = match agent_type {
            "red_team" => std::env::var("PROMPT_RED_TEAM").unwrap_or_else(|_| {
                "You are RED_TEAM_SHADOW, an ethical adversary simulation agent for PHOENIX ORCH. \
                 Your role is to identify vulnerabilities and simulate attack scenarios. \
                 Always operate within ethical bounds and authorized scope.".to_string()
            }),
            "blue_team" => std::env::var("PROMPT_BLUE_TEAM").unwrap_or_else(|_| {
                "You are BLUE_TEAM_SENTINEL, an autonomous defense and incident response agent for PHOENIX ORCH. \
                 Your role is to protect systems, detect anomalies, and respond to threats. \
                 Prioritize containment, evidence preservation, and system stability.".to_string()
            }),
            _ => std::env::var("PROMPT_MASTER").unwrap_or_else(|_| {
                "You are the PHOENIX ORCH Master Agent, coordinating cybersecurity operations. \
                 Delegate to specialized agents when appropriate. \
                 Maintain situational awareness and ensure safe operations.".to_string()
            }),
        };

        let mut prompt = format!("{}\n\n", base_prompt);
        
        // Add Identity & Sentiment Context if available
        if !identity_info.is_empty() || !sentiment_info.is_empty() {
            prompt.push_str("## User Context\n");
            if !identity_info.is_empty() {
                prompt.push_str(&format!("- Identity: {}\n", identity_info));
            }
            if !sentiment_info.is_empty() {
                prompt.push_str(&format!("- Sentiment: {}\n", sentiment_info));
            }
            prompt.push('\n');
        }

        // Add compiled context JSON if it's not empty
        if !compiled_context_json.is_empty() {
            prompt.push_str("## Compiled Context\n");
            prompt.push_str(&format!("```json\n{}\n```\n", compiled_context_json));
        }

        prompt
    }

    /// Compile context using LLM service
    pub async fn compile_context(
        &self,
        request_id: &str,
        context_entries: &[ContextEntry],
        query: &str,
    ) -> Result<String> {
        let mut llm_client = match self.get_llm_client().await {
            Some(client) => client,
            None => {
                tracing::warn!("LLM client not initialized for context compilation");
                return Ok("".to_string());
            }
        };

        let schema = match self.context_schema.read().unwrap().clone() {
            Some(schema) => schema,
            None => {
                tracing::warn!("Context schema not initialized");
                return Ok("".to_string());
            }
        };

        let raw_data = RawContextData {
            user_id: "default-user".to_string(),
            entries: context_entries.to_vec(),
            query: query.to_string(),
            metadata: HashMap::new(),
        };

        let compile_request = agi_core::CompileContextRequest {
            request_id: request_id.to_string(),
            raw_data: Some(raw_data),
            schema: Some(schema),
            max_output_tokens: 1000,
        };

        match llm_client.compile_context(tonic::Request::new(compile_request)).await {
            Ok(response) => {
                let inner = response.into_inner();
                tracing::info!("Context compilation successful: {} tokens used", inner.tokens_used);
                Ok(inner.compiled_json)
            }
            Err(e) => {
                tracing::error!("Failed to compile context: {}", e);
                Ok("".to_string())
            }
        }
    }

    /// Get recent context entries
    pub async fn get_recent_context(&self, kb_sources: &[String], limit: i32) -> Vec<ContextEntry> {
        let cache = self.context_cache.read().await;
        
        cache.iter()
            .filter(|e| {
                kb_sources.is_empty() || kb_sources.contains(&e.source_kb)
            })
            .cloned()
            .take(limit as usize)
            .collect()
    }

    /// Add entries to context cache
    pub async fn add_to_cache(&self, entries: Vec<ContextEntry>) {
        let mut cache = self.context_cache.write().await;
        cache.extend(entries);
        
        // Keep only last 100 entries
        let cache_len = cache.len();
        if cache_len > 100 {
            let new_cache = cache.split_off(cache_len - 100);
            *cache = new_cache;
        }
    }

    /// Get service health status
    pub async fn health_check(&self) -> Result<bool> {
        Ok(true) // Basic health check
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}