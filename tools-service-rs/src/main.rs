// tools-service-rs/src/main.rs
// Main Entry Point for tools-service-rs
// Implements the ToolsService gRPC server with Enhanced Validation
// Provides comprehensive command parameter validation and sanitization

use tonic::{transport::Server, Request, Response, Status};
use std::sync::Arc;
use std::time::Instant;
use std::net::SocketAddr;
use std::env;
use std::collections::HashMap;
use once_cell::sync::Lazy;
use serde_json::json;

// Import the tool-sdk for API client management
use tool_sdk::{
    config::{ConfigProvider, EnvConfigProvider, CompositeConfigProvider, ServiceConfig},
    serpapi::SerpAPIClient,
    serpapi::SerpAPIConfig,
    Resilience, RetryConfig, CircuitBreakerConfig,
};

// Import our validation module
mod validation;
use validation::{
    validate_command_execution,
    validate_input_simulation,
    sanitize_command,
    sanitize_command_args,
    ToolValidationError,
    validate_command_name,
    validate_input_type,
    validate_input_params
};

/// Validate tool request parameters based on the tool name
fn validate_tool_request(tool_name: &str, params: &HashMap<String, String>) -> Option<String> {
    match tool_name {
        "execute_command" => {
            // Required parameters check
            let cmd = match params.get("command") {
                Some(cmd) => cmd,
                None => return Some("Missing required 'command' parameter".to_string())
            };
            
            // Validate command name
            if let Err(e) = validate_command_name(cmd, None, None) {
                return Some(format!("Command validation failed: {}", e));
            }
            
            // Parse arguments if provided
            let args_str = params.get("args").cloned().unwrap_or_default();
            let args = if args_str.is_empty() {
                vec![]
            } else {
                args_str.split_whitespace().map(|s| s.to_string()).collect::<Vec<String>>()
            };
            
            // Validate arguments
            if !args.is_empty() {
                match validation::validate_command_args(&args) {
                    Ok(_) => {},
                    Err(e) => return Some(format!("Argument validation failed: {}", e))
                }
            }
        },
        "execute_python" => {
            // Required parameters check
            let code = match params.get("code") {
                Some(code) => code,
                None => return Some("Missing required 'code' parameter".to_string())
            };
            
            // Check for security issues in Python code
            if let Err(e) = input_validation_rs::validators::security::default_security_scan(code) {
                return Some(format!("Python code security check failed: {}", e));
            }
        },
        "simulate_input" => {
            // Required parameters check
            let input_type = match params.get("type") {
                Some(t) => t,
                None => return Some("Missing required 'type' parameter".to_string())
            };
            
            // Validate input type
            if let Err(e) = validate_input_type(input_type) {
                return Some(format!("Input type validation failed: {}", e));
            }
            
            // Create a copy of params without the type parameter
            let mut input_params = params.clone();
            input_params.remove("type");
            
            // Validate input parameters
            if let Err(e) = validate_input_params(&input_params, input_type) {
                return Some(format!("Input parameters validation failed: {}", e));
            }
        },
        _ => {
            // Skip validation for other tools
        }
    }
    
    // All validations passed
    None
}

// Track service start time for uptime reporting
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

// Import Generated Code and Types
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    tools_service_server::{ToolsService, ToolsServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    executor_service_client::ExecutorServiceClient,
    ToolRequest,
    ToolResponse,
    ListToolsRequest,
    ListToolsResponse,
    CommandRequest,
    InputRequest,
    HealthRequest,
    HealthResponse,
};

// Define the Tools Server Structure
#[derive(Debug)]
pub struct ToolsServer {
    executor_addr: String,
    // SDK API clients
    serpapi_client: SerpAPIClient,
    // Add other clients as needed (openai_client, etc.)
    // Configuration provider for environment variables
    config_provider: Arc<dyn ConfigProvider>,
}

impl Default for ToolsServer {
    fn default() -> Self {
        // Create a service-specific ConfigProvider
        // First, create environment-based provider with proper prefixes
        let env_provider = EnvConfigProvider::new()
            .with_prefix("PHOENIX");
            
        // Create a composite provider for fallbacks
        let mut composite_provider = CompositeConfigProvider::new();
        composite_provider.add_provider(env_provider);
        
        // Create shared config provider
        let config_provider = Arc::new(composite_provider);
        
        // Initialize SerpAPI client with resilience patterns
        let serpapi_config = SerpAPIConfig::from_provider(&*config_provider)
            .unwrap_or_else(|e| {
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
            executor_addr: env::var("EXECUTOR_ADDR").unwrap_or_else(|_| "http://127.0.0.1:50062".to_string()),
            serpapi_client,
            config_provider,
        }
    }
}

impl ToolsServer {
    async fn forward_command(&self, cmd: String, args: Vec<String>) -> Result<String, String> {
        // Validate and sanitize the command and arguments
        match validate_command_execution(&cmd, &args) {
            Ok(_) => {
                log::info!("Command validation successful for: {} {:?}", cmd, args);
                
                // Sanitize command and args for extra safety
                let sanitized_cmd = sanitize_command(&cmd);
                let sanitized_args = sanitize_command_args(&args);
                
                // Log if sanitization changed anything
                if sanitized_cmd != cmd || sanitized_args != args {
                    log::warn!("Command sanitized: '{}' -> '{}'", cmd, sanitized_cmd);
                }
                
                // Forward the sanitized command to the executor
                let mut client = ExecutorServiceClient::connect(self.executor_addr.clone())
                    .await
                    .map_err(|e| format!("Failed to connect to executor: {}", e))?;

                let request = CommandRequest {
                    command: sanitized_cmd,
                    args: sanitized_args,
                    env: HashMap::new(),
                };

                let response = client.execute_command(request).await
                    .map_err(|e| format!("Executor call failed: {}", e))?
                    .into_inner();

                if response.exit_code == 0 {
                    Ok(response.stdout)
                } else {
                    Err(format!("Command failed (exit {}): {} {}",
                        response.exit_code, response.stdout, response.stderr))
                }
            },
            Err(e) => {
                log::error!("Command validation failed: {}", e);
                Err(format!("Command validation failed: {}", e))
            }
        }
    }

    async fn forward_input(&self, input_type: String, params: HashMap<String, String>) -> Result<String, String> {
        // Validate input type and parameters
        match validate_input_simulation(&input_type, &params) {
            Ok(_) => {
                log::info!("Input validation successful for type: {} with params: {:?}",
                    input_type, params);
                
                // Forward the validated input to the executor
                let mut client = ExecutorServiceClient::connect(self.executor_addr.clone())
                    .await
                    .map_err(|e| format!("Failed to connect to executor: {}", e))?;

                let request = InputRequest {
                    input_type,
                    parameters: params,
                };

                let response = client.simulate_input(request).await
                    .map_err(|e| format!("Executor call failed: {}", e))?
                    .into_inner();

                if response.success {
                    Ok("Input simulated successfully".to_string())
                } else {
                    Err(format!("Input simulation failed: {}", response.error))
                }
            },
            Err(e) => {
                log::error!("Input validation failed: {}", e);
                Err(format!("Input validation failed: {}", e))
            }
        }
    }

    /// Execute Python code with enhanced security and validation
    async fn execute_python(&self, code: String, env_vars: HashMap<String, String>) -> Result<String, String> {
        // Add explicit warning logs about local execution risks
        log::warn!("SECURITY ALERT: Executing Python code locally. This has potential security implications!");
        
        // Check for security issues in Python code
        if let Err(e) = input_validation_rs::validators::security::default_security_scan(&code) {
            log::error!("Python code security check failed: {}", e);
            return Err(format!("Python code security check failed: {}", e));
        }
        
        log::info!("Python code validation successful. Proceeding with execution.");
        
        // Connect to the executor service
        let mut client = ExecutorServiceClient::connect(self.executor_addr.clone())
            .await
            .map_err(|e| format!("Failed to connect to executor: {}", e))?;
        
        // Create a temporary file for the Python code
        let temp_dir = std::env::temp_dir();
        let file_name = format!("script_{}.py", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis());
        let file_path = temp_dir.join(&file_name);
        
        // Write the code to a temporary file
        if let Err(e) = std::fs::write(&file_path, &code) {
            log::error!("Failed to write Python code to temporary file: {}", e);
            return Err(format!("Failed to write Python code to temporary file: {}", e));
        }
        
        // Create a command to execute the Python file
        let cmd = "python";
        let args = vec![file_path.to_string_lossy().to_string()];
        
        // Define execution timeout and output size limits
        let timeout_seconds = 10; // 10 seconds max execution time
        let max_output_size = 1024 * 1024; // 1MB max output size
        
        // Execute the command
        let command_req = CommandRequest {
            command: cmd.to_string(),
            args,
            env: env_vars,
        };
        
        // Execute with timeout
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_seconds),
            client.execute_command(command_req)
        ).await
            .map_err(|_| format!("Python execution timed out after {} seconds", timeout_seconds))?
            .map_err(|e| format!("Executor call failed: {}", e))?
            .into_inner();
        
        // Clean up the temporary file
        if let Err(e) = std::fs::remove_file(file_path) {
            log::warn!("Failed to remove temporary Python file: {}", e);
        }
        
        if response.exit_code == 0 {
            // Check if output size exceeds the limit
            if response.stdout.len() > max_output_size {
                log::warn!("Python execution output size ({} bytes) exceeded limit of {} bytes. Truncating output.",
                    response.stdout.len(), max_output_size);
                
                // Truncate the output
                let truncated = &response.stdout[0..max_output_size];
                Ok(format!("{}\n... [Output truncated due to size limit]", truncated))
            } else {
                Ok(response.stdout)
            }
        } else {
            Err(format!("Python execution failed (exit code {}): {}{}",
                response.exit_code,
                response.stdout,
                if !response.stderr.is_empty() { format!("\nError: {}", response.stderr) } else { String::new() }
            ))
        }
    }
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
        let params = req_data.parameters;
        
        log::info!("Received ExecuteTool request: tool_name={}", tool_name);

        // Enhanced validation for tool parameters
        let validation_result = if let Some(validation_error) = validate_tool_request(&tool_name, &params) {
            Err(validation_error)
        } else {
            Ok(())
        };
        
        let result = match validation_result {
            Err(error) => Err(error),
            Ok(()) => match tool_name.as_str() {
                "execute_command" => {
                    let cmd = params.get("command").cloned().unwrap_or_default();
                    let args_str = params.get("args").cloned().unwrap_or_default();
                    let args = if args_str.is_empty() {
                        vec![]
                    } else {
                        args_str.split_whitespace().map(|s| s.to_string()).collect()
                    };
                    
                    self.forward_command(cmd, args).await
                },
                "execute_python" => {
                    let code = params.get("code").cloned().unwrap_or_default();
                    // Empty environment variables by default
                    let env_vars = HashMap::new();
                    
                    self.execute_python(code, env_vars).await
                },
                "simulate_input" => {
                    let input_type = params.get("type").cloned().unwrap_or_default();
                    
                    // Create a clone of params without the type parameter
                    let mut input_params = params.clone();
                    input_params.remove("type");
                    
                    self.forward_input(input_type, input_params).await
                },
            _ => {
                // Stub for other tools
                Ok(format!(
                    "Tools Service executed stub for tool: '{}' with parameters: {:?}",
                    tool_name, params
                ))
            }
        };

        match result {
            Ok(res) => {
                log::info!("Tool '{}' executed successfully", tool_name);
                Ok(Response::new(ToolResponse {
                    success: true,
                    result: res,
                    error: String::new(),
                }))
            },
            Err(e) => {
                log::error!("Tool '{}' failed: {}", tool_name, e);
                Ok(Response::new(ToolResponse {
                    success: false,
                    result: String::new(),
                    error: e,
                }))
            }
        }
    }

    async fn list_tools(
        &self,
        request: Request<ListToolsRequest>,
    ) -> Result<Response<ListToolsResponse>, Status> {
        let req_data = request.into_inner();
        
        log::info!("Received ListTools request: category={:?}", req_data.category);
        
        let available_tools = vec![
            "execute_command".to_string(),
            "execute_python".to_string(),
            "simulate_input".to_string(),
            "send_email".to_string(),
            "get_weather".to_string(),
            "execute_code".to_string(),
            "web_search".to_string(),
            "read_file".to_string(),
            "write_file".to_string(),
            "database_query".to_string(),
            "api_call".to_string(),
        ];

        // Filter by category if provided
        let filtered_tools = if !req_data.category.is_empty() {
            log::info!("Filtering tools by category: {}", req_data.category);
            available_tools
                .into_iter()
                .filter(|tool| {
                    match req_data.category.as_str() {
                        "system" => tool.contains("execute") || tool.contains("input") || tool.contains("python"),
                        "communication" => tool.contains("email") || tool.contains("api"),
                        "data" => tool.contains("file") || tool.contains("database"),
                        "external" => tool.contains("weather") || tool.contains("web"),
                        _ => true,
                    }
                })
                .collect()
        } else {
            available_tools
        };

        let reply = ListToolsResponse {
            tools: filtered_tools.clone(),
        };

        log::info!("Returning {} available tool(s)", filtered_tools.len());

        Ok(Response::new(reply))
    }
}

// Main function to start the gRPC server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Read address from environment variable or use the default port 50054
    let addr_str = env::var("TOOLS_SERVICE_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50054".to_string());
    
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
            Ok(true) => {
                "HEALTHY".to_string()
            },
            Ok(false) => {
                log::warn!("SerpAPI client is not healthy");
                system_healthy = false;
                "DEGRADED".to_string()
            },
            Err(e) => {
                log::error!("SerpAPI client health check error: {}", e);
                system_healthy = false;
                format!("ERROR: {}", e)
            }
        };
        dependencies.insert("serpapi".to_string(), serpapi_status);
        
        // Check for telemetry metrics
        if let Some(serpapi_metrics) = self.serpapi_client.metrics() {
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
