// logging-service-rs/src/main.rs
// Main Entry Point for logging-service-rs
// Implements the LoggingService gRPC server

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tonic::{Request, Response, Status, transport::Server};

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    HealthRequest, HealthResponse, LogEntry, LogResponse, MetricsRequest, MetricsResponse,
    health_service_server::{HealthService, HealthServiceServer},
    logging_service_server::{LoggingService, LoggingServiceServer},
};

// Define the Logging Server Structure
#[derive(Debug, Default)]
pub struct LoggingServer;

// Implement the LoggingService Trait
#[tonic::async_trait]
impl LoggingService for LoggingServer {
    async fn log(&self, request: Request<LogEntry>) -> Result<Response<LogResponse>, Status> {
        let log_entry = request.into_inner();

        log::info!(
            "Received log entry: level={}, service={}, message={}",
            log_entry.level,
            log_entry.service,
            log_entry.message
        );

        // --- LOG RECORDING STUB ---
        // In a real scenario, this would involve:
        // 1. Parsing and validating the log entry
        // 2. Routing to appropriate log sinks (file, ElasticSearch, CloudWatch, etc.)
        // 3. Applying log retention policies
        // 4. Indexing for searchability
        // 5. Alerting on critical errors
        // For now, we return a stub response

        // Generate a unique log ID for tracking
        let log_id = format!("log-{}", log_entry.timestamp);

        // In a real implementation, this would write to persistent storage
        println!(
            "[{}] {} | {} | {} | Metadata: {:?}",
            log_entry.timestamp,
            log_entry.level,
            log_entry.service,
            log_entry.message,
            log_entry.metadata
        );

        let reply = LogResponse {
            success: true,
            log_id: log_id.clone(),
        };

        log::debug!("Log entry recorded with ID: {}", log_id);

        Ok(Response::new(reply))
    }

    async fn get_metrics(
        &self,
        request: Request<MetricsRequest>,
    ) -> Result<Response<MetricsResponse>, Status> {
        let req_data = request.into_inner();

        log::info!(
            "Received GetMetrics request: service={}, metric_name={}, start={}, end={}",
            req_data.service,
            req_data.metric_name,
            req_data.start_time,
            req_data.end_time
        );

        // --- METRIC RETRIEVAL STUB ---
        // In a real scenario, this would involve:
        // 1. Querying a time-series database (Prometheus, InfluxDB, etc.)
        // 2. Aggregating metrics over the time range
        // 3. Calculating statistics (avg, min, max, percentiles)
        // 4. Filtering by service and metric name
        // 5. Returning formatted metric data
        // For now, we return stub metrics

        let mut metrics = HashMap::new();

        // Generate stub metrics based on request
        if req_data.service.is_empty() || req_data.service == "all" {
            // Return aggregate metrics for all services
            metrics.insert("total_requests".to_string(), 1250.0);
            metrics.insert("total_errors".to_string(), 12.0);
            metrics.insert("avg_latency_ms".to_string(), 45.3);
            metrics.insert("p95_latency_ms".to_string(), 120.5);
            metrics.insert("p99_latency_ms".to_string(), 250.8);
        } else {
            // Return service-specific metrics
            metrics.insert(format!("{}_requests", req_data.service), 450.0);
            metrics.insert(format!("{}_errors", req_data.service), 3.0);
            metrics.insert(format!("{}_avg_latency_ms", req_data.service), 38.2);
        }

        // Add metric-specific data if requested
        if !req_data.metric_name.is_empty() {
            metrics.insert(req_data.metric_name.clone(), 123.45);
        }

        let reply = MetricsResponse {
            metrics: metrics.clone(),
        };

        log::info!(
            "Returning {} metric(s) for service: {}",
            metrics.len(),
            if req_data.service.is_empty() {
                "all"
            } else {
                &req_data.service
            }
        );

        Ok(Response::new(reply))
    }
}

// Main function to start the gRPC server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Load centralized configuration
    let config = shared_types_rs::PhoenixConfig::load().map_err(|e| {
        log::error!("Failed to load configuration: {}", e);
        e
    })?;

    // Get bind address from centralized config
    let addr_str = config.get_bind_address("logging");
    let addr: SocketAddr = addr_str.parse()?;

    let logging_server = LoggingServer::default();

    log::info!(
        "LoggingService starting on {} (environment: {})",
        addr,
        config.system.environment
    );
    println!("LoggingService listening on {}", addr);

    let _ = *START_TIME;
    let logging_server = Arc::new(logging_server);
    let log_for_health = logging_server.clone();

    Server::builder()
        .add_service(LoggingServiceServer::from_arc(logging_server))
        .add_service(HealthServiceServer::from_arc(log_for_health))
        .serve(addr)
        .await?;

    Ok(())
}

#[tonic::async_trait]
impl HealthService for LoggingServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let mut dependencies = HashMap::new();
        dependencies.insert("log_storage".to_string(), "ACTIVE".to_string());
        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "logging-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}
