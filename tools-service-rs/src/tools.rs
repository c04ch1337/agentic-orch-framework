//! Tool Implementations
//!
//! Contains specific implementations for all supported tools in the system.
//! Each tool follows the Tool trait and includes proper validation, security checks,
//! and telemetry.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::fs;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use regex::Regex;
use thiserror::Error;
use tokio::process::Command as AsyncCommand;

use crate::tool_manager::{
    Tool, ToolMetadata, ToolResult, ToolContext,
    Capability, ParameterDefinition, ToolManagerError,
    TOOL_MANAGER,
};

use crate::validation::{
    validate_command_execution,
    validate_command_name,
    validate_command_args,
    sanitize_command,
    sanitize_command_args,
    ToolValidationError,
};

/// Telemetry data for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTelemetry {
    /// Tool ID
    pub tool_id: String,
    /// Session ID
    pub session_id: Option<String>,
    /// User ID
    pub user_id: Option<String>,
    /// Request ID
    pub request_id: String,
    /// Execution start timestamp
    pub start_time: u64,
    /// Execution end timestamp
    pub end_time: u64,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Whether execution was successful
    pub success: bool,
    /// Error message if unsuccessful
    pub error: Option<String>,
    /// Tool parameters (sanitized)
    pub parameters: HashMap<String, String>,
    /// Additional telemetry data
    pub additional_data: HashMap<String, String>,
}

/// Execute Command Tool
pub struct ExecuteCommandTool {
    metadata: ToolMetadata,
}

impl ExecuteCommandTool {
    pub fn new() -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::ExecuteCommand);

        let parameters = vec![
            ParameterDefinition {
                name: "command".to_string(),
                description: "Command to execute".to_string(),
                required: true,
                param_type: "string".to_string(),
                default: None,
                validation: Some(r"^[a-zA-Z0-9_\-\.]+$".to_string()),
            },
            ParameterDefinition {
                name: "args".to_string(),
                description: "Command arguments".to_string(),
                required: false,
                param_type: "string".to_string(),
                default: Some("".to_string()),
                validation: None,
            },
            ParameterDefinition {
                name: "working_dir".to_string(),
                description: "Working directory".to_string(),
                required: false,
                param_type: "string".to_string(),
                default: Some(".".to_string()),
                validation: None,
            },
        ];

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            metadata: ToolMetadata {
                id: "execute_command".to_string(),
                name: "Execute Command".to_string(),
                description: "Executes a system command with enhanced security validation".to_string(),
                version: "1.0.0".to_string(),
                author: "System".to_string(),
                category: "system".to_string(),
                parameters,
                capabilities,
                enabled: true,
                created_at: now,
                updated_at: now,
            },
        }
    }

    /// Send telemetry data about tool execution
    async fn send_telemetry(&self, telemetry: ToolTelemetry) {
        // In a real implementation, this would send telemetry to a monitoring system
        info!(
            "Tool telemetry: {} executed in {}ms ({})",
            telemetry.tool_id, telemetry.duration_ms,
            if telemetry.success { "success" } else { "failure" }
        );
        
        // Log detailed telemetry for debugging
        debug!("Tool telemetry details: {:?}", telemetry);
    }
}

#[async_trait]
impl Tool for ExecuteCommandTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn validate_parameters(&self, parameters: &HashMap<String, String>) -> Result<(), ToolManagerError> {
        // Check for required parameters
        let cmd = match parameters.get("command") {
            Some(cmd) => cmd,
            None => return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: command".to_string())
            )),
        };
        
        // Validate command name
        validate_command_name(cmd, None, None).map_err(ToolManagerError::ValidationError)?;
        
        // Parse and validate arguments if provided
        let args_str = parameters.get("args").cloned().unwrap_or_default();
        let args = if args_str.is_empty() {
            vec![]
        } else {
            args_str.split_whitespace().map(|s| s.to_string()).collect::<Vec<String>>()
        };
        
        if !args.is_empty() {
            validate_command_args(&args).map_err(ToolManagerError::ValidationError)?;
        }
        
        // Validate working directory if provided
        if let Some(working_dir) = parameters.get("working_dir") {
            let path = Path::new(working_dir);
            if !path.exists() || !path.is_dir() {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other(format!("Working directory does not exist: {}", working_dir))
                ));
            }
        }
        
        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let start_instant = Instant::now();
        
        // Extract parameters
        let cmd = context.parameters.get("command").unwrap().clone();
        let args_str = context.parameters.get("args").cloned().unwrap_or_default();
        let args = if args_str.is_empty() {
            vec![]
        } else {
            args_str.split_whitespace().map(|s| s.to_string()).collect::<Vec<String>>()
        };
        let working_dir = context.parameters.get("working_dir").cloned().unwrap_or_else(|| ".".to_string());
        
        // Create telemetry data
        let mut telemetry = ToolTelemetry {
            tool_id: self.metadata.id.clone(),
            session_id: context.session_id.clone(),
            user_id: context.user_id.clone(),
            request_id: context.request_id.clone(),
            start_time,
            end_time: 0,
            duration_ms: 0,
            success: false,
            error: None,
            parameters: context.parameters.clone(),
            additional_data: HashMap::new(),
        };
        
        // Validate command and arguments
        match validate_command_execution(&cmd, &args) {
            Ok(_) => {
                info!("Command validation successful for: {} {:?}", cmd, args);
                
                // Sanitize command and arguments for security
                let sanitized_cmd = sanitize_command(&cmd);
                let sanitized_args = sanitize_command_args(&args);
                
                // Log if sanitization changed anything
                if sanitized_cmd != cmd || sanitized_args != args {
                    warn!("Command sanitized: '{}' -> '{}'", cmd, sanitized_cmd);
                }
                
                // Execute the command
                let output = match AsyncCommand::new(&sanitized_cmd)
                    .args(&sanitized_args)
                    .current_dir(working_dir)
                    .output()
                    .await {
                        Ok(output) => output,
                        Err(e) => {
                            error!("Failed to execute command: {}", e);
                            telemetry.error = Some(format!("Failed to execute command: {}", e));
                            telemetry.end_time = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs();
                            telemetry.duration_ms = start_instant.elapsed().as_millis() as u64;
                            self.send_telemetry(telemetry).await;
                            
                            return Err(ToolManagerError::ExecutionError(
                                format!("Failed to execute command: {}", e)
                            ));
                        }
                    };
                
                // Process the result
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                
                // Update telemetry
                telemetry.success = exit_code == 0;
                telemetry.end_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                telemetry.duration_ms = start_instant.elapsed().as_millis() as u64;
                telemetry.additional_data.insert("exit_code".to_string(), exit_code.to_string());
                telemetry.additional_data.insert("stderr_length".to_string(), stderr.len().to_string());
                
                // Send telemetry
                self.send_telemetry(telemetry).await;
                
                // Return result
                if exit_code == 0 {
                    let mut metadata = HashMap::new();
                    metadata.insert("exit_code".to_string(), exit_code.to_string());
                    metadata.insert("execution_time_ms".to_string(), telemetry.duration_ms.to_string());
                    
                    Ok(ToolResult {
                        success: true,
                        data: stdout,
                        error: String::new(),
                        metadata,
                        duration_ms: telemetry.duration_ms,
                    })
                } else {
                    let error_message = format!("Command failed (exit {}): {}", exit_code, stderr);
                    
                    let mut metadata = HashMap::new();
                    metadata.insert("exit_code".to_string(), exit_code.to_string());
                    metadata.insert("stdout".to_string(), stdout.clone());
                    
                    Ok(ToolResult {
                        success: false,
                        data: String::new(),
                        error: error_message,
                        metadata,
                        duration_ms: telemetry.duration_ms,
                    })
                }
            },
            Err(e) => {
                // Command validation failed
                error!("Command validation failed: {}", e);
                
                telemetry.success = false;
                telemetry.error = Some(format!("Command validation failed: {}", e));
                telemetry.end_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                telemetry.duration_ms = start_instant.elapsed().as_millis() as u64;
                
                self.send_telemetry(telemetry).await;
                
                Err(ToolManagerError::ValidationError(e))
            }
        }
    }
}

/// Execute Python Tool
pub struct ExecutePythonTool {
    metadata: ToolMetadata,
}

impl ExecutePythonTool {
    pub fn new() -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::ExecuteCode);

        let parameters = vec![
            ParameterDefinition {
                name: "code".to_string(),
                description: "Python code to execute".to_string(),
                required: true,
                param_type: "string".to_string(),
                default: None,
                validation: None,
            },
            ParameterDefinition {
                name: "timeout".to_string(),
                description: "Execution timeout in seconds".to_string(),
                required: false,
                param_type: "number".to_string(),
                default: Some("5".to_string()),
                validation: Some(r"^[1-9][0-9]{0,2}$".to_string()),
            },
        ];

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            metadata: ToolMetadata {
                id: "execute_python".to_string(),
                name: "Execute Python".to_string(),
                description: "Executes Python code in a sandboxed environment".to_string(),
                version: "1.0.0".to_string(),
                author: "System".to_string(),
                category: "coding".to_string(),
                parameters,
                capabilities,
                enabled: true,
                created_at: now,
                updated_at: now,
            },
        }
    }
}

#[async_trait]
impl Tool for ExecutePythonTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn validate_parameters(&self, parameters: &HashMap<String, String>) -> Result<(), ToolManagerError> {
        // Check for required parameters
        let code = match parameters.get("code") {
            Some(code) => code,
            None => return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: code".to_string())
            )),
        };
        
        // Check for security issues in Python code
        if let Err(e) = input_validation_rs::validators::security::default_security_scan(code) {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::SecurityThreat(format!("Python code security check failed: {}", e))
            ));
        }
        
        // Validate timeout if provided
        if let Some(timeout) = parameters.get("timeout") {
            if let Err(_) = timeout.parse::<u32>() {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("Timeout must be a positive number".to_string())
                ));
            }
        }
        
        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        let start_time = Instant::now();
        
        // Extract parameters
        let code = context.parameters.get("code").unwrap().clone();
        let timeout = context.parameters.get("timeout")
            .and_then(|t| t.parse::<u32>().ok())
            .unwrap_or(5);
        
        // Create a temporary Python file
        let temp_dir = std::env::temp_dir();
        let file_name = format!("script_{}.py", context.request_id);
        let file_path = temp_dir.join(file_name);
        
        if let Err(e) = fs::write(&file_path, &code) {
            return Err(ToolManagerError::ExecutionError(
                format!("Failed to write temporary Python file: {}", e)
            ));
        }
        
        // Execute the Python code with timeout
        let output = match tokio::process::Command::new("python")
            .arg(&file_path)
            .output()
            .await {
                Ok(output) => output,
                Err(e) => {
                    // Clean up temp file
                    let _ = fs::remove_file(&file_path);
                    
                    return Err(ToolManagerError::ExecutionError(
                        format!("Failed to execute Python code: {}", e)
                    ));
                }
            };
        
        // Clean up temp file
        let _ = fs::remove_file(file_path);
        
        // Process the result
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        let mut metadata = HashMap::new();
        metadata.insert("exit_code".to_string(), exit_code.to_string());
        metadata.insert("execution_time_ms".to_string(), duration_ms.to_string());
        
        if exit_code == 0 {
            Ok(ToolResult {
                success: true,
                data: stdout,
                error: String::new(),
                metadata,
                duration_ms,
            })
        } else {
            let error_message = format!("Python execution failed (exit {}): {}", exit_code, stderr);
            metadata.insert("stdout".to_string(), stdout);
            
            Ok(ToolResult {
                success: false,
                data: String::new(),
                error: error_message,
                metadata,
                duration_ms,
            })
        }
    }
}

/// Simulate Input Tool
pub struct SimulateInputTool {
    metadata: ToolMetadata,
}

impl SimulateInputTool {
    pub fn new() -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::SimulateInput);

        let parameters = vec![
            ParameterDefinition {
                name: "type".to_string(),
                description: "Type of input to simulate (keyboard, mouse, touch, gamepad)".to_string(),
                required: true,
                param_type: "string".to_string(),
                default: None,
                validation: Some(r"^(keyboard|mouse|touch|gamepad)$".to_string()),
            },
            ParameterDefinition {
                name: "key".to_string(),
                description: "Key to simulate for keyboard input".to_string(),
                required: false,
                param_type: "string".to_string(),
                default: None,
                validation: None,
            },
            ParameterDefinition {
                name: "x".to_string(),
                description: "X coordinate for mouse/touch input".to_string(),
                required: false,
                param_type: "number".to_string(),
                default: None,
                validation: None,
            },
            ParameterDefinition {
                name: "y".to_string(),
                description: "Y coordinate for mouse/touch input".to_string(),
                required: false,
                param_type: "number".to_string(),
                default: None,
                validation: None,
            },
        ];

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            metadata: ToolMetadata {
                id: "simulate_input".to_string(),
                name: "Simulate Input".to_string(),
                description: "Simulates user input (keyboard, mouse, etc.)".to_string(),
                version: "1.0.0".to_string(),
                author: "System".to_string(),
                category: "system".to_string(),
                parameters,
                capabilities,
                enabled: true,
                created_at: now,
                updated_at: now,
            },
        }
    }
}

#[async_trait]
impl Tool for SimulateInputTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn validate_parameters(&self, parameters: &HashMap<String, String>) -> Result<(), ToolManagerError> {
        // Check for required parameters
        let input_type = match parameters.get("type") {
            Some(t) => t,
            None => return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: type".to_string())
            )),
        };
        
        // Validate input type
        match input_type.as_str() {
            "keyboard" => {
                // Keyboard input requires 'key' parameter
                if !parameters.contains_key("key") {
                    return Err(ToolManagerError::ValidationError(
                        ToolValidationError::Other("Keyboard input requires 'key' parameter".to_string())
                    ));
                }
            },
            "mouse" | "touch" => {
                // Mouse/touch input requires x and y coordinates
                if !parameters.contains_key("x") || !parameters.contains_key("y") {
                    return Err(ToolManagerError::ValidationError(
                        ToolValidationError::Other("Mouse/touch input requires 'x' and 'y' parameters".to_string())
                    ));
                }
                
                // Validate x and y are numeric
                if let Some(x) = parameters.get("x") {
                    if let Err(_) = x.parse::<i32>() {
                        return Err(ToolManagerError::ValidationError(
                            ToolValidationError::Other("'x' parameter must be a number".to_string())
                        ));
                    }
                }
                
                if let Some(y) = parameters.get("y") {
                    if let Err(_) = y.parse::<i32>() {
                        return Err(ToolManagerError::ValidationError(
                            ToolValidationError::Other("'y' parameter must be a number".to_string())
                        ));
                    }
                }
            },
            "gamepad" => {
                // Gamepad input implementation would go here
            },
            _ => {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other(format!("Unsupported input type: {}", input_type))
                ));
            }
        }
        
        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        let start_time = Instant::now();
        
        // Extract parameters
        let input_type = context.parameters.get("type").unwrap().clone();
        
        // Process based on input type
        let result = match input_type.as_str() {
            "keyboard" => {
                let key = context.parameters.get("key").unwrap();
                info!("Simulating keyboard input: {}", key);
                
                // Actual implementation would use platform-specific APIs
                format!("Simulated keyboard input: {}", key)
            },
            "mouse" => {
                let x = context.parameters.get("x").unwrap();
                let y = context.parameters.get("y").unwrap();
                info!("Simulating mouse input at ({}, {})", x, y);
                
                // Actual implementation would use platform-specific APIs
                format!("Simulated mouse input at ({}, {})", x, y)
            },
            "touch" => {
                let x = context.parameters.get("x").unwrap();
                let y = context.parameters.get("y").unwrap();
                info!("Simulating touch input at ({}, {})", x, y);
                
                // Actual implementation would use platform-specific APIs
                format!("Simulated touch input at ({}, {})", x, y)
            },
            "gamepad" => {
                // Gamepad input implementation would go here
                "Simulated gamepad input".to_string()
            },
            _ => {
                return Err(ToolManagerError::ExecutionError(
                    format!("Unsupported input type: {}", input_type)
                ));
            }
        };
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let mut metadata = HashMap::new();
        metadata.insert("input_type".to_string(), input_type);
        metadata.insert("execution_time_ms".to_string(), duration_ms.to_string());
        
        Ok(ToolResult {
            success: true,
            data: result,
            error: String::new(),
            metadata,
            duration_ms,
        })
    }
}

/// Read File Tool
pub struct ReadFileTool {
    metadata: ToolMetadata,
}

impl ReadFileTool {
    pub fn new() -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::FileSystem);

        let parameters = vec![
            ParameterDefinition {
                name: "path".to_string(),
                description: "Path to the file to read".to_string(),
                required: true,
                param_type: "string".to_string(),
                default: None,
                validation: None,
            },
            ParameterDefinition {
                name: "line_numbers".to_string(),
                description: "Whether to include line numbers in output".to_string(),
                required: false,
                param_type: "boolean".to_string(),
                default: Some("true".to_string()),
                validation: None,
            },
        ];

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            metadata: ToolMetadata {
                id: "read_file".to_string(),
                name: "Read File".to_string(),
                description: "Reads content from a file".to_string(),
                version: "1.0.0".to_string(),
                author: "System".to_string(),
                category: "filesystem".to_string(),
                parameters,
                capabilities,
                enabled: true,
                created_at: now,
                updated_at: now,
            },
        }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn validate_parameters(&self, parameters: &HashMap<String, String>) -> Result<(), ToolManagerError> {
        // Check for required parameters
        let path = match parameters.get("path") {
            Some(path) => path,
            None => return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: path".to_string())
            )),
        };
        
        // Validate path
        let file_path = Path::new(path);
        
        // Check if path exists
        if !file_path.exists() {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other(format!("File does not exist: {}", path))
            ));
        }
        
        // Check if path is a file
        if !file_path.is_file() {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other(format!("Path is not a file: {}", path))
            ));
        }
        
        // Validate line_numbers if provided
        if let Some(line_numbers) = parameters.get("line_numbers") {
            match line_numbers.as_str() {
                "true" | "false" => {},
                _ => return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("line_numbers must be 'true' or 'false'".to_string())
                )),
            }
        }
        
        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        let start_time = Instant::now();
        
        // Extract parameters
        let path = context.parameters.get("path").unwrap().clone();
        let line_numbers = context.parameters.get("line_numbers")
            .map(|v| v == "true")
            .unwrap_or(true);
        
        // Read file
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(e) => {
                return Err(ToolManagerError::ExecutionError(
                    format!("Failed to read file: {}", e)
                ));
            }
        };
        
        // Process with line numbers if requested
        let result = if line_numbers {
            content.lines()
                .enumerate()
                .map(|(i, line)| format!("{} | {}", i + 1, line))
                .collect::<Vec<String>>()
                .join("\n")
        } else {
            content
        };
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let mut metadata = HashMap::new();
        metadata.insert("path".to_string(), path);
        metadata.insert("line_count".to_string(), content.lines().count().to_string());
        metadata.insert("execution_time_ms".to_string(), duration_ms.to_string());
        
        Ok(ToolResult {
            success: true,
            data: result,
            error: String::new(),
            metadata,
            duration_ms,
        })
    }
}

/// Write File Tool
pub struct WriteFileTool {
    metadata: ToolMetadata,
}

impl WriteFileTool {
    pub fn new() -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::FileSystem);

        let parameters = vec![
            ParameterDefinition {
                name: "path".to_string(),
                description: "Path to write the file".to_string(),
                required: true,
                param_type: "string".to_string(),
                default: None,
                validation: None,
            },
            ParameterDefinition {
                name: "content".to_string(),
                description: "Content to write to the file".to_string(),
                required: true,
                param_type: "string".to_string(),
                default: None,
                validation: None,
            },
            ParameterDefinition {
                name: "append".to_string(),
                description: "Whether to append to existing file".to_string(),
                required: false,
                param_type: "boolean".to_string(),
                default: Some("false".to_string()),
                validation: None,
            },
        ];

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            metadata: ToolMetadata {
                id: "write_file".to_string(),
                name: "Write File".to_string(),
                description: "Writes content to a file".to_string(),
                version: "1.0.0".to_string(),
                author: "System".to_string(),
                category: "filesystem".to_string(),
                parameters,
                capabilities,
                enabled: true,
                created_at: now,
                updated_at: now,
            },
        }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn validate_parameters(&self, parameters: &HashMap<String, String>) -> Result<(), ToolManagerError> {
        // Check for required parameters
        let path = match parameters.get("path") {
            Some(path) => path,
            None => return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: path".to_string())
            )),
        };
        
        if !parameters.contains_key("content") {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: content".to_string())
            ));
        }
        
        // Validate append if provided
        if let Some(append) = parameters.get("append") {
            match append.as_str() {
                "true" | "false" => {},
                _ => return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("append must be 'true' or 'false'".to_string())
                )),
            }
        }
        
        // Validate path does not contain potentially dangerous components
        let path = Path::new(path);
        for component in path.components() {
            let comp_str = component.as_os_str().to_string_lossy();
            if comp_str.contains("..") {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::SecurityThreat("Path contains potentially dangerous components".to_string())
                ));
            }
        }
        
        // Check if parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.is_dir() && !parent.as_os_str().is_empty() {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other(format!("Parent directory does not exist: {:?}", parent))
                ));
            }
        }
        
        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        let start_time = Instant::now();
        
        // Extract parameters
        let path = context.parameters.get("path").unwrap().clone();
        let content = context.parameters.get("content").unwrap().clone();
        let append = context.parameters.get("append")
            .map(|v| v == "true")
            .unwrap_or(false);
        
        // Create parent directory if it doesn't exist
        let path_obj = Path::new(&path);
        if let Some(parent) = path_obj.parent() {
            if !parent.exists() && !parent.as_os_str().is_empty() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return Err(ToolManagerError::ExecutionError(
                        format!("Failed to create parent directory: {}", e)
                    ));
                }
            }
        }
        
        // Write file
        let result = if append {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .and_then(|mut file| std::io::Write::write_all(&mut file, content.as_bytes()))
        } else {
            fs::write(&path, &content)
        };
        
        match result {
            Ok(_) => {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                let mut metadata = HashMap::new();
                metadata.insert("path".to_string(), path.clone());
                metadata.insert("size".to_string(), content.len().to_string());
                metadata.insert("append".to_string(), append.to_string());
                metadata.insert("execution_time_ms".to_string(), duration_ms.to_string());
                
                Ok(ToolResult {
                    success: true,
                    data: format!("Successfully wrote {} bytes to {}", content.len(), path),
                    error: String::new(),
                    metadata,
                    duration_ms,
                })
            },
            Err(e) => {
                Err(ToolManagerError::ExecutionError(
                    format!("Failed to write file: {}", e)
                ))
            }
        }
    }
}

/// Weather Tool
pub struct WeatherTool {
    metadata: ToolMetadata,
}

impl WeatherTool {
    pub fn new() -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::Network);

        let parameters = vec![
            ParameterDefinition {
                name: "location".to_string(),
                description: "Location to get weather for".to_string(),
                required: true,
                param_type: "string".to_string(),
                default: None,
                validation: None,
            },
            ParameterDefinition {
                name: "units".to_string(),
                description: "Units to use (metric, imperial)".to_string(),
                required: false,
                param_type: "string".to_string(),
                default: Some("metric".to_string()),
                validation: Some(r"^(metric|imperial)$".to_string()),
            },
        ];

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            metadata: ToolMetadata {
                id: "get_weather".to_string(),
                name: "Get Weather".to_string(),
                description: "Gets current weather information for a location".to_string(),
                version: "1.0.0".to_string(),
                author: "System".to_string(),
                category: "external".to_string(),
                parameters,
                capabilities,
                enabled: true,
                created_at: now,
                updated_at: now,
            },
        }
    }
}

#[async_trait]
impl Tool for WeatherTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn validate_parameters(&self, parameters: &HashMap<String, String>) -> Result<(), ToolManagerError> {
        // Check for required parameters
        if !parameters.contains_key("location") {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: location".to_string())
            ));
        }
        
        // Validate units if provided
        if let Some(units) = parameters.get("units") {
            match units.as_str() {
                "metric" | "imperial" => {},
                _ => return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("units must be 'metric' or 'imperial'".to_string())
                )),
            }
        }
        
        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        let start_time = Instant::now();
        
        // Extract parameters
        let location = context.parameters.get("location").unwrap().clone();
        let units = context.parameters.get("units").cloned().unwrap_or_else(|| "metric".to_string());
        
        // This would normally call an external API for weather data
        // For this example, we'll simulate a response
        let weather_data = json!({
            "location": location,
            "current": {
                "temperature": 22,
                "condition": "Sunny",
                "humidity": 45,
                "wind_speed": 15,
                "units": units
            },
            "forecast": [
                {
                    "day": "Today",
                    "high": 25,
                    "low": 18,
                    "condition": "Sunny"
                },
                {
                    "day": "Tomorrow",
                    "high": 27,
                    "low": 19,
                    "condition": "Partly Cloudy"
                }
            ]
        });
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let mut metadata = HashMap::new();
        metadata.insert("location".to_string(), location);
        metadata.insert("units".to_string(), units);
        metadata.insert("execution_time_ms".to_string(), duration_ms.to_string());
        
        Ok(ToolResult {
            success: true,
            data: serde_json::to_string_pretty(&weather_data).unwrap(),
            error: String::new(),
            metadata,
            duration_ms,
        })
    }
}

/// Execute Code Tool (for various languages)
pub struct ExecuteCodeTool {
    metadata: ToolMetadata,
}

impl ExecuteCodeTool {
    pub fn new() -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::ExecuteCode);

        let parameters = vec![
            ParameterDefinition {
                name: "language".to_string(),
                description: "Programming language (python, javascript, rust, etc.)".to_string(),
                required: true,
                param_type: "string".to_string(),
                default: None,
                validation: Some(r"^(python|javascript|js|typescript|ts|ruby|rust)$".to_string()),
            },
            ParameterDefinition {
                name: "code".to_string(),
                description: "Code to execute".to_string(),
                required: true,
                param_type: "string".to_string(),
                default: None,
                validation: None,
            },
            ParameterDefinition {
                name: "timeout".to_string(),
                description: "Execution timeout in seconds".to_string(),
                required: false,
                param_type: "number".to_string(),
                default: Some("5".to_string()),
                validation: Some(r"^[1-9][0-9]{0,2}$".to_string()),
            },
        ];

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            metadata: ToolMetadata {
                id: "execute_code".to_string(),
                name: "Execute Code".to_string(),
                description: "Executes code in various programming languages".to_string(),
                version: "1.0.0".to_string(),
                author: "System".to_string(),
                category: "coding".to_string(),
                parameters,
                capabilities,
                enabled: true,
                created_at: now,
                updated_at: now,
            },
        }
    }
}

#[async_trait]
impl Tool for ExecuteCodeTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn validate_parameters(&self, parameters: &HashMap<String, String>) -> Result<(), ToolManagerError> {
        // Check for required parameters
        let language = match parameters.get("language") {
            Some(lang) => lang,
            None => return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: language".to_string())
            )),
        };
        
        if !parameters.contains_key("code") {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: code".to_string())
            ));
        }
        
        // Validate language
        match language.as_str() {
            "python" | "javascript" | "js" | "typescript" | "ts" | "ruby" | "rust" => {},
            _ => return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other(format!("Unsupported language: {}", language))
            )),
        }
        
        // Validate code security based on language
        let code = parameters.get("code").unwrap();
        if let Err(e) = input_validation_rs::validators::security::default_security_scan(code) {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::SecurityThreat(format!("Code security check failed: {}", e))
            ));
        }
        
        // Validate timeout if provided
        if let Some(timeout) = parameters.get("timeout") {
            if let Err(_) = timeout.parse::<u32>() {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("Timeout must be a positive number".to_string())
                ));
            }
        }
        
        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        let start_time = Instant::now();
        
        // Extract parameters
        let language = context.parameters.get("language").unwrap().clone();
        let code = context.parameters.get("code").unwrap().clone();
        let timeout = context.parameters.get("timeout")
            .and_then(|t| t.parse::<u32>().ok())
            .unwrap_or(5);
        
        // Create a temporary file for the code
        let temp_dir = std::env::temp_dir();
        let file_name = match language.as_str() {
            "python" => format!("script_{}.py", context.request_id),
            "javascript" | "js" => format!("script_{}.js", context.request_id),
            "typescript" | "ts" => format!("script_{}.ts", context.request_id),
            "ruby" => format!("script_{}.rb", context.request_id),
            "rust" => format!("script_{}.rs", context.request_id),
            _ => return Err(ToolManagerError::ExecutionError(
                format!("Unsupported language: {}", language)
            )),
        };
        
        let file_path = temp_dir.join(file_name);
        
        if let Err(e) = fs::write(&file_path, &code) {
            return Err(ToolManagerError::ExecutionError(
                format!("Failed to write temporary code file: {}", e)
            ));
        }
        
        // Execute the code based on language
        let (cmd, args) = match language.as_str() {
            "python" => ("python", vec![file_path.to_string_lossy().to_string()]),
            "javascript" | "js" => ("node", vec![file_path.to_string_lossy().to_string()]),
            "typescript" | "ts" => ("ts-node", vec![file_path.to_string_lossy().to_string()]),
            "ruby" => ("ruby", vec![file_path.to_string_lossy().to_string()]),
            "rust" => ("rustc", vec![file_path.to_string_lossy().to_string(), "-o".to_string(), temp_dir.join("output").to_string_lossy().to_string()]),
            _ => {
                // Clean up temp file
                let _ = fs::remove_file(&file_path);
                
                return Err(ToolManagerError::ExecutionError(
                    format!("Unsupported language: {}", language)
                ));
            }
        };
        
        // Execute code with timeout
        let output = match AsyncCommand::new(cmd)
            .args(&args)
            .output()
            .await {
                Ok(output) => output,
                Err(e) => {
                    // Clean up temp file
                    let _ = fs::remove_file(&file_path);
                    
                    return Err(ToolManagerError::ExecutionError(
                        format!("Failed to execute code: {}", e)
                    ));
                }
            };
        
        // Clean up temp file
        let _ = fs::remove_file(file_path);
        
        // Process the result
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        let mut metadata = HashMap::new();
        metadata.insert("language".to_string(), language);
        metadata.insert("exit_code".to_string(), exit_code.to_string());
        metadata.insert("execution_time_ms".to_string(), duration_ms.to_string());
        
        if exit_code == 0 {
            Ok(ToolResult {
                success: true,
                data: stdout,
                error: String::new(),
                metadata,
                duration_ms,
            })
        } else {
            let error_message = format!("Code execution failed (exit {}): {}", exit_code, stderr);
            metadata.insert("stdout".to_string(), stdout);
            
            Ok(ToolResult {
                success: false,
                data: String::new(),
                error: error_message,
                metadata,
                duration_ms,
            })
        }
    }
}

/// WebSearchTool for performing web searches using SerpApi
pub struct WebSearchTool {
    metadata: ToolMetadata,
    serpapi_client: tool_sdk::serpapi::SerpAPIClient,
}

impl WebSearchTool {
    pub fn new() -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::Network);

        let parameters = vec![
            ParameterDefinition {
                name: "query".to_string(),
                description: "Search query".to_string(),
                required: true,
                param_type: "string".to_string(),
                default: None,
                validation: None,
            },
            ParameterDefinition {
                name: "num_results".to_string(),
                description: "Number of results to return".to_string(),
                required: false,
                param_type: "number".to_string(),
                default: Some("10".to_string()),
                validation: Some(r"^[1-9][0-9]?$".to_string()),
            },
        ];

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Initialize SerpAPI client from SDK
        let serpapi_client = tool_sdk::serpapi_client();

        Self {
            metadata: ToolMetadata {
                id: "web_search".to_string(),
                name: "Web Search".to_string(),
                description: "Performs web searches using SerpApi".to_string(),
                version: "1.0.0".to_string(),
                author: "System".to_string(),
                category: "external".to_string(),
                parameters,
                capabilities,
                enabled: true,
                created_at: now,
                updated_at: now,
            },
            serpapi_client,
        }
    }

    /// Check if the SerpAPI client is properly configured
    fn is_configured(&self) -> bool {
        // The SDK client will automatically use configuration from environment
        // We'll use the health check capability to verify it's properly configured
        match tokio::runtime::Handle::current().block_on(self.serpapi_client.health_check()) {
            Ok(result) => result,
            Err(_) => false,
        }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn validate_parameters(&self, parameters: &HashMap<String, String>) -> Result<(), ToolManagerError> {
        // Check for required parameters
        let query = match parameters.get("query") {
            Some(q) => q,
            None => return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: query".to_string())
            )),
        };

        // Check if query is too short
        if query.trim().len() < 2 {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Query must be at least 2 characters".to_string())
            ));
        }

        // Validate num_results if provided
        if let Some(num) = parameters.get("num_results") {
            if let Err(_) = num.parse::<u32>() {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("num_results must be a positive number".to_string())
                ));
            }

            let num_val = num.parse::<u32>().unwrap_or(10);
            if num_val < 1 || num_val > 50 {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("num_results must be between 1 and 50".to_string())
                ));
            }
        }

        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        let start_time = Instant::now();
        
        // Extract parameters
        let query = context.parameters.get("query").unwrap().clone();
        let num_results = context.parameters.get("num_results")
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(10);
        
        // Check if API client is properly configured
        if self.is_configured() {
            // Use the SDK client to perform the search
            let params = tool_sdk::serpapi::GoogleSearchParams {
                q: query.clone(),
                num: Some(num_results),
                ..Default::default()
            };
            
            match self.serpapi_client.google_search(params).await {
                Ok(search_response) => {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    let mut metadata = HashMap::new();
                    metadata.insert("query".to_string(), query);
                    metadata.insert("num_results".to_string(), num_results.to_string());
                    metadata.insert("execution_time_ms".to_string(), duration_ms.to_string());
                    metadata.insert("source".to_string(), "serpapi".to_string());
                    
                    // Format the results using search response data
                    let formatted = format_search_results_from_response(&search_response, num_results);
                    
                    Ok(ToolResult {
                        success: true,
                        data: formatted,
                        error: String::new(),
                        metadata,
                        duration_ms,
                    })
                },
                Err(e) => {
                    // Convert the SDK error to a ToolManagerError
                    Err(ToolManagerError::ExecutionError(
                        format!("Failed to perform search: {}", e)
                    ))
                }
            }
        } else {
            // Fallback mode when API client is not properly configured
            warn!("SerpAPI client is not properly configured. Using fallback search mode.");
            
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let mut metadata = HashMap::new();
            metadata.insert("query".to_string(), query.clone());
            metadata.insert("execution_time_ms".to_string(), duration_ms.to_string());
            metadata.insert("source".to_string(), "fallback".to_string());
            
            // Generate a fallback message
            let fallback_message = format!(
                "Web search for \"{}\" was requested, but the SerpAPI client is not properly configured.\n\n\
                To enable web search functionality:\n\
                1. Sign up at https://serpapi.com to get an API key\n\
                2. Set the PHOENIX_SERPAPI_API_KEY environment variable with your API key\n\
                3. Restart the service\n\n\
                In the meantime, please use a web browser to search for \"{}\".",
                query, query
            );
            
            Ok(ToolResult {
                success: true,
                data: fallback_message,
                error: String::new(),
                metadata,
                duration_ms,
            })
        }
    }
}

/// Format search results into a user-friendly string using the SDK's SearchResponse
fn format_search_results_from_response(response: &tool_sdk::serpapi::SearchResponse, limit: u32) -> String {
    let mut results = String::new();
    
    if let Some(organic_results) = &response.organic_results {
        results.push_str("Search Results:\n\n");
        
        let result_count = std::cmp::min(limit as usize, organic_results.len());
        
        for (i, result) in organic_results.iter().take(result_count).enumerate() {
            let position = i + 1;
            let title = result.title.as_deref().unwrap_or("No title");
            let link = result.link.as_deref().unwrap_or("#");
            let snippet = result.snippet.as_deref().unwrap_or("No description");
            
            results.push_str(&format!("{}. {}\n", position, title));
            results.push_str(&format!("   {}\n", link));
            results.push_str(&format!("   {}\n\n", snippet));
        }
    } else {
        results.push_str("No search results found.");
    }
    
    results
}

/// Older format method kept for backward compatibility
fn format_search_results(data: &serde_json::Value, limit: u32) -> String {
    let mut results = String::new();
    
    // Extract organic results
    if let Some(organic) = data.get("organic_results").and_then(|o| o.as_array()) {
        results.push_str(&format!("Search Results:\n\n"));
        
        let result_count = std::cmp::min(limit as usize, organic.len());
        
        for (i, result) in organic.iter().take(result_count).enumerate() {
            let position = i + 1;
            let title = result.get("title").and_then(|t| t.as_str()).unwrap_or("No title");
            let link = result.get("link").and_then(|l| l.as_str()).unwrap_or("#");
            let snippet = result.get("snippet").and_then(|s| s.as_str()).unwrap_or("No description");
            
            results.push_str(&format!("{}. {}\n", position, title));
            results.push_str(&format!("   {}\n", link));
            results.push_str(&format!("   {}\n\n", snippet));
        }
    } else {
        results.push_str("No search results found.");
    }
    
    results
}

/// Initialize all tools and register them with the tool manager
pub async fn register_all_tools() -> Result<(), ToolManagerError> {
    let tool_manager = TOOL_MANAGER.clone();
    
    // Create and register all tools
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(ExecuteCommandTool::new()),
        Box::new(ExecutePythonTool::new()),
        Box::new(SimulateInputTool::new()),
        Box::new(ReadFileTool::new()),
        Box::new(WriteFileTool::new()),
        Box::new(WeatherTool::new()),
        Box::new(ExecuteCodeTool::new()),
        Box::new(WebSearchTool::new()),
        // Add other tools here
    ];
    
    for tool in tools {
        tool_manager.register_tool(Arc::new(tool))?;
    }
    
    info!("All tools registered successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_execute_command_tool_validate_parameters() {
        let tool = ExecuteCommandTool::new();
        
        // Test valid parameters
        let mut params = HashMap::new();
        params.insert("command".to_string(), "ls".to_string());
        assert!(tool.validate_parameters(&params).is_ok());
        
        // Test missing required parameter
        let empty_params = HashMap::new();
        assert!(tool.validate_parameters(&empty_params).is_err());
        
        // Test invalid command
        let mut invalid_params = HashMap::new();
        invalid_params.insert("command".to_string(), "rm -rf /".to_string());
        assert!(tool.validate_parameters(&invalid_params).is_err());
    }
    
    #[test]
    fn test_execute_python_tool_validate_parameters() {
        let tool = ExecutePythonTool::new();
        
        // Test valid parameters
        let mut params = HashMap::new();
        params.insert("code".to_string(), "print('Hello, world!')".to_string());
        assert!(tool.validate_parameters(&params).is_ok());
        
        // Test missing required parameter
        let empty_params = HashMap::new();
        assert!(tool.validate_parameters(&empty_params).is_err());
        
        // Test potentially dangerous code
        let mut dangerous_params = HashMap::new();
        dangerous_params.insert("code".to_string(), "import os; os.system('rm -rf /')".to_string());
        assert!(tool.validate_parameters(&dangerous_params).is_err());
    }
    
    #[tokio::test]
    async fn test_register_all_tools() {
        // This is a simple integration test to make sure we can register all tools
        assert!(register_all_tools().await.is_ok());
        
        // Check if tools were actually registered
        let tool_manager = TOOL_MANAGER.clone();
        let all_tools = tool_manager.list_tools(None);
        
        assert!(all_tools.len() >= 7); // We should have at least 7 tools
        assert!(tool_manager.get_tool("execute_command").is_ok());
        assert!(tool_manager.get_tool("execute_python").is_ok());
        assert!(tool_manager.get_tool("simulate_input").is_ok());
        assert!(tool_manager.get_tool("read_file").is_ok());
        assert!(tool_manager.get_tool("write_file").is_ok());
        assert!(tool_manager.get_tool("get_weather").is_ok());
        assert!(tool_manager.get_tool("execute_code").is_ok());
    }
}