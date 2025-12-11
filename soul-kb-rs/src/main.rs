// soul-kb-rs/src/main.rs
// Main Entry Point for soul-kb-rs
// Implements the SoulKBService gRPC server

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
    validate_core_value, validate_ethics_check_request, validate_min_priority, validate_query,
    validate_retrieve_request, validate_store_request, validate_store_value_request,
    validate_value_id, validate_value_name,
};

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    CoreValue,
    EthicsCheckRequest,
    EthicsCheckResponse,
    GetValueRequest,
    GetValueResponse,
    HealthRequest,
    HealthResponse,
    ListValuesRequest,
    ListValuesResponse,
    QueryRequest,
    QueryResponse,
    RetrieveRequest,
    RetrieveResponse,
    StoreRequest,
    StoreResponse,
    // Ethics and values types
    StoreValueRequest,
    StoreValueResponse,
    health_service_server::{HealthService, HealthServiceServer},
    soul_kb_service_server::{SoulKbService, SoulKbServiceServer},
};

// Define the Soul KB Server Structure
#[derive(Debug, Default)]
pub struct SoulKBServer;

// Implement the SoulKbService Trait
// This KB handles core values, identity, and long-term aspirational goals
#[tonic::async_trait]
impl SoulKbService for SoulKBServer {
    async fn query_kb(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        let req_data = request.into_inner();

        log::info!(
            "Soul-KB received QueryKB request: query={}, limit={}",
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

        // --- QUERY STUB (Core Values/Identity) ---
        // In a real scenario, this would involve:
        // 1. Querying core values and ethical principles
        // 2. Retrieving long-term aspirational goals
        // 3. Getting mission statement and fundamental identity
        // 4. Returning moral compass and purpose for decision-making
        // Retrieves the agent's core values, mission statement, or long-term goals

        // Stub: return mock identity/values data
        let results = vec![
            format!("Soul-KB stub core identity for query: '{}' - Value: Integrity, Goal: Maximum Utility, Mission: Serve Humanity", req_data.query).into_bytes(),
            format!("Soul-KB stub long-term goals: Aspiration: AGI Alignment, Principle: Beneficence, Constraint: Non-maleficence",).into_bytes(),
        ];

        let reply = QueryResponse {
            results: results.clone(),
            count: results.len() as i32,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("kb_type".to_string(), "soul".to_string());
                meta.insert("state_type".to_string(), "identity".to_string());
                meta.insert("query_type".to_string(), "core_values".to_string());
                meta
            },
        };

        log::info!(
            "Soul-KB query returned {} identity/values result(s)",
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
            "Soul-KB received StoreFact request: key={}, value_size={} bytes",
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

        // --- STORE STUB (Identity Update) ---
        // In a real scenario, this would involve:
        // 1. Validating core value/identity data structure
        // 2. Updating foundational values or mission statement
        // 3. Storing long-term aspirational goals
        // 4. Applying high-level ethical constraints
        // Stores/updates foundational values, mission updates, or high-level ethical constraints

        // Generate a stored ID
        let stored_id = format!("soul-{}", req_data.key);

        let reply = StoreResponse {
            success: true,
            stored_id: stored_id.clone(),
        };

        log::info!("Soul-KB stored core value with ID: {}", stored_id);

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
            "Soul-KB received Retrieve request: key={}, filters={:?}",
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
        // 1. Looking up core value/identity data by key
        // 2. Applying filters if provided
        // 3. Retrieving foundational values or long-term goals
        // 4. Returning the stored identity value
        // For now, we return a stub response

        // Stub: return mock identity data
        let value = format!("Soul-KB retrieved core value for key: '{}' - Value: Integrity, Goal: Maximum Utility, Mission: Serve Humanity", req_data.key).into_bytes();

        let reply = RetrieveResponse {
            value: value.clone(),
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("kb_type".to_string(), "soul".to_string());
                meta.insert(
                    "retrieved_at".to_string(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        .to_string(),
                );
                meta.insert("state_type".to_string(), "identity".to_string());
                meta
            },
            found: true,
        };

        log::info!("Soul-KB retrieved core value for key: {}", req_data.key);

        Ok(Response::new(reply))
    }

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        // Alias for QueryKB - delegate to the same implementation
        self.query_kb(request).await
    }

    // Ethics and Values RPCs

    async fn store_value(
        &self,
        request: Request<StoreValueRequest>,
    ) -> Result<Response<StoreValueResponse>, Status> {
        let req = request.into_inner();

        // Validate store value request
        let validated_req = match validate_store_value_request(&req) {
            Ok(validated) => validated,
            Err(err) => {
                log::warn!("Core value validation failed: {}", err);
                return Err(Status::invalid_argument(format!(
                    "Invalid core value: {}",
                    err
                )));
            }
        };

        let value = validated_req.value.unwrap_or_default();
        log::info!("Soul-KB StoreValue: name={}", value.name);

        Ok(Response::new(StoreValueResponse {
            success: true,
            value_id: format!("soul-value-{}", uuid::Uuid::new_v4()),
        }))
    }

    async fn get_value(
        &self,
        request: Request<GetValueRequest>,
    ) -> Result<Response<GetValueResponse>, Status> {
        let req = request.into_inner();

        // Validate value ID and name
        let sanitized_id = match validate_value_id(&req.value_id) {
            Ok(id) => id,
            Err(err) => {
                log::warn!("Value ID validation failed: {}", err);
                return Err(Status::invalid_argument(format!(
                    "Invalid value ID: {}",
                    err
                )));
            }
        };

        let sanitized_name = match validate_value_name(&req.name) {
            Ok(name) => name,
            Err(err) => {
                log::warn!("Value name validation failed: {}", err);
                return Err(Status::invalid_argument(format!(
                    "Invalid value name: {}",
                    err
                )));
            }
        };

        log::info!(
            "Soul-KB GetValue: id={}, name={}",
            sanitized_id,
            sanitized_name
        );

        // Create value (would be from database in real implementation)
        let core_value = CoreValue {
            value_id: sanitized_id,
            name: if req.name.is_empty() {
                "user_safety".to_string()
            } else {
                sanitized_name
            },
            description: "Core ethical value".to_string(),
            priority: 3, // PRIORITY_CRITICAL
            constraint: "Must not harm users".to_string(),
            is_active: true,
            metadata: HashMap::new(),
        };

        // Validate the generated value (defense in depth)
        let validated_value = match validate_core_value(&core_value) {
            Ok(validated) => validated,
            Err(err) => {
                log::warn!("Generated core value validation failed: {}", err);
                return Err(Status::internal(format!("Error in core value: {}", err)));
            }
        };

        Ok(Response::new(GetValueResponse {
            found: true,
            value: Some(validated_value),
        }))
    }

    async fn list_values(
        &self,
        request: Request<ListValuesRequest>,
    ) -> Result<Response<ListValuesResponse>, Status> {
        let req = request.into_inner();

        // Validate min priority
        let validated_min_priority = match validate_min_priority(req.min_priority) {
            Ok(priority) => priority,
            Err(err) => {
                log::warn!("Min priority validation failed: {}", err);
                return Err(Status::invalid_argument(format!(
                    "Invalid min priority: {}",
                    err
                )));
            }
        };

        log::info!(
            "Soul-KB ListValues: min_priority={}",
            validated_min_priority
        );

        Ok(Response::new(ListValuesResponse {
            values: vec![
                CoreValue {
                    value_id: "safety-001".to_string(),
                    name: "user_safety".to_string(),
                    description: "Prioritize user safety".to_string(),
                    priority: 4, // PRIORITY_IMMUTABLE
                    constraint: "Never harm users".to_string(),
                    is_active: true,
                    metadata: HashMap::new(),
                },
                CoreValue {
                    value_id: "privacy-001".to_string(),
                    name: "data_privacy".to_string(),
                    description: "Protect user data".to_string(),
                    priority: 3, // PRIORITY_CRITICAL
                    constraint: "Never expose user data".to_string(),
                    is_active: true,
                    metadata: HashMap::new(),
                },
            ],
            total_count: 2,
        }))
    }

    async fn check_ethics(
        &self,
        request: Request<EthicsCheckRequest>,
    ) -> Result<Response<EthicsCheckResponse>, Status> {
        let req = request.into_inner();

        // Validate ethics check request
        let validated_req = match validate_ethics_check_request(&req) {
            Ok(validated) => validated,
            Err(err) => {
                log::warn!("Ethics check validation failed: {}", err);
                return Err(Status::invalid_argument(format!(
                    "Invalid ethics check request: {}",
                    err
                )));
            }
        };

        log::info!("Soul-KB CheckEthics: action={}", validated_req.action);

        // Simple ethics check - block destructive actions
        let blocked_actions = [
            "delete_user",
            "expose_pii",
            "bypass_auth",
            "disable_logging",
        ];
        let action_lower = validated_req.action.to_lowercase();

        let allowed = !blocked_actions.iter().any(|b| action_lower.contains(b));
        let violated = if !allowed {
            vec!["user_safety".to_string(), "data_privacy".to_string()]
        } else {
            vec![]
        };

        Ok(Response::new(EthicsCheckResponse {
            allowed,
            violated_values: violated,
            recommendation: if allowed {
                "Action permitted".to_string()
            } else {
                "Action blocked by ethical constraints".to_string()
            },
        }))
    }
}

// Main function to start the gRPC server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Read address from environment variable or use the default port 50061
    let addr_str = env::var("SOUL_KB_ADDR").unwrap_or_else(|_| "0.0.0.0:50061".to_string());

    // Parse the address, handling both "0.0.0.0:50061" and "http://127.0.0.1:50061" formats
    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str
            .strip_prefix("http://")
            .unwrap_or(&addr_str)
            .parse()?
    } else {
        addr_str.parse()?
    };

    let soul_kb_server = SoulKBServer::default();

    log::info!("Soul-KB Service starting on {}", addr);
    println!("Soul-KB Service listening on {}", addr);

    let _ = *START_TIME;
    let soul_kb_server = Arc::new(soul_kb_server);
    let kb_for_health = soul_kb_server.clone();

    Server::builder()
        .add_service(SoulKbServiceServer::from_arc(soul_kb_server))
        .add_service(HealthServiceServer::from_arc(kb_for_health))
        .serve(addr)
        .await?;

    Ok(())
}

#[tonic::async_trait]
impl HealthService for SoulKBServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let mut dependencies = HashMap::new();
        dependencies.insert("kb_storage".to_string(), "ACTIVE".to_string());
        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "soul-kb-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}
