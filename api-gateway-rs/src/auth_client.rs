use std::sync::Arc;
use std::time::Duration;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use tonic::{Request, Status};
use config_rs::ServiceConfig;

// Proto generated client
use phoenix_orch_proto::auth_service::{
    auth_service_client::AuthServiceClient,
    ValidateTokenRequest, ValidateTokenResponse,
    GenerateTokenRequest, TokenData,
    CheckPermissionRequest, CheckPermissionResponse,
    ValidateApiKeyRequest, ValidateApiKeyResponse,
    TokenRevokeRequest, TokenRevokeResponse,
    HealthCheckRequest, HealthCheckResponse,
};

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

// Static client that's accessible across the application
static AUTH_CLIENT: Lazy<RwLock<Option<AuthClient>>> = Lazy::new(|| RwLock::new(None));

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

#[derive(Clone)]
pub struct AuthClient {
    inner: AuthServiceClient<Channel>,
    config: ConnectionConfig,
    service_token: RwLock<Option<String>>,
    token_expiry: RwLock<Option<i64>>,
}

impl AuthClient {
    /// Create a new client with the specified configuration
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
        let channel = if use_mtls {
            // Configure mTLS for secure service-to-service authentication
            if let (Some(cert_path), Some(key_path)) = (cert_path, key_path) {
                // Read certificate and key files
                let cert = match tokio::fs::read(cert_path).await {
                    Ok(c) => c,
                    Err(e) => return Err(AuthError::ConfigurationError(format!("Failed to read certificate: {}", e))),
                };
                
                let key = match tokio::fs::read(key_path).await {
                    Ok(k) => k,
                    Err(e) => return Err(AuthError::ConfigurationError(format!("Failed to read key: {}", e))),
                };
                
                let server_ca = if let Some(ca_path) = ca_path {
                    match tokio::fs::read(ca_path).await {
                        Ok(ca) => Some(ca),
                        Err(e) => {
                            log::warn!("Failed to read CA: {}", e);
                            None
                        }
                    }
                } else {
                    None
                };
                
                // Set up TLS configuration with client identity for mutual TLS
                let identity = Identity::from_pem(cert, key);
                let mut tls_config = ClientTlsConfig::new()
                    .identity(identity)
                    .domain_name("auth-service"); // Verify server with this name
                
                // Add CA if provided
                if let Some(ca) = server_ca {
                    tls_config = tls_config.ca_certificate(Certificate::from_pem(ca));
                }
                
                // Connect with TLS config
                Channel::from_shared(addr.to_string())?
                    .tls_config(tls_config)?
                    .connect()
                    .await?
            } else {
                return Err(AuthError::ConfigurationError(
                    "mTLS enabled but certificate/key not provided".to_string(),
                ));
            }
        } else {
            // Standard insecure connection (should only be used in dev)
            log::warn!("Using insecure connection to auth service. NOT FOR PRODUCTION.");
            Channel::from_shared(addr.to_string())?
                .connect()
                .await?
        };
        
        // Store configuration
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
        
        let client = Self {
            inner: AuthServiceClient::new(channel),
            config,
            service_token: RwLock::new(None),
            token_expiry: RwLock::new(None),
        };
        
        // Immediately get a service token
        client.get_service_token().await?;
        
        Ok(client)
    }
    
    // Get a service token for this service, used for service-to-service auth
    async fn get_service_token(&self) -> Result<String, AuthError> {
        // Check if we already have a valid token
        {
            let expiry = self.token_expiry.read().await;
            let token = self.service_token.read().await;
            
            if let (Some(token), Some(expiry)) = (token.as_ref(), *expiry) {
                // Check if token is still valid with 5 minute buffer
                let now = chrono::Utc::now().timestamp();
                if now + 300 < expiry {
                    return Ok(token.clone());
                }
            }
        }
        
        // Need a new token, create request
        let mut client = self.inner.clone();
        
        let request = Request::new(GenerateTokenRequest {
            client_id: self.config.client_id.clone(),
            client_secret: self.config.client_secret.clone(),
            service_id: Some(self.config.service_id.clone()),
            roles: vec!["service".to_string()],
            expires_in_seconds: Some(3600), // 1 hour token
            metadata: std::collections::HashMap::new(),
        });
        
        // Send request
        let response = match client.generate_token(request).await {
            Ok(response) => response.into_inner(),
            Err(status) => return Err(AuthError::from(status)),
        };
        
        let token_data = match response.token_data {
            Some(data) => data,
            None => return Err(AuthError::TokenError("No token data returned".to_string())),
        };
        
        // Store token and expiry
        {
            let mut token_guard = self.service_token.write().await;
            *token_guard = Some(token_data.token.clone());
            
            let mut expiry_guard = self.token_expiry.write().await;
            *expiry_guard = token_data.expires_at;
        }
        
        Ok(token_data.token)
    }
    
    /// Validate a user token
    pub async fn validate_token(&self, token: &str) -> Result<TokenData, AuthError> {
        let mut client = self.inner.clone();
        
        // Get service token for authentication
        let service_token = self.get_service_token().await?;
        
        // Create request with service token in metadata
        let mut request = Request::new(ValidateTokenRequest {
            token: token.to_string(),
        });
        
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", service_token).parse().unwrap(),
        );
        
        // Send request
        let response = match client.validate_token(request).await {
            Ok(response) => response.into_inner(),
            Err(status) => return Err(AuthError::from(status)),
        };
        
        match response.token_data {
            Some(data) => Ok(data),
            None => Err(AuthError::AuthenticationFailed("Invalid token".to_string())),
        }
    }
    
    /// Check if a token has a required permission
    pub async fn check_permission(&self, token: &str, permission: &str) -> Result<bool, AuthError> {
        let mut client = self.inner.clone();
        
        // Get service token for authenticating this request
        let service_token = self.get_service_token().await?;
        
        // Create request
        let mut request = Request::new(CheckPermissionRequest {
            token: token.to_string(),
            permission: permission.to_string(),
            service_id: self.config.service_id.clone(),
            resource: None, // Optional resource-level permissions
        });
        
        // Add service token for authentication
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", service_token).parse().unwrap(),
        );
        
        // Send request
        let response = match client.check_permission(request).await {
            Ok(response) => response.into_inner(),
            Err(status) => return Err(AuthError::from(status)),
        };
        
        Ok(response.has_permission)
    }
    
    /// Validate an API key (for backward compatibility with existing clients)
    pub async fn validate_api_key(&self, api_key: &str) -> Result<TokenData, AuthError> {
        let mut client = self.inner.clone();
        
        // Get service token
        let service_token = self.get_service_token().await?;
        
        // Create request
        let mut request = Request::new(ValidateApiKeyRequest {
            api_key: api_key.to_string(),
            service_id: self.config.service_id.clone(),
        });
        
        // Add service token for authentication
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", service_token).parse().unwrap(),
        );
        
        // Send request
        let response = match client.validate_api_key(request).await {
            Ok(response) => response.into_inner(),
            Err(status) => return Err(AuthError::from(status)),
        };
        
        match response.token_data {
            Some(data) => Ok(data),
            None => Err(AuthError::AuthenticationFailed("Invalid API key".to_string())),
        }
    }
    
    /// Get the configured address with proper protocol
    fn get_formatted_address(&self) -> String {
        // Ensure address has proper protocol prefix
        if self.config.addr.starts_with("http://") || self.config.addr.starts_with("https://") {
            self.config.addr.clone()
        } else {
            if self.config.use_mtls {
                format!("https://{}", self.config.addr)
            } else {
                format!("http://{}", self.config.addr)
            }
        }
    }

    /// Revoke a token or all tokens for a particular user/service
    pub async fn revoke_token(&self, token: &str, revoke_all: bool) -> Result<(), AuthError> {
        let mut client = self.inner.clone();
        
        // Get service token
        let service_token = self.get_service_token().await?;
        
        // Create request
        let mut request = Request::new(TokenRevokeRequest {
            token: token.to_string(),
            revoke_all,
        });
        
        // Add service token for authentication
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", service_token).parse().unwrap(),
        );
        
        // Send request
        let _response = match client.revoke_token(request).await {
            Ok(response) => response.into_inner(),
            Err(status) => return Err(AuthError::from(status)),
        };
        
        Ok(())
    }
    
    /// Check if the auth service is healthy
    pub async fn is_healthy(&self) -> bool {
        let mut client = self.inner.clone();
        let request = Request::new(HealthCheckRequest {});
        
        match client.health_check(request).await {
            Ok(response) => {
                let response = response.into_inner();
                response.status == "SERVING"
            }
            Err(_) => false,
        }
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
    
    log::info!("Initializing auth client with address: {}", resolved_addr);
    
    let client = AuthClient::connect(
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
    let auth_client_guard = AUTH_CLIENT.read().await;
    
    match &*auth_client_guard {
        Some(client) => {
            let service_token = client.get_service_token().await?;
            let mut client_inner = client.inner.clone();
            
            let mut request = Request::new(GenerateTokenRequest {
                client_id: client.config.client_id.clone(),
                client_secret: client.config.client_secret.clone(),
                service_id: None, // Not a service token
                roles: vec!["client".to_string()],
                expires_in_seconds: Some(86400), // 24 hour token for clients
                metadata: std::collections::HashMap::new(),
            });
            
            // Add service token for authentication
            request.metadata_mut().insert(
                "authorization",
                format!("Bearer {}", service_token).parse().unwrap(),
            );
            
            // Send request
            let response = match client_inner.generate_token(request).await {
                Ok(response) => response.into_inner(),
                Err(status) => return Err(AuthError::from(status)),
            };
            
            match response.token_data {
                Some(data) => Ok(data),
                None => Err(AuthError::TokenError("No token data returned".to_string())),
            }
        }
        None => Err(AuthError::ConfigurationError("Auth client not initialized".to_string())),
    }
}