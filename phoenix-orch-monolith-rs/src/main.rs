//! Phoenix ORCH Windows Service
//! Implements Windows Service integration with emergency resilience

use std::ffi::OsString;
use std::sync::mpsc;
use std::time::Duration;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandler},
    service_dispatcher,
};
use windows_eventlog::{EventLog, EventLogRecord};
use anyhow::Result;

const SERVICE_NAME: &str = "PhoenixOrchService";
const SERVICE_DISPLAY_NAME: &str = "Phoenix ORCH Service";
const SERVICE_DESCRIPTION: &str = "Phoenix ORCH Monolithic Service with Emergency Resilience";

define_windows_service!(ffi_service_main, service_main);

fn main() -> Result<()> {
    // Initialize logging to Windows Event Log
    let event_log = EventLog::new("Application")?;
    event_log.report_information(
        "Phoenix ORCH Service",
        &format!("Starting {} service", SERVICE_NAME)
    )?;

    // Dispatch service main
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;

    Ok(())
}

fn service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service() {
        // Log error to Windows Event Log
        if let Ok(event_log) = EventLog::new("Application") {
            let _ = event_log.report_error(
                "Phoenix ORCH Service",
                &format!("Service error: {}", e)
            );
        }
    }
}

fn run_service() -> Result<()> {
    // Create service event channel
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    // Initialize service status handler
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                shutdown_tx.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register service control handler
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    // Initialize service status
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    // Update service status
    status_handle.set_service_status(next_status)?;

    // Initialize Phoenix ORCH
    let runtime = tokio::runtime::Runtime::new()?;
    let orch = runtime.block_on(phoenix_orch::PhoenixOrch::new())?;

    // Log successful initialization
    let event_log = EventLog::new("Application")?;
    event_log.report_information(
        "Phoenix ORCH Service",
        "Service initialized successfully"
    )?;

    // Start Phoenix ORCH
    runtime.block_on(orch.start())?;

    // Wait for shutdown signal
    shutdown_rx.recv()?;

    // Update service status to stopping
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StopPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(10),
        process_id: None,
    };
    status_handle.set_service_status(next_status)?;

    // Stop Phoenix ORCH
    runtime.block_on(orch.stop())?;

    // Update service status to stopped
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };
    status_handle.set_service_status(next_status)?;

    // Log successful shutdown
    event_log.report_information(
        "Phoenix ORCH Service",
        "Service stopped successfully"
    )?;

    Ok(())
}

type ServiceControlHandlerResult = windows_service::service_control_handler::ServiceControlHandlerResult;