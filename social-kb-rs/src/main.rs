// social-kb-rs/src/main.rs
// Main Entry Point for social-kb-rs
// Implements the SocialKBService gRPC server

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tonic::{Request, Response, Status, transport::Server};

// Import validation module
mod validation;
use validation::{
    validate_query, validate_register_user_request, validate_retrieve_request,
    validate_role_filter, validate_store_request, validate_user_id, validate_user_identity,
};

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    GetUserRequest,
    GetUserResponse,
    HealthRequest,
    HealthResponse,
    ListUsersRequest,
    ListUsersResponse,
    QueryRequest,
    QueryResponse,
    // User identity types
    RegisterUserRequest,
    RegisterUserResponse,
    RetrieveRequest,
    RetrieveResponse,
    StoreRequest,
    StoreResponse,
    UserIdentity,
    health_service_server::{HealthService, HealthServiceServer},
    social_kb_service_server::{SocialKbService, SocialKbServiceServer},
};

// Define the Social KB Server Structure
#[derive(Debug, Default)]
pub struct SocialKBServer;

// Implement the SocialKbService Trait
// This KB handles social dynamics, relationship history, and social protocols
#[tonic::async_trait]
impl SocialKbService for SocialKBServer {
    async fn query_kb(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        let req_data = request.into_inner();

        log::info!(
            "Social-KB received QueryKB request: query={}, limit={}",
            req_data.query,
            req_data.limit
        );

        // Validate query and limit
        if let Err(err) = validate_query(&req_data.query, req_data.limit) {
            log::warn!("Query validation failed: {}", err);
            return Err(Status::invalid_argument(format!(
                "Invalid query parameters: {}",
                err
            )));
        }

        // --- QUERY STUB (Social Context) ---
        // In a real scenario, this would involve:
        // 1. Querying relationship status and trust scores
        // 2. Retrieving social interaction history
        // 3. Getting social protocols and behavioral norms
        // 4. Returning social context for appropriate behavior
        // Retrieves relationship status, trust scores, and social history for a user/agent

        // Stub: return mock social data
        let results = vec![
            format!("Social-KB stub relationship status for query: '{}' - Trust: 0.9, Role: Colleague, Interaction Count: 42", req_data.query).into_bytes(),
            format!("Social-KB stub social protocol: Communication Style: Professional, Preferred Channel: Text",).into_bytes(),
        ];

        let reply = QueryResponse {
            results: results.clone(),
            count: results.len() as i32,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("kb_type".to_string(), "social".to_string());
                meta.insert("state_type".to_string(), "relationship".to_string());
                meta.insert("query_type".to_string(), "social_context".to_string());
                meta
            },
        };

        log::info!(
            "Social-KB query returned {} social result(s)",
            results.len()
        );

        Ok(Response::new(reply))
    }

    async fn store_fact(
        &self,
        request: Request<StoreRequest>,
    ) -> Result<Response<StoreResponse>, Status> {
        let req_data = request.into_inner();

        log::info!(
            "Social-KB received StoreFact request: key={}, value_size={} bytes",
            req_data.key,
            req_data.value.len()
        );

        // Validate store request
        match validate_store_request(&req_data.key, &req_data.value, &req_data.metadata) {
            Ok((sanitized_key, sanitized_value, sanitized_metadata)) => {
                // Use sanitized values in real implementation
                log::info!(
                    "Validated store request: key={}, value_size={} bytes",
                    sanitized_key,
                    sanitized_value.len()
                );

                // For now we just log the sanitized data since this is a stub implementation
            }
            Err(err) => {
                log::warn!("Store request validation failed: {}", err);
                return Err(Status::invalid_argument(format!(
                    "Invalid store request: {}",
                    err
                )));
            }
        }

        // --- STORE STUB (Social Update) ---
        // In a real scenario, this would involve:
        // 1. Validating social interaction data structure
        // 2. Updating relationship scores and trust levels
        // 3. Storing social interaction history
        // 4. Applying social protocol updates
        // Stores new social interaction data, updating relationship scores and history

        // Generate a stored ID
        let stored_id = format!("social-{}", req_data.key);

        let reply = StoreResponse {
            success: true,
            stored_id: stored_id.clone(),
        };

        log::info!("Social-KB stored social fact with ID: {}", stored_id);

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
            "Social-KB received Retrieve request: key={}, filters={:?}",
            req_data.key,
            req_data.filters
        );

        // Validate retrieve request
        match validate_retrieve_request(&req_data.key, &req_data.filters) {
            Ok((sanitized_key, sanitized_filters)) => {
                // Use sanitized values in real implementation
                log::info!(
                    "Validated retrieve request: key={}, filters={:?}",
                    sanitized_key,
                    sanitized_filters
                );

                // For now we just log the sanitized data since this is a stub implementation
            }
            Err(err) => {
                log::warn!("Retrieve request validation failed: {}", err);
                return Err(Status::invalid_argument(format!(
                    "Invalid retrieve request: {}",
                    err
                )));
            }
        }

        // --- RETRIEVE STUB ---
        // In a real scenario, this would involve:
        // 1. Looking up social/relationship data by key
        // 2. Applying filters if provided
        // 3. Retrieving relationship history or social protocols
        // 4. Returning the stored social value
        // For now, we return a stub response

        // Stub: return mock social data
        let value = format!("Social-KB retrieved relationship for key: '{}' - Trust: 0.9, Role: Colleague, Last Interaction: recent", req_data.key).into_bytes();

        let reply = RetrieveResponse {
            value: value.clone(),
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("kb_type".to_string(), "social".to_string());
                meta.insert(
                    "retrieved_at".to_string(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        .to_string(),
                );
                meta.insert("state_type".to_string(), "relationship".to_string());
                meta
            },
            found: true,
        };

        log::info!("Social-KB retrieved relationship for key: {}", req_data.key);

        Ok(Response::new(reply))
    }

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        // Alias for QueryKB - delegate to the same implementation
        self.query_kb(request).await
    }

    // User Identity RPCs

    async fn register_user(
        &self,
        request: Request<RegisterUserRequest>,
    ) -> Result<Response<RegisterUserResponse>, Status> {
        let req = request.into_inner();

        // Validate registration request
        let validated_req = match validate_register_user_request(&req) {
            Ok(validated) => validated,
            Err(err) => {
                log::warn!("User registration validation failed: {}", err);
                return Err(Status::invalid_argument(format!(
                    "Invalid user registration: {}",
                    err
                )));
            }
        };

        let identity = validated_req.identity.unwrap_or_default();
        log::info!(
            "Social-KB RegisterUser: name={}, role={}",
            identity.name,
            identity.role
        );

        Ok(Response::new(RegisterUserResponse {
            success: true,
            user_id: if identity.user_id.is_empty() {
                format!("user-{}", uuid::Uuid::new_v4())
            } else {
                identity.user_id
            },
        }))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let req = request.into_inner();

        // Validate user ID
        let sanitized_user_id = match validate_user_id(&req.user_id) {
            Ok(id) => id,
            Err(err) => {
                log::warn!("User ID validation failed: {}", err);
                return Err(Status::invalid_argument(format!(
                    "Invalid user ID: {}",
                    err
                )));
            }
        };

        log::info!("Social-KB GetUser: user_id={}", sanitized_user_id);

        // Create identity (would be from database in real implementation)
        let user_identity = UserIdentity {
            user_id: sanitized_user_id.clone(),
            name: "Default User".to_string(),
            role: 1, // ROLE_USER
            permissions: vec!["read".to_string(), "write".to_string()],
            created_at: 0,
            last_active: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            attributes: HashMap::new(),
        };

        // Validate the generated identity (defense in depth)
        let validated_identity = match validate_user_identity(&user_identity) {
            Ok(validated) => validated,
            Err(err) => {
                log::warn!("Generated user identity validation failed: {}", err);
                return Err(Status::internal(format!("Error in user identity: {}", err)));
            }
        };

        Ok(Response::new(GetUserResponse {
            found: true,
            identity: Some(validated_identity),
            preferences: None,
        }))
    }

    async fn list_users(
        &self,
        request: Request<ListUsersRequest>,
    ) -> Result<Response<ListUsersResponse>, Status> {
        let req = request.into_inner();

        // Validate role filter
        let validated_role_filter = match validate_role_filter(req.role_filter) {
            Ok(role) => role,
            Err(err) => {
                log::warn!("Role filter validation failed: {}", err);
                return Err(Status::invalid_argument(format!(
                    "Invalid role filter: {}",
                    err
                )));
            }
        };

        log::info!("Social-KB ListUsers: role_filter={}", validated_role_filter);

        Ok(Response::new(ListUsersResponse {
            users: vec![],
            total_count: 0,
        }))
    }
}

// Main function to start the gRPC server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Read address from environment variable or use the default port 50060
    let addr_str = env::var("SOCIAL_KB_ADDR").unwrap_or_else(|_| "0.0.0.0:50060".to_string());

    // Parse the address, handling both "0.0.0.0:50060" and "http://127.0.0.1:50060" formats
    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str
            .strip_prefix("http://")
            .unwrap_or(&addr_str)
            .parse()?
    } else {
        addr_str.parse()?
    };

    let social_kb_server = SocialKBServer::default();

    log::info!("Social-KB Service starting on {}", addr);
    println!("Social-KB Service listening on {}", addr);

    let _ = *START_TIME;
    let social_kb_server = Arc::new(social_kb_server);
    let kb_for_health = social_kb_server.clone();

    Server::builder()
        .add_service(SocialKbServiceServer::from_arc(social_kb_server))
        .add_service(HealthServiceServer::from_arc(kb_for_health))
        .serve(addr)
        .await?;

    Ok(())
}

#[tonic::async_trait]
impl HealthService for SocialKBServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let mut dependencies = HashMap::new();
        dependencies.insert("kb_storage".to_string(), "ACTIVE".to_string());
        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "social-kb-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}
