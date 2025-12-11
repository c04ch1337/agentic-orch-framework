//! Tool Implementations
//!
//! General-purpose tools exposed by tools-service-rs:
//! - web_search
//! - execute_code
//! - read_file
//! - write_file

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use log::{info, warn};

use crate::tool_manager::{
    Capability, ParameterDefinition, Tool, ToolContext, ToolManagerError, ToolMetadata, ToolResult,
    TOOL_MANAGER,
};

use crate::validation::ToolValidationError;

/// Read File Tool
pub struct ReadFileTool {
    metadata: ToolMetadata,
}

impl ReadFileTool {
    pub fn new() -> Self {
        let mut capabilities = std::collections::HashSet::new();
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

    fn validate_parameters(
        &self,
        parameters: &HashMap<String, String>,
    ) -> Result<(), ToolManagerError> {
        let path = match parameters.get("path") {
            Some(path) => path,
            None => {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("Missing required parameter: path".to_string()),
                ));
            }
        };

        let file_path = Path::new(path);

        if !file_path.exists() {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other(format!("File does not exist: {}", path)),
            ));
        }

        if !file_path.is_file() {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other(format!("Path is not a file: {}", path)),
            ));
        }

        if let Some(line_numbers) = parameters.get("line_numbers") {
            match line_numbers.as_str() {
                "true" | "false" => {}
                _ => {
                    return Err(ToolManagerError::ValidationError(
                        ToolValidationError::Other(
                            "line_numbers must be 'true' or 'false'".to_string(),
                        ),
                    ));
                }
            }
        }

        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        let start_time = Instant::now();

        let path = context.parameters.get("path").unwrap().clone();
        let line_numbers = context
            .parameters
            .get("line_numbers")
            .map(|v| v == "true")
            .unwrap_or(true);

        let content = fs::read_to_string(&path)
            .map_err(|e| ToolManagerError::ExecutionError(format!("Failed to read file: {}", e)))?;

        let result = if line_numbers {
            content
                .lines()
                .enumerate()
                .map(|(i, line)| format!("{} | {}", i + 1, line))
                .collect::<Vec<String>>()
                .join("\n")
        } else {
            content.clone()
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let mut metadata = HashMap::new();
        metadata.insert("path".to_string(), path);
        metadata.insert(
            "line_count".to_string(),
            content.lines().count().to_string(),
        );
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
        let mut capabilities = std::collections::HashSet::new();
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

    fn validate_parameters(
        &self,
        parameters: &HashMap<String, String>,
    ) -> Result<(), ToolManagerError> {
        let path = match parameters.get("path") {
            Some(path) => path,
            None => {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("Missing required parameter: path".to_string()),
                ));
            }
        };

        if !parameters.contains_key("content") {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: content".to_string()),
            ));
        }

        if let Some(append) = parameters.get("append") {
            match append.as_str() {
                "true" | "false" => {}
                _ => {
                    return Err(ToolManagerError::ValidationError(
                        ToolValidationError::Other("append must be 'true' or 'false'".to_string()),
                    ));
                }
            }
        }

        let path_obj = Path::new(path);
        for component in path_obj.components() {
            let comp_str = component.as_os_str().to_string_lossy();
            if comp_str.contains("..") {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::SecurityThreat(
                        "Path contains potentially dangerous components".to_string(),
                    ),
                ));
            }
        }

        if let Some(parent) = path_obj.parent() {
            if !parent.is_dir() && !parent.as_os_str().is_empty() {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other(format!(
                        "Parent directory does not exist: {:?}",
                        parent
                    )),
                ));
            }
        }

        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        let start_time = Instant::now();

        let path = context.parameters.get("path").unwrap().clone();
        let content = context.parameters.get("content").unwrap().clone();
        let append = context
            .parameters
            .get("append")
            .map(|v| v == "true")
            .unwrap_or(false);

        let path_obj = Path::new(&path);
        if let Some(parent) = path_obj.parent() {
            if !parent.exists() && !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| {
                    ToolManagerError::ExecutionError(format!(
                        "Failed to create parent directory: {}",
                        e
                    ))
                })?;
            }
        }

        if append {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .and_then(|mut file| std::io::Write::write_all(&mut file, content.as_bytes()))
                .map_err(|e| {
                    ToolManagerError::ExecutionError(format!("Failed to write file: {}", e))
                })?;
        } else {
            fs::write(&path, &content).map_err(|e| {
                ToolManagerError::ExecutionError(format!("Failed to write file: {}", e))
            })?;
        }

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
    }
}

/// Execute Code Tool (for various languages)
pub struct ExecuteCodeTool {
    metadata: ToolMetadata,
}

impl ExecuteCodeTool {
    pub fn new() -> Self {
        let mut capabilities = std::collections::HashSet::new();
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

    fn validate_parameters(
        &self,
        parameters: &HashMap<String, String>,
    ) -> Result<(), ToolManagerError> {
        let language = match parameters.get("language") {
            Some(lang) => lang,
            None => {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("Missing required parameter: language".to_string()),
                ));
            }
        };

        if !parameters.contains_key("code") {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Missing required parameter: code".to_string()),
            ));
        }

        match language.as_str() {
            "python" | "javascript" | "js" | "typescript" | "ts" | "ruby" | "rust" => {}
            _ => {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other(format!("Unsupported language: {}", language)),
                ));
            }
        }

        let code = parameters.get("code").unwrap();
        if let Err(e) = input_validation_rs::validators::security::default_security_scan(code) {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::SecurityThreat(format!("Code security check failed: {}", e)),
            ));
        }

        if let Some(timeout) = parameters.get("timeout") {
            if timeout.parse::<u32>().is_err() {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("Timeout must be a positive number".to_string()),
                ));
            }
        }

        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        use tokio::process::Command as AsyncCommand;

        let start_time = Instant::now();

        let language = context.parameters.get("language").unwrap().clone();
        let code = context.parameters.get("code").unwrap().clone();
        let _timeout = context
            .parameters
            .get("timeout")
            .and_then(|t| t.parse::<u32>().ok())
            .unwrap_or(5);

        let temp_dir = std::env::temp_dir();
        let file_name = match language.as_str() {
            "python" => format!("script_{}.py", context.request_id),
            "javascript" | "js" => format!("script_{}.js", context.request_id),
            "typescript" | "ts" => format!("script_{}.ts", context.request_id),
            "ruby" => format!("script_{}.rb", context.request_id),
            "rust" => format!("script_{}.rs", context.request_id),
            _ => {
                return Err(ToolManagerError::ExecutionError(format!(
                    "Unsupported language: {}",
                    language
                )));
            }
        };

        let file_path = temp_dir.join(file_name);
        fs::write(&file_path, &code).map_err(|e| {
            ToolManagerError::ExecutionError(format!("Failed to write temporary code file: {}", e))
        })?;

        let (cmd, args) = match language.as_str() {
            "python" => ("python", vec![file_path.to_string_lossy().to_string()]),
            "javascript" | "js" => ("node", vec![file_path.to_string_lossy().to_string()]),
            "typescript" | "ts" => ("ts-node", vec![file_path.to_string_lossy().to_string()]),
            "ruby" => ("ruby", vec![file_path.to_string_lossy().to_string()]),
            "rust" => (
                "rustc",
                vec![
                    file_path.to_string_lossy().to_string(),
                    "-o".to_string(),
                    temp_dir.join("output").to_string_lossy().to_string(),
                ],
            ),
            _ => {
                let _ = fs::remove_file(&file_path);
                return Err(ToolManagerError::ExecutionError(format!(
                    "Unsupported language: {}",
                    language
                )));
            }
        };

        let output = AsyncCommand::new(cmd)
            .args(&args)
            .output()
            .await
            .map_err(|e| {
                let _ = fs::remove_file(&file_path);
                ToolManagerError::ExecutionError(format!("Failed to execute code: {}", e))
            })?;

        let _ = fs::remove_file(&file_path);

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
        let mut capabilities = std::collections::HashSet::new();
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

    fn is_configured(&self) -> bool {
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

    fn validate_parameters(
        &self,
        parameters: &HashMap<String, String>,
    ) -> Result<(), ToolManagerError> {
        let query = match parameters.get("query") {
            Some(q) => q,
            None => {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("Missing required parameter: query".to_string()),
                ));
            }
        };

        if query.trim().len() < 2 {
            return Err(ToolManagerError::ValidationError(
                ToolValidationError::Other("Query must be at least 2 characters".to_string()),
            ));
        }

        if let Some(num) = parameters.get("num_results") {
            if num.parse::<u32>().is_err() {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("num_results must be a positive number".to_string()),
                ));
            }

            let num_val = num.parse::<u32>().unwrap_or(10);
            if num_val < 1 || num_val > 50 {
                return Err(ToolManagerError::ValidationError(
                    ToolValidationError::Other("num_results must be between 1 and 50".to_string()),
                ));
            }
        }

        Ok(())
    }

    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
        let start_time = Instant::now();

        let query = context.parameters.get("query").unwrap().clone();
        let num_results = context
            .parameters
            .get("num_results")
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(10);

        if self.is_configured() {
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

                    let formatted =
                        format_search_results_from_response(&search_response, num_results);

                    Ok(ToolResult {
                        success: true,
                        data: formatted,
                        error: String::new(),
                        metadata,
                        duration_ms,
                    })
                }
                Err(e) => Err(ToolManagerError::ExecutionError(format!(
                    "Failed to perform search: {}",
                    e
                ))),
            }
        } else {
            warn!("SerpAPI client is not properly configured. Using fallback search mode.");

            let duration_ms = start_time.elapsed().as_millis() as u64;
            let mut metadata = HashMap::new();
            metadata.insert("query".to_string(), query.clone());
            metadata.insert("execution_time_ms".to_string(), duration_ms.to_string());
            metadata.insert("source".to_string(), "fallback".to_string());

            let fallback_message = format!(
                "Web search for \"{}\" was requested, but the SerpAPI client is not configured. Please perform this search manually in a web browser.",
                query
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
fn format_search_results_from_response(
    response: &tool_sdk::serpapi::SearchResponse,
    limit: u32,
) -> String {
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

/// Initialize all tools and register them with the tool manager
pub async fn register_all_tools() -> Result<(), ToolManagerError> {
    let tool_manager = TOOL_MANAGER.clone();

    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(ReadFileTool::new()),
        Box::new(WriteFileTool::new()),
        Box::new(ExecuteCodeTool::new()),
        Box::new(WebSearchTool::new()),
    ];

    for tool in tools {
        tool_manager.register_tool(Arc::new(tool))?;
    }

    info!("General-purpose tools registered successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_all_tools() {
        assert!(register_all_tools().await.is_ok());

        let tool_manager = TOOL_MANAGER.clone();
        let all_tools = tool_manager.list_tools(None);

        assert!(all_tools.len() >= 4);
        assert!(tool_manager.get_tool("read_file").is_ok());
        assert!(tool_manager.get_tool("write_file").is_ok());
        assert!(tool_manager.get_tool("execute_code").is_ok());
        assert!(tool_manager.get_tool("web_search").is_ok());
    }
}
