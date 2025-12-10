// llm-service-rs/src/main.rs
// Main Entry Point for llm-service-rs
// Implements the LLMService gRPC server

use tonic::{transport::Server, Request, Response, Status};
use std::sync::Arc;
use std::time::Instant;
use std::net::SocketAddr;
use std::env;
use std::collections::HashMap;
use dotenv::dotenv;
use once_cell::sync::Lazy;
use config_rs;

mod llm_client;
mod secrets_client;  // Add secrets client module
use llm_client::LLMClient;

// Track service start time for uptime reporting
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

// Import Generated Code and Types
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    llm_service_server::{LlmService, LlmServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    GenerateRequest,
    GenerateResponse,
    LlmProcessRequest,
    LlmProcessResponse,
    HealthRequest,
    HealthResponse,
    CompileContextRequest,
    CompiledContextResponse,
    ContextSummarySchema,
    RawContextData,
};

// Define the LLM Server Structure
#[derive(Debug)]
pub struct LlmServer {
    client: LLMClient,
}

// The client initialization is now async, so we can't use Default
impl LlmServer {
    async fn new() -> Self {
        // Initialize LLM client with secrets support
        let client = LLMClient::new().await;
        Self { client }
    }
}

// Implement the LlmService Trait
#[tonic::async_trait]
impl LlmService for LlmServer {
    async fn generate_text(
        &self,
        request: Request<GenerateRequest>,
    ) -> Result<Response<GenerateResponse>, Status> {
        let req_data = request.into_inner();
        
        log::info!("Received GenerateText request: prompt length={}", req_data.prompt.len());

        // Determine system prompt based on parameters or context
        // In a real system, this would be more sophisticated
        let system_prompt = if let Some(role) = req_data.parameters.get("role") {
            match role.as_str() {
                "blue_team" => Some("You are the Blue Team Agent, responsible for defensive security operations. Analyze the situation and recommend defensive actions."),
                "red_team" => Some("You are the Red Team Agent, responsible for adversarial simulation. Identify vulnerabilities and propose attack vectors."),
                "master" => Some("You are the Master Orchestrator, responsible for high-level planning and coordination of the Digital Twin system."),
                _ => None,
            }
        } else {
            None
        };

        // Call LLM Client with improved error handling
        match self.client.generate_text_string(&req_data.prompt, system_prompt).await {
            Ok(text) => {
                let reply = GenerateResponse {
                    text,
                    metadata: {
                        let mut meta = std::collections::HashMap::new();
                        meta.insert("status".to_string(), "success".to_string());
                        meta
                    },
                };
                Ok(Response::new(reply))
            }
            Err(e) => {
                log::error!("LLM generation failed: {}", e);
                
                // Classify error type for better client feedback
                let (status, error_type) = if e.contains("Rate limit exceeded") {
                    ("rate_limited", "RATE_LIMITED")
                } else if e.contains("Invalid request") || e.contains("Unauthorized") || e.contains("Forbidden") {
                    ("client_error", "CLIENT_ERROR")
                } else if e.contains("Server error") {
                    ("server_error", "SERVER_ERROR")
                } else if e.contains("Network error") {
                    ("network_error", "NETWORK_ERROR")
                } else {
                    ("error", "UNKNOWN_ERROR")
                };
                
                // Return a failure response
                let reply = GenerateResponse {
                    text: format!("Error generating text: {}", e),
                    metadata: {
                        let mut meta = std::collections::HashMap::new();
                        meta.insert("status".to_string(), status.to_string());
                        meta.insert("error".to_string(), e.clone());
                        meta.insert("error_type".to_string(), error_type.to_string());
                        meta
                    },
                };
                Ok(Response::new(reply))
            }
        }
    }

    async fn generate(
        &self,
        request: Request<GenerateRequest>,
    ) -> Result<Response<GenerateResponse>, Status> {
        // Alias for GenerateText - delegate to the same implementation
        self.generate_text(request).await
    }

    async fn process(
        &self,
        request: Request<LlmProcessRequest>,
    ) -> Result<Response<LlmProcessResponse>, Status> {
        let req_data = request.into_inner();
        
        log::info!("Received Process request: operation={}, text length={}", 
            req_data.operation, req_data.text.len());

        // --- LLM PROCESSING STUB ---
        // In a real scenario, this would involve:
        // 1. Processing text based on the operation type
        // 2. Applying NLP operations (summarization, translation, etc.)
        // 3. Returning the processed result
        // For now, we return a stub response
        
        let result = format!(
            "LLM processed text (operation: {}): \"{}\"",
            req_data.operation, req_data.text
        );

        let reply = LlmProcessResponse {
            result,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("operation".to_string(), req_data.operation);
                meta.insert("status".to_string(), "success".to_string());
                meta
            },
        };

        Ok(Response::new(reply))
    }

    async fn embed_text(
        &self,
        request: Request<LlmProcessRequest>,
    ) -> Result<Response<LlmProcessResponse>, Status> {
        let req_data = request.into_inner();
        
        log::info!("Received EmbedText request: text length={}", req_data.text.len());

        // --- LLM EMBEDDING STUB ---
        // In a real scenario, this would involve:
        // 1. Calling an embedding model (OpenAI, sentence-transformers, etc.)
        // 2. Converting text to a vector representation
        // 3. Returning the embedding vector
        // For now, we return a stub response with a string representation
        
        // Stub embedding vector representation (1536 dimensions, all 0.0 for now)
        let embedding_vec = vec![0.0f32; 1536];
        let embedding_vector = serde_json::to_string(&embedding_vec).unwrap_or_default();

        let reply = LlmProcessResponse {
            result: embedding_vector,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("embedding_model".to_string(), "stub-embedding-model".to_string());
                meta.insert("dimensions".to_string(), "1536".to_string());
                meta.insert("status".to_string(), "success".to_string());
                meta
            },
        };

        Ok(Response::new(reply))
    }

    async fn compile_context(
        &self,
        request: Request<CompileContextRequest>,
    ) -> Result<Response<CompiledContextResponse>, Status> {
        let req_data = request.into_inner();
        
        log::info!(
            "Received CompileContext request: request_id={}, schema_id={}",
            req_data.request_id,
            req_data.schema.schema_id
        );

        // Create the system prompt for context compilation
        let system_prompt = Some(
            format!(
                "You are a Context Compiler responsible for structuring and condensing raw context data.

Your task is to analyze the provided raw context data and compile it into a concise, structured JSON format
that strictly adheres to the provided schema specification.

SCHEMA SPECIFICATION:
ID: {}
Fields: {}
Description: {}

Your goals are to:
1. Strictly follow the schema field definitions
2. Extract only the most relevant and high-signal information from the raw data
3. Condense the information into the smallest possible representation while preserving meaning
4. Return a valid, well-formed JSON object that matches the schema
5. Ensure all required fields are present in the output
6. Omit any redundant or low-value information

The quality of your compilation directly impacts the LLM's working memory efficiency.
Be selective and precise in what you include.",
                req_data.schema.schema_id,
                req_data.schema.field_definitions.join(", "),
                req_data.schema.schema_description
            )
        );

        // Prepare the prompt content from raw data
        // Convert raw context entries to a readable format
        let mut raw_content = format!("User ID: {}\nQuery: {}\n\nContext Entries:\n",
            req_data.raw_data.user_id,
            req_data.raw_data.query);
        
        for (i, entry) in req_data.raw_data.entries.iter().enumerate() {
            raw_content.push_str(&format!("Entry {}:\n  Source: {}\n  Content: {}\n  Relevance: {}\n  Timestamp: {}\n\n",
                i+1, entry.source_kb, entry.content, entry.relevance_score, entry.timestamp));
        }
        
        // Add any metadata as additional context
        if !req_data.raw_data.metadata.is_empty() {
            raw_content.push_str("Additional Metadata:\n");
            for (key, value) in &req_data.raw_data.metadata {
                raw_content.push_str(&format!("  {}: {}\n", key, value));
            }
        }

        // Call LLM Client with the prepared prompt and system prompt
        match self.client.generate_text_string(&raw_content, system_prompt).await {
            Ok(json_text) => {
                // Try to validate the response is proper JSON
                match serde_json::from_str::<serde_json::Value>(&json_text) {
                    Ok(_) => {
                        // Successfully parsed as JSON
                        let tokens_used = json_text.split_whitespace().count() as i32;
                        
                        let reply = CompiledContextResponse {
                            request_id: req_data.request_id,
                            compiled_json: json_text,
                            tokens_used,
                            metadata: {
                                let mut meta = std::collections::HashMap::new();
                                meta.insert("status".to_string(), "success".to_string());
                                meta.insert("schema_id".to_string(), req_data.schema.schema_id.clone());
                                meta
                            },
                        };
                        Ok(Response::new(reply))
                    },
                    Err(e) => {
                        // The response wasn't valid JSON, return an error
                        log::error!("Generated text is not valid JSON: {}", e);
                        
                        // Return a failure response
                        let reply = CompiledContextResponse {
                            request_id: req_data.request_id,
                            compiled_json: "{}".to_string(),  // Empty JSON object
                            tokens_used: 0,
                            metadata: {
                                let mut meta = std::collections::HashMap::new();
                                meta.insert("status".to_string(), "format_error".to_string());
                                meta.insert("error".to_string(), format!("Invalid JSON format: {}", e));
                                meta
                            },
                        };
                        Ok(Response::new(reply))
                    }
                }
            },
            Err(e) => {
                log::error!("Context compilation failed: {}", e);
                
                // Return a failure response
                let reply = CompiledContextResponse {
                    request_id: req_data.request_id,
                    compiled_json: "{}".to_string(),  // Empty JSON object
                    tokens_used: 0,
                    metadata: {
                        let mut meta = std::collections::HashMap::new();
                        meta.insert("status".to_string(), "error".to_string());
                        meta.insert("error".to_string(), e.clone());
                        meta
                    },
                };
                Ok(Response::new(reply))
            }
        }
    }
}

// Main function to start the gRPC server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenv().ok();

    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Get bind address from standard configuration
    let addr = config_rs::get_bind_address("LLM", 50053);

    // Create the LLM server with async initialization
    let llm_server = LlmServer::new().await;

    log::info!("LlmService starting on {}", addr);
    println!("LlmService listening on {}", addr);

    // Initialize start time
    let _ = *START_TIME;

    // Initialize secrets client to verify connectivity
    match secrets_client::SecretsClient::new().await {
        Ok(client) => {
            log::info!("Successfully connected to secrets service");
            if client.is_mock() {
                log::warn!("Secrets service running in mock mode - API keys will be loaded from environment variables");
            }
        },
        Err(e) => {
            log::warn!("Failed to connect to secrets service: {}. Using environment variables for API keys.", e);
        }
    }

    let llm_server = Arc::new(llm_server);
    let llm_for_health = llm_server.clone();

    Server::builder()
        .add_service(LlmServiceServer::from_arc(llm_server))
        .add_service(HealthServiceServer::from_arc(llm_for_health))
        .serve(addr)
        .await?;

    Ok(())
}

// Implement HealthService for LlmServer
#[tonic::async_trait]
impl HealthService for LlmServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        
        let mut dependencies = HashMap::new();
        // Check if LLM client is configured
        // Use async check_api_key method
        let llm_configured = self.client.check_api_key().await;
        dependencies.insert("llm_provider".to_string(),
            if llm_configured { "CONFIGURED".to_string() } else { "NOT_CONFIGURED".to_string() });

        // Add secrets service as a dependency
        let secrets_healthy = match secrets_client::SecretsClient::new().await {
            Ok(client) => client.is_healthy().await,
            Err(_) => false,
        };
        dependencies.insert("secrets_service".to_string(),
            if secrets_healthy { "HEALTHY".to_string() } else { "UNHEALTHY".to_string() });

        Ok(Response::new(HealthResponse {
            healthy: llm_configured,
            service_name: "llm-service".to_string(),
            uptime_seconds: uptime,
            status: if llm_configured { "SERVING".to_string() } else { "DEGRADED".to_string() },
            dependencies,
        }))
    }
}
