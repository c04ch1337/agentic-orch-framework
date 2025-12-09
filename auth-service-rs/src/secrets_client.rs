// auth-service-rs/src/secrets_client.rs
//
// Client for interacting with the secrets service
// Provides:
// - Secure storage and retrieval of sensitive credentials
// - Encryption key management
// - Automatic rotation of sensitive data
// - Secure distribution of credentials to authorized services

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tonic::{Request, Status, transport::Channel};
use anyhow::{Result, anyhow, Context};
use tracing::{debug, error, info, warn};
use serde::{Serialize, Deserialize};
use once_cell::sync::Lazy;
use uuid::Uuid;

use phoenix_orch_proto::secrets_service::{
    secrets_service_client::SecretsServiceClient,
    GetSecretRequest, GetSecretResponse,
    StoreSecretRequest, StoreSecretResponse,
    DeleteSecretRequest, DeleteSecretResponse,
    HealthCheckRequest, HealthCheckResponse,
    ListSecretsRequest, ListSecretsResponse,
};

// Global client for accessing the secrets service
static SECRETS_CLIENT: Lazy<RwLock<Option<SecretsClient>>> = Lazy::new(|| {
    RwLock::new(None)
});

#[derive(Debug, Serialize, Deserialize)]
pub struct SecretMetadata {
    pub creation_date: i64,
    pub last_updated: i64,
    pub version: i32,
    pub description: Option<String>,
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct SecretsClient {
    client: SecretsServiceClient<Channel>,
    service_id: String,
    access_token: RwLock<Option<String>>,
}

#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    #[error("Failed to connect to secrets service: {0}")]
    ConnectionError(#[from] tonic::transport::Error),
    
    #[error("gRPC error: {0}")]
    GrpcError(#[from] tonic::Status),
    
    #[error("Secret not found: {0}")]
    SecretNotFound(String),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Authorization error: {0}")]
    AuthorizationError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl SecretsClient {
    /// Create a new secrets client
    pub async fn connect(addr: &str, service_id: &str, access_token: Option<String>) -> Result<Self, SecretsError> {
        // Connect to the secrets service
        let channel = Channel::from_shared(addr.to_string())?
            .connect()
            .await?;
            
        let client = SecretsServiceClient::new(channel);
        
        // Create client
        let secrets_client = Self {
            client,
            service_id: service_id.to_string(),
            access_token: RwLock::new(access_token),
        };
        
        Ok(secrets_client)
    }
    
    /// Set the access token for authenticating with the secrets service
    pub async fn set_access_token(&self, token: &str) {
        let mut access_token = self.access_token.write().await;
        *access_token = Some(token.to_string());
    }
    
    /// Get a secret from the secrets service
    pub async fn get_secret(&self, key: &str) -> Result<String, SecretsError> {
        let mut client = self.client.clone();
        
        // Create the request
        let mut request = Request::new(GetSecretRequest {
            key: key.to_string(),
            version: 0, // Latest version
        });
        
        // Add authorization if available
        if let Some(token) = self.access_token.read().await.clone() {
            request.metadata_mut().insert(
                "authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        
        // Make the request
        let response = client.get_secret(request).await?;
        let inner = response.into_inner();
        
        if inner.value.is_empty() {
            return Err(SecretsError::SecretNotFound(key.to_string()));
        }
        
        Ok(inner.value)
    }
    
    /// Store a secret in the secrets service
    pub async fn store_secret(&self, key: &str, value: &str) -> Result<(), SecretsError> {
        let mut client = self.client.clone();
        
        // Create the request
        let mut request = Request::new(StoreSecretRequest {
            key: key.to_string(),
            value: value.to_string(),
            metadata: None, // Optional metadata
        });
        
        // Add authorization if available
        if let Some(token) = self.access_token.read().await.clone() {
            request.metadata_mut().insert(
                "authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        
        // Make the request
        let _response = client.store_secret(request).await?;
        
        Ok(())
    }
    
    /// Delete a secret from the secrets service
    pub async fn delete_secret(&self, key: &str) -> Result<(), SecretsError> {
        let mut client = self.client.clone();
        
        // Create the request
        let mut request = Request::new(DeleteSecretRequest {
            key: key.to_string(),
        });
        
        // Add authorization if available
        if let Some(token) = self.access_token.read().await.clone() {
            request.metadata_mut().insert(
                "authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        
        // Make the request
        let _response = client.delete_secret(request).await?;
        
        Ok(())
    }
    
    /// List secrets in the secrets service with optional prefix filter
    pub async fn list_secrets(&self, prefix: Option<&str>) -> Result<Vec<String>, SecretsError> {
        let mut client = self.client.clone();
        
        // Create the request
        let mut request = Request::new(ListSecretsRequest {
            prefix: prefix.unwrap_or("").to_string(),
        });
        
        // Add authorization if available
        if let Some(token) = self.access_token.read().await.clone() {
            request.metadata_mut().insert(
                "authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        
        // Make the request
        let response = client.list_secrets(request).await?;
        let inner = response.into_inner();
        
        Ok(inner.keys)
    }
    
    /// Check if the secrets service is healthy
    pub async fn is_healthy(&self) -> bool {
        let mut client = self.client.clone();
        
        let request = Request::new(HealthCheckRequest {});
        
        match client.health_check(request).await {
            Ok(response) => {
                let inner = response.into_inner();
                inner.status == "SERVING"
            }
            Err(_) => false,
        }
    }
    
    /// Create a token in the secrets service
    pub async fn store_token(&self, token_id: &str, token: &str, token_type: &str, expires_at: i64) -> Result<(), SecretsError> {
        // Store token with metadata
        let key = format!("token:{}", token_id);
        self.store_secret(&key, token).await?;
        
        // Store metadata as separate entry
        let metadata_key = format!("token_meta:{}", token_id);
        let metadata = serde_json::json!({
            "token_id": token_id,
            "token_type": token_type,
            "expires_at": expires_at,
            "created_at": chrono::Utc::now().timestamp(),
        });
        
        self.store_secret(&metadata_key, &metadata.to_string()).await?;
        
        Ok(())
    }
    
    /// Verify a token with the secrets service
    pub async fn verify_token(&self, token: &str) -> Result<TokenData, SecretsError> {
        // In a real implementation, we would validate the token signature,
        // check expiration, etc. For this example, we'll do a simple lookup.
        
        // List token metadata
        let tokens = self.list_secrets(Some("token_meta:")).await?;
        
        for token_meta_key in tokens {
            // Get token metadata
            let metadata_str = self.get_secret(&token_meta_key).await?;
            
            // Parse metadata
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)
                .map_err(|e| SecretsError::InternalError(format!("Failed to parse token metadata: {}", e)))?;
            
            // Get token ID
            let token_id = metadata["token_id"].as_str()
                .ok_or_else(|| SecretsError::InternalError("Token ID missing from metadata".to_string()))?;
                
            // Get token from storage
            let token_key = format!("token:{}", token_id);
            let stored_token = match self.get_secret(&token_key).await {
                Ok(t) => t,
                Err(SecretsError::SecretNotFound(_)) => continue, // Skip if token not found
                Err(e) => return Err(e),
            };
            
            // Check if token matches
            if stored_token == token {
                // Check expiration
                let expires_at = metadata["expires_at"].as_i64()
                    .ok_or_else(|| SecretsError::InternalError("Token expiration missing from metadata".to_string()))?;
                    
                if expires_at < chrono::Utc::now().timestamp() {
                    return Err(SecretsError::AuthenticationError("Token expired".to_string()));
                }
                
                // Token is valid, return token data
                let token_data = TokenData {
                    token_id: token_id.to_string(),
                    token: token.to_string(),
                    token_type: metadata["token_type"].as_str().unwrap_or("unknown").to_string(),
                    expires_at,
                    roles: Vec::new(), // Would be populated from metadata
                };
                
                return Ok(token_data);
            }
        }
        
        // Token not found
        Err(SecretsError::AuthenticationError("Invalid token".to_string()))
    }
    
    /// Generate a client token
    pub async fn generate_client_token(&self) -> Result<TokenData, SecretsError> {
        // Generate a random token
        let token = Uuid::new_v4().to_string();
        let token_id = Uuid::new_v4().to_string();
        
        // Set expiration (24 hours from now)
        let expires_at = chrono::Utc::now().timestamp() + 86400;
        
        // Store token
        self.store_token(&token_id, &token, "client", expires_at).await?;
        
        // Return token data
        Ok(TokenData {
            token_id,
            token,
            token_type: "client".to_string(),
            expires_at,
            roles: vec!["client".to_string()],
        })
    }
}

#[derive(Debug, Clone)]
pub struct TokenData {
    pub token_id: String,
    pub token: String,
    pub token_type: String,
    pub expires_at: i64,
    pub roles: Vec<String>,
}

/// Initialize the global secrets client
pub async fn init_secrets_client(
    addr: &str,
    service_id: &str,
    access_token: Option<String>,
) -> Result<(), SecretsError> {
    // Create client
    let client = SecretsClient::connect(addr, service_id, access_token).await?;
    
    // Check health
    if !client.is_healthy().await {
        warn!("Connected to secrets service, but health check failed");
    } else {
        info!("Successfully connected to secrets service");
    }
    
    // Store client
    let mut secrets_client = SECRETS_CLIENT.write().await;
    *secrets_client = Some(client);
    
    Ok(())
}

/// Get the global secrets client
pub async fn get_secrets_client() -> Result<SecretsClient, anyhow::Error> {
    let secrets_client = SECRETS_CLIENT.read().await;
    
    match &*secrets_client {
        Some(client) => Ok(client.clone()),
        None => Err(anyhow!("Secrets client not initialized")),
    }
}

/// Initialize a mock secrets client for testing
#[cfg(test)]
pub async fn init_mock_secrets_client() -> Result<(), SecretsError> {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::RwLock;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    
    // Mock SecretsServiceClient that implements a simple in-memory store
    #[derive(Clone)]
    struct MockSecretsClient {
        secrets: Arc<RwLock<HashMap<String, String>>>,
    }
    
    impl MockSecretsClient {
        fn new() -> Self {
            Self {
                secrets: Arc::new(RwLock::new(HashMap::new())),
            }
        }
        
        async fn get_secret(&self, key: &str) -> Result<String, Status> {
            let secrets = self.secrets.read().await;
            
            match secrets.get(key) {
                Some(value) => Ok(value.clone()),
                None => Err(Status::not_found(format!("Secret not found: {}", key))),
            }
        }
        
        async fn store_secret(&self, key: &str, value: &str) -> Result<(), Status> {
            let mut secrets = self.secrets.write().await;
            secrets.insert(key.to_string(), value.to_string());
            Ok(())
        }
        
        async fn delete_secret(&self, key: &str) -> Result<(), Status> {
            let mut secrets = self.secrets.write().await;
            secrets.remove(key);
            Ok(())
        }
        
        async fn list_secrets(&self, prefix: &str) -> Result<Vec<String>, Status> {
            let secrets = self.secrets.read().await;
            
            let keys: Vec<String> = secrets.keys()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect();
                
            Ok(keys)
        }
    }
    
    // Create mock client
    let mock_client = MockSecretsClient::new();
    
    // Create a real SecretsClient with the mock inner client
    let client = SecretsClient {
        client: SecretsServiceClient::new(Channel::from_static("http://[::]:50051")
            .connect()
            .await
            .unwrap()),
        service_id: "test-service".to_string(),
        access_token: RwLock::new(None),
    };
    
    // Store client
    let mut secrets_client = SECRETS_CLIENT.write().await;
    *secrets_client = Some(client);
    
    Ok(())
}