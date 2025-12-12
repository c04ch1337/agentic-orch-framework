//! Tool Manager Module
//!
//! Provides a central registry for tool registration, discovery, and management.
//! Implements metadata support, version control, and dynamic tool loading functionality.

use async_trait::async_trait;
use log::{debug, error, info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tokio::sync::Mutex as AsyncMutex;

use crate::validation::{validate_command_name, ToolValidationError};

/// Tool capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Can execute system commands
    ExecuteCommand,
    /// Can access filesystem
    FileSystem,
    /// Can make network connections
    Network,
    /// Can execute code in various languages
    ExecuteCode,
    /// Can simulate user input
    SimulateInput,
    /// Can access sensitive data
    AccessSensitiveData,
}

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefinition {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Whether parameter is required
    pub required: bool,
    /// Parameter type (string, number, boolean, etc.)
    pub param_type: String,
    /// Default value, if any
    pub default: Option<String>,
    /// Parameter validation pattern (regex for strings, etc.)
    pub validation: Option<String>,
}

/// Tool metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Unique tool identifier
    pub id: String,
    /// Human-readable tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool version
    pub version: String,
    /// Tool author
    pub author: String,
    /// Tool category
    pub category: String,
    /// Required parameters for this tool
    pub parameters: Vec<ParameterDefinition>,
    /// Tool capabilities required
    pub capabilities: HashSet<Capability>,
    /// Whether the tool is enabled
    pub enabled: bool,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
}

/// Tool Manager Error types
#[derive(Debug, Error)]
pub enum ToolManagerError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool already exists: {0}")]
    ToolAlreadyExists(String),

    #[error("Invalid tool metadata: {0}")]
    InvalidMetadata(String),

    #[error("Missing required capability: {0}")]
    MissingCapability(String),

    #[error("Tool validation error: {0}")]
    ValidationError(#[from] ToolValidationError),

    #[error("Tool loading error: {0}")]
    LoadingError(String),

    #[error("Tool execution error: {0}")]
    ExecutionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("SDK service error: {0}")]
    ServiceError(String),
}

impl From<tool_sdk::error::ServiceError> for ToolManagerError {
    fn from(error: tool_sdk::error::ServiceError) -> Self {
        match error {
            tool_sdk::error::ServiceError::Authentication(msg) => {
                ToolManagerError::ValidationError(ToolValidationError::Other(format!(
                    "Authentication error: {}",
                    msg
                )))
            }
            tool_sdk::error::ServiceError::Authorization(msg) => ToolManagerError::ValidationError(
                ToolValidationError::Other(format!("Authorization error: {}", msg)),
            ),
            tool_sdk::error::ServiceError::RateLimit(msg) => {
                ToolManagerError::ExecutionError(format!("Rate limit exceeded: {}", msg))
            }
            tool_sdk::error::ServiceError::Validation(msg) => {
                ToolManagerError::ValidationError(ToolValidationError::Other(msg))
            }
            tool_sdk::error::ServiceError::NotFound(msg) => {
                ToolManagerError::ExecutionError(format!("Resource not found: {}", msg))
            }
            tool_sdk::error::ServiceError::Network(msg) => {
                ToolManagerError::ExecutionError(format!("Network error: {}", msg))
            }
            tool_sdk::error::ServiceError::Parsing(msg) => {
                ToolManagerError::ExecutionError(format!("Parsing error: {}", msg))
            }
            tool_sdk::error::ServiceError::Configuration(msg) => {
                ToolManagerError::ExecutionError(format!("Configuration error: {}", msg))
            }
            tool_sdk::error::ServiceError::Timeout(msg) => {
                ToolManagerError::ExecutionError(format!("Timeout error: {}", msg))
            }
            tool_sdk::error::ServiceError::CircuitBroken(msg) => {
                ToolManagerError::ExecutionError(format!("Circuit breaker open: {}", msg))
            }
            tool_sdk::error::ServiceError::ExternalService(msg) => {
                ToolManagerError::ExecutionError(format!("External service error: {}", msg))
            }
            tool_sdk::error::ServiceError::Unknown(msg) => {
                ToolManagerError::ExecutionError(format!("Unknown error: {}", msg))
            }
            tool_sdk::error::ServiceError::Service(msg) => {
                ToolManagerError::ExecutionError(format!("Service error: {}", msg))
            }
            tool_sdk::error::ServiceError::Internal(msg) => {
                ToolManagerError::ExecutionError(format!("Internal error: {}", msg))
            }
            tool_sdk::error::ServiceError::WithContext { inner, .. } => {
                ToolManagerError::from(*inner)
            }
        }
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether execution was successful
    pub success: bool,
    /// Result data (if successful)
    pub data: String,
    /// Error message (if unsuccessful)
    pub error: String,
    /// Execution metadata
    pub metadata: HashMap<String, String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Parameters for tool execution
    pub parameters: HashMap<String, String>,
    /// User ID requesting tool execution
    pub user_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Request ID for tracing
    pub request_id: String,
    /// Additional context data
    pub context_data: HashMap<String, String>,
}

/// Tool interface trait
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get tool metadata
    fn metadata(&self) -> &ToolMetadata;

    /// Execute the tool with given parameters
    async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError>;

    /// Validate the tool's parameters
    fn validate_parameters(
        &self,
        parameters: &HashMap<String, String>,
    ) -> Result<(), ToolManagerError>;

    /// Return tool help information
    fn help(&self) -> String {
        let metadata = self.metadata();
        let mut help = format!("Tool: {} ({})\n", metadata.name, metadata.id);
        help.push_str(&format!("Version: {}\n", metadata.version));
        help.push_str(&format!("Description: {}\n", metadata.description));
        help.push_str("Parameters:\n");

        for param in &metadata.parameters {
            let required = if param.required {
                "required"
            } else {
                "optional"
            };
            help.push_str(&format!(
                "  - {} ({}): {}\n",
                param.name, required, param.description
            ));
            if let Some(default) = &param.default {
                help.push_str(&format!("    Default: {}\n", default));
            }
        }

        help
    }
}

/// The central Tool Manager
pub struct ToolManager {
    /// Registry of all available tools
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
    /// Active tool version tracking
    versions: RwLock<HashMap<String, Vec<String>>>,
    /// Permission and capability registry
    capability_registry: RwLock<HashMap<String, HashSet<Capability>>>,
    /// Active user sessions with capability grants
    session_grants: AsyncMutex<HashMap<String, HashSet<Capability>>>,
}

/// Global instance of ToolManager
pub static TOOL_MANAGER: Lazy<Arc<ToolManager>> = Lazy::new(|| Arc::new(ToolManager::new()));

impl ToolManager {
    /// Create a new ToolManager instance
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            versions: RwLock::new(HashMap::new()),
            capability_registry: RwLock::new(HashMap::new()),
            session_grants: AsyncMutex::new(HashMap::new()),
        }
    }

    /// Register a new tool with the manager
    pub fn register_tool(&self, tool: Arc<dyn Tool>) -> Result<(), ToolManagerError> {
        let metadata = tool.metadata();
        let tool_id = metadata.id.clone();
        let version = metadata.version.clone();

        // Check if tool already exists
        {
            let tools = self.tools.read().unwrap();
            if tools.contains_key(&tool_id) {
                return Err(ToolManagerError::ToolAlreadyExists(tool_id));
            }
        }

        // Register tool
        {
            let mut tools = self.tools.write().unwrap();
            let mut versions = self.versions.write().unwrap();

            tools.insert(tool_id.clone(), tool.clone());

            // Update version tracking
            let tool_versions = versions.entry(tool_id.clone()).or_insert_with(Vec::new);
            tool_versions.push(version);

            // Update capability registry
            let mut capability_registry = self.capability_registry.write().unwrap();
            capability_registry.insert(tool_id.clone(), metadata.capabilities.clone());
        }

        info!(
            "Tool {} v{} registered successfully",
            metadata.name, metadata.version
        );
        Ok(())
    }

    /// Unregister a tool from the manager
    pub fn unregister_tool(&self, tool_id: &str) -> Result<(), ToolManagerError> {
        let mut tools = self.tools.write().unwrap();

        if tools.remove(tool_id).is_none() {
            return Err(ToolManagerError::ToolNotFound(tool_id.to_string()));
        }

        // Remove from version tracking
        let mut versions = self.versions.write().unwrap();
        versions.remove(tool_id);

        // Remove from capability registry
        let mut capability_registry = self.capability_registry.write().unwrap();
        capability_registry.remove(tool_id);

        info!("Tool {} unregistered successfully", tool_id);
        Ok(())
    }

    /// Get a tool by ID
    pub fn get_tool(&self, tool_id: &str) -> Result<Arc<dyn Tool>, ToolManagerError> {
        let tools = self.tools.read().unwrap();

        match tools.get(tool_id) {
            Some(tool) => Ok(tool.clone()),
            None => Err(ToolManagerError::ToolNotFound(tool_id.to_string())),
        }
    }

    /// List all registered tools
    pub fn list_tools(&self, category: Option<&str>) -> Vec<ToolMetadata> {
        let tools = self.tools.read().unwrap();

        tools
            .values()
            .map(|tool| tool.metadata().clone())
            .filter(|metadata| {
                // Filter by category if provided
                if let Some(cat) = category {
                    metadata.category == cat
                } else {
                    true
                }
            })
            .filter(|metadata| metadata.enabled)
            .collect()
    }

    /// Check if a tool has the required capabilities
    pub fn check_tool_capabilities(
        &self,
        tool_id: &str,
        required: &HashSet<Capability>,
    ) -> Result<(), ToolManagerError> {
        let capability_registry = self.capability_registry.read().unwrap();

        match capability_registry.get(tool_id) {
            Some(capabilities) => {
                for cap in required {
                    if !capabilities.contains(cap) {
                        return Err(ToolManagerError::MissingCapability(format!("{:?}", cap)));
                    }
                }
                Ok(())
            }
            None => Err(ToolManagerError::ToolNotFound(tool_id.to_string())),
        }
    }

    /// Grant capabilities to a session
    pub async fn grant_session_capabilities(
        &self,
        session_id: &str,
        capabilities: HashSet<Capability>,
    ) {
        let mut session_grants = self.session_grants.lock().await;

        let session_capabilities = session_grants
            .entry(session_id.to_string())
            .or_insert_with(HashSet::new);
        for cap in capabilities {
            session_capabilities.insert(cap);
        }

        debug!(
            "Granted capabilities to session {}: {:?}",
            session_id, session_capabilities
        );
    }

    /// Check if a session has the required capabilities
    pub async fn check_session_capabilities(
        &self,
        session_id: &str,
        required: &HashSet<Capability>,
    ) -> Result<(), ToolManagerError> {
        let session_grants = self.session_grants.lock().await;

        match session_grants.get(session_id) {
            Some(capabilities) => {
                for cap in required {
                    if !capabilities.contains(cap) {
                        return Err(ToolManagerError::MissingCapability(format!("{:?}", cap)));
                    }
                }
                Ok(())
            }
            None => Err(ToolManagerError::MissingCapability(
                "Session has no capabilities".to_string(),
            )),
        }
    }

    /// Execute a tool with the given context
    pub async fn execute_tool(
        &self,
        tool_id: &str,
        context: ToolContext,
    ) -> Result<ToolResult, ToolManagerError> {
        // Get the tool
        let tool = self.get_tool(tool_id)?;

        // Validate parameters
        tool.validate_parameters(&context.parameters)?;

        // Check if session has required capabilities
        if let Some(session_id) = &context.session_id {
            let required_capabilities = tool.metadata().capabilities.clone();
            self.check_session_capabilities(session_id, &required_capabilities)
                .await?;
        }

        // Execute the tool
        let result = tool.execute(context).await?;

        Ok(result)
    }

    /// Load tools dynamically from a directory
    pub fn load_tools_from_directory(
        &self,
        directory: &str,
    ) -> Result<Vec<String>, ToolManagerError> {
        // This would be implemented to dynamically load tool plugins
        // In a real implementation, this would use libloading or similar
        // For this example, we'll just return an error

        Err(ToolManagerError::LoadingError(
            "Dynamic loading not implemented yet".to_string(),
        ))
    }

    /// Get tool versions
    pub fn get_tool_versions(&self, tool_id: &str) -> Result<Vec<String>, ToolManagerError> {
        let versions = self.versions.read().unwrap();

        match versions.get(tool_id) {
            Some(v) => Ok(v.clone()),
            None => Err(ToolManagerError::ToolNotFound(tool_id.to_string())),
        }
    }

    /// Validate a tool's metadata for correctness
    pub fn validate_tool_metadata(&self, metadata: &ToolMetadata) -> Result<(), ToolManagerError> {
        // Check for required fields
        if metadata.id.is_empty() {
            return Err(ToolManagerError::InvalidMetadata(
                "Tool ID cannot be empty".to_string(),
            ));
        }

        if metadata.name.is_empty() {
            return Err(ToolManagerError::InvalidMetadata(
                "Tool name cannot be empty".to_string(),
            ));
        }

        if metadata.version.is_empty() {
            return Err(ToolManagerError::InvalidMetadata(
                "Tool version cannot be empty".to_string(),
            ));
        }

        // Validate version format (should be semver)
        if !metadata.version.contains('.') {
            return Err(ToolManagerError::InvalidMetadata(
                "Tool version should follow semver format".to_string(),
            ));
        }

        // Check parameters
        for param in &metadata.parameters {
            if param.name.is_empty() {
                return Err(ToolManagerError::InvalidMetadata(
                    "Parameter name cannot be empty".to_string(),
                ));
            }

            // If parameter has a validation pattern, check if it's a valid regex
            if let Some(validation) = &param.validation {
                if let Err(_) = regex::Regex::new(validation) {
                    return Err(ToolManagerError::InvalidMetadata(format!(
                        "Invalid validation pattern for parameter {}: {}",
                        param.name, validation
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct MockTool {
        metadata: ToolMetadata,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn metadata(&self) -> &ToolMetadata {
            &self.metadata
        }

        async fn execute(&self, context: ToolContext) -> Result<ToolResult, ToolManagerError> {
            let start = SystemTime::now();
            let since_epoch = start.duration_since(UNIX_EPOCH).unwrap();

            Ok(ToolResult {
                success: true,
                data: "Mock tool executed successfully".to_string(),
                error: String::new(),
                metadata: HashMap::new(),
                duration_ms: 1,
            })
        }

        fn validate_parameters(
            &self,
            parameters: &HashMap<String, String>,
        ) -> Result<(), ToolManagerError> {
            // For testing, just make sure required parameters are present
            for param in &self.metadata.parameters {
                if param.required && !parameters.contains_key(&param.name) {
                    return Err(ToolManagerError::ValidationError(
                        ToolValidationError::Other(format!(
                            "Missing required parameter: {}",
                            param.name
                        )),
                    ));
                }
            }

            Ok(())
        }
    }

    fn create_mock_tool() -> MockTool {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::ExecuteCommand);

        let parameters = vec![ParameterDefinition {
            name: "command".to_string(),
            description: "Command to execute".to_string(),
            required: true,
            param_type: "string".to_string(),
            default: None,
            validation: Some(r"^[a-zA-Z0-9_\-]+$".to_string()),
        }];

        MockTool {
            metadata: ToolMetadata {
                id: "mock_tool".to_string(),
                name: "Mock Tool".to_string(),
                description: "A mock tool for testing".to_string(),
                version: "1.0.0".to_string(),
                author: "Test Author".to_string(),
                category: "testing".to_string(),
                parameters,
                capabilities,
                enabled: true,
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                updated_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            },
        }
    }

    #[test]
    fn test_register_and_get_tool() {
        let manager = ToolManager::new();
        let mock_tool = create_mock_tool();

        // Register the tool
        let result = manager.register_tool(Arc::new(mock_tool));
        assert!(result.is_ok());

        // Get the tool
        let get_result = manager.get_tool("mock_tool");
        assert!(get_result.is_ok());
    }

    #[test]
    fn test_unregister_tool() {
        let manager = ToolManager::new();
        let mock_tool = create_mock_tool();

        // Register the tool
        let register_result = manager.register_tool(Arc::new(mock_tool));
        assert!(register_result.is_ok());

        // Unregister the tool
        let unregister_result = manager.unregister_tool("mock_tool");
        assert!(unregister_result.is_ok());

        // Try to get the tool after unregistering
        let get_result = manager.get_tool("mock_tool");
        assert!(get_result.is_err());
    }

    #[test]
    fn test_list_tools() {
        let manager = ToolManager::new();
        let mock_tool = create_mock_tool();

        // Register the tool
        let register_result = manager.register_tool(Arc::new(mock_tool));
        assert!(register_result.is_ok());

        // List all tools
        let all_tools = manager.list_tools(None);
        assert_eq!(all_tools.len(), 1);

        // List tools in specific category
        let testing_tools = manager.list_tools(Some("testing"));
        assert_eq!(testing_tools.len(), 1);

        // List tools in non-existent category
        let invalid_tools = manager.list_tools(Some("invalid"));
        assert_eq!(invalid_tools.len(), 0);
    }
}
