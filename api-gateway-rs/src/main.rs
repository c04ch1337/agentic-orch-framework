// api-gateway-rs/src/main.rs
// API Gateway - REST to gRPC Translation Layer with Enhanced Validation
// Port 8000 - HTTP/REST entry point for external clients
//
// Implements:
// - Secure API key management and token-based authentication
// - Comprehensive input validation and sanitization
// - Request payload size limits
// - Schema-based request validation
// - Content-type validation and enforcement

use axum::{
    Router,
    routing::{get, post},
    http::StatusCode,
    response::IntoResponse,
    Json,
    extract::{State, Path},
    http::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE},
    middleware::{self, Next},
    body::{Body, Bytes},
    response::{Response},
    BoxError,
};
use std::sync::Arc;
use std::time::Instant;
use std::env;
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;
use tokio::sync::{RwLock, Mutex};
use tower_http::cors::{CorsLayer, Any};
use tower_http::limit::RequestBodyLimitLayer;
use config_rs::ServiceConfig;

// Import our module
mod validation;
mod secrets_client;
mod auth_client;
mod auth_middleware;

// Import dependencies
use secrets_client::{SecretsClient, SecretsError};
use auth_middleware::{auth_middleware, permission_middleware, init_auth_client, generate_client_token, is_auth_healthy};
use validation::{
    validate_content_type,
    validate_request,
    sanitize_request,
    sanitize_json_input,
    payload_limit_config,
    ApiValidationError,
    ValidationErrorResponse,
    MAX_PAYLOAD_SIZE
};

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

/// Secrets client for API key management
static SECRETS_CLIENT: Lazy<Mutex<Option<SecretsClient>>> = Lazy::new(|| {
    Mutex::new(None)
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
    secrets_client: Arc<Mutex<Option<SecretsClient>>>,
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

// New secure API key validation that uses the auth service
async fn validate_api_key(headers: &HeaderMap, state: &AppState) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    // Auth middleware now handles all token validation
    // This function is only kept for backwards compatibility
    // and will be removed in the future
    Ok(())
}

/// Middleware for validating request content type
async fn validate_content_type_middleware(
    headers: HeaderMap,
    uri: axum::http::Uri,
    next: Next,
) -> Result<Response, (StatusCode, Json<ValidationErrorResponse>)> {
    let path = uri.path();
    
    // Skip validation for non-API paths
    if !path.starts_with("/api/v1/") {
        return Ok(next.run().await);
    }
    
    // Different content-type requirements per path
    let required_content_type = match path {
        "/api/v1/execute" => "application/json",
        _ => return Ok(next.run().await), // Skip validation for other paths
    };
    
    // Validate content type
    if let Err(err) = validate_content_type(&headers, required_content_type) {
        let (status, response) = err.to_response();
        return Err((status, response));
    }
    
    Ok(next.run().await)
}

/// Middleware for validating and sanitizing request body
async fn validate_request_middleware(
    uri: axum::http::Uri,
    method: axum::http::Method,
    headers: HeaderMap,
    body: axum::body::Body,
    next: Next,
) -> Result<Response, (StatusCode, Json<ValidationErrorResponse>)> {
    let path = uri.path();
    
    // Skip validation for non-API paths or non-POST methods
    if !path.starts_with("/api/v1/") || method != axum::http::Method::POST {
        return Ok(next.run().await);
    }
    
    // Skip validation for specific endpoints
    if path == "/api/v1/token" || path == "/api/v1/health" {
        return Ok(next.run().await);
    }
    
    // Collect the full body
    let body_bytes = match hyper::body::to_bytes(body).await {
        Ok(bytes) => bytes,
        Err(e) => {
            let error = ApiValidationError::InvalidFormat(
                format!("Failed to read request body: {}", e)
            );
            return Err(error.to_response());
        }
    };
    
    // Check request size
    if body_bytes.len() > MAX_PAYLOAD_SIZE {
        let error = ApiValidationError::PayloadTooLarge(
            format!("Request size {} exceeds maximum allowed size {}", body_bytes.len(), MAX_PAYLOAD_SIZE)
        );
        return Err(error.to_response());
    }
    
    // Convert body to UTF-8 string
    let body_str = match std::str::from_utf8(&body_bytes) {
        Ok(s) => s,
        Err(_) => {
            let error = ApiValidationError::InvalidFormat("Request body is not valid UTF-8".to_string());
            return Err(error.to_response());
        }
    };
    
    // Sanitize and parse JSON
    let sanitized_result = sanitize_json_input(body_str);
    if let Err(err) = sanitized_result {
        return Err(err.to_response());
    }
    
    let mut json_value = sanitized_result.unwrap();
    
    // Validate against endpoint schema
    if let Err(err) = validate_request(path, &json_value) {
        return Err(err.to_response());
    }
    
    // Sanitize request based on endpoint
    sanitize_request(path, &mut json_value);
    
    // Convert back to string
    let sanitized_body = match serde_json::to_string(&json_value) {
        Ok(body) => body,
        Err(err) => {
            let error = ApiValidationError::InvalidFormat(
                format!("Failed to serialize sanitized request: {}", err)
            );
            return Err(error.to_response());
        }
    };
    
    // Create new request with sanitized body
    let req_builder = axum::http::Request::builder()
        .uri(uri)
        .method(method);
    
    // Copy original headers
    let mut req_builder = headers.iter().fold(req_builder, |builder, (name, value)| {
        builder.header(name, value)
    });
    
    let request = req_builder
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(sanitized_body))
        .unwrap();
    
    // Continue to next middleware or handler
    Ok(next.run(request).await)
}

/// POST /api/v1/execute - Execute request via Orchestrator
async fn execute_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ExecuteRequest>,
) -> impl IntoResponse {
    // Auth middleware has already validated the token
    // We can proceed directly to execution
    
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
async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime = START_TIME.elapsed().as_secs() as i64;
    
    // Check if secrets client is available
    let secrets_healthy = {
        let secrets_guard = state.secrets_client.lock().await;
        if let Some(secrets) = &*secrets_guard {
            secrets.is_healthy().await
        } else {
            false
        }
    };
    
    // Check if auth service is healthy
    let auth_healthy = is_auth_healthy().await;
    
    // Determine service status
    let status = if secrets_healthy && auth_healthy {
        "SERVING"
    } else if !secrets_healthy && !auth_healthy {
        "CRITICAL"
    } else {
        "DEGRADED"
    };
    
    // Create a response with dependency status
    let response = HealthResponse {
        healthy: secrets_healthy || auth_healthy, // Gateway is healthy if either auth or secrets is available
        service_name: "api-gateway".to_string(),
        uptime_seconds: uptime,
        status: status.to_string(),
    };
    
    Json(response)
}

/// GET /token - Generate a new client token using the auth service
async fn generate_token_handler(headers: HeaderMap) -> impl IntoResponse {
    // Generate a new token using auth service
    match generate_client_token().await {
        Ok(token_data) => {
            Json(serde_json::json!({
                "token": token_data.token,
                "expires_at": token_data.expires_at,
                "roles": token_data.roles,
                "permissions": token_data.permissions,
            })).into_response()
        }
        Err(err) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to generate token: {}", err),
                    code: 500,
                }),
            ).into_response()
        }
    }
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
    // Load .env file
    dotenv::dotenv().ok();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let _ = *START_TIME;
    
    // Use standardized configuration for ports and addresses
    let service_config = ServiceConfig::new("api-gateway");
    let port = service_config.get_service_port(8000);
    let orchestrator_addr = service_config.get_client_address("orchestrator", 50051);
    
    log::info!("Using API Gateway port: {}", port);
    log::info!("Using Orchestrator address: {}", orchestrator_addr);
        
    // Initialize secrets client
    let secrets_client = match SecretsClient::new().await {
        Ok(client) => {
            log::info!("Successfully connected to secrets service");
            if client.is_mock() {
                log::warn!("Secrets service running in mock mode - falling back to environment variables");
            }
            Some(client)
        },
        Err(err) => {
            log::warn!("Failed to connect to secrets service: {}. Using environment variable for API key.", err);
            None
        }
    };
    
    // Initialize global secrets client for other functions to use
    {
        let mut global_client = SECRETS_CLIENT.lock().await;
        *global_client = secrets_client.clone();
    }
    
    // Try to get the default API key from secrets service
    if let Some(client) = &secrets_client {
        match client.get_default_api_key().await {
            Ok(_) => log::info!("Successfully retrieved default API key from secrets service"),
            Err(err) => log::warn!("Failed to retrieve default API key: {}. Using environment variable.", err),
        }
    }
    
    let state = Arc::new(AppState {
        orchestrator_addr: orchestrator_addr.clone(),
        secrets_client: Arc::new(Mutex::new(secrets_client)),
    });
    
    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
        
    // Add request size limit layer
    let limit_layer = payload_limit_config();
    
    // Initialize auth client
    // Use standardized configuration for auth service address
    let auth_service_addr = service_config.get_client_address("auth-service", 50090);
    log::info!("Using Auth Service address: {}", auth_service_addr);
        
    let service_id = env::var("SERVICE_ID")
        .unwrap_or_else(|_| "api-gateway".to_string());
        
    let client_id = env::var("CLIENT_ID")
        .unwrap_or_else(|_| "api-gateway-client".to_string());
        
    let client_secret = env::var("CLIENT_SECRET")
        .unwrap_or_else(|_| "default-client-secret".to_string());
        
    let use_mtls = env::var("USE_MTLS")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
        
    // Initialize auth client
    match init_auth_client(
        &auth_service_addr,
        &service_id,
        &client_id,
        &client_secret,
        use_mtls,
        None, // cert_path
        None, // key_path
        None, // ca_path
    ).await {
        Ok(_) => {
            log::info!("Successfully initialized auth client");
        }
        Err(err) => {
            log::warn!("Failed to initialize auth client: {}", err);
            log::warn!("Proceeding with limited authentication capabilities");
        }
    }
    
    // Build router with middleware chain
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/api/v1/execute", post(execute_handler))
        .route("/api/v1/token", get(generate_token_handler))
        // Add auth middleware first
        .layer(middleware::from_fn(auth_middleware))
        .layer(middleware::from_fn(permission_middleware))
        // Then validation middleware
        .layer(middleware::from_fn(validate_content_type_middleware))
        .layer(middleware::from_fn(validate_request_middleware))
        .layer(limit_layer)
        .layer(cors)
        .with_state(state);
    
    // Use standardized bind address function
    let addr = service_config.get_bind_address(port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    log::info!("API Gateway starting on {}", addr);
    log::info!("Orchestrator target: {}", orchestrator_addr);
    println!("API Gateway listening on {}", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
