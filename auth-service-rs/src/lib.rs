use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

pub mod jwt;
pub mod rbac;
pub mod audit;
pub mod admin;
pub mod certificates;
pub mod storage;
pub mod secrets_client;
pub mod middleware;
pub mod service_mesh;
pub mod delegation;
pub mod token_manager;

use jwt::{TokenManager, Claims};
use rbac::{RbacManager, Role, Permission, PermissionEffect, PrincipalType, PermissionDecision};
use admin::{AdminManager, User, Service, UserStatus, ServiceStatus};
use certificates::CertificateManager;
use token_manager::TokenRotationManager;
use delegation::TokenDelegator;
use secrets_client::SecretsClient;
use storage::StorageBackend;

/// Core authentication service implementation
pub struct AuthService {
    // Core components
    token_manager: Arc<TokenManager>,
    rbac_manager: Arc<RbacManager>,
    admin_manager: Arc<AdminManager>,
    cert_manager: Arc<CertificateManager>,
    rotation_manager: Arc<TokenRotationManager>,
    storage: Arc<dyn StorageBackend>,
    
    // Optional components
    token_delegator: Option<Arc<TokenDelegator>>,
    secrets_client: Option<Arc<SecretsClient>>,
    
    // Service configuration
    service_id: String,
    issuer: String,
}

impl AuthService {
    /// Create a new auth service instance
    pub async fn new(
        jwt_secret: String,
        redis_url: String,
        secrets_addr: String,
        storage: Arc<dyn StorageBackend>,
    ) -> Result<Self> {
        // Initialize the service ID and issuer
        let service_id = std::env::var("SERVICE_ID").unwrap_or_else(|_| "auth-service".to_string());
        let issuer = std::env::var("TOKEN_ISSUER").unwrap_or_else(|_| "phoenix-orch-agi".to_string());
        
        // Initialize JWT token manager
        let token_manager = TokenManager::new(&jwt_secret, &redis_url, &issuer).await?;
        let token_manager = Arc::new(token_manager);
        
        // Initialize RBAC manager
        let rbac_manager = RbacManager::new(storage.clone()).await?;
        let rbac_manager = Arc::new(rbac_manager);
        
        // Initialize certificate manager
        let cert_manager = CertificateManager::new(storage.clone()).await?;
        let cert_manager = Arc::new(cert_manager);
        
        // Initialize token rotation manager
        let rotation_manager = TokenRotationManager::new(
            token_manager.clone(),
            None, // Use default token rotation interval
            None, // Use default key rotation interval
        ).await?;
        let rotation_manager = Arc::new(rotation_manager);
        
        // Start token rotation tasks
        rotation_manager.clone().start_rotation_tasks().await;
        
        // Initialize secrets client (if available)
        let secrets_client = match SecretsClient::connect(&secrets_addr, &service_id, None).await {
            Ok(client) => {
                tracing::info!("Connected to secrets service at {}", secrets_addr);
                Some(Arc::new(client))
            },
            Err(err) => {
                tracing::warn!("Failed to connect to secrets service: {}. Some features will be limited.", err);
                None
            }
        };
        
        // Initialize token delegator (optional)
        let token_delegator = if token_manager.clone().is_some() {
            match TokenDelegator::new(token_manager.clone(), storage.clone()).await {
                Ok(delegator) => Some(Arc::new(delegator)),
                Err(err) => {
                    tracing::warn!("Failed to initialize token delegator: {}. Token delegation will not be available.", err);
                    None
                }
            }
        } else {
            None
        };
        
        // Initialize admin manager
        let admin_manager = AdminManager::new(
            rbac_manager.clone(),
            token_manager.clone(),
            cert_manager.clone(),
            storage.clone(),
            token_delegator.clone(),
        ).await?;
        let admin_manager = Arc::new(admin_manager);
        
        Ok(Self {
            token_manager,
            rbac_manager,
            admin_manager,
            cert_manager,
            rotation_manager,
            storage,
            token_delegator,
            secrets_client,
            service_id,
            issuer,
        })
    }

    /// Initialize the service
    pub async fn initialize(&self) -> Result<()> {
        // Initialize storage
        self.storage.initialize().await?;
        
        // Initialize RBAC system
        self.rbac_manager.initialize_system_roles().await?;
        
        Ok(())
    }

    /// Generate a new token
    pub async fn generate_token(
        &self,
        subject: &str,
        audience: &str,
        token_type: &str,
        ttl: u64,
        roles: Vec<String>,
        scopes: Option<Vec<String>>,
        custom_claims: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<(String, Claims)> {
        self.token_manager.generate_token(
            subject,
            audience,
            token_type,
            ttl,
            roles,
            scopes,
            custom_claims,
        ).await
    }

    /// Validate a token
    pub async fn validate_token(
        &self,
        token: &str,
        expected_audience: Option<&str>,
        validate_expiration: bool,
    ) -> Result<Claims> {
        self.token_manager.validate_token(token, expected_audience, validate_expiration).await
    }

    /// Check if a principal has permission
    pub async fn check_permission(
        &self,
        principal_id: &str,
        principal_type: &PrincipalType,
        resource: &str,
        action: &str,
        context: Option<&HashMap<String, String>>,
    ) -> Result<PermissionDecision> {
        self.rbac_manager.check_permission(
            principal_id,
            principal_type,
            resource,
            action,
            context,
        ).await
    }

    /// Get service health status
    pub async fn health_check(&self) -> Result<bool> {
        // Check core dependencies
        let storage_healthy = self.storage.is_healthy().await;
        let token_manager_healthy = true; // Internal component
        
        let secrets_healthy = if let Some(client) = &self.secrets_client {
            client.is_healthy().await
        } else {
            false // Secrets service not available
        };
        
        Ok(storage_healthy && token_manager_healthy)
    }
}

// Re-export key types
pub use jwt::{Claims, TokenManager};
pub use rbac::{Role, Permission, PermissionEffect, PrincipalType, PermissionDecision, RbacManager};
pub use admin::{User, Service, UserStatus, ServiceStatus, AdminManager};
pub use certificates::CertificateManager;
pub use token_manager::TokenRotationManager;
pub use delegation::TokenDelegator;
pub use secrets_client::SecretsClient;
pub use storage::StorageBackend;