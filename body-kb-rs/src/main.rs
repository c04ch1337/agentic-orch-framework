// body-kb-rs/src/main.rs
// Main Entry Point for body-kb-rs
// Implements the BodyKBService gRPC server

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tonic::{Request, Response, Status, transport::Server};
use tonic_health::server::{HealthReporter, HealthServer};

// Import validation module
mod validation;
use validation::{validate_query, validate_retrieve_request, validate_store_request};

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    HealthRequest, HealthResponse, QueryRequest, QueryResponse, RetrieveRequest, RetrieveResponse,
    StoreRequest, StoreResponse,
    body_kb_service_server::{BodyKbService, BodyKbServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
};

// Define the Body KB Server Structure
#[derive(Debug, Default)]
pub struct BodyKBServer;

// Implement the BodyKbService Trait
// This KB handles physical/digital embodiment state (sensors/actuators)
#[tonic::async_trait]
impl BodyKbService for BodyKBServer {
    async fn query_kb(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        let req_data = request.into_inner();

        log::info!(
            "Body-KB received QueryKB request: query={}, limit={}",
            req_data.query,
            req_data.limit
        );

        // Validate query and limit using our validation library
        if let Err(err) = validate_query(&req_data.query, req_data.limit) {
            log::warn!("Query validation failed: {}", err);
            return Err(Status::invalid_argument(format!(
                "Invalid query parameters: {}",
                err
            )));
        }

        // --- QUERY STUB (Physical State) ---
        // In a real scenario, this would involve:
        // 1. Querying current sensor readings (temperature, position, velocity, etc.)
        // 2. Retrieving actuator states and capabilities
        // 3. Getting environmental context (location, orientation, etc.)
        // 4. Returning current embodiment state
        // This KB retrieves current sensor data or digital environment state

        // Stub: return mock state data
        let results = vec![
            format!(
                "Body-KB stub state data for query: '{}' - sensor_status=OK",
                req_data.query
            )
            .into_bytes(),
            format!("Body-KB stub actuator state: position=normal, health=100%",).into_bytes(),
        ];

        let reply = QueryResponse {
            results: results.clone(),
            count: results.len() as i32,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("kb_type".to_string(), "body".to_string());
                meta.insert("state_type".to_string(), "embodiment".to_string());
                meta.insert("query_type".to_string(), "sensor_data".to_string());
                meta
            },
        };

        log::info!("Body-KB query returned {} state result(s)", results.len());

        Ok(Response::new(reply))
    }

    async fn store_fact(
        &self,
        request: Request<StoreRequest>,
    ) -> Result<Response<StoreResponse>, Status> {
        let req_data = request.into_inner();

        log::info!(
            "Body-KB received StoreFact request: key={}, value_size={} bytes",
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

        // --- STORE STUB (State Update) ---
        // In a real scenario, this would involve:
        // 1. Validating state data structure
        // 2. Updating sensor readings or actuator states
        // 3. Storing in time-series database for state history
        // 4. Triggering state change notifications if needed
        // State updates (e.g., actuator commands, new sensor readings) are persisted here

        // Generate a stored ID
        let stored_id = format!("body-{}", req_data.key);

        let reply = StoreResponse {
            success: true,
            stored_id: stored_id.clone(),
        };

        log::info!("Body-KB stored state with ID: {}", stored_id);

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
            "Body-KB received Retrieve request: key={}, filters={:?}",
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
        // 1. Looking up state data by key
        // 2. Applying filters if provided
        // 3. Retrieving current sensor/actuator state
        // 4. Returning the stored state value
        // For now, we return a stub response

        // Stub: return mock state data
        let value = format!(
            "Body-KB retrieved state for key: '{}' - status=operational",
            req_data.key
        )
        .into_bytes();

        let reply = RetrieveResponse {
            value: value.clone(),
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("kb_type".to_string(), "body".to_string());
                meta.insert(
                    "retrieved_at".to_string(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        .to_string(),
                );
                meta.insert("state_type".to_string(), "embodiment".to_string());
                meta
            },
            found: true,
        };

        log::info!("Body-KB retrieved state for key: {}", req_data.key);

        Ok(Response::new(reply))
    }

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        // Alias for QueryKB - delegate to the same implementation
        self.query_kb(request).await
    }
}

// Main function to start the gRPC server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Read address from environment variable or use the default port 50058
    let addr_str = env::var("BODY_KB_ADDR").unwrap_or_else(|_| "0.0.0.0:50058".to_string());

    // Parse the address, handling both "0.0.0.0:50058" and "http://127.0.0.1:50058" formats
    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str
            .strip_prefix("http://")
            .unwrap_or(&addr_str)
            .parse()?
    } else {
        addr_str.parse()?
    };

    let body_kb_server = BodyKBServer::default();

    log::info!("Body-KB Service starting on {}", addr);
    println!("Body-KB Service listening on {}", addr);

    let _ = *START_TIME;
    let body_kb_server = Arc::new(body_kb_server);
    let kb_for_health = body_kb_server.clone();

    // Create a health reporter for the standard gRPC health checking protocol
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();

    // Register the service with the health reporter
    health_reporter
        .set_service_status("BODY_KB_SERVICE", tonic_health::ServingStatus::NotServing)
        .await;

    // Set status to serving only after successful initialization
    health_reporter
        .set_service_status("BODY_KB_SERVICE", tonic_health::ServingStatus::Serving)
        .await;
    log::info!("Body-KB Service health status set to SERVING");

    Server::builder()
        .add_service(BodyKbServiceServer::from_arc(body_kb_server))
        .add_service(HealthServiceServer::from_arc(kb_for_health))
        .add_service(health_service) // Add the standard gRPC health service
        .serve(addr)
        .await?;

    Ok(())
}

#[tonic::async_trait]
impl HealthService for BodyKBServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let mut dependencies = HashMap::new();
        dependencies.insert("kb_storage".to_string(), "ACTIVE".to_string());
        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "body-kb-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}
