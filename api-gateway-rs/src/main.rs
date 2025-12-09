// api-gateway-rs/src/main.rs
// API Gateway - REST to gRPC Translation Layer
// Port 8000 - HTTP/REST entry point for external clients

use axum::{
    Router,
    routing::{get, post},
    http::StatusCode,
    response::IntoResponse,
    Json,
    extract::State,
    http::header::{HeaderMap, AUTHORIZATION},
};
use std::sync::Arc;
use std::time::Instant;
use std::env;
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use tower_http::cors::{CorsLayer, Any};

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

/// Expected API Key (set via environment variable)
static API_KEY: Lazy<String> = Lazy::new(|| {
    env::var("API_KEY").unwrap_or_else(|_| "phoenix-default-key".to_string())
});

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    orchestrator_service_client::OrchestratorServiceClient,
    Request as ProtoRequest,
};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    orchestrator_addr: String,
}

/// Execute request body (JSON)
#[derive(Debug, Deserialize)]
pub struct ExecuteRequest {
    pub id: Option<String>,
    pub method: String,
    pub payload: String,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Execute response body (JSON)
#[derive(Debug, Serialize)]
pub struct ExecuteResponse {
    pub id: String,
    pub status_code: i32,
    pub payload: String,
    pub error: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub healthy: bool,
    pub service_name: String,
    pub uptime_seconds: i64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

/// Validate API key from Authorization header
fn validate_api_key(headers: &HeaderMap) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    let key = if auth_header.starts_with("Bearer ") {
        auth_header.strip_prefix("Bearer ").unwrap_or("")
    } else {
        auth_header
    };
    
    if key != API_KEY.as_str() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid or missing API key".to_string(),
                code: 401,
            }),
        ));
    }
    
    Ok(())
}

/// POST /api/v1/execute - Execute request via Orchestrator
async fn execute_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ExecuteRequest>,
) -> impl IntoResponse {
    // Validate API key
    if let Err(err_response) = validate_api_key(&headers) {
        return err_response.into_response();
    }
    
    log::info!("Execute request: method={}, id={:?}", request.method, request.id);
    
    // Connect to Orchestrator
    let client_result = OrchestratorServiceClient::connect(state.orchestrator_addr.clone()).await;
    
    match client_result {
        Ok(mut client) => {
            let request_id = request.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            
            // Create gRPC request
            let proto_request = ProtoRequest {
                id: request_id.clone(),
                service: "orchestrator".to_string(),
                method: request.method.clone(),
                payload: request.payload.into_bytes(),
                metadata: request.metadata,
            };
            
            // Call Orchestrator directly with Request
            match client.process_request(tonic::Request::new(proto_request)).await {
                Ok(response) => {
                    let inner = response.into_inner();
                    let is_success = inner.status_code >= 200 && inner.status_code < 300;
                    (
                        StatusCode::OK,
                        Json(ExecuteResponse {
                            id: inner.id,
                            status_code: inner.status_code,
                            payload: String::from_utf8_lossy(&inner.payload).to_string(),
                            error: if is_success { None } else { Some(inner.error) },
                            metadata: inner.metadata,
                        }),
                    ).into_response()
                }
                Err(e) => {
                    log::error!("Orchestrator gRPC error: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: format!("Orchestrator error: {}", e),
                            code: 500,
                        }),
                    ).into_response()
                }
            }
        }
        Err(e) => {
            log::error!("Failed to connect to Orchestrator: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: format!("Orchestrator unavailable: {}", e),
                    code: 503,
                }),
            ).into_response()
        }
    }
}

/// GET /health - Health check endpoint
async fn health_handler() -> impl IntoResponse {
    let uptime = START_TIME.elapsed().as_secs() as i64;
    
    Json(HealthResponse {
        healthy: true,
        service_name: "api-gateway".to_string(),
        uptime_seconds: uptime,
        status: "SERVING".to_string(),
    })
}

/// GET / - Root endpoint
async fn root_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "PHOENIX ORCH API Gateway",
        "version": "1.0.0",
        "endpoints": [
            "GET /health",
            "POST /api/v1/execute"
        ]
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let _ = *START_TIME;
    let _ = API_KEY.as_str(); // Initialize API key
    
    let port = env::var("API_GATEWAY_PORT").unwrap_or_else(|_| "8000".to_string());
    let orchestrator_addr = env::var("ORCHESTRATOR_ADDR")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    
    let state = Arc::new(AppState {
        orchestrator_addr: orchestrator_addr.clone(),
    });
    
    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // Build router
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/api/v1/execute", post(execute_handler))
        .layer(cors)
        .with_state(state);
    
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    log::info!("API Gateway starting on {}", addr);
    log::info!("Orchestrator target: {}", orchestrator_addr);
    println!("API Gateway listening on {}", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
