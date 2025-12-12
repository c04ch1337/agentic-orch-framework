// tools-service-rs/src/main.rs
// Main Entry Point for tools-service-rs
// Implements the ToolsService gRPC server with Enhanced Validation
// Provides comprehensive command parameter validation and sanitization

use once_cell::sync::Lazy;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tonic::{transport::Server, Request, Response, Status};

// Import the tool-sdk for API client management
use tool_sdk::{
    config::{CompositeConfigProvider, ConfigProvider, EnvConfigProvider, SerpAPIConfig},
    serpapi::SerpAPIClient,
    resilience::{CircuitBreakerConfig, RetryConfig},
    ServiceClient, Telemetry,
};

// Import our validation module (used by tool_manager and tools)
mod tool_manager;
mod tools;
mod validation;

// Track service start time for uptime reporting
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

// Import Generated Code and Types
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    health_service_server::{HealthService, HealthServiceServer},
    tools_service_server::{ToolsService, ToolsServiceServer},
    DirectiveResponse, EmergencyDirective, HealthRequest, HealthResponse, ListToolsRequest,
    ListToolsResponse, ToolRequest, ToolResponse,
};

// Define the Tools Server Structure
pub struct ToolsServer {
    // SDK API clients
    serpapi_client: SerpAPIClient,
    // Configuration provider for environment variables
    config_provider: Arc<dyn ConfigProvider>,
}

impl Default for ToolsServer {
    fn default() -> Self {
        // Create a service-specific ConfigProvider
        // First, create environment-based provider with proper prefixes
        let env_provider = EnvConfigProvider::new().with_prefix("PHOENIX");

        // Create a composite provider for fallbacks
        let mut composite_provider = CompositeConfigProvider::new();
        composite_provider.add_provider(env_provider);

        // Create shared config provider
        let config_provider = Arc::new(composite_provider);

        // Initialize SerpAPI client with resilience patterns
        let serpapi_config = SerpAPIConfig::from_provider(&*config_provider).unwrap_or_else(|e| {
            log::warn!("Failed to load SerpAPI config from environment: {}", e);
            log::info!("Using default SerpAPI configuration");
            SerpAPIConfig::default()
        });

        // Create resilience configuration
        let retry_config = RetryConfig {
            max_retries: 3,
            initial_interval: std::time::Duration::from_millis(500),
            max_interval: std::time::Duration::from_secs(10),
            backoff_factor: 2.0,
            retry_status_codes: vec![429, 500, 502, 503, 504],
            ..RetryConfig::default()
        };

        let circuit_breaker_config = CircuitBreakerConfig {
            failure_threshold: 5,
            reset_timeout: std::time::Duration::from_secs(30),
            half_open_success_threshold: 2,
            ..CircuitBreakerConfig::default()
        };

        // Create SerpAPI client with builder pattern
        let serpapi_client = SerpAPIClient::builder()
            .api_key(serpapi_config.api_key.clone())
            .base_url(serpapi_config.base_url.clone())
            .timeout(serpapi_config.timeout_seconds)
            .retry(retry_config)
            .circuit_breaker(circuit_breaker_config)
            .build()
            .unwrap_or_else(|e| {
                log::error!("Failed to build SerpAPI client: {}", e);
                log::warn!("Using default SerpAPI client configuration");
                SerpAPIClient::new()
            });

        log::info!("SDK clients initialized with resilience patterns");

        Self {
            serpapi_client,
            config_provider,
        }
    }
}

impl ToolsServer {
    // No internal helper methods are required for the general-purpose tools.
}

// Implement the ToolsService Trait
#[tonic::async_trait]
impl ToolsService for ToolsServer {
    async fn execute_tool(
        &self,
        request: Request<ToolRequest>,
    ) -> Result<Response<ToolResponse>, Status> {
        let req_data = request.into_inner();
        let tool_name = req_data.tool_name;
        let parameters = req_data.parameters;

        log::info!("Received ExecuteTool request: tool_name={}", tool_name);

        // Build ToolContext and dispatch via the ToolManager
        let request_id = format!(
            "tools-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        let context = crate::tool_manager::ToolContext {
            parameters,
            user_id: None,
            session_id: None,
            request_id,
            context_data: HashMap::new(),
        };

        let manager = crate::tool_manager::TOOL_MANAGER.clone();
        let result = manager.execute_tool(&tool_name, context).await;

        match result {
            Ok(tool_result) => {
                log::info!("Tool '{}' executed successfully", tool_name);
                Ok(Response::new(ToolResponse {
                    success: tool_result.success,
                    result: if tool_result.success {
                        tool_result.data
                    } else {
                        String::new()
                    },
                    error: if tool_result.success {
                        String::new()
                    } else {
                        tool_result.error
                    },
                }))
            }
            Err(e) => {
                log::error!("Tool '{}' failed: {}", tool_name, e);
                Ok(Response::new(ToolResponse {
                    success: false,
                    result: String::new(),
                    error: e.to_string(),
                }))
            }
        }
    }

    async fn list_tools(
        &self,
        request: Request<ListToolsRequest>,
    ) -> Result<Response<ListToolsResponse>, Status> {
        let req_data = request.into_inner();

        log::info!(
            "Received ListTools request: category={:?}",
            req_data.category
        );

        let manager = crate::tool_manager::TOOL_MANAGER.clone();
        let category = if req_data.category.is_empty() {
            None
        } else {
            Some(req_data.category.as_str())
        };

        let metadata_list = manager.list_tools(category);
        let tools = metadata_list
            .into_iter()
            .map(|m| m.id)
            .collect::<Vec<String>>();

        let reply = ListToolsResponse {
            tools: tools.clone(),
        };

        log::info!("Returning {} available tool(s)", reply.tools.len());

        Ok(Response::new(reply))
    }

    async fn execute_emergency_directive(
        &self,
        _request: Request<EmergencyDirective>,
    ) -> Result<Response<DirectiveResponse>, Status> {
        log::warn!("ExecuteEmergencyDirective was called but is not implemented in tools-service");

        let reply = DirectiveResponse {
            success: false,
            execution_id: String::new(),
            result: "ExecuteEmergencyDirective is not supported by tools-service; this deployment only exposes general-purpose tools (web_search, execute_code, read_file, write_file).".to_string(),
        };

        Ok(Response::new(reply))
    }
}

// Main function to start the gRPC server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Register general-purpose tools in the ToolManager
    if let Err(e) = tools::register_all_tools().await {
        log::error!("Failed to register tools: {}", e);
        return Err(Box::new(e) as Box<dyn std::error::Error>);
    }

    // Read address from environment variable or use the default port 50054
    let addr_str = env::var("TOOLS_SERVICE_ADDR").unwrap_or_else(|_| "0.0.0.0:50054".to_string());

    // Parse the address, handling both "0.0.0.0:50054" and "http://127.0.0.1:50054" formats
    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str
            .strip_prefix("http://")
            .unwrap_or(&addr_str)
            .parse()?
    } else {
        addr_str.parse()?
    };

    let tools_server = ToolsServer::default();

    log::info!("ToolsService starting on {}", addr);
    println!("ToolsService listening on {}", addr);

    // Initialize start time
    let _ = *START_TIME;

    let tools_server = Arc::new(tools_server);
    let tools_for_health = tools_server.clone();

    Server::builder()
        .add_service(ToolsServiceServer::from_arc(tools_server))
        .add_service(HealthServiceServer::from_arc(tools_for_health))
        .serve(addr)
        .await?;

    Ok(())
}

// Implement HealthService for ToolsServer
#[tonic::async_trait]
impl HealthService for ToolsServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let mut system_healthy = true;

        let mut dependencies = HashMap::new();
        dependencies.insert("executor".to_string(), "CONFIGURED".to_string());

        // Check SerpAPI client health
        let serpapi_status = match self.serpapi_client.health_check().await {
            Ok(true) => "HEALTHY".to_string(),
            Ok(false) => {
                log::warn!("SerpAPI client is not healthy");
                system_healthy = false;
                "DEGRADED".to_string()
            }
            Err(e) => {
                log::error!("SerpAPI client health check error: {}", e);
                system_healthy = false;
                format!("ERROR: {}", e)
            }
        };
        dependencies.insert("serpapi".to_string(), serpapi_status);

        // Check for telemetry metrics
        if let Some(serpapi_metrics) = ServiceClient::metrics(&self.serpapi_client) {
            // Add key metrics to dependencies
            if let Some(req_count) = serpapi_metrics.get("request_count") {
                dependencies.insert("serpapi_requests".to_string(), req_count.clone());
            }
            if let Some(error_count) = serpapi_metrics.get("error_count") {
                dependencies.insert("serpapi_errors".to_string(), error_count.clone());
            }
        }

        // Generate overall service status
        let status = if system_healthy {
            "SERVING".to_string()
        } else {
            "DEGRADED".to_string()
        };

        Ok(Response::new(HealthResponse {
            healthy: system_healthy,
            service_name: "tools-service".to_string(),
            uptime_seconds: uptime,
            status,
            dependencies,
        }))
    }
}
