use std::sync::Arc;
use std::time::Duration;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use config_rs::ServiceConfig;
use serde::{Deserialize, Serialize};

// Local TokenData definition (matches the one in auth_middleware.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub token: String,
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub expires_at: i64,
}

// Errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Token error: {0}")]
    TokenError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
    
    #[error("Communication error: {0}")]
    CommunicationError(#[from] tonic::transport::Error),
    
    #[error("gRPC error: {0}")]
    GrpcError(#[from] tonic::Status),
}

// Stubbed auth client for now - will be implemented when auth service proto is available
// Static client that's accessible across the application
static AUTH_CLIENT: Lazy<RwLock<Option<MockAuthClient>>> = Lazy::new(|| RwLock::new(None));

#[derive(Debug, Clone)]
struct ConnectionConfig {
    addr: String,
    service_id: String,
    client_id: String,
    client_secret: String,
    use_mtls: bool,
    cert_path: Option<String>,
    key_path: Option<String>,
    ca_path: Option<String>,
}

// Mock auth client for compilation
#[derive(Clone)]
pub struct MockAuthClient {
    config: ConnectionConfig,
}

impl MockAuthClient {
    pub async fn connect(
        addr: &str,
        service_id: &str,
        client_id: &str,
        client_secret: &str,
        use_mtls: bool,
        cert_path: Option<&str>,
        key_path: Option<&str>,
        ca_path: Option<&str>,
    ) -> Result<Self, AuthError> {
        log::info!("Creating mock auth client (auth service integration disabled)");
        
        let config = ConnectionConfig {
            addr: addr.to_string(),
            service_id: service_id.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            use_mtls,
            cert_path: cert_path.map(|s| s.to_string()),
            key_path: key_path.map(|s| s.to_string()),
            ca_path: ca_path.map(|s| s.to_string()),
        };
        
        Ok(Self { config })
    }
    
    pub async fn validate_token(&self, token: &str) -> Result<TokenData, AuthError> {
        // Mock validation - accept any token starting with "valid-"
        if token.starts_with("valid-") || token.starts_with("Bearer valid-") {
            Ok(TokenData {
                token: token.to_string(),
                user_id: "mock-user".to_string(),
                roles: vec!["user".to_string()],
                permissions: vec!["execute:invoke".to_string()],
                expires_at: chrono::Utc::now().timestamp() + 3600,
            })
        } else {
            Err(AuthError::AuthenticationFailed("Invalid mock token".to_string()))
        }
    }
    
    pub async fn check_permission(&self, _token: &str, permission: &str) -> Result<bool, AuthError> {
        // Mock permission check - allow execute:invoke
        Ok(permission == "execute:invoke")
    }
    
    pub async fn validate_api_key(&self, api_key: &str) -> Result<TokenData, AuthError> {
        // Mock API key validation
        Ok(TokenData {
            token: format!("api-key-{}", api_key),
            user_id: "api-user".to_string(),
            roles: vec!["api-client".to_string()],
            permissions: vec!["execute:invoke".to_string()],
            expires_at: chrono::Utc::now().timestamp() + 86400,
        })
    }
    
    pub async fn revoke_token(&self, _token: &str, _revoke_all: bool) -> Result<(), AuthError> {
        // Mock revocation
        Ok(())
    }
    
    pub async fn is_healthy(&self) -> bool {
        true // Mock is always healthy
    }
    
    pub async fn get_service_token(&self) -> Result<String, AuthError> {
        Ok("mock-service-token".to_string())
    }
}

// Public interface functions to provide static access to the auth client

pub async fn init_auth_client(
    addr: &str,
    service_id: &str,
    client_id: &str,
    client_secret: &str,
    use_mtls: bool,
    cert_path: Option<&str>,
    key_path: Option<&str>,
    ca_path: Option<&str>,
) -> Result<(), AuthError> {
    // Use standardized config to get auth service address if addr is empty or "default"
    let config = ServiceConfig::new("api-gateway");
    let resolved_addr = if addr.is_empty() || addr == "default" {
        config.get_client_address("auth-service", 50090)
    } else {
        addr.to_string()
    };
    
    log::info!("Initializing mock auth client (auth service integration disabled)");
    
    let client = MockAuthClient::connect(
        &resolved_addr,
        service_id,
        client_id,
        client_secret,
        use_mtls,
        cert_path,
        key_path,
        ca_path,
    ).await?;
    
    let mut auth_client_guard = AUTH_CLIENT.write().await;
    *auth_client_guard = Some(client);
    
    Ok(())
}

pub async fn validate_token(token: &str) -> Result<TokenData, AuthError> {
    let auth_client_guard = AUTH_CLIENT.read().await;
    
    match &*auth_client_guard {
        Some(client) => client.validate_token(token).await,
        None => Err(AuthError::ConfigurationError("Auth client not initialized".to_string())),
    }
}

pub async fn check_permission(token: &str, permission: &str) -> Result<bool, AuthError> {
    let auth_client_guard = AUTH_CLIENT.read().await;
    
    match &*auth_client_guard {
        Some(client) => client.check_permission(token, permission).await,
        None => Err(AuthError::ConfigurationError("Auth client not initialized".to_string())),
    }
}

pub async fn validate_api_key(api_key: &str) -> Result<TokenData, AuthError> {
    let auth_client_guard = AUTH_CLIENT.read().await;
    
    match &*auth_client_guard {
        Some(client) => client.validate_api_key(api_key).await,
        None => Err(AuthError::ConfigurationError("Auth client not initialized".to_string())),
    }
}

pub async fn revoke_token(token: &str, revoke_all: bool) -> Result<(), AuthError> {
    let auth_client_guard = AUTH_CLIENT.read().await;
    
    match &*auth_client_guard {
        Some(client) => client.revoke_token(token, revoke_all).await,
        None => Err(AuthError::ConfigurationError("Auth client not initialized".to_string())),
    }
}

pub async fn is_auth_healthy() -> bool {
    let auth_client_guard = AUTH_CLIENT.read().await;
    
    match &*auth_client_guard {
        Some(client) => client.is_healthy().await,
        None => false,
    }
}

pub async fn generate_client_token() -> Result<TokenData, AuthError> {
    // Mock token generation
    Ok(TokenData {
        token: format!("mock-client-token-{}", uuid::Uuid::new_v4()),
        user_id: "mock-client".to_string(),
        roles: vec!["client".to_string()],
        permissions: vec!["execute:invoke".to_string()],
        expires_at: chrono::Utc::now().timestamp() + 86400,
    })
}