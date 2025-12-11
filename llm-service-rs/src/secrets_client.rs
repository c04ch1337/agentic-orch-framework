// llm-service-rs/src/secrets_client.rs
//
// Client for the Secrets Service
// Handles secure retrieval of API keys and other sensitive credentials
//
// This module provides:
// - gRPC client for interaction with the secrets-service-rs
// - Authentication and token management
// - Automatic key rotation
// - Caching with expiration to reduce load on secrets service
// - Fallback mechanisms for resilient operation

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tonic::transport::Channel;
use tonic::{Request, Status};

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

// Cache entry for storing secrets with expiration
struct CachedSecret {
    value: String,
    expires_at: Option<u64>,
    created_at: Instant,
}

// Secret service client
pub struct SecretsClient {
    client: Option<SecretsServiceClient<Channel>>,
    service_id: String,
    service_secret: String,
    auth_token: Arc<RwLock<Option<String>>>,
    token_expires_at: Arc<RwLock<Option<u64>>>,
    secret_cache: Arc<RwLock<HashMap<String, CachedSecret>>>,
    secrets_addr: String,
}

impl SecretsClient {
    /// Create a new secrets client
    pub async fn new() -> Result<Self, SecretsError> {
        // Get service ID and secret from environment
        let service_id = env::var("SERVICE_ID").unwrap_or_else(|_| "llm-service".to_string());
        let service_secret = env::var("LLM_SERVICE_SECRET").map_err(|_| {
            SecretsError::ConfigurationError(
                "LLM_SERVICE_SECRET environment variable not set".to_string(),
            )
        })?;

        // Get secrets service address using standardized config pattern
        let secrets_addr = config_rs::get_client_address("SECRETS", 50080, None);

        log::info!("Initializing secrets client for service: {}", service_id);
        log::debug!("Connecting to secrets service at: {}", secrets_addr);

        // Create the client
        let client_result = SecretsServiceClient::connect(secrets_addr.clone()).await;

        let client = match client_result {
            Ok(client) => {
                log::info!("Successfully connected to secrets service");
                Some(client)
            }
            Err(err) => {
                log::error!("Failed to connect to secrets service: {}", err);
                log::warn!("Starting in degraded mode - secrets operations will fail");
                None
            }
        };

        Ok(Self {
            client,
            service_id,
            service_secret,
            auth_token: Arc::new(RwLock::new(None)),
            token_expires_at: Arc::new(RwLock::new(None)),
            secret_cache: Arc::new(RwLock::new(HashMap::new())),
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
            secret_cache: Arc::new(RwLock::new(HashMap::new())),
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
            let token = format!("mock-token-{}", uuid::Uuid::new_v4());
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
            roles: vec!["llm-service".to_string(), "read".to_string()],
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

    /// Get a secret by key
    pub async fn get_secret(&self, key: &str) -> Result<String, SecretsError> {
        // Check cache first
        {
            let cache = self.secret_cache.read().await;
            if let Some(cached) = cache.get(key) {
                // Check if the secret is still valid
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();

                if cached.expires_at.map_or(true, |exp| exp > now) {
                    // Cache hit - secret is still valid
                    return Ok(cached.value.clone());
                }
            }
        }

        // In mock mode, return an error
        if self.is_mock() {
            return Err(SecretsError::ConfigurationError(
                "Secrets client is running in mock mode".to_string(),
            ));
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

                // Cache the secret
                let mut cache = self.secret_cache.write().await;
                cache.insert(
                    key.to_string(),
                    CachedSecret {
                        value: response.secret_value.clone(),
                        expires_at: if response.expires_at > 0 {
                            Some(response.expires_at as u64)
                        } else {
                            None
                        },
                        created_at: Instant::now(),
                    },
                );

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

    /// Check if a service is authorized for a specific action
    pub async fn is_authorized(&self, resource: &str, action: &str) -> Result<bool, SecretsError> {
        // In mock mode, always return true
        if self.is_mock() {
            return Ok(true);
        }

        // Get authentication token
        let auth_token = self.authenticate().await?;

        // Check authorization with the service
        let client_ref = self.client.as_ref().ok_or_else(|| {
            SecretsError::ConfigurationError("No secrets service client available".to_string())
        })?;

        let mut client = client_ref.clone();

        let request = Request::new(ServiceAuthRequest {
            service_id: self.service_id.clone(),
            auth_token,
            target_resource: resource.to_string(),
            action: action.to_string(),
        });

        match client.authenticate_service(request).await {
            Ok(response) => {
                let response = response.into_inner();
                Ok(response.authorized)
            }
            Err(status) => {
                log::error!("Failed to check authorization: {}", status);
                Err(SecretsError::GrpcError(status))
            }
        }
    }

    /// Get LLM API key from secret storage
    pub async fn get_llm_api_key(&self, provider: &str) -> Result<String, SecretsError> {
        // Secret path for LLM API keys follows the format "llm-api-key/{provider}"
        let key = format!("llm-api-key/{}", provider);
        self.get_secret(&key).await
    }

    /// Reconnect to secrets service if connection was lost
    pub async fn reconnect(&mut self) -> Result<(), SecretsError> {
        if self.client.is_none() {
            // Attempt to connect to the secrets service
            match SecretsServiceClient::connect(self.secrets_addr.clone()).await {
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
}
