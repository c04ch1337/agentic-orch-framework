// sensor-rs/src/main.rs
// Main Entry Point for sensor-rs
// Monitors host system state and streams data to Body-KB

use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::time;

mod system_monitor;

use system_monitor::{SystemMonitor, SystemState};

// Import Generated gRPC Code
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::body_kb_service_client::BodyKbServiceClient;
use agi_core::StoreRequest;

/// Default Body-KB service address
const DEFAULT_BODY_KB_ADDR: &str = "http://127.0.0.1:50058";

/// Polling interval in seconds
const POLL_INTERVAL_SECS: u64 = 5;

/// Convert SystemState to a StoreRequest for Body-KB
fn state_to_store_request(state: &SystemState) -> StoreRequest {
    // Serialize state to JSON bytes
    let value = serde_json::to_vec(state).unwrap_or_default();

    // Create metadata for the fact
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "sensor-rs".to_string());
    metadata.insert("fact_type".to_string(), "system_state".to_string());
    metadata.insert("timestamp".to_string(), state.timestamp.to_string());

    if let Some(ref title) = state.active_window_title {
        metadata.insert("active_window".to_string(), title.clone());
    }

    StoreRequest {
        key: format!("system_state_{}", state.timestamp),
        value,
        metadata,
    }
}

/// Main sensor loop
async fn run_sensor_loop(body_kb_addr: String) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Connecting to Body-KB at {}", body_kb_addr);

    // Create gRPC client connection
    let mut client = BodyKbServiceClient::connect(body_kb_addr.clone()).await?;
    log::info!("Successfully connected to Body-KB");

    // Initialize system monitor
    let mut monitor = SystemMonitor::new();

    // Create interval timer
    let mut interval = time::interval(Duration::from_secs(POLL_INTERVAL_SECS));

    log::info!(
        "Starting sensor loop - polling every {} seconds",
        POLL_INTERVAL_SECS
    );

    loop {
        // Wait for next tick
        interval.tick().await;

        // Poll system state
        let state = monitor.poll_state();

        log::info!(
            "Polled state: CPU={:.1}%, RAM={:.1}%, Window={:?}",
            state.cpu_usage_percent,
            state.memory_usage_percent,
            state.active_window_title.as_deref().unwrap_or("N/A")
        );

        // Convert to store request
        let request = state_to_store_request(&state);

        // Send to Body-KB
        match client.store_fact(tonic::Request::new(request)).await {
            Ok(response) => {
                let resp = response.into_inner();
                if resp.success {
                    log::debug!("StoreFact succeeded: stored_id={}", resp.stored_id);
                } else {
                    log::warn!("StoreFact returned success=false");
                }
            }
            Err(e) => {
                log::error!("Failed to store fact in Body-KB: {}", e);
                // Attempt to reconnect on next iteration
                if let Ok(new_client) = BodyKbServiceClient::connect(body_kb_addr.clone()).await {
                    client = new_client;
                    log::info!("Reconnected to Body-KB");
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("=== sensor-rs: Digital Twin Perception Module ===");

    // Read Body-KB address from environment or use default
    let body_kb_addr =
        env::var("BODY_KB_ADDR").unwrap_or_else(|_| DEFAULT_BODY_KB_ADDR.to_string());

    // Ensure address has http:// prefix for tonic
    let body_kb_addr =
        if body_kb_addr.starts_with("http://") || body_kb_addr.starts_with("https://") {
            body_kb_addr
        } else {
            format!("http://{}", body_kb_addr)
        };

    println!(
        "sensor-rs starting - streaming to Body-KB at {}",
        body_kb_addr
    );

    // Run the sensor loop
    run_sensor_loop(body_kb_addr).await?;

    Ok(())
}
