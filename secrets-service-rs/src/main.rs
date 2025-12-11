// secrets-service-rs/src/main.rs
// Secrets Management Service - Centralized secure secrets provider

use config_rs;
use std::env;
use tonic::transport::Server;

// Import modules
mod auth;
mod service;
mod vault_client;

// Import generated protobuf code
pub mod secrets_service {
    tonic::include_proto!("secrets_service");
}

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use secrets_service::secrets_service_server::SecretsServiceServer;

// Import our service implementation
use service::SecretsServiceImpl;
use vault_client::VaultClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Get bind address using standardized config pattern
    let addr = config_rs::get_bind_address("SECRETS", 50080);

    // Log service startup
    log::info!("Secrets Service starting on {}", addr);

    // Initialize Vault client
    let vault_client = match VaultClient::new().await {
        Ok(client) => {
            log::info!("Successfully connected to HashiCorp Vault");
            client
        }
        Err(e) => {
            log::error!("Failed to connect to HashiCorp Vault: {}", e);
            log::warn!("Starting in degraded mode - secrets operations will fail");
            VaultClient::new_mock()
        }
    };

    // Create service implementation
    let service = SecretsServiceImpl::new(vault_client).await;
    let service = SecretsServiceServer::new(service);

    log::info!("Starting gRPC server...");

    // Start gRPC server
    Server::builder().add_service(service).serve(addr).await?;

    Ok(())
}
