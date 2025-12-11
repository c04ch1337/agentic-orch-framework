use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, Mutex};
use axum::{
    Router,
    routing::{get, post},
    http::StatusCode,
    response::IntoResponse,
    Json,
    extract::{State, Path},
    http::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE},
    middleware::{self, Next},
    body::{Body, Bytes, to_bytes},
    response::{Response},
    BoxError,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{CorsLayer, Any};
use tower_http::limit::RequestBodyLimitLayer;

pub mod validation;
pub mod secrets_client;
pub mod auth_client;
pub mod auth_middleware;
pub mod phoenix_auth;
pub mod rate_limit;

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

/// Execute request body (JSON)
#[derive(Debug, Deserialize)]
pub struct ExecuteRequest {
    pub id: Option<String>,
    pub method: String,
    pub payload: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

 /// Execute response body (JSON) - Using the unified AgiResponse schema
#[derive(Debug, Serialize)]
pub struct ExecuteResponse {
    pub final_answer: String,
    pub execution_plan: String,
    pub routed_service: String,
    pub phoenix_session_id: String,
    pub output_artifact_urls: Vec<String>,
}

/// Determine whether this request should use PlanAndExecute orchestration.
fn is_plan_and_execute(request: &ExecuteRequest) -> bool {
    let method = request.method.to_ascii_lowercase();

    let method_indicates_plan = matches!(
        method.as_str(),
        "plan_and_execute" | "orchestrated_chat"
    );

    let metadata_indicates_plan = request
        .metadata
        .get("orchestration_mode")
        .map(|v| v.eq_ignore_ascii_case("plan_and_execute"))
        .unwrap_or(false);

    method_indicates_plan || metadata_indicates_plan
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

/// Core API Gateway state and functionality
pub struct ApiGateway {
    orchestrator_addr: String,
    secrets_client: Arc<Mutex<Option<secrets_client::SecretsClient>>>,
}

impl ApiGateway {
    pub fn new(orchestrator_addr: String) -> Self {
        Self {
            orchestrator_addr,
            secrets_client: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize the API Gateway with required clients and middleware
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize secrets client
        let secrets_client = match secrets_client::SecretsClient::new().await {
            Ok(client) => {
                tracing::info!("Successfully connected to secrets service");
                if client.is_mock() {
                    tracing::warn!("Secrets service running in mock mode");
                }
                Some(client)
            },
            Err(err) => {
                tracing::warn!("Failed to connect to secrets service: {}", err);
                None
            }
        };

        let mut guard = self.secrets_client.lock().await;
        *guard = secrets_client;

        Ok(())
    }

    /// Create the Axum router with all routes and middleware
    pub fn create_router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/", get(Self::root_handler))
            .route("/health", get(Self::health_handler))
            .route("/api/v1/execute", post(Self::execute_handler))
            .route("/api/v1/token", get(Self::generate_token_handler))
            // Add Phoenix auth and rate limiting first
            .layer(middleware::from_fn(phoenix_auth::phoenix_auth_interceptor))
            .layer(middleware::from_fn(rate_limit::tower_governor_rate_limiter))
            // Then existing auth middleware
            .layer(middleware::from_fn(auth_middleware::auth_middleware))
            .layer(middleware::from_fn(auth_middleware::permission_middleware))
            // Then validation middleware
            .layer(middleware::from_fn(Self::validate_content_type_middleware))
            .layer(middleware::from_fn(Self::validate_request_middleware))
            .layer(payload_limit_config())
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
            .with_state(self)
    }

    // Handler implementations moved from main.rs
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

    async fn health_handler(State(state): State<Arc<Self>>) -> impl IntoResponse {
        let uptime = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

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
        let auth_healthy = auth_middleware::is_auth_healthy().await;

        // Determine service status
        let status = if secrets_healthy && auth_healthy {
            "SERVING"
        } else if !secrets_healthy && !auth_healthy {
            "CRITICAL"
        } else {
            "DEGRADED"
        };

        Json(HealthResponse {
            healthy: secrets_healthy || auth_healthy,
            service_name: "api-gateway".to_string(),
            uptime_seconds: uptime,
            status: status.to_string(),
        })
    }

    async fn execute_handler(
        State(state): State<Arc<Self>>,
        headers: HeaderMap,
        Json(request): Json<ExecuteRequest>,
    ) -> impl IntoResponse {
        tracing::info!(
            "Execute request: method={}, id={:?}",
            request.method,
            request.id
        );

        // Connect to Orchestrator
        let client_result = tonic::transport::Channel::from_shared(state.orchestrator_addr.clone())
            .unwrap()
            .connect()
            .await;

        match client_result {
            Ok(channel) => {
                let request_id = request
                    .id
                    .clone()
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                let is_plan = is_plan_and_execute(&request);

                let service_for_plan = request
                    .metadata
                    .get("target_service")
                    .cloned()
                    // empty string means "orchestrator decides", which will default to llm-service
                    .unwrap_or_else(String::new);

                // Create gRPC request
                let proto_request = agi_core::Request {
                    id: request_id.clone(),
                    service: if is_plan {
                        service_for_plan
                    } else {
                        "orchestrator".to_string()
                    },
                    method: request.method.clone(),
                    payload: request.payload.clone().into_bytes(),
                    metadata: request.metadata.clone(),
                };

                // Call Orchestrator
                let mut client =
                    agi_core::orchestrator_service_client::OrchestratorServiceClient::new(channel);

                if is_plan {
                    match client.plan_and_execute(tonic::Request::new(proto_request)).await {
                        Ok(response) => {
                            let agi_response = response.into_inner();

                            (
                                StatusCode::OK,
                                Json(ExecuteResponse {
                                    final_answer: agi_response.final_answer,
                                    execution_plan: agi_response.execution_plan,
                                    routed_service: agi_response.routed_service,
                                    phoenix_session_id: agi_response.phoenix_session_id,
                                    output_artifact_urls: agi_response.output_artifact_urls,
                                }),
                            )
                                .into_response()
                        }
                        Err(e) => {
                            tracing::error!(
                                "Orchestrator PlanAndExecute gRPC error: {}",
                                e
                            );
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ErrorResponse {
                                    error: format!(
                                        "Orchestrator PlanAndExecute error: {}",
                                        e
                                    ),
                                    code: 500,
                                }),
                            )
                                .into_response()
                        }
                    }
                } else {
                    match client.process_request(tonic::Request::new(proto_request)).await {
                        Ok(response) => {
                            let agi_response = response.into_inner();

                            (
                                StatusCode::OK,
                                Json(ExecuteResponse {
                                    final_answer: agi_response.final_answer,
                                    execution_plan: agi_response.execution_plan,
                                    routed_service: agi_response.routed_service,
                                    phoenix_session_id: agi_response.phoenix_session_id,
                                    output_artifact_urls: agi_response.output_artifact_urls,
                                }),
                            )
                                .into_response()
                        }
                        Err(e) => {
                            tracing::error!("Orchestrator gRPC error: {}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ErrorResponse {
                                    error: format!("Orchestrator error: {}", e),
                                    code: 500,
                                }),
                            )
                                .into_response()
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to connect to Orchestrator: {}", e);
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(ErrorResponse {
                        error: format!("Orchestrator unavailable: {}", e),
                        code: 503,
                    }),
                )
                    .into_response()
            }
        }
    }

    async fn generate_token_handler(headers: HeaderMap) -> impl IntoResponse {
        match auth_middleware::generate_client_token().await {
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

        // Middleware implementations moved from main.rs
        async fn validate_content_type_middleware(
            req: axum::http::Request<Body>,
            next: Next,
        ) -> Result<Response, (StatusCode, Json<ValidationErrorResponse>)> {
            let path = req.uri().path().to_string();
    
            if !path.starts_with("/api/v1/") {
                return Ok(next.run(req).await);
            }
    
            let required_content_type = match path.as_str() {
                "/api/v1/execute" => "application/json",
                _ => return Ok(next.run(req).await),
            };
    
            if let Err(err) = validate_content_type(req.headers(), required_content_type) {
                let (status, response) = err.to_response();
                return Err((status, response));
            }
    
            Ok(next.run(req).await)
        }
    
        async fn validate_request_middleware(
            req: axum::http::Request<Body>,
            next: Next,
        ) -> Result<Response, (StatusCode, Json<ValidationErrorResponse>)> {
            // Implementation moved from main.rs
            let (parts, body) = req.into_parts();
            let uri = parts.uri.clone();
            let method = parts.method.clone();
            let headers = parts.headers.clone();
    
            let path = uri.path().to_string();
    
            if !path.starts_with("/api/v1/") || method != axum::http::Method::POST {
                let req = axum::http::Request::from_parts(parts, body);
                return Ok(next.run(req).await);
            }
    
            if path == "/api/v1/token" || path == "/api/v1/health" {
                let req = axum::http::Request::from_parts(parts, body);
                return Ok(next.run(req).await);
            }
    
            let body_bytes = match to_bytes(body, MAX_PAYLOAD_SIZE).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    let error = ApiValidationError::InvalidFormat(
                        format!("Failed to read request body: {}", e)
                    );
                    return Err(error.to_response());
                }
            };
    
            if body_bytes.len() > MAX_PAYLOAD_SIZE {
                let error = ApiValidationError::PayloadTooLarge(
                    format!("Request size {} exceeds maximum allowed size {}", body_bytes.len(), MAX_PAYLOAD_SIZE)
                );
                return Err(error.to_response());
            }
    
            let body_str = match std::str::from_utf8(&body_bytes) {
                Ok(s) => s,
                Err(_) => {
                    let error = ApiValidationError::InvalidFormat("Request body is not valid UTF-8".to_string());
                    return Err(error.to_response());
                }
            };
    
            let sanitized_result = sanitize_json_input(body_str);
            if let Err(err) = sanitized_result {
                return Err(err.to_response());
            }
    
            let mut json_value = sanitized_result.unwrap();
    
            if let Err(err) = validate_request(&path, &json_value) {
                return Err(err.to_response());
            }
    
            sanitize_request(&path, &mut json_value);
    
            let sanitized_body = match serde_json::to_string(&json_value) {
                Ok(body) => body,
                Err(err) => {
                    let error = ApiValidationError::InvalidFormat(
                        format!("Failed to serialize sanitized request: {}", err)
                    );
                    return Err(error.to_response());
                }
            };
    
            let req_builder = axum::http::Request::builder()
                .uri(uri)
                .method(method);
    
            let req_builder = headers.iter().fold(req_builder, |builder, (name, value)| {
                builder.header(name, value)
            });
    
            let request = req_builder
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(sanitized_body))
                .unwrap();
    
            Ok(next.run(request).await)
        }
}

// Re-export proto types
pub mod agi_core {
    tonic::include_proto!("agi_core");
}