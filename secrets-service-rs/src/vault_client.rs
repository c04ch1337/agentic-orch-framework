// secrets-service-rs/src/vault_client.rs
// HashiCorp Vault Client Implementation

use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use hashicorp_vault::{client::{VaultClient as Client, VaultClientSettingsBuilder}, error::ClientError};
use serde_json::Value;
use async_trait::async_trait;
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;

// Define error types for Vault operations
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("Client error: {0}")]
    ClientError(#[from] ClientError),
    
    #[error("Secret not found: {0}")]
    SecretNotFound(String),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

// Secret metadata struct
pub struct SecretMetadata {
    pub key: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub expires_at: u64,
    pub metadata: HashMap<String, String>,
}

// Token metadata struct
pub struct TokenMetadata {
    pub token: String,
    pub expires_at: u64,
    pub roles: Vec<String>,
    pub service_id: String,
}

// Define trait for Vault operations to allow for mocking
#[async_trait]
pub trait VaultOperations: Send + Sync {
    async fn get_secret(&self, key: &str, auth_token: &str) -> Result<String, VaultError>;
    async fn set_secret(&self, key: &str, value: &str, ttl: u64, metadata: HashMap<String, String>, auth_token: &str) -> Result<(), VaultError>;
    async fn delete_secret(&self, key: &str, auth_token: &str) -> Result<(), VaultError>;
    async fn list_secrets(&self, path_prefix: &str, auth_token: &str) -> Result<Vec<SecretMetadata>, VaultError>;
    async fn authenticate_service(&self, service_id: &str, service_secret: &str) -> Result<TokenMetadata, VaultError>;
    async fn verify_token(&self, token: &str) -> Result<TokenMetadata, VaultError>;
    async fn is_authorized(&self, token: &str, resource: &str, action: &str) -> Result<bool, VaultError>;
}

// Vault client implementation
pub struct VaultClient {
    client: Option<Client>,
    token_cache: Arc<RwLock<HashMap<String, TokenMetadata>>>,
    vault_addr: String,
    vault_token: String,
}

impl VaultClient {
    /// Create a new Vault client connected to the Vault server
    pub async fn new() -> Result<Self, VaultError> {
        let vault_addr = env::var("VAULT_ADDR")
            .unwrap_or_else(|_| "http://localhost:8200".to_string());
        
        let vault_token = env::var("VAULT_TOKEN").map_err(|_| {
            VaultError::ConfigurationError("VAULT_TOKEN environment variable not set".to_string())
        })?;
        
        // Configure Vault client
        let vault_settings = VaultClientSettingsBuilder::default()
            .address(&vault_addr)
            .token(&vault_token)
            .build()
            .map_err(|e| VaultError::ConfigurationError(e.to_string()))?;
        
        let client = Client::new(vault_settings)
            .map_err(|e| VaultError::ClientError(e))?;
            
        // Test connection to Vault
        client.get_secret("sys/health")
            .map_err(|e| VaultError::ClientError(e))?;
        
        Ok(Self {
            client: Some(client),
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            vault_addr,
            vault_token,
        })
    }
    
    /// Create a new mock Vault client for testing or fallback mode
    pub fn new_mock() -> Self {
        Self {
            client: None,
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            vault_addr: "mock://vault".to_string(),
            vault_token: "mock-token".to_string(),
        }
    }
    
    /// Check if Vault client is in mock mode
    pub fn is_mock(&self) -> bool {
        self.client.is_none()
    }
}

#[async_trait]
impl VaultOperations for VaultClient {
    /// Get a secret by its key
    async fn get_secret(&self, key: &str, auth_token: &str) -> Result<String, VaultError> {
        // Verify the token first
        let token_data = self.verify_token(auth_token).await?;
        
        // Check if token has permission to access this secret
        if !self.is_authorized(auth_token, &format!("secret/{}", key), "read").await? {
            return Err(VaultError::PermissionDenied(
                format!("Token does not have permission to read secret: {}", key)
            ));
        }
        
        // If in mock mode, return error
        if self.is_mock() {
            return Err(VaultError::ConfigurationError(
                "Vault client is running in mock mode".to_string()
            ));
        }
        
        // Get the secret from Vault
        match &self.client {
            Some(client) => {
                let secret_path = format!("secret/data/{}", key);
                match client.get_secret(&secret_path) {
                    Ok(secret) => {
                        // Try to extract the secret value from the response
                        if let Some(data) = secret.get("data") {
                            if let Some(data_obj) = data.as_object() {
                                if let Some(value) = data_obj.get("value") {
                                    if let Some(value_str) = value.as_str() {
                                        return Ok(value_str.to_string());
                                    }
                                }
                            }
                        }
                        Err(VaultError::SecretNotFound(
                            format!("Secret found but value not extractable: {}", key)
                        ))
                    }
                    Err(e) => {
                        if e.to_string().contains("404") {
                            Err(VaultError::SecretNotFound(key.to_string()))
                        } else {
                            Err(VaultError::ClientError(e))
                        }
                    }
                }
            }
            None => Err(VaultError::ConfigurationError(
                "Vault client not initialized".to_string()
            )),
        }
    }
    
    /// Set a secret with a key and value
    async fn set_secret(
        &self, 
        key: &str, 
        value: &str, 
        ttl: u64, 
        metadata: HashMap<String, String>,
        auth_token: &str
    ) -> Result<(), VaultError> {
        // Verify the token first
        let token_data = self.verify_token(auth_token).await?;
        
        // Check if token has permission to write this secret
        if !self.is_authorized(auth_token, &format!("secret/{}", key), "write").await? {
            return Err(VaultError::PermissionDenied(
                format!("Token does not have permission to write secret: {}", key)
            ));
        }
        
        // If in mock mode, return error
        if self.is_mock() {
            return Err(VaultError::ConfigurationError(
                "Vault client is running in mock mode".to_string()
            ));
        }
        
        // Write the secret to Vault
        match &self.client {
            Some(client) => {
                let secret_path = format!("secret/data/{}", key);
                
                // Construct the data object with the secret value and metadata
                let mut data = serde_json::Map::new();
                data.insert("value".to_string(), Value::String(value.to_string()));
                
                // Add metadata
                for (k, v) in metadata {
                    data.insert(k, Value::String(v));
                }
                
                // Construct the complete secret data object
                let mut secret_data = serde_json::Map::new();
                secret_data.insert("data".to_string(), Value::Object(data));
                
                // Add TTL if provided
                if ttl > 0 {
                    let options = {
                        let mut opt_map = serde_json::Map::new();
                        opt_map.insert("ttl".to_string(), Value::String(format!("{}s", ttl)));
                        Value::Object(opt_map)
                    };
                    secret_data.insert("options".to_string(), options);
                }
                
                match client.set_secret(&secret_path, &Value::Object(secret_data)) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(VaultError::ClientError(e)),
                }
            }
            None => Err(VaultError::ConfigurationError(
                "Vault client not initialized".to_string()
            )),
        }
    }
    
    /// Delete a secret by its key
    async fn delete_secret(&self, key: &str, auth_token: &str) -> Result<(), VaultError> {
        // Verify the token first
        let token_data = self.verify_token(auth_token).await?;
        
        // Check if token has permission to delete this secret
        if !self.is_authorized(auth_token, &format!("secret/{}", key), "delete").await? {
            return Err(VaultError::PermissionDenied(
                format!("Token does not have permission to delete secret: {}", key)
            ));
        }
        
        // If in mock mode, return error
        if self.is_mock() {
            return Err(VaultError::ConfigurationError(
                "Vault client is running in mock mode".to_string()
            ));
        }
        
        // Delete the secret from Vault
        match &self.client {
            Some(client) => {
                let secret_path = format!("secret/data/{}", key);
                match client.delete_secret(&secret_path) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(VaultError::ClientError(e)),
                }
            }
            None => Err(VaultError::ConfigurationError(
                "Vault client not initialized".to_string()
            )),
        }
    }
    
    /// List secrets with the given path prefix
    async fn list_secrets(&self, path_prefix: &str, auth_token: &str) -> Result<Vec<SecretMetadata>, VaultError> {
        // Verify the token first
        let token_data = self.verify_token(auth_token).await?;
        
        // Check if token has permission to list secrets
        if !self.is_authorized(auth_token, "secret", "list").await? {
            return Err(VaultError::PermissionDenied(
                "Token does not have permission to list secrets".to_string()
            ));
        }
        
        // If in mock mode, return empty list
        if self.is_mock() {
            return Ok(Vec::new());
        }
        
        // List secrets from Vault
        match &self.client {
            Some(client) => {
                let list_path = if path_prefix.is_empty() {
                    "secret/metadata".to_string()
                } else {
                    format!("secret/metadata/{}", path_prefix)
                };
                
                match client.list_secrets(&list_path) {
                    Ok(secrets) => {
                        let mut result = Vec::new();
                        
                        // Extract keys from the response
                        if let Some(keys) = secrets.get("keys").and_then(|k| k.as_array()) {
                            for key in keys {
                                if let Some(key_str) = key.as_str() {
                                    // For simplicity, using current time for metadata
                                    // In a real implementation, we would fetch actual metadata
                                    let now = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap_or(Duration::from_secs(0))
                                        .as_secs();
                                    
                                    result.push(SecretMetadata {
                                        key: key_str.to_string(),
                                        created_at: now,
                                        updated_at: now,
                                        expires_at: 0, // No expiration by default
                                        metadata: HashMap::new(),
                                    });
                                }
                            }
                        }
                        
                        Ok(result)
                    }
                    Err(e) => Err(VaultError::ClientError(e)),
                }
            }
            None => Err(VaultError::ConfigurationError(
                "Vault client not initialized".to_string()
            )),
        }
    }
    
    /// Authenticate a service with its ID and secret
    async fn authenticate_service(
        &self, 
        service_id: &str, 
        service_secret: &str
    ) -> Result<TokenMetadata, VaultError> {
        // In mock mode, generate a fake token
        if self.is_mock() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs();
                
            return Ok(TokenMetadata {
                token: format!("mock-token-{}", uuid::Uuid::new_v4()),
                expires_at: now + 3600, // 1 hour
                roles: vec!["read".to_string(), "write".to_string()],
                service_id: service_id.to_string(),
            });
        }
        
        // In a real implementation, we would:
        // 1. Use Vault's AppRole or token auth method
        // 2. Generate a properly scoped token for the service
        // 3. Store token metadata and return it
        
        match &self.client {
            Some(client) => {
                // This is a simplified example - in a real implementation,
                // we would use Vault's auth methods properly
                
                // Check service credentials against Vault
                // For this example, we're simplifying by creating a token directly
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();
                    
                // Create a new token with 1 hour TTL
                let token = format!("s.{}", uuid::Uuid::new_v4());
                let expires_at = now + 3600; // 1 hour
                
                // Determine roles based on service ID (simplified example)
                let roles = match service_id {
                    "llm-service" => vec!["read:llm-keys".to_string(), "read:models".to_string()],
                    "api-gateway" => vec!["read:api-keys".to_string(), "write:api-keys".to_string()],
                    _ => vec!["read".to_string()], // Default minimal permissions
                };
                
                // Create token metadata
                let token_data = TokenMetadata {
                    token: token.clone(),
                    expires_at,
                    roles: roles.clone(),
                    service_id: service_id.to_string(),
                };
                
                // Cache the token
                let mut cache = self.token_cache.write().await;
                cache.insert(token, token_data.clone());
                
                Ok(token_data)
            }
            None => Err(VaultError::ConfigurationError(
                "Vault client not initialized".to_string()
            )),
        }
    }
    
    /// Verify a token and return its metadata
    async fn verify_token(&self, token: &str) -> Result<TokenMetadata, VaultError> {
        // Check token cache first
        let cache = self.token_cache.read().await;
        if let Some(metadata) = cache.get(token) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs();
                
            // Check if token is expired
            if metadata.expires_at < now {
                drop(cache); // Release read lock before write lock
                
                // Remove expired token from cache
                let mut write_cache = self.token_cache.write().await;
                write_cache.remove(token);
                
                return Err(VaultError::AuthenticationError(
                    "Token expired".to_string()
                ));
            }
            
            return Ok(metadata.clone());
        }
        
        // Token not in cache, verify with Vault
        if self.is_mock() {
            // In mock mode, accept any token starting with "mock-token-"
            if token.starts_with("mock-token-") {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();
                    
                let metadata = TokenMetadata {
                    token: token.to_string(),
                    expires_at: now + 3600, // 1 hour
                    roles: vec!["read".to_string(), "write".to_string()],
                    service_id: "mock-service".to_string(),
                };
                
                // Cache the token
                drop(cache); // Release read lock before write lock
                let mut write_cache = self.token_cache.write().await;
                write_cache.insert(token.to_string(), metadata.clone());
                
                return Ok(metadata);
            } else {
                return Err(VaultError::AuthenticationError(
                    "Invalid mock token".to_string()
                ));
            }
        }
        
        // In a real implementation, we would validate the token with Vault
        // For this example, we'll return an error for uncached tokens
        Err(VaultError::AuthenticationError(
            "Token not found in cache and verification with Vault not implemented".to_string()
        ))
    }
    
    /// Check if a token is authorized for an action on a resource
    async fn is_authorized(
        &self, 
        token: &str, 
        resource: &str, 
        action: &str
    ) -> Result<bool, VaultError> {
        // First verify the token
        let token_data = self.verify_token(token).await?;
        
        // In a real implementation, we would check the token's policies
        // For this example, we'll use a simplified role-based approach
        
        match (resource, action) {
            // LLM API keys can be read by llm-service
            (r, "read") if r.starts_with("secret/llm-api-key") => {
                Ok(token_data.roles.contains(&"read:llm-keys".to_string()))
            }
            
            // API keys can be read by api-gateway
            (r, "read") if r.starts_with("secret/api-key") => {
                Ok(token_data.roles.contains(&"read:api-keys".to_string()))
            }
            
            // API keys can be written by api-gateway
            (r, "write") if r.starts_with("secret/api-key") => {
                Ok(token_data.roles.contains(&"write:api-keys".to_string()))
            }
            
            // Generic read permission
            (_, "read") => Ok(token_data.roles.contains(&"read".to_string())),
            
            // Generic write permission
            (_, "write") => Ok(token_data.roles.contains(&"write".to_string())),
            
            // Generic list permission
            (_, "list") => Ok(token_data.roles.contains(&"list".to_string()) 
                            || token_data.roles.contains(&"read".to_string())),
            
            // Generic delete permission
            (_, "delete") => Ok(token_data.roles.contains(&"delete".to_string())),
            
            // Default to unauthorized
            _ => Ok(false),
        }
    }
}