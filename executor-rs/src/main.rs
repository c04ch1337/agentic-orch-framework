// executor-rs/src/main.rs
// Main Entry Point for executor-rs
// Implements the ExecutorService gRPC server
// PHOENIX ORCH: The Ashen Guard Edition AGI - Windows Native Execution

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tonic::{transport::Server, Request, Response, Status};
use windows_service::{
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandler},
    service_dispatcher,
};

// Import configuration module
mod config;
use config::{init_config_manager, start_config_monitoring};

const SERVICE_NAME: &str = "PhoenixExecutorService";
const SERVICE_DISPLAY_NAME: &str = "Phoenix Executor Service";
const SERVICE_DESCRIPTION: &str = "Phoenix ORCH Native Execution Service";

// Service recovery settings
const RESTART_DELAY_MS: u32 = 30000; // 30 seconds
const MAX_RESTART_ATTEMPTS: u32 = 3;

mod execution_logic;
use execution_logic::{
    execute_python_sandboxed, execute_shell_command, get_execution_stats, simulate_input,
};

// Windows executor module for native control
#[cfg(target_os = "windows")]
mod windows_executor;

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

// Import monitoring, performance, and security modules
mod monitoring;
mod performance;
mod security;
use monitoring::{init_monitoring, get_monitoring_stats};
use performance::{init_performance_optimizer, get_performance_stats};
use security::{init_security_manager, get_security_stats};

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    executor_service_server::{ExecutorService, ExecutorServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    CommandRequest, CommandResponse, HealthRequest, HealthResponse, InputRequest, InputResponse,
};

// Define the Executor Server Structure
#[derive(Debug, Default)]
pub struct ExecutorServer;

#[tonic::async_trait]
impl ExecutorService for ExecutorServer {
    async fn execute_command(
        &self,
        request: Request<CommandRequest>,
    ) -> Result<Response<CommandResponse>, Status> {
        let req = request.into_inner();

        log::info!("Received ExecuteCommand request: {}", req.command);

        match execute_shell_command(&req.command, &req.args, &req.env).await {
            Ok((stdout, stderr, exit_code)) => Ok(Response::new(CommandResponse {
                stdout,
                stderr,
                exit_code,
            })),
            Err(e) => {
                log::error!("Command execution failed: {}", e);
                Ok(Response::new(CommandResponse {
                    stdout: "".to_string(),
                    stderr: e,
                    exit_code: -1,
                }))
            }
        }
    }

    async fn simulate_input(
        &self,
        request: Request<InputRequest>,
    ) -> Result<Response<InputResponse>, Status> {
        let req = request.into_inner();

        log::info!("Received SimulateInput request: {}", req.input_type);

        match simulate_input(&req.input_type, &req.parameters) {
            Ok(_) => Ok(Response::new(InputResponse {
                success: true,
                error: "".to_string(),
            })),
            Err(e) => {
                log::error!("Input simulation failed: {}", e);
                Ok(Response::new(InputResponse {
                    success: false,
                    error: e,
                }))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = run_service() {
        log::error!("Service error: {}", e);
        return Err(e.into());
    }
    Ok(())
}

fn run_service() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Windows service dispatcher
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}

// Windows service entry point
extern "system" fn ffi_service_main(_: u32, _: *mut *mut u16) {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting PHOENIX ORCH Executor Service - Windows Native Edition");

    // Initialize monitoring system
    init_monitoring();
    log::info!("Enhanced monitoring system initialized");

    // Initialize performance optimizer
    init_performance_optimizer(performance::PerformanceConfig::default());
    log::info!("Performance optimization system initialized");

    // Initialize security manager
    init_security_manager(security::SecurityConfig::default());
    log::info!("Comprehensive security system initialized");

    // Check if running on Windows and initialize
    #[cfg(target_os = "windows")]
    {
        log::info!("Windows platform detected - Using native Job Object control");

        // Initialize configuration system
        if let Err(e) = init_config_manager().await {
            log::error!("Failed to initialize configuration manager: {}", e);
            // Fallback to default sandbox directory
            let sandbox_path = std::path::Path::new(r"C:\phoenix_sandbox");
            if !sandbox_path.exists() {
                std::fs::create_dir_all(sandbox_path).expect("Failed to create sandbox directory");
                log::info!("Created fallback sandbox directory at: {}", sandbox_path.display());
            }
        } else {
            // Start configuration monitoring
            start_config_monitoring().await;

            // Get sandbox directory from configuration
            let config = config::get_config().await;
            let sandbox_path = std::path::Path::new(&config.sandbox_dir);

            // Create sandbox directory if it doesn't exist
            if !sandbox_path.exists() {
                std::fs::create_dir_all(sandbox_path).expect("Failed to create sandbox directory");
                log::info!("Created sandbox directory at: {}", sandbox_path.display());
            }

            // Perform sandbox integrity check
            match check_sandbox_integrity() {
                Ok(_) => log::info!("Sandbox integrity check passed"),
                Err(e) => {
                    log::error!("Sandbox integrity check failed: {}", e);
                    // In production, you might want to fail startup here
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        log::warn!("Non-Windows platform detected - Limited execution capabilities");
    }

    // Read address from environment variable or use the default port 50055 (as per requirements)
    let addr_str = env::var("EXECUTOR_ADDR").unwrap_or_else(|_| "0.0.0.0:50055".to_string());

    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str
            .strip_prefix("http://")
            .unwrap_or(&addr_str)
            .parse()?
    } else {
        addr_str.parse()?
    };

    let executor_server = ExecutorServer::default();

    log::info!("PHOENIX ORCH Executor Service starting on {}", addr);
    println!(
        "PHOENIX ORCH Executor Service (Windows Native) listening on {}",
        addr
    );

    // Log execution configuration
    let exec_stats = execution_logic::get_execution_stats();
    log::info!("Execution configuration:");
    for (key, value) in exec_stats.iter() {
        log::info!("  {}: {}", key, value);
    }

    // Initialize service status handle
    let status_handle = match service_control_handler::register(SERVICE_NAME, service_handler) {
        Ok(handle) => handle,
        Err(e) => {
            log::error!("Failed to register service control handler: {}", e);
            return;
        }
    };

    // Configure service recovery
    let recovery_config = windows_service::service::ServiceRecoveryConfig {
        reset_period: Duration::from_secs(86400), // Reset counter after 24 hours
        actions: vec![
            windows_service::service::RecoveryAction::Restart {
                delay: Duration::from_millis(RESTART_DELAY_MS as u64),
            },
            windows_service::service::RecoveryAction::Restart {
                delay: Duration::from_millis(RESTART_DELAY_MS as u64),
            },
            windows_service::service::RecoveryAction::Restart {
                delay: Duration::from_millis(RESTART_DELAY_MS as u64),
            },
        ],
    };

    // Apply recovery settings
    if let Err(e) = status_handle.set_recovery_config(&recovery_config) {
        log::error!("Failed to set service recovery config: {}", e);
    } else {
        log::info!(
            "Service recovery settings configured: {} restart attempts, {}ms delay",
            MAX_RESTART_ATTEMPTS,
            RESTART_DELAY_MS
        );
    }

    // Update service status to running
    if let Err(e) = status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    }) {
        log::error!("Failed to set service status: {}", e);
        return;
    }

    let _ = *START_TIME;
    let executor_server = Arc::new(executor_server);
    let exec_for_health = executor_server.clone();

    // Run the gRPC server
    let server_result = tokio::runtime::Runtime::new().unwrap().block_on(async {
        Server::builder()
            .add_service(ExecutorServiceServer::from_arc(executor_server))
            .add_service(HealthServiceServer::from_arc(exec_for_health))
            .serve(addr)
            .await
    });

    if let Err(e) = server_result {
        log::error!("Server error: {}", e);
        // Update service status to stopped with error
        let _ = status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(1),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        });
    }
}

#[tonic::async_trait]
impl HealthService for ExecutorServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let mut dependencies = HashMap::new();

        // Check configuration health
        let config_healthy = config::check_config_health().await;
        let config_status = if config_healthy { "HEALTHY" } else { "UNHEALTHY" };

        // Get monitoring health status
        let monitoring_stats = get_monitoring_stats().await;
        let monitoring_healthy = matches!(
            monitoring_stats.health_status.overall_health,
            monitoring::HealthLevel::Healthy
        );
        let monitoring_status = if monitoring_healthy {
            "HEALTHY"
        } else {
            "UNHEALTHY"
        };

        // Determine overall health
        let overall_healthy = config_healthy && monitoring_healthy;

        #[cfg(target_os = "windows")]
        {
            dependencies.insert("windows_job_object".to_string(), "AVAILABLE".to_string());
            dependencies.insert("process_watchdog".to_string(), "AVAILABLE".to_string());
            dependencies.insert("sandbox_directory".to_string(), "CONFIGURED".to_string());
            dependencies.insert("configuration".to_string(), config_status.to_string());
            dependencies.insert("monitoring".to_string(), monitoring_status.to_string());
        }

        dependencies.insert("shell".to_string(), "AVAILABLE".to_string());
        dependencies.insert("input_simulation".to_string(), "AVAILABLE".to_string());

        // Add monitoring metrics to dependencies
        dependencies.insert(
            "execution_health".to_string(),
            format!("{:?}", monitoring_stats.health_status.overall_health),
        );
        dependencies.insert(
            "total_executions".to_string(),
            monitoring_stats.execution_stats.total_executions.to_string(),
        );
        dependencies.insert(
            "active_processes".to_string(),
            monitoring_stats.resource_metrics.active_processes.to_string(),
        );

        Ok(Response::new(HealthResponse {
            healthy: overall_healthy,
            service_name: "executor-service-windows-native".to_string(),
            uptime_seconds: uptime,
            status: if overall_healthy {
                "SERVING"
            } else {
                "DEGRADED"
            }.to_string(),
            dependencies,
        }))
    }
}

// Service control handler
fn service_handler(control_event: ServiceControl) -> ServiceControlHandlerResult {
    match control_event {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            log::info!("Service stop/shutdown requested");
            // Initiate graceful shutdown
            std::process::exit(0);
        }
        _ => ServiceControlHandlerResult::NoError,
    }
}

type ServiceControlHandlerResult = Result<(), windows_service::Error>;
