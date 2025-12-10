use std::ffi::OsString;
use std::sync::mpsc;
use std::time::Duration;
use tokio::runtime::Runtime;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, 
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
};

use crate::{PhoenixState, Result};

const SERVICE_NAME: &str = "PhoenixOrchMonolith";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

pub fn service_main(arguments: Vec<OsString>) {
    if let Err(e) = run_service(arguments) {
        tracing::error!("Service failed: {}", e);
    }
}

define_windows_service!(ffi_service_main, service_main);

fn run_service(arguments: Vec<OsString>) -> Result<()> {
    // Set up logging to Windows Event Log
    let event_logger = windows_eventlog::SimpleLogger::new(SERVICE_NAME)?;
    tracing::subscriber::set_global_default(event_logger)?;

    // Create channel for service control events
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    // Initialize service control handler
    let status_handle = service_control_handler::register(SERVICE_NAME, move |control_event| {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                tracing::info!("Service shutdown requested");
                shutdown_tx.send(()).unwrap_or_default();
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    })?;

    // Update service status to running
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    // Create tokio runtime
    let runtime = Runtime::new()?;

    // Initialize PhoenixState
    let config = runtime.block_on(async {
        config_rs::Config::load().await.map_err(|e| {
            tracing::error!("Failed to load config: {}", e);
            e
        })?
    })?;

    let state = runtime.block_on(async {
        PhoenixState::new(config).await.map_err(|e| {
            tracing::error!("Failed to initialize state: {}", e);
            e
        })?
    })?;

    // Initialize emergency resilience
    runtime.block_on(async {
        state.emergency.initialize().await.map_err(|e| {
            tracing::error!("Failed to initialize emergency resilience: {}", e);
            e
        })?
    })?;

    tracing::info!("Service started successfully");

    // Wait for shutdown signal
    shutdown_rx.recv()?;

    // Update service status to stopping
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::StopPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(30),
        process_id: None,
    })?;

    // Perform cleanup
    runtime.block_on(async {
        if let Err(e) = cleanup(&state).await {
            tracing::error!("Cleanup error: {}", e);
        }
    });

    // Update service status to stopped
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}

async fn cleanup(state: &PhoenixState) -> Result<()> {
    // Create final snapshot before shutdown
    state.emergency.create_snapshot().await?;
    
    // Rotate any secrets if needed
    state.emergency.rotate_secrets().await?;
    
    // Additional cleanup as needed
    Ok(())
}