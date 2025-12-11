// api-gateway-rs/src/secrets_client.rs
//
// Client for the Secrets Service
// Handles secure retrieval of API keys and authentication tokens
//
// This module provides:
// - gRPC client for interaction with the secrets-service-rs
// - Token verification and validation
// - Authentication against HashiCorp Vault
// - Caching with expiration to reduce load on secrets service

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tonic::transport::Channel;
use tonic::{Request, Status};
use uuid::Uuid;

// Import the generated protobuf code
pub mod secrets_service {
    tonic::include_proto!("secrets_service");
}

use secrets_service::secrets_service_client::SecretsServiceClient;
use secrets_service::{SecretRequest, ServiceAuthRequest, TokenRequest};

#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Vault error: {0}")]
    VaultError(String),

    #[error("GRPC error: {0}")]
    GrpcError(#[from] Status),

    #[error("Transport error: {0}")]
    TransportError(#[from] tonic::transport::Error),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

// Cache entry for storing API keys with expiration
struct CachedToken {
    token_data: TokenData,
    created_at: Instant,
}

// Token data structure
#[derive(Clone, Debug)]
pub struct TokenData {
    pub token: String,
    pub expires_at: u64,
    pub service_id: String,
    pub roles: Vec<String>,
}

// Secret service client
pub struct SecretsClient {
    client: Option<SecretsServiceClient<Channel>>,
    service_id: String,
    service_secret: String,
    auth_token: Arc<RwLock<Option<String>>>,
    token_expires_at: Arc<RwLock<Option<u64>>>,
    token_cache: Arc<RwLock<HashMap<String, CachedToken>>>, // Cache for verified tokens
    api_key_cache: Arc<RwLock<HashMap<String, String>>>,    // Cache for API keys
    secrets_addr: String,
}

impl SecretsClient {
    /// Create a new secrets client
    pub async fn new() -> Result<Self, SecretsError> {
        // Get service ID and secret from environment
        let service_id = env::var("SERVICE_ID").unwrap_or_else(|_| "api-gateway".to_string());
        let service_secret =
            env::var("SERVICE_SECRET").unwrap_or_else(|_| "dev-secret".to_string());

        // Use standardized config to get secrets service address
        let secrets_addr = config_rs::get_client_address("secrets-service", 50080, None);

        log::info!("Connecting to secrets service at {}", secrets_addr);

        // Create the gRPC client
        let client_result = SecretsServiceClient::connect(secrets_addr.clone()).await;

        let client = match client_result {
            Ok(client) => {
                log::info!("Successfully connected to secrets service");
                Some(client)
            }
            Err(err) => {
                log::error!("Failed to connect to secrets service: {}", err);
                log::warn!(
                    "Starting in degraded mode - secrets operations will use default API key"
                );
                None
            }
        };

        Ok(Self {
            client,
            service_id,
            service_secret,
            auth_token: Arc::new(RwLock::new(None)),
            token_expires_at: Arc::new(RwLock::new(None)),
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            api_key_cache: Arc::new(RwLock::new(HashMap::new())),
            secrets_addr,
        })
    }

    /// Create a mock secrets client for testing
    #[cfg(test)]
    pub fn new_mock() -> Self {
        Self {
            client: None,
            service_id: "mock-service".to_string(),
            service_secret: "mock-secret".to_string(),
            auth_token: Arc::new(RwLock::new(None)),
            token_expires_at: Arc::new(RwLock::new(None)),
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            api_key_cache: Arc::new(RwLock::new(HashMap::new())),
            secrets_addr: "mock://localhost:50080".to_string(),
        }
    }

    /// Check if the client is in mock mode
    pub fn is_mock(&self) -> bool {
        self.client.is_none()
    }

    /// Authenticate with the secrets service and get a token
    async fn authenticate(&self) -> Result<String, SecretsError> {
        // Check if we have a valid token already
        {
            let token = self.auth_token.read().await;
            let expires_at = self.token_expires_at.read().await;

            if let (Some(token_str), Some(expiry)) = (token.as_ref(), *expires_at) {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();

                // Token still valid with at least 5 min remaining
                if expiry > now + 300 {
                    return Ok(token_str.clone());
                }
            }
        }

        // In mock mode, generate a fake token
        if self.is_mock() {
            let token = format!("mock-token-{}", Uuid::new_v4());
            let expires_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs()
                + 3600; // 1 hour

            // Update token in storage
            let mut token_writer = self.auth_token.write().await;
            *token_writer = Some(token.clone());

            let mut expiry_writer = self.token_expires_at.write().await;
            *expiry_writer = Some(expires_at);

            return Ok(token);
        }

        // We need a new token, let's authenticate
        let client_ref = self.client.as_ref().ok_or_else(|| {
            SecretsError::ConfigurationError("No secrets service client available".to_string())
        })?;

        let mut client = client_ref.clone();

        // Request token with 30 min validity
        let request = Request::new(TokenRequest {
            service_id: self.service_id.clone(),
            service_secret: self.service_secret.clone(),
            ttl: 1800, // 30 minutes
            roles: vec![
                "api-gateway".to_string(),
                "read".to_string(),
                "write".to_string(),
            ],
        });

        match client.generate_token(request).await {
            Ok(response) => {
                let response = response.into_inner();

                if !response.success {
                    return Err(SecretsError::AuthenticationError(response.error.clone()));
                }

                // Update token in storage
                let mut token_writer = self.auth_token.write().await;
                *token_writer = Some(response.token.clone());

                let mut expiry_writer = self.token_expires_at.write().await;
                *expiry_writer = Some(response.expires_at as u64);

                log::info!("Successfully authenticated with secrets service");
                log::debug!("Token expires at: {}", response.expires_at);

                Ok(response.token)
            }
            Err(status) => {
                log::error!("Failed to authenticate with secrets service: {}", status);
                Err(SecretsError::GrpcError(status))
            }
        }
    }

    /// Get a secret by key - used for API key retrieval
    pub async fn get_secret(&self, key: &str) -> Result<String, SecretsError> {
        // Check cache first
        {
            let cache = self.api_key_cache.read().await;
            if let Some(value) = cache.get(key) {
                return Ok(value.clone());
            }
        }

        // In mock mode, return a mock API key or default
        if self.is_mock() {
            if key == "api-key/default" {
                let api_key =
                    env::var("API_KEY").unwrap_or_else(|_| "phoenix-default-key".to_string());

                // Cache the key
                let mut cache = self.api_key_cache.write().await;
                cache.insert(key.to_string(), api_key.clone());

                return Ok(api_key);
            }
            return Err(SecretsError::SecretNotFound(key.to_string()));
        }

        // Get authentication token
        let auth_token = self.authenticate().await?;

        // Retrieve the secret from the service
        let client_ref = self.client.as_ref().ok_or_else(|| {
            SecretsError::ConfigurationError("No secrets service client available".to_string())
        })?;

        let mut client = client_ref.clone();

        let request = Request::new(SecretRequest {
            key: key.to_string(),
            service_id: self.service_id.clone(),
            auth_token,
        });

        match client.get_secret(request).await {
            Ok(response) => {
                let response = response.into_inner();

                if !response.success {
                    return Err(if response.error.contains("not found") {
                        SecretsError::SecretNotFound(key.to_string())
                    } else {
                        SecretsError::VaultError(response.error)
                    });
                }

                // Cache the API key
                let mut cache = self.api_key_cache.write().await;
                cache.insert(key.to_string(), response.secret_value.clone());

                Ok(response.secret_value)
            }
            Err(status) => {
                log::error!("Failed to retrieve secret {}: {}", key, status);

                // Map gRPC status codes to appropriate errors
                if status.code() == tonic::Code::NotFound {
                    Err(SecretsError::SecretNotFound(key.to_string()))
                } else if status.code() == tonic::Code::PermissionDenied {
                    Err(SecretsError::PermissionDenied(status.message().to_string()))
                } else if status.code() == tonic::Code::Unauthenticated {
                    // Authentication failed, clear our token
                    let mut token_writer = self.auth_token.write().await;
                    *token_writer = None;

                    Err(SecretsError::AuthenticationError(
                        status.message().to_string(),
                    ))
                } else {
                    Err(SecretsError::GrpcError(status))
                }
            }
        }
    }

    /// Verify an API token
    pub async fn verify_token(&self, token: &str) -> Result<TokenData, SecretsError> {
        // Check cache first
        {
            let cache = self.token_cache.read().await;
            if let Some(cached) = cache.get(token) {
                // Check if the token is still valid
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();

                if cached.token_data.expires_at > now {
                    // Cache hit, token is still valid
                    return Ok(cached.token_data.clone());
                }
            }
        }

        // In mock mode, verify based on predefined rules
        if self.is_mock() {
            // In mock mode, allow the default key from the environment
            let default_key =
                env::var("API_KEY").unwrap_or_else(|_| "phoenix-default-key".to_string());

            if token == default_key {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();

                let token_data = TokenData {
                    token: token.to_string(),
                    expires_at: now + 3600, // 1 hour
                    service_id: "api-client".to_string(),
                    roles: vec!["api-client".to_string()],
                };

                // Cache the validated token
                let mut cache = self.token_cache.write().await;
                cache.insert(
                    token.to_string(),
                    CachedToken {
                        token_data: token_data.clone(),
                        created_at: Instant::now(),
                    },
                );

                return Ok(token_data);
            } else {
                return Err(SecretsError::AuthenticationError(
                    "Invalid token".to_string(),
                ));
            }
        }

        // Get authentication token for our service
        let auth_token = self.authenticate().await?;

        // Use the secrets service to verify the client token
        let client_ref = self.client.as_ref().ok_or_else(|| {
            SecretsError::ConfigurationError("No secrets service client available".to_string())
        })?;

        let mut client = client_ref.clone();

        // Verify the token with the secrets service
        let request = Request::new(ServiceAuthRequest {
            service_id: "api-client".to_string(), // The client's service id
            auth_token: token.to_string(),        // The token to verify
            target_resource: "api-gateway".to_string(),
            action: "access".to_string(),
        });

        match client.authenticate_service(request).await {
            Ok(response) => {
                let response = response.into_inner();

                if !response.authenticated {
                    return Err(SecretsError::AuthenticationError(response.error.clone()));
                }

                if !response.authorized {
                    return Err(SecretsError::PermissionDenied(response.error.clone()));
                }

                // Get token expiration
                // For now, we'll set a fixed expiration of 1 hour
                // In a real implementation, this would come from the token verification
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();

                let token_data = TokenData {
                    token: token.to_string(),
                    expires_at: now + 3600, // 1 hour
                    service_id: "api-client".to_string(),
                    roles: response.permissions,
                };

                // Cache the validated token
                let mut cache = self.token_cache.write().await;
                cache.insert(
                    token.to_string(),
                    CachedToken {
                        token_data: token_data.clone(),
                        created_at: Instant::now(),
                    },
                );

                Ok(token_data)
            }
            Err(status) => {
                log::error!("Failed to verify token: {}", status);

                if status.code() == tonic::Code::Unauthenticated {
                    Err(SecretsError::AuthenticationError(
                        status.message().to_string(),
                    ))
                } else if status.code() == tonic::Code::PermissionDenied {
                    Err(SecretsError::PermissionDenied(status.message().to_string()))
                } else {
                    Err(SecretsError::GrpcError(status))
                }
            }
        }
    }

    /// Get the default API key for the API gateway
    pub async fn get_default_api_key(&self) -> Result<String, SecretsError> {
        // Path for the default API key in the secret store
        self.get_secret("api-key/default").await
    }

    /// Reconnect to secrets service if connection was lost
    pub async fn reconnect(&mut self) -> Result<(), SecretsError> {
        if self.client.is_none() {
            // Get updated address using standard config
            let secrets_addr = config_rs::get_client_address("secrets-service", 50080, None);

            // Update stored address
            self.secrets_addr = secrets_addr.clone();

            // Attempt to connect to the secrets service
            match SecretsServiceClient::connect(secrets_addr).await {
                Ok(client) => {
                    log::info!("Reconnected to secrets service");
                    self.client = Some(client);
                    Ok(())
                }
                Err(err) => {
                    log::error!("Failed to reconnect to secrets service: {}", err);
                    Err(SecretsError::ConnectionError(err.to_string()))
                }
            }
        } else {
            Ok(())
        }
    }

    /// Check connection health
    pub async fn is_healthy(&self) -> bool {
        if self.is_mock() {
            return true;
        }

        match self.authenticate().await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Generate a new client token with expiration
    pub async fn generate_client_token(&self) -> Result<TokenData, SecretsError> {
        // In mock mode, generate a mock token
        if self.is_mock() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs();

            return Ok(TokenData {
                token: format!("mock-client-token-{}", Uuid::new_v4()),
                expires_at: now + 3600, // 1 hour
                service_id: "api-client".to_string(),
                roles: vec!["api-client".to_string()],
            });
        }

        // Get auth token first
        let _auth_token = self.authenticate().await?;

        // Create a random token with UUID
        let token = format!("ct.{}", Uuid::new_v4());

        // Set expiration time (1 hour from now)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        let expires_at = now + 3600; // 1 hour

        let token_data = TokenData {
            token: token.clone(),
            expires_at,
            service_id: "api-client".to_string(),
            roles: vec!["api-client".to_string()],
        };

        // Cache the token
        let mut cache = self.token_cache.write().await;
        cache.insert(
            token.clone(),
            CachedToken {
                token_data: token_data.clone(),
                created_at: Instant::now(),
            },
        );

        Ok(token_data)
    }
}
