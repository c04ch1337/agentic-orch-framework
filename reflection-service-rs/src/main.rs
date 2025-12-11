// reflection-service-rs/src/main.rs
// Reflection Service - Self-reflection and action evaluation

use std::env;
use std::net::SocketAddr;
use tonic::transport::Server;

// Import modules - will be implemented next
mod logging_client;
mod reflection_logic;
mod service;
mod soul_kb_client;
mod tests;

// Import generated protobuf code
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::reflection_service_server::ReflectionServiceServer;

// Import our service implementation
use service::ReflectionServiceImpl;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Get service port from env or use default
    let port = env::var("REFLECTION_SERVICE_PORT").unwrap_or_else(|_| "50065".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    // Log service startup
    log::info!("Reflection Service starting on {}", addr);

    // Initialize the reflection service
    let service = ReflectionServiceImpl::new().await;
    let service = ReflectionServiceServer::new(service);

    log::info!("Starting gRPC server...");

    // Start gRPC server
    Server::builder().add_service(service).serve(addr).await?;

    Ok(())
}
