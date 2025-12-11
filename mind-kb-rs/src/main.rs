// mind-kb-rs/src/main.rs
// Main Entry Point for mind-kb-rs
// Implements the MindKBService gRPC server with in-memory vector store

use input_validation_rs::ValidationResult;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;
use tonic::{transport::Server, Request, Response, Status};

mod vector_store;
use vector_store::VectorStore;

// NLP Text Preprocessing Module
mod text_preprocessor;

// Input Validation Module
mod validation;
use validation::{
    validate_content, validate_embedding, validate_metadata, validate_query,
    validate_retrieve_request, validate_store_request,
};

// Track service start time for uptime reporting
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

// Import Generated Code and Types
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    health_service_server::{HealthService, HealthServiceServer},
    llm_service_client::LlmServiceClient,
    mind_kb_service_server::{MindKbService, MindKbServiceServer},
    HealthRequest, HealthResponse, LlmProcessRequest, QueryRequest, QueryResponse, RetrieveRequest,
    RetrieveResponse, StoreRequest, StoreResponse,
};

// Memory configuration from environment variables
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    short_term_limit: usize,
    retention_threshold: f32,
    search_depth: usize,
    recency_weight: f32,
    importance_weight: f32,
    emotional_weight: f32,
    similarity_threshold: f32,
    max_context_items: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            short_term_limit: 100,
            retention_threshold: 0.6,
            search_depth: 20,
            recency_weight: 0.4,
            importance_weight: 0.6,
            emotional_weight: 0.5,
            similarity_threshold: 0.75,
            max_context_items: 15,
        }
    }
}

// Define the Mind KB Server Structure
#[derive(Debug)]
pub struct MindKBServer {
    vector_store: Arc<VectorStore>,
    llm_client_url: String,
    memory_config: MemoryConfig,
}

impl MindKBServer {
    async fn get_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        log::info!("Generating embedding for text of length: {}", text.len());

        // Attempt to connect to LLM service for real embeddings with retry logic
        for attempt in 1..=3 {
            match LlmServiceClient::connect(self.llm_client_url.clone()).await {
                Ok(mut client) => {
                    let request = LlmProcessRequest {
                        operation: "embed".to_string(),
                        text: text.to_string(),
                    };

                    match client.embed_text(request).await {
                        Ok(response) => {
                            let result = response.into_inner().result;
                            match serde_json::from_str::<Vec<f32>>(&result) {
                                Ok(embedding) => {
                                    if embedding.iter().all(|&val| val == 0.0) {
                                        log::warn!("LLM service returned zero embedding (stubbed response). Using improved fallback embedding");
                                        return Ok(Self::generate_improved_embedding(text));
                                    }

                                    log::info!(
                                        "Successfully generated embedding vector of dimension: {}",
                                        embedding.len()
                                    );
                                    return Ok(embedding);
                                }
                                Err(e) => {
                                    log::warn!("Failed to parse embedding response: {}", e);
                                    if attempt < 3 {
                                        log::info!(
                                            "Retrying embedding generation (attempt {}/3)",
                                            attempt + 1
                                        );
                                        tokio::time::sleep(tokio::time::Duration::from_millis(500))
                                            .await;
                                        continue;
                                    } else {
                                        log::warn!(
                                            "All embedding parsing attempts failed, using fallback"
                                        );
                                        return Ok(Self::generate_improved_embedding(text));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("LLM embedding request failed: {}", e);
                            if attempt < 3 {
                                log::info!(
                                    "Retrying embedding request (attempt {}/3)",
                                    attempt + 1
                                );
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                continue;
                            } else {
                                log::warn!("All embedding request attempts failed, using fallback");
                                return Ok(Self::generate_improved_embedding(text));
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to connect to LLM service: {}", e);
                    if attempt < 3 {
                        log::info!("Retrying connection (attempt {}/3)", attempt + 1);
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        continue;
                    } else {
                        log::warn!("All connection attempts failed, using fallback embedding");
                        return Ok(Self::generate_improved_embedding(text));
                    }
                }
            }
        }

        // If we reach here, all attempts have failed
        log::warn!("All embedding attempts failed, using fallback embedding");
        Ok(Self::generate_improved_embedding(text))
    }

    // Improved embedding generation for fallback (works without LLM service)
    // Uses a more sophisticated approach than the simple hash-based method
    fn generate_improved_embedding(text: &str) -> Vec<f32> {
        // Create a fixed-size embedding vector (1536 is standard for many embedding models)
        let mut embedding = vec![0.0f32; 1536];

        // Basic preprocessing - lowercase and trim
        let text = text.to_lowercase();
        let text = text.trim();

        // Use text_preprocessor to get a more meaningful representation
        let words = text_preprocessor::tokenize(text);

        // Generate embedding using a deterministic word-based approach
        // This is still a fallback but better than pure random hash
        for (i, word) in words.iter().enumerate() {
            // Use a word hashing approach for more semantic-like representations
            let word_hash: u64 = word.chars().fold(0, |acc, c| {
                acc.wrapping_add((c as u64).wrapping_mul(31u64.pow(4)))
            });

            // Distribute the word's influence across different dimensions
            for j in 0..5 {
                let idx = ((word_hash + (j as u64 * 127)) % 1536) as usize;
                let value = (((word_hash >> j) & 0xFF) as f32) / 255.0;

                // Weight earlier words more heavily (approximating importance)
                let position_weight = 1.0 / (1.0 + (i as f32 / 10.0));
                embedding[idx] += value * position_weight;
            }
        }

        // Apply positional encoding to capture word order information
        for (pos, word) in words.iter().enumerate().take(100) {
            // Limit to first 100 words
            let pos_f = pos as f32;
            let word_hash = word.chars().fold(0, |acc, c| {
                acc.wrapping_add((c as u32).wrapping_mul(31u32.pow(2)))
            }) as usize;

            // Apply sine wave encoding inspired by transformer models
            for i in 0..8 {
                let dim = word_hash % 192 + i * 192; // Distribute across 1536 dimensions
                if dim < 1536 {
                    let freq = 1.0 / (10000.0_f32.powf(i as f32 / 4.0));
                    embedding[dim] += (pos_f * freq).sin();
                    if dim + 1 < 1536 {
                        embedding[dim + 1] += (pos_f * freq).cos();
                    }
                }
            }
        }

        // Normalize the vector to unit length for cosine similarity
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }

        embedding
    }

    // Keep the simple hash-based embedding as fallback of last resort
    fn generate_simple_embedding(text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0f32; 1536];

        // Simple hash-based approach - deterministic for same text
        for (i, c) in text.chars().enumerate() {
            let idx = (i + c as usize) % 1536;
            embedding[idx] += (c as u32 as f32) / 1000.0;
        }

        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }

        embedding
    }
}

// Implement the MindKbService Trait
#[tonic::async_trait]
impl MindKbService for MindKBServer {
    async fn query_kb(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        let req_data = request.into_inner();

        log::info!(
            "Mind-KB received QueryKB request: query={}, limit={}",
            req_data.query,
            req_data.limit
        );

        // Validate the query and limit
        validate_query(&req_data.query, req_data.limit)
            .map_err(|e| Status::invalid_argument(format!("Invalid query: {}", e)))?;

        // Get embedding for query
        let embedding = self
            .get_embedding(&req_data.query)
            .await
            .map_err(|e| Status::internal(format!("Failed to get embedding: {}", e)))?;

        // Validate the embedding
        validate_embedding(&embedding)
            .map_err(|e| Status::internal(format!("Invalid embedding: {}", e)))?;

        // Apply memory configuration parameters
        let search_limit = if req_data.limit > 0 {
            req_data.limit as u64
        } else {
            self.memory_config.search_depth as u64
        };

        // Search in-memory vector store with configured parameters
        let search_results = self
            .vector_store
            .search_with_config(
                embedding,
                search_limit,
                self.memory_config.similarity_threshold,
            )
            .await
            .map_err(|e| Status::internal(format!("Vector search failed: {}", e)))?;

        // Format results
        let results: Vec<Vec<u8>> = search_results
            .into_iter()
            .map(|(id, score, text)| {
                format!("ID: {}, Score: {:.4}, Content: {}", id, score, text).into_bytes()
            })
            .collect();

        let reply = QueryResponse {
            results: results.clone(),
            count: results.len() as i32,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("kb_type".to_string(), "mind".to_string());
                meta.insert("memory_type".to_string(), "vector".to_string());
                meta.insert("backend".to_string(), "in_memory".to_string());
                meta.insert(
                    "total_entries".to_string(),
                    self.vector_store.entry_count().to_string(),
                );
                meta
            },
        };

        log::info!("Mind-KB query returned {} result(s)", results.len());

        Ok(Response::new(reply))
    }

    async fn store_fact(
        &self,
        request: Request<StoreRequest>,
    ) -> Result<Response<StoreResponse>, Status> {
        let req_data = request.into_inner();

        log::info!(
            "Mind-KB received StoreFact request: key={}, value_size={} bytes",
            req_data.key,
            req_data.value.len()
        );

        // Validate the request
        validate_store_request(&req_data.key, &req_data.value, &req_data.metadata)
            .map_err(|e| Status::invalid_argument(format!("Invalid store request: {}", e)))?;

        // Convert value bytes to string
        let text = String::from_utf8(req_data.value.clone())
            .map_err(|e| Status::invalid_argument(format!("Value must be UTF-8 string: {}", e)))?;

        // Sanitize content
        let sanitized_text = validate_content(&text)
            .map_err(|e| Status::invalid_argument(format!("Content validation failed: {}", e)))?;

        // Validate and sanitize metadata
        let sanitized_metadata = validate_metadata(&req_data.metadata)
            .map_err(|e| Status::invalid_argument(format!("Metadata validation failed: {}", e)))?;

        // Get embedding
        let embedding = self
            .get_embedding(&sanitized_text)
            .await
            .map_err(|e| Status::internal(format!("Failed to get embedding: {}", e)))?;

        // Validate the embedding
        validate_embedding(&embedding)
            .map_err(|e| Status::internal(format!("Invalid embedding: {}", e)))?;

        // Store in vector store
        let mut metadata = sanitized_metadata;
        metadata.insert("key".to_string(), req_data.key.clone());

        let stored_id = self
            .vector_store
            .store(&sanitized_text, embedding, metadata)
            .await
            .map_err(|e| Status::internal(format!("Failed to store: {}", e)))?;

        let reply = StoreResponse {
            success: true,
            stored_id: stored_id.clone(),
        };

        log::info!("Mind-KB stored fact with ID: {}", stored_id);

        Ok(Response::new(reply))
    }

    async fn store(
        &self,
        request: Request<StoreRequest>,
    ) -> Result<Response<StoreResponse>, Status> {
        self.store_fact(request).await
    }

    async fn retrieve(
        &self,
        request: Request<RetrieveRequest>,
    ) -> Result<Response<RetrieveResponse>, Status> {
        let req_data = request.into_inner();

        log::info!("Mind-KB received Retrieve request: key={}", req_data.key);

        // Validate the retrieve request
        validate_retrieve_request(&req_data.key)
            .map_err(|e| Status::invalid_argument(format!("Invalid retrieve request: {}", e)))?;

        // For key-based retrieval, search with the key as query
        let embedding = self
            .get_embedding(&req_data.key)
            .await
            .map_err(|e| Status::internal(format!("Failed to get embedding: {}", e)))?;

        // Validate the embedding
        validate_embedding(&embedding)
            .map_err(|e| Status::internal(format!("Invalid embedding: {}", e)))?;

        // Use memory configuration similarity threshold
        let results = self
            .vector_store
            .search_with_config(embedding, 1, self.memory_config.similarity_threshold)
            .await
            .map_err(|e| Status::internal(format!("Search failed: {}", e)))?;

        if let Some((_, score, text)) = results.first() {
            if *score > self.memory_config.similarity_threshold {
                return Ok(Response::new(RetrieveResponse {
                    value: text.as_bytes().to_vec(),
                    metadata: {
                        let mut meta = std::collections::HashMap::new();
                        meta.insert("kb_type".to_string(), "mind".to_string());
                        meta.insert("score".to_string(), score.to_string());
                        meta
                    },
                    found: true,
                }));
            }
        }

        Ok(Response::new(RetrieveResponse {
            value: Vec::new(),
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("kb_type".to_string(), "mind".to_string());
                meta
            },
            found: false,
        }))
    }

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        self.query_kb(request).await
    }
}

// Helper function to read environment variables with defaults
fn get_env_var<T: FromStr>(name: &str, default: T) -> T {
    env::var(name)
        .ok()
        .and_then(|v| v.parse::<T>().ok())
        .unwrap_or(default)
}

// Main function to start the gRPC server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Read address from environment variable or use the default port 50057
    let addr_str = env::var("MIND_KB_ADDR").unwrap_or_else(|_| "0.0.0.0:50057".to_string());

    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str
            .strip_prefix("http://")
            .unwrap_or(&addr_str)
            .parse()?
    } else {
        addr_str.parse()?
    };

    // Load memory configuration from environment variables
    let memory_config = MemoryConfig {
        short_term_limit: get_env_var("AGENT_MEMORY_SHORT_TERM_LIMIT", 100),
        retention_threshold: get_env_var("AGENT_MEMORY_RETENTION_THRESHOLD", 0.6),
        search_depth: get_env_var("AGENT_MEMORY_SEARCH_DEPTH", 20),
        recency_weight: get_env_var("AGENT_MEMORY_RECENCY_WEIGHT", 0.4),
        importance_weight: get_env_var("AGENT_MEMORY_IMPORTANCE_WEIGHT", 0.6),
        emotional_weight: get_env_var("AGENT_MEMORY_EMOTIONAL_WEIGHT", 0.5),
        similarity_threshold: get_env_var("AGENT_MEMORY_SIMILARITY_THRESHOLD", 0.75),
        max_context_items: get_env_var("AGENT_MEMORY_MAX_CONTEXT_ITEMS", 15),
    };

    log::info!("Mind-KB loaded memory configuration: {:?}", memory_config);

    // Initialize in-memory vector store with memory configuration
    let vector_store = Arc::new(VectorStore::new_with_config(
        memory_config.short_term_limit,
        memory_config.retention_threshold,
    ));

    // LLM Service URL (optional - fallback embedding works without it)
    let llm_service_url =
        env::var("LLM_SERVICE_ADDR").unwrap_or_else(|_| "http://127.0.0.1:50053".to_string());

    let mind_kb_server = MindKBServer {
        vector_store,
        llm_client_url: llm_service_url,
        memory_config,
    };

    log::info!("Mind-KB Service (bare-metal) starting on {}", addr);
    println!("Mind-KB Service listening on {}", addr);

    // Initialize start time
    let _ = *START_TIME;

    let mind_kb_server = Arc::new(mind_kb_server);
    let kb_for_health = mind_kb_server.clone();

    // Clone the vector store for the background task
    let vector_store_for_background = mind_kb_server.vector_store.clone();

    // Start background task for temporal decay processing
    tokio::spawn(async move {
        // Begin the first run after a short delay to let the server start
        time::sleep(Duration::from_secs(30)).await;

        // Create an interval for 12 hours (in seconds)
        let decay_interval = env::var("MIND_DECAY_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(12 * 60 * 60); // Default: 12 hours

        log::info!(
            "Starting temporal decay background task - running every {} seconds",
            decay_interval
        );

        // Create a repeating interval
        let mut interval = time::interval(Duration::from_secs(decay_interval));

        loop {
            interval.tick().await; // Wait for the next interval

            // Run the decay processing
            match vector_store_for_background.process_decay().await {
                Ok(count) => log::info!(
                    "Temporal decay processing completed successfully. Processed {} entries",
                    count
                ),
                Err(e) => log::error!("Temporal decay processing failed: {}", e),
            }
        }
    });

    log::info!("Background maintenance tasks initialized");

    Server::builder()
        .add_service(MindKbServiceServer::from_arc(mind_kb_server))
        .add_service(HealthServiceServer::from_arc(kb_for_health))
        .serve(addr)
        .await?;

    Ok(())
}

// Implement HealthService for MindKBServer
#[tonic::async_trait]
impl HealthService for MindKBServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;

        let entry_count = self.vector_store.len();
        let mut dependencies = HashMap::new();
        dependencies.insert(
            "vector_store".to_string(),
            format!("ACTIVE ({} entries)", entry_count),
        );

        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "mind-kb-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}
