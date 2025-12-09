// context-manager-rs/src/main.rs
// Context Manager Service - KB context aggregation and prompt enrichment
// Port 50064

use std::sync::Arc;
use std::time::Instant;
use std::collections::HashMap;
use tonic::{transport::Server, Request, Response, Status};
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use chrono::Utc;
use prost::Message;
use dotenv::dotenv;

// Track service start time for uptime reporting
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    context_manager_service_server::{ContextManagerService, ContextManagerServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    data_router_service_client::DataRouterServiceClient,
    ContextRequest,
    EnrichedContext,
    ContextEntry,
    ContextQuery,
    ContextResponse,
    HealthRequest,
    HealthResponse,
    RouteRequest,
    Request as ProtoRequest,
    GetStateRequest,
    GetUserRequest,
    AgiState,
    UserIdentity,
};

// Context Manager Server - simplified design
#[derive(Debug)]
pub struct ContextManagerServer {
    // Recent context cache for compaction
    context_cache: Arc<RwLock<Vec<ContextEntry>>>,
    // Client for Data Router to access KBs
    data_router_client: Arc<RwLock<Option<DataRouterServiceClient<tonic::transport::Channel>>>>,
}

impl ContextManagerServer {
    pub fn new() -> Self {
        Self {
            context_cache: Arc::new(RwLock::new(Vec::new())),
            data_router_client: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the Data Router Service client
    pub async fn init_data_router_client(&self, router_addr: String) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Connecting to Data Router Service at {}", router_addr);
        let client = DataRouterServiceClient::connect(router_addr).await?;
        let mut guard = self.data_router_client.write().await;
        *guard = Some(client);
        log::info!("Connected to Data Router Service");
        Ok(())
    }

    // Helper to get Data Router client
    async fn get_client(&self) -> Option<DataRouterServiceClient<tonic::transport::Channel>> {
        self.data_router_client.read().await.clone()
    }

    // Fetch user sentiment from Heart-KB
    async fn get_user_sentiment(&self, user_id: &str) -> Option<AgiState> {
        let mut client = self.get_client().await?;
        
        // Create GetStateRequest
        let req = GetStateRequest { source_id: user_id.to_string() };
        let payload = prost::Message::encode_to_vec(&req);
        
        let route_req = RouteRequest {
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
                log::warn!("Failed to fetch sentiment: {}", e);
                None
            }
        }
    }

    // Fetch user identity from Social-KB
    async fn get_user_identity(&self, user_id: &str) -> Option<UserIdentity> {
        let mut client = self.get_client().await?;
        
        // Create GetUserRequest
        let req = GetUserRequest { user_id: user_id.to_string() };
        let payload = prost::Message::encode_to_vec(&req);
        
        let route_req = RouteRequest {
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
                         // GetUserResponse contains UserIdentity
                         if let Ok(user_resp) = agi_core::GetUserResponse::decode(response.payload.as_slice()) {
                             return user_resp.identity;
                         }
                     }
                }
                None
            }
            Err(e) => {
                log::warn!("Failed to fetch identity: {}", e);
                None
            }
        }
    }

    // Query a specific KB for relevant context
    // Note: Returns stub data - real KB integration to be added
    async fn query_kb(&self, kb_name: &str, _query: &str) -> Vec<ContextEntry> {
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

    // Build system prompt based on agent type and context
    fn build_system_prompt(
        &self, 
        agent_type: &str, 
        context_entries: &[ContextEntry],
        sentiment_info: &str,
        identity_info: &str
    ) -> String {
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
        
        // Add Identity & Sentiment Context
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

        prompt.push_str("## Knowledge Base Context\n");
        if !context_entries.is_empty() {
            for entry in context_entries {
                prompt.push_str(&format!(
                    "- [{}] {}\n",
                    entry.source_kb.to_uppercase(),
                    entry.content
                ));
            }
        } else {
            prompt.push_str("No additional context available.\n");
        }

        prompt
    }

    // Estimate token count (simple approximation: 4 chars = 1 token)
    fn estimate_tokens(&self, text: &str) -> i32 {
        (text.len() / 4) as i32
    }
}

impl Default for ContextManagerServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl ContextManagerService for ContextManagerServer {
    async fn enrich_context(
        &self,
        request: Request<ContextRequest>,
    ) -> Result<Response<EnrichedContext>, Status> {
        let req = request.into_inner();
        
        log::info!("EnrichContext request: id={}, query={}, agent_type={}", 
            req.request_id, req.query, req.agent_type);

        // 1. Fetch Cognitive State (Heart & Social KBs)
        // In a real scenario, we'd extract user_id from metadata
        let user_id = "default-user"; 
        
        let sentiment_task = self.get_user_sentiment(user_id);
        let identity_task = self.get_user_identity(user_id);
        
        let (sentiment_opt, identity_opt) = tokio::join!(sentiment_task, identity_task);
        
        let sentiment_info = if let Some(s) = sentiment_opt {
            format!("Current Emotion: {} (Confidence: {:.2})", s.dominant_emotion, s.confidence)
        } else {
            String::new()
        };
        
        let identity_info = if let Some(i) = identity_opt {
            format!("User: {} (Role: {})", i.name, i.role)
        } else {
            String::new()
        };

        // 2. Determine which KBs to query for additional context
        let kb_sources: Vec<&str> = if req.kb_sources.is_empty() {
            vec!["mind", "soul"]  // Default: mind for facts, soul for values
        } else {
            req.kb_sources.iter().map(|s| s.as_str()).collect()
        };

        // 3. Query each KB and collect context
        let mut all_entries = Vec::new();
        for kb in &kb_sources {
            let entries = self.query_kb(kb, &req.query).await;
            all_entries.extend(entries);
        }

        // 4. Sort by relevance and limit to token budget
        all_entries.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));

        let max_tokens = if req.max_context_tokens > 0 { req.max_context_tokens } else { 2000 };
        let mut selected_entries = Vec::new();
        let mut token_count = 0;

        for entry in all_entries {
            let entry_tokens = self.estimate_tokens(&entry.content);
            if token_count + entry_tokens <= max_tokens {
                token_count += entry_tokens;
                selected_entries.push(entry);
            }
        }

        // 5. Build the enriched system prompt with all context
        let system_prompt = self.build_system_prompt(
            &req.agent_type, 
            &selected_entries,
            &sentiment_info,
            &identity_info
        );

        // Cache recent context
        {
            let mut cache = self.context_cache.write().await;
            cache.extend(selected_entries.clone());
            // Keep only last 100 entries
            let cache_len = cache.len();
            if cache_len > 100 {
                let new_cache = cache.split_off(cache_len - 100);
                *cache = new_cache;
            }
        }

        let reply = EnrichedContext {
            request_id: req.request_id,
            original_query: req.query,
            system_prompt,
            context_entries: selected_entries,
            total_tokens_used: token_count,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("kb_sources".to_string(), kb_sources.join(","));
                meta.insert("agent_type".to_string(), req.agent_type);
                meta.insert("sentiment_included".to_string(), (!sentiment_info.is_empty()).to_string());
                meta.insert("identity_included".to_string(), (!identity_info.is_empty()).to_string());
                meta
            },
        };

        log::info!("EnrichContext complete: {} entries, {} tokens", 
            reply.context_entries.len(), token_count);

        Ok(Response::new(reply))
    }

    async fn get_recent_context(
        &self,
        request: Request<ContextQuery>,
    ) -> Result<Response<ContextResponse>, Status> {
        let req = request.into_inner();
        
        log::info!("GetRecentContext: query={}, limit={}", req.query, req.limit);

        let cache = self.context_cache.read().await;
        
        // Filter by KB sources if specified
        let filtered: Vec<ContextEntry> = cache.iter()
            .filter(|e| {
                req.kb_sources.is_empty() || req.kb_sources.contains(&e.source_kb)
            })
            .cloned()
            .take(req.limit as usize)
            .collect();

        let total_count = filtered.len() as i32;

        Ok(Response::new(ContextResponse {
            entries: filtered,
            total_count,
        }))
    }
}

#[tonic::async_trait]
impl HealthService for ContextManagerServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        
        // For now, report as healthy since we don't have real KB connections
        let mut dependencies = HashMap::new();
        dependencies.insert("kb_integration".to_string(), "STUB".to_string());

        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "context-manager-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Initialize start time
    let _ = *START_TIME;

    let addr = std::env::var("CONTEXT_MANAGER_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50064".to_string())
        .parse()?;

    let server = Arc::new(ContextManagerServer::new());

    log::info!("ContextManagerService starting on {}", addr);
    println!("ContextManagerService listening on {}", addr);

    let server_for_health = server.clone();

    Server::builder()
        .add_service(ContextManagerServiceServer::from_arc(server))
        .add_service(HealthServiceServer::from_arc(server_for_health))
        .serve(addr)
        .await?;

    Ok(())
}
