// heart-kb-rs/src/main.rs
// Main Entry Point for heart-kb-rs
// Implements the HeartKBService gRPC server with Sentiment Analysis

use tonic::{transport::Server, Request, Response, Status};
use std::sync::Arc;
use std::time::Instant;
use std::net::SocketAddr;
use std::env;
use std::collections::HashMap;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

// Import validation module
mod validation;
use validation::{
    validate_query,
    validate_store_request,
    validate_retrieve_request,
    validate_sentiment_fact,
    validate_source_id,
};

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    heart_kb_service_server::{HeartKbService, HeartKbServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    QueryRequest,
    QueryResponse,
    StoreRequest,
    StoreResponse,
    RetrieveRequest,
    RetrieveResponse,
    HealthRequest,
    HealthResponse,
    // Sentiment types
    StoreSentimentRequest,
    StoreSentimentResponse,
    GetStateRequest,
    AgiState,
    SentimentFact,
    Sentiment,
};

/// User sentiment state tracking
#[derive(Debug, Clone)]
struct UserSentimentState {
    current_sentiment: i32,
    confidence: f32,
    last_updated: i64,
    history: Vec<SentimentFact>,
}

impl Default for UserSentimentState {
    fn default() -> Self {
        Self {
            current_sentiment: 0, // SENTIMENT_NEUTRAL
            confidence: 0.5,
            last_updated: 0,
            history: Vec::new(),
        }
    }
}

// Define the Heart KB Server Structure
#[derive(Debug)]
pub struct HeartKBServer {
    /// Per-user sentiment states
    sentiment_states: Arc<RwLock<HashMap<String, UserSentimentState>>>,
}

impl Default for HeartKBServer {
    fn default() -> Self {
        Self {
            sentiment_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// Implement the HeartKbService Trait
// This KB handles personality, emotional state, and motivational drives
#[tonic::async_trait]
impl HeartKbService for HeartKBServer {
    async fn query_kb(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        let req_data = request.into_inner();
        
        log::info!(
            "Heart-KB received QueryKB request: query={}, limit={}",
            req_data.query,
            req_data.limit
        );

        // Validate query and limit using our validation library
        if let Err(err) = validate_query(&req_data.query, req_data.limit) {
            log::warn!("Query validation failed: {}", err);
            return Err(Status::invalid_argument(format!("Invalid query parameters: {}", err)));
        }

        // --- QUERY STUB (Emotional/Motivational State) ---
        // In a real scenario, this would involve:
        // 1. Querying current emotional state (happiness, curiosity, frustration, etc.)
        // 2. Retrieving motivational drives and their intensities
        // 3. Getting personality traits and preferences
        // 4. Returning emotional context for decision-making
        // Retrieves current emotional state (e.g., 'Curiosity: 0.8', 'Frustration: 0.1')
        
        // Stub: return mock emotional/motivational data
        let results = vec![
            format!("Heart-KB stub emotional state for query: '{}' - Happiness: 0.7, Curiosity: 0.9, Frustration: 0.1", req_data.query).into_bytes(),
            format!("Heart-KB stub motivational drives: Exploration: 0.8, Learning: 0.9, Connection: 0.6",).into_bytes(),
        ];

        let reply = QueryResponse {
            results: results.clone(),
            count: results.len() as i32,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("kb_type".to_string(), "heart".to_string());
                meta.insert("state_type".to_string(), "emotional".to_string());
                meta.insert("query_type".to_string(), "motivational".to_string());
                meta
            },
        };

        log::info!("Heart-KB query returned {} emotional/motivational result(s)", results.len());

        Ok(Response::new(reply))
    }

    async fn store_fact(
        &self,
        request: Request<StoreRequest>,
    ) -> Result<Response<StoreResponse>, Status> {
        let req_data = request.into_inner();
        
        log::info!(
            "Heart-KB received StoreFact request: key={}, value_size={} bytes",
            req_data.key,
            req_data.value.len()
        );

        // Validate store request
        match validate_store_request(&req_data.key, &req_data.value, &req_data.metadata) {
            Ok((sanitized_key, sanitized_value, sanitized_metadata)) => {
                // Use sanitized values in real implementation
                log::info!("Validated store request: key={}, value_size={} bytes",
                    sanitized_key, sanitized_value.len());
                
                // For now we just log the sanitized data since this is a stub implementation
            },
            Err(err) => {
                log::warn!("Store request validation failed: {}", err);
                return Err(Status::invalid_argument(format!("Invalid store request: {}", err)));
            }
        }

        // --- STORE STUB (Emotional Update) ---
        // In a real scenario, this would involve:
        // 1. Validating emotional/motivational data structure
        // 2. Updating personality variables or emotional baseline
        // 3. Storing motivational drives and their intensities
        // 4. Applying emotional state transitions
        // Updates the agent's core personality variables, emotional baseline, or motivational drives
        
        // Generate a stored ID
        let stored_id = format!("heart-{}", req_data.key);

        let reply = StoreResponse {
            success: true,
            stored_id: stored_id.clone(),
        };

        log::info!("Heart-KB stored emotional fact with ID: {}", stored_id);

        Ok(Response::new(reply))
    }

    async fn store(
        &self,
        request: Request<StoreRequest>,
    ) -> Result<Response<StoreResponse>, Status> {
        // Alias for StoreFact - delegate to the same implementation
        self.store_fact(request).await
    }

    async fn retrieve(
        &self,
        request: Request<RetrieveRequest>,
    ) -> Result<Response<RetrieveResponse>, Status> {
        let req_data = request.into_inner();
        
        log::info!(
            "Heart-KB received Retrieve request: key={}, filters={:?}",
            req_data.key,
            req_data.filters
        );

        // Validate retrieve request
        match validate_retrieve_request(&req_data.key, &req_data.filters) {
            Ok((sanitized_key, sanitized_filters)) => {
                // Use sanitized values in real implementation
                log::info!("Validated retrieve request: key={}, filters={:?}",
                    sanitized_key, sanitized_filters);
                
                // For now we just log the sanitized data since this is a stub implementation
            },
            Err(err) => {
                log::warn!("Retrieve request validation failed: {}", err);
                return Err(Status::invalid_argument(format!("Invalid retrieve request: {}", err)));
            }
        }

        // --- RETRIEVE STUB ---
        // In a real scenario, this would involve:
        // 1. Looking up emotional/motivational data by key
        // 2. Applying filters if provided
        // 3. Retrieving personality traits or emotional states
        // 4. Returning the stored emotional value
        // For now, we return a stub response
        
        // Stub: return mock emotional data
        let value = format!("Heart-KB retrieved emotional state for key: '{}' - Happiness: 0.7, Curiosity: 0.9", req_data.key).into_bytes();

        let reply = RetrieveResponse {
            value: value.clone(),
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("kb_type".to_string(), "heart".to_string());
                meta.insert("retrieved_at".to_string(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string());
                meta.insert("state_type".to_string(), "emotional".to_string());
                meta
            },
            found: true,
        };

        log::info!("Heart-KB retrieved emotional state for key: {}", req_data.key);

        Ok(Response::new(reply))
    }

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        // Alias for QueryKB - delegate to the same implementation
        self.query_kb(request).await
    }

    /// Store sentiment from user interaction
    async fn store_sentiment(
        &self,
        request: Request<StoreSentimentRequest>,
    ) -> Result<Response<StoreSentimentResponse>, Status> {
        let req = request.into_inner();
        let fact = req.fact.ok_or_else(|| Status::invalid_argument("Missing sentiment fact"))?;
        
        // Validate sentiment fact
        let validated_fact = match validate_sentiment_fact(&fact) {
            Ok(validated) => validated,
            Err(err) => {
                log::warn!("Sentiment fact validation failed: {}", err);
                return Err(Status::invalid_argument(format!("Invalid sentiment fact: {}", err)));
            }
        };
        
        let source_id = validated_fact.source_id.clone();
        let new_sentiment = validated_fact.sentiment;
        
        log::info!("Heart-KB StoreSentiment: source={}, sentiment={:?}, confidence={}",
            source_id, new_sentiment, validated_fact.confidence_score);
        
        let mut states = self.sentiment_states.write().await;
        let state = states.entry(source_id.clone()).or_default();
        
        // Check for significant sentiment shift
        let previous_sentiment = state.current_sentiment;
        let sentiment_shift = previous_sentiment != new_sentiment;
        
        // Update state with validated data
        state.current_sentiment = new_sentiment;
        state.confidence = validated_fact.confidence_score;
        state.last_updated = validated_fact.timestamp;
        state.history.push(validated_fact);
        
        // Keep history bounded (last 100 entries)
        if state.history.len() > 100 {
            state.history.remove(0);
        }
        
        if sentiment_shift {
            log::warn!("Sentiment SHIFT detected for {}: {:?} -> {:?}",
                source_id, previous_sentiment, new_sentiment);
            // TODO: Notify Orchestrator via Data Router for priority re-evaluation
        }
        
        Ok(Response::new(StoreSentimentResponse {
            success: true,
            sentiment_shift_detected: sentiment_shift,
            previous_sentiment,
            current_sentiment: new_sentiment,
        }))
    }

    /// Query current AGI state for a source
    async fn query_state(
        &self,
        request: Request<GetStateRequest>,
    ) -> Result<Response<AgiState>, Status> {
        let req = request.into_inner();
        let source_id = req.source_id;
        
        // Validate source ID
        let sanitized_source_id = match validate_source_id(&source_id) {
            Ok(id) => id,
            Err(err) => {
                log::warn!("Source ID validation failed: {}", err);
                return Err(Status::invalid_argument(format!("Invalid source ID: {}", err)));
            }
        };
        
        log::info!("Heart-KB QueryState: source={}", sanitized_source_id);
        
        let states = self.sentiment_states.read().await;
        
        let state = states.get(&sanitized_source_id)
            .cloned()
            .unwrap_or_default();
        
        let dominant_emotion = match state.current_sentiment {
            0 => "Neutral",     // SENTIMENT_NEUTRAL
            1 => "Urgent",      // SENTIMENT_URGENT
            2 => "Anxious",     // SENTIMENT_ANXIOUS
            3 => "Frustrated",  // SENTIMENT_FRUSTRATED
            4 => "Confident",   // SENTIMENT_CONFIDENT
            5 => "Positive",    // SENTIMENT_POSITIVE
            6 => "Negative",    // SENTIMENT_NEGATIVE
            _ => "Unknown",
        };
        
        Ok(Response::new(AgiState {
            current_sentiment: state.current_sentiment,
            confidence: state.confidence,
            last_updated: state.last_updated,
            dominant_emotion: dominant_emotion.to_string(),
            context: HashMap::new(),
        }))
    }
}

// Main function to start the gRPC server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Read address from environment variable or use the default port 50059
    let addr_str = env::var("HEART_KB_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50059".to_string());
    
    // Parse the address, handling both "0.0.0.0:50059" and "http://127.0.0.1:50059" formats
    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str
            .strip_prefix("http://")
            .unwrap_or(&addr_str)
            .parse()?
    } else {
        addr_str.parse()?
    };

    let heart_kb_server = HeartKBServer::default();

    log::info!("Heart-KB Service starting on {}", addr);
    println!("Heart-KB Service listening on {}", addr);

    let _ = *START_TIME;
    let heart_kb_server = Arc::new(heart_kb_server);
    let kb_for_health = heart_kb_server.clone();

    Server::builder()
        .add_service(HeartKbServiceServer::from_arc(heart_kb_server))
        .add_service(HealthServiceServer::from_arc(kb_for_health))
        .serve(addr)
        .await?;

    Ok(())
}

#[tonic::async_trait]
impl HealthService for HeartKBServer {
    async fn get_health(&self, _request: Request<HealthRequest>) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let mut dependencies = HashMap::new();
        dependencies.insert("kb_storage".to_string(), "ACTIVE".to_string());
        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "heart-kb-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}
