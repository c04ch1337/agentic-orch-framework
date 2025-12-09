// executor-rs/src/main.rs
// Main Entry Point for executor-rs
// Implements the ExecutorService gRPC server

use tonic::{transport::Server, Request, Response, Status};
use std::sync::Arc;
use std::time::Instant;
use std::net::SocketAddr;
use std::env;
use std::collections::HashMap;
use once_cell::sync::Lazy;

mod execution_logic;
use execution_logic::{execute_shell_command, execute_python_sandboxed, simulate_input};

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    executor_service_server::{ExecutorService, ExecutorServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    CommandRequest,
    CommandResponse,
    InputRequest,
    InputResponse,
    HealthRequest,
    HealthResponse,
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
            Ok((stdout, stderr, exit_code)) => {
                Ok(Response::new(CommandResponse {
                    stdout,
                    stderr,
                    exit_code,
                }))
            }
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
            Ok(_) => {
                Ok(Response::new(InputResponse {
                    success: true,
                    error: "".to_string(),
                }))
            }
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
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Read address from environment variable or use the default port 50062
    let addr_str = env::var("EXECUTOR_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50062".to_string());
    
    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str
            .strip_prefix("http://")
            .unwrap_or(&addr_str)
            .parse()?
    } else {
        addr_str.parse()?
    };

    let executor_server = ExecutorServer::default();

    log::info!("Executor Service starting on {}", addr);
    println!("Executor Service listening on {}", addr);

    let _ = *START_TIME;
    let executor_server = Arc::new(executor_server);
    let exec_for_health = executor_server.clone();

    Server::builder()
        .add_service(ExecutorServiceServer::from_arc(executor_server))
        .add_service(HealthServiceServer::from_arc(exec_for_health))
        .serve(addr)
        .await?;

    Ok(())
}

#[tonic::async_trait]
impl HealthService for ExecutorServer {
    async fn get_health(&self, _request: Request<HealthRequest>) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let mut dependencies = HashMap::new();
        dependencies.insert("shell".to_string(), "AVAILABLE".to_string());
        dependencies.insert("input_simulation".to_string(), "AVAILABLE".to_string());
        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "executor-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}
