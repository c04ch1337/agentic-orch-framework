// auth-service-rs/src/main.rs
//
// Authentication and Authorization Service for Phoenix ORCH AGI
// Provides centralized auth services for all system components
//
// Primary features:
// - JWT token generation and validation
// - RBAC (Role-based access control)
// - Fine-grained permissions
// - Service-to-service authentication
// - mTLS support
// - Audit logging

mod proto {
    pub mod auth_service {
        include!(concat!(env!("OUT_DIR"), "/auth_service.rs"));
    }
    
    pub mod agi_core {
        include!(concat!(env!("OUT_DIR"), "/agi_core.rs"));
    }
}

mod jwt;
mod rbac;
mod audit;
mod auth_service;
mod certificates;
mod storage;
mod secrets_client;
mod utils;

use tonic::transport::Server;
use std::env;
use log::{info, warn, error};
use auth_service::AuthServiceImpl;

use proto::auth_service::auth_service_server::AuthServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("Starting Auth Service...");
    
    // Get service configuration from environment
    let port = env::var("AUTH_SERVICE_PORT").unwrap_or_else(|_| "50090".to_string());
    let addr = format!("0.0.0.0:{}", port).parse()?;
    
    // Initialize Redis connection for token caching and revocation
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    info!("Connecting to Redis at {}", redis_url);
    
    // Initialize Secrets Service client
    let secrets_addr = env::var("SECRETS_SERVICE_ADDR")
        .unwrap_or_else(|_| "http://localhost:50080".to_string());
    info!("Connecting to Secrets Service at {}", secrets_addr);

    // Initialize JWT signing keys
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
        warn!("JWT_SECRET not set, using random value (insecure for production)");
        uuid::Uuid::new_v4().to_string()
    });
    
    // Initialize storage backend
    info!("Initializing auth storage");
    let storage = storage::init_storage().await?;
    
    // Create the AuthService implementation
    let auth_service = AuthServiceImpl::new(
        jwt_secret,
        redis_url.clone(),
        secrets_addr,
        storage
    ).await?;
    
    // Create and start the gRPC server
    info!("Starting Auth Service gRPC server on {}", addr);
    let server_future = Server::builder()
        .add_service(AuthServiceServer::new(auth_service))
        .serve(addr);
    
    println!("Auth Service running on port {}", port);
    
    // Wait for the server to finish
    server_future.await?;
    
    Ok(())
}