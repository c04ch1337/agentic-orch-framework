// auth-service-rs/src/auth_service.rs
//
// Main auth service implementation
// Provides the gRPC service implementation that exposes all auth functionality

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status, Code};
use anyhow::{Result, anyhow};
use tracing::{debug, error, info, warn};

use phoenix_orch_proto::auth_service::auth_service_server::AuthService;
use phoenix_orch_proto::auth_service::*;

use crate::jwt::{TokenManager, Claims};
use crate::rbac::{RbacManager, Role, Permission, PermissionEffect, PrincipalType, PermissionDecision};
use crate::audit::{self, EventType, Outcome};
use crate::admin::{AdminManager, User, Service, UserStatus, ServiceStatus};
use crate::certificates::CertificateManager;
use crate::token_manager::TokenRotationManager;
use crate::delegation::TokenDelegator;
use crate::secrets_client::SecretsClient;
use crate::service_mesh;
use crate::storage::StorageBackend;

pub struct AuthServiceImpl {
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

impl AuthServiceImpl {
    // Create a new auth service implementation
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
                info!("Connected to secrets service at {}", secrets_addr);
                Some(Arc::new(client))
            },
            Err(err) => {
                warn!("Failed to connect to secrets service: {}. Some features will be limited.", err);
                None
            }
        };
        
        // Initialize token delegator (optional)
        let token_delegator = if token_manager.clone().is_some() {
            match TokenDelegator::new(token_manager.clone(), storage.clone()).await {
                Ok(delegator) => Some(Arc::new(delegator)),
                Err(err) => {
                    warn!("Failed to initialize token delegator: {}. Token delegation will not be available.", err);
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
    
    // Extract the authenticated principal from the request metadata
    async fn get_authenticated_principal(&self, request: &Request<impl prost::Message>) -> Result<(String, PrincipalType, Option<String>), Status> {
        // Extract authorization header
        let auth = match request.metadata().get("authorization") {
            Some(t) => {
                let token_str = t.to_str().map_err(|_| {
                    Status::unauthenticated("Invalid authorization header")
                })?;
                
                // Handle Bearer prefix
                if token_str.starts_with("Bearer ") {
                    token_str[7..].to_string()
                } else {
                    token_str.to_string()
                }
            },
            None => {
                return Err(Status::unauthenticated("Missing authorization token"));
            }
        };
        
        // Validate the token
        let claims = self.token_manager.validate_token(&auth, None, true).await
            .map_err(|e| {
                audit::log_system_event(
                    EventType::TokenValidationFailed,
                    &format!("Token validation failed: {}", e),
                    None,
                ).await.ok();
                
                Status::unauthenticated(format!("Invalid token: {}", e))
            })?;
            
        // Determine principal type
        let principal_type = if claims.custom_claims.get("service_name").is_some() {
            PrincipalType::Service
        } else {
            PrincipalType::User
        };
        
        Ok((claims.sub, principal_type, Some(auth)))
    }
    
    // Check if a principal has a required permission
    async fn check_principal_permission(
        &self,
        principal_id: &str,
        principal_type: &PrincipalType,
        resource: &str,
        action: &str,
    ) -> Result<(), Status> {
        match self.rbac_manager.check_permission(
            principal_id,
            principal_type,
            resource,
            action,
            None,
        ).await {
            Ok(decision) => {
                // Log the access decision
                audit::log_access_decision(
                    principal_id,
                    &format!("{:?}", principal_type),
                    resource,
                    action,
                    decision.allowed,
                    &decision.reason,
                    None,
                ).await.ok();
                
                if !decision.allowed {
                    return Err(Status::permission_denied(format!(
                        "Permission denied: {} on {}", action, resource
                    )));
                }
                
                Ok(())
            },
            Err(e) => {
                // Log the error
                error!("Permission check failed: {}", e);
                
                Err(Status::internal(format!(
                    "Failed to check permission: {}", e
                )))
            }
        }
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    // Token validation
    async fn validate_token(
        &self,
        request: Request<ValidateTokenRequest>,
    ) -> Result<Response<ValidateTokenResponse>, Status> {
        let token = &request.get_ref().token;
        
        // Validate the token
        match self.token_manager.validate_token(token, None, true).await {
            Ok(claims) => {
                // Convert claims to TokenData
                let token_data = TokenData {
                    id: Some(claims.jti.clone()),
                    token: token.clone(),
                    token_type: Some(claims.typ.clone()),
                    subject: claims.sub.clone(),
                    audience: Some(claims.aud.clone()),
                    issuer: Some(claims.iss.clone()),
                    issued_at: Some(claims.iat as i64),
                    not_before: Some(claims.nbf as i64),
                    expires_at: Some(claims.exp as i64),
                    roles: claims.roles.clone(),
                    permissions: claims.scopes.unwrap_or_default(),
                    service_name: claims.service_name,
                    username: claims.user_name,
                    metadata: claims.custom_claims.iter()
                        .filter_map(|(k, v)| {
                            v.as_str().map(|s| (k.clone(), s.to_string()))
                        })
                        .collect(),
                };
                
                // Log the validation
                audit::log_system_event(
                    EventType::Other,
                    &format!("Token validated for {}", claims.sub),
                    Some(HashMap::from([
                        ("token_id".to_string(), claims.jti),
                        ("subject".to_string(), claims.sub),
                    ])),
                ).await.ok();
                
                // Return success response
                let response = ValidateTokenResponse {
                    is_valid: true,
                    token_data: Some(token_data),
                };
                
                Ok(Response::new(response))
            },
            Err(e) => {
                // Log the validation failure
                audit::log_system_event(
                    EventType::TokenValidationFailed,
                    &format!("Token validation failed: {}", e),
                    None,
                ).await.ok();
                
                // Return failure response
                let response = ValidateTokenResponse {
                    is_valid: false,
                    token_data: None,
                };
                
                Ok(Response::new(response))
            }
        }
    }

    // Token generation
    async fn generate_token(
        &self,
        request: Request<GenerateTokenRequest>,
    ) -> Result<Response<GenerateTokenResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/token",
            "generate",
        ).await?;
        
        // Determine subject (user_id or service_id)
        let subject = if let Some(service_id) = &req.service_id {
            // Creating a service token
            service_id.clone()
        } else {
            // Creating a user/client token
            req.client_id.clone()
        };
        
        // Determine token type
        let token_type = if req.service_id.is_some() {
            "service"
        } else {
            "client"
        };
        
        // Determine expiration
        let ttl = req.expires_in_seconds.unwrap_or(3600); // Default: 1 hour
        
        // Custom claims
        let mut custom_claims = HashMap::new();
        for (key, value) in &req.metadata {
            custom_claims.insert(key.clone(), serde_json::Value::String(value.clone()));
        }
        
        // Generate the token
        let (token, claims) = self.token_manager.generate_token(
            &subject,
            "phoenix-orch-agi", // audience
            token_type,
            ttl,
            req.roles.clone(),
            Some(req.roles.clone()), // Using roles as scopes for simplicity
            Some(custom_claims),
        ).await.map_err(|e| {
            error!("Failed to generate token: {}", e);
            Status::internal(format!("Failed to generate token: {}", e))
        })?;
        
        // Convert to TokenData
        let token_data = TokenData {
            id: Some(claims.jti.clone()),
            token: token.clone(),
            token_type: Some(token_type.to_string()),
            subject: subject.clone(),
            audience: Some("phoenix-orch-agi".to_string()),
            issuer: Some(self.issuer.clone()),
            issued_at: Some(claims.iat as i64),
            not_before: Some(claims.nbf as i64),
            expires_at: Some(claims.exp as i64),
            roles: claims.roles.clone(),
            permissions: claims.scopes.unwrap_or_default(),
            service_name: claims.service_name,
            username: claims.user_name,
            metadata: claims.custom_claims.iter()
                .filter_map(|(k, v)| {
                    v.as_str().map(|s| (k.clone(), s.to_string()))
                })
                .collect(),
        };
        
        // Log token generation
        audit::log_system_event(
            EventType::TokenIssued,
            &format!("{} token generated for {}", token_type, subject),
            Some(HashMap::from([
                ("token_id".to_string(), claims.jti),
                ("subject".to_string(), subject),
                ("issuer".to_string(), self.issuer.clone()),
                ("token_type".to_string(), token_type.to_string()),
                ("generated_by".to_string(), principal_id),
            ])),
        ).await.ok();
        
        // Return response
        let response = GenerateTokenResponse {
            token_data: Some(token_data),
        };
        
        Ok(Response::new(response))
    }

    // Token renewal
    async fn renew_token(
        &self,
        request: Request<RenewTokenRequest>,
    ) -> Result<Response<RenewTokenResponse>, Status> {
        let req = request.get_ref();
        
        // Validate the current token but don't check expiration
        // This allows renewing expired tokens as long as they're not revoked
        let claims = self.token_manager.validate_token(&req.token, None, false).await
            .map_err(|e| {
                error!("Token renewal failed: {}", e);
                Status::invalid_argument(format!("Invalid token: {}", e))
            })?;
        
        // Calculate new expiration
        let ttl = req.expires_in_seconds.unwrap_or(3600); // Default: 1 hour
        
        // Generate a new token with the same properties but new expiration
        let (new_token, new_claims) = self.token_manager.generate_token(
            &claims.sub,
            &claims.aud,
            &claims.typ,
            ttl,
            claims.roles,
            claims.scopes,
            Some(claims.custom_claims),
        ).await.map_err(|e| {
            error!("Failed to generate renewal token: {}", e);
            Status::internal(format!("Failed to generate renewal token: {}", e))
        })?;
        
        // Convert to TokenData
        let token_data = TokenData {
            id: Some(new_claims.jti.clone()),
            token: new_token,
            token_type: Some(new_claims.typ.clone()),
            subject: new_claims.sub.clone(),
            audience: Some(new_claims.aud.clone()),
            issuer: Some(new_claims.iss.clone()),
            issued_at: Some(new_claims.iat as i64),
            not_before: Some(new_claims.nbf as i64),
            expires_at: Some(new_claims.exp as i64),
            roles: new_claims.roles.clone(),
            permissions: new_claims.scopes.unwrap_or_default(),
            service_name: new_claims.service_name,
            username: new_claims.user_name,
            metadata: new_claims.custom_claims.iter()
                .filter_map(|(k, v)| {
                    v.as_str().map(|s| (k.clone(), s.to_string()))
                })
                .collect(),
        };
        
        // Revoke the old token
        if let Err(e) = self.token_manager.revoke_token(&claims.jti).await {
            warn!("Failed to revoke old token during renewal: {}", e);
        }
        
        // Log token renewal
        audit::log_system_event(
            EventType::TokenRefreshed,
            &format!("Token renewed for {}", claims.sub),
            Some(HashMap::from([
                ("old_token_id".to_string(), claims.jti),
                ("new_token_id".to_string(), new_claims.jti),
                ("subject".to_string(), claims.sub),
            ])),
        ).await.ok();
        
        // Return response
        let response = RenewTokenResponse {
            token_data: Some(token_data),
        };
        
        Ok(Response::new(response))
    }

    // Token revocation
    async fn revoke_token(
        &self,
        request: Request<TokenRevokeRequest>,
    ) -> Result<Response<TokenRevokeResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/token",
            "revoke",
        ).await?;
        
        // Extract the token
        let token = &req.token;
        
        // Parse the token to get the token ID (jti claim)
        let claims = self.token_manager.validate_token(token, None, false).await
            .map_err(|e| {
                warn!("Failed to parse token for revocation: {}", e);
                Status::invalid_argument(format!("Invalid token: {}", e))
            })?;
            
        let token_id = claims.jti.clone();
        
        // Use the rotation manager for comprehensive revocation
        match self.rotation_manager.revoke_token(
            &token_id,
            "Explicitly revoked by API call",
            Some(&principal_id),
            None,
        ).await {
            Ok(_) => {
                // If revoke_all is true, revoke all tokens for the subject
                let revoked_count = if req.revoke_all {
                    match self.rotation_manager.revoke_all_tokens(
                        &claims.sub,
                        if claims.typ == "service" { "service" } else { "user" },
                        "All tokens revoked by API call",
                        Some(&principal_id),
                    ).await {
                        Ok(count) => count as i32,
                        Err(e) => {
                            warn!("Failed to revoke all tokens: {}", e);
                            1 // We at least revoked the one token
                        }
                    }
                } else {
                    1 // Just the one token
                };
                
                // Return success
                let response = TokenRevokeResponse {
                    success: true,
                    tokens_revoked: revoked_count,
                };
                
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Failed to revoke token: {}", e);
                
                Status::internal(format!("Failed to revoke token: {}", e)).into()
            }
        }
    }

    // API key validation (for backwards compatibility)
    async fn validate_api_key(
        &self,
        request: Request<ValidateApiKeyRequest>,
    ) -> Result<Response<ValidateApiKeyResponse>, Status> {
        let req = request.get_ref();
        
        // Check if secrets client is available (required for API key validation)
        let secrets_client = match &self.secrets_client {
            Some(client) => client.clone(),
            None => {
                return Err(Status::unavailable("API key validation not available: Secrets service not connected"));
            }
        };
        
        // Validate the API key using the secrets service
        match secrets_client.verify_token(&req.api_key).await {
            Ok(token_data) => {
                // Convert to TokenData for response
                let response_token_data = TokenData {
                    id: Some(token_data.token_id.clone()),
                    token: token_data.token.clone(),
                    token_type: Some(token_data.token_type.clone()),
                    subject: "api_key_user".to_string(), // Generic subject for API keys
                    audience: Some(req.service_id.clone()),
                    issuer: Some(self.issuer.clone()),
                    issued_at: Some(0), // Not available in this legacy format
                    not_before: Some(0), // Not available in this legacy format
                    expires_at: Some(token_data.expires_at),
                    roles: token_data.roles,
                    permissions: vec![],
                    service_name: None,
                    username: None,
                    metadata: HashMap::new(),
                };
                
                // Log API key validation
                audit::log_system_event(
                    EventType::Other,
                    &format!("API key validated for service {}", req.service_id),
                    Some(HashMap::from([
                        ("service_id".to_string(), req.service_id.clone()),
                    ])),
                ).await.ok();
                
                // Return success response
                let response = ValidateApiKeyResponse {
                    is_valid: true,
                    token_data: Some(response_token_data),
                };
                
                Ok(Response::new(response))
            }
            Err(e) => {
                // Log the validation failure
                audit::log_system_event(
                    EventType::AccessDenied,
                    &format!("API key validation failed for service {}: {}", 
                             req.service_id, e),
                    None,
                ).await.ok();
                
                // Return failure response
                let response = ValidateApiKeyResponse {
                    is_valid: false,
                    token_data: None,
                };
                
                Ok(Response::new(response))
            }
        }
    }

    // Permission checking
    async fn check_permission(
        &self,
        request: Request<CheckPermissionRequest>,
    ) -> Result<Response<CheckPermissionResponse>, Status> {
        let req = request.get_ref();
        
        // Validate the token
        let claims = self.token_manager.validate_token(&req.token, None, true).await
            .map_err(|e| {
                warn!("Token validation failed during permission check: {}", e);
                Status::unauthenticated(format!("Invalid token: {}", e))
            })?;
            
        // Determine principal type
        let principal_type = if claims.custom_claims.get("service_name").is_some() {
            PrincipalType::Service
        } else {
            PrincipalType::User
        };
        
        // Check the permission
        let resource = if let Some(specific_resource) = &req.resource {
            specific_resource.clone()
        } else {
            format!("{}/{}", req.service_id, req.permission)
        };
        
        let decision = self.rbac_manager.check_permission(
            &claims.sub,
            &principal_type,
            &resource,
            "access", // Generic action
            None,
        ).await.map_err(|e| {
            error!("Permission check failed: {}", e);
            Status::internal(format!("Failed to check permission: {}", e))
        })?;
        
        // Log the access decision
        audit::log_access_decision(
            &claims.sub,
            &format!("{:?}", principal_type),
            &resource,
            &req.permission,
            decision.allowed,
            &decision.reason,
            None,
        ).await.ok();
        
        // Return response
        let response = CheckPermissionResponse {
            has_permission: decision.allowed,
        };
        
        Ok(Response::new(response))
    }

    // Get user permissions
    async fn get_user_permissions(
        &self,
        request: Request<GetUserPermissionsRequest>,
    ) -> Result<Response<GetUserPermissionsResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check: Only allow users to get their own permissions or admins
        if principal_id != req.user_id {
            self.check_principal_permission(
                &principal_id,
                &principal_type,
                "auth/permissions",
                "read",
            ).await?;
        }
        
        // Get the user's roles
        let roles = self.rbac_manager.get_principal_roles(
            &req.user_id,
            &PrincipalType::User,
        ).await.map_err(|e| {
            error!("Failed to get user roles: {}", e);
            Status::internal(format!("Failed to get user roles: {}", e))
        })?;
        
        // Extract permissions from roles
        let mut all_permissions = Vec::new();
        
        for role in &roles {
            for permission in &role.permissions {
                // Convert internal permission to API format
                let resource_type = if let Some(idx) = permission.resource_pattern.find('/') {
                    permission.resource_pattern[0..idx].to_string()
                } else {
                    permission.resource_pattern.clone()
                };
                
                // Check if service-specific
                if let Some(service_id) = &req.service_id {
                    // Only include permissions for this service
                    if !permission.resource_pattern.starts_with(service_id) && 
                       permission.resource_pattern != "*" {
                        continue;
                    }
                }
                
                // Add permissions for each action
                for action in &permission.actions {
                    all_permissions.push(Permission {
                        name: format!("{}:{}", permission.resource_pattern, action),
                        description: format!("Can {} on {}", action, permission.resource_pattern),
                        service_id: req.service_id.clone().unwrap_or_default(),
                        resource_type,
                    });
                }
            }
        }
        
        // Log the query
        audit::log_system_event(
            EventType::Other,
            &format!("User permissions queried for {}{}", req.user_id,
                     if let Some(sid) = &req.service_id { 
                         format!(" on service {}", sid) 
                     } else { 
                         "".to_string() 
                     }),
            Some(HashMap::from([
                ("user_id".to_string(), req.user_id.clone()),
                ("queried_by".to_string(), principal_id),
            ])),
        ).await.ok();
        
        // Return response
        let response = GetUserPermissionsResponse {
            permissions: all_permissions,
        };
        
        Ok(Response::new(response))
    }

    // Create a role
    async fn create_role(
        &self,
        request: Request<CreateRoleRequest>,
    ) -> Result<Response<RoleResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/role",
            "create",
        ).await?;
        
        // Create the role
        let role = self.admin_manager.create_role(
            req.clone(),
            &principal_id,
        ).await.map_err(|e| {
            error!("Failed to create role: {}", e);
            Status::internal(format!("Failed to create role: {}", e))
        })?;
        
        // Convert to API response
        let role_response = RoleResponse {
            role: Some(role.into()),
        };
        
        Ok(Response::new(role_response))
    }

    // Get a role
    async fn get_role(
        &self,
        request: Request<GetRoleRequest>,
    ) -> Result<Response<RoleResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/role",
            "read",
        ).await?;
        
        // Get the role
        let role = self.rbac_manager.get_role(&req.id).await.map_err(|e| {
            if e.to_string().contains("not found") {
                Status::not_found(format!("Role not found: {}", req.id))
            } else {
                error!("Failed to get role: {}", e);
                Status::internal(format!("Failed to get role: {}", e))
            }
        })?;
        
        // Convert to API response
        let role_response = RoleResponse {
            role: Some(role.into()),
        };
        
        Ok(Response::new(role_response))
    }

    // Update a role
    async fn update_role(
        &self,
        request: Request<UpdateRoleRequest>,
    ) -> Result<Response<RoleResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/role",
            "update",
        ).await?;
        
        // Update the role
        let role = self.admin_manager.update_role(
            req.clone(),
            &principal_id,
        ).await.map_err(|e| {
            if e.to_string().contains("not found") {
                Status::not_found(format!("Role not found: {}", req.id))
            } else {
                error!("Failed to update role: {}", e);
                Status::internal(format!("Failed to update role: {}", e))
            }
        })?;
        
        // Convert to API response
        let role_response = RoleResponse {
            role: Some(role.into()),
        };
        
        Ok(Response::new(role_response))
    }

    // Delete a role
    async fn delete_role(
        &self,
        request: Request<DeleteRoleRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/role",
            "delete",
        ).await?;
        
        // Delete the role
        self.admin_manager.delete_role(
            &req.id,
            &principal_id,
        ).await.map_err(|e| {
            if e.to_string().contains("not found") {
                Status::not_found(format!("Role not found: {}", req.id))
            } else {
                error!("Failed to delete role: {}", e);
                Status::internal(format!("Failed to delete role: {}", e))
            }
        })?;
        
        Ok(Response::new(()))
    }

    // List roles
    async fn list_roles(
        &self,
        request: Request<ListRolesRequest>,
    ) -> Result<Response<ListRolesResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/role",
            "list",
        ).await?;
        
        // List roles
        let (roles, next_page_token, total_count) = self.admin_manager.list_roles(
            req.clone(),
        ).await.map_err(|e| {
            error!("Failed to list roles: {}", e);
            Status::internal(format!("Failed to list roles: {}", e))
        })?;
        
        // Convert to API response
        let response = ListRolesResponse {
            roles: roles.into_iter().map(|role| role.into()).collect(),
            next_page_token,
            total_count,
        };
        
        Ok(Response::new(response))
    }

    // Add user to role
    async fn add_user_to_role(
        &self,
        request: Request<AddUserToRoleRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/role",
            "assign",
        ).await?;
        
        // Add user to role
        self.admin_manager.add_user_to_role(
            &req.user_id,
            &req.role_id,
            &principal_id,
        ).await.map_err(|e| {
            error!("Failed to add user to role: {}", e);
            Status::internal(format!("Failed to add user to role: {}", e))
        })?;
        
        Ok(Response::new(()))
    }

    // Remove user from role
    async fn remove_user_from_role(
        &self,
        request: Request<RemoveUserFromRoleRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/role",
            "revoke",
        ).await?;
        
        // Remove user from role
        self.admin_manager.remove_user_from_role(
            &req.user_id,
            &req.role_id,
            &principal_id,
        ).await.map_err(|e| {
            error!("Failed to remove user from role: {}", e);
            Status::internal(format!("Failed to remove user from role: {}", e))
        })?;
        
        Ok(Response::new(()))
    }

    // User management APIs
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/user",
            "create",
        ).await?;
        
        // Create user
        let user = self.admin_manager.create_user(
            req.clone(),
            &principal_id,
        ).await.map_err(|e| {
            error!("Failed to create user: {}", e);
            Status::internal(format!("Failed to create user: {}", e))
        })?;
        
        // Convert to API response
        let response = UserResponse {
            user: Some(user.into()),
        };
        
        Ok(Response::new(response))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/user",
            "read",
        ).await?;
        
        // Get user
        let user = self.admin_manager.get_user(
            &req.id,
        ).await.map_err(|e| {
            if e.to_string().contains("not found") {
                Status::not_found(format!("User not found: {}", req.id))
            } else {
                error!("Failed to get user: {}", e);
                Status::internal(format!("Failed to get user: {}", e))
            }
        })?;
        
        // Get the user's roles for a complete response
        let roles = self.rbac_manager.get_principal_roles(
            &user.id,
            &PrincipalType::User,
        ).await.map_err(|e| {
            error!("Failed to get user roles: {}", e);
            Status::internal(format!("Failed to get user roles: {}", e))
        })?;
        
        // Extract role IDs
        let role_ids: Vec<String> = roles.iter().map(|r| r.id.clone()).collect();
        
        // Convert to API response
        let mut api_user: user_response::User = user.into();
        api_user.roles = role_ids;
        
        let response = UserResponse {
            user: Some(api_user),
        };
        
        Ok(Response::new(response))
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<UserResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/user",
            "update",
        ).await?;
        
        // Update user
        let user = self.admin_manager.update_user(
            req.clone(),
            &principal_id,
        ).await.map_err(|e| {
            if e.to_string().contains("not found") {
                Status::not_found(format!("User not found: {}", req.id))
            } else {
                error!("Failed to update user: {}", e);
                Status::internal(format!("Failed to update user: {}", e))
            }
        })?;
        
        // Convert to API response
        let response = UserResponse {
            user: Some(user.into()),
        };
        
        Ok(Response::new(response))
    }

    async fn delete_user(
        &self,
        request: Request<DeleteUserRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/user",
            "delete",
        ).await?;
        
        // Delete user
        self.admin_manager.delete_user(
            &req.id,
            &principal_id,
        ).await.map_err(|e| {
            if e.to_string().contains("not found") {
                Status::not_found(format!("User not found: {}", req.id))
            } else {
                error!("Failed to delete user: {}", e);
                Status::internal(format!("Failed to delete user: {}", e))
            }
        })?;
        
        // Revoke all tokens for this user
        if let Ok(manager) = TokenRotationManager::get_global().await {
            if let Err(e) = manager.revoke_all_tokens(
                &req.id,
                "user",
                "User deleted",
                Some(&principal_id),
            ).await {
                warn!("Failed to revoke tokens for deleted user: {}", e);
            }
        }
        
        Ok(Response::new(()))
    }

    async fn list_users(
        &self,
        request: Request<ListUsersRequest>,
    ) -> Result<Response<ListUsersResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/user",
            "list",
        ).await?;
        
        // List users
        let (users, next_page_token, total_count) = self.admin_manager.list_users(
            req.clone(),
        ).await.map_err(|e| {
            error!("Failed to list users: {}", e);
            Status::internal(format!("Failed to list users: {}", e))
        })?;
        
        // Convert to API response
        let response = ListUsersResponse {
            users: users.into_iter().map(|user| user.into()).collect(),
            next_page_token,
            total_count,
        };
        
        Ok(Response::new(response))
    }

    // Service management APIs
    async fn register_service(
        &self,
        request: Request<RegisterServiceRequest>,
    ) -> Result<Response<ServiceResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/service",
            "create",
        ).await?;
        
        // Register service
        let service = self.admin_manager.register_service(
            req.clone(),
            &principal_id,
        ).await.map_err(|e| {
            error!("Failed to register service: {}", e);
            Status::internal(format!("Failed to register service: {}", e))
        })?;
        
        // Convert to API response
        let response = ServiceResponse {
            service: Some(service.into()),
        };
        
        Ok(Response::new(response))
    }

    async fn get_service(
        &self,
        request: Request<GetServiceRequest>,
    ) -> Result<Response<ServiceResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/service",
            "read",
        ).await?;
        
        // Get service from storage
        let service_entry: Service = self.storage.get_entity(&req.id).await.map_err(|e| {
            if e.to_string().contains("not found") {
                Status::not_found(format!("Service not found: {}", req.id))
            } else {
                error!("Failed to get service: {}", e);
                Status::internal(format!("Failed to get service: {}", e))
            }
        })?;
        
        // Get the service's roles for a complete response
        let roles = self.rbac_manager.get_principal_roles(
            &service_entry.id,
            &PrincipalType::Service,
        ).await.map_err(|e| {
            error!("Failed to get service roles: {}", e);
            Status::internal(format!("Failed to get service roles: {}", e))
        })?;
        
        // Extract role IDs
        let role_ids: Vec<String> = roles.iter().map(|r| r.id.clone()).collect();
        
        // Convert to API response
        let mut api_service: service_response::Service = service_entry.into();
        api_service.roles = role_ids;
        
        let response = ServiceResponse {
            service: Some(api_service),
        };
        
        Ok(Response::new(response))
    }

    async fn update_service(
        &self,
        request: Request<UpdateServiceRequest>,
    ) -> Result<Response<ServiceResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/service",
            "update",
        ).await?;
        
        // Update service
        let service = self.admin_manager.update_service(
            req.clone(),
            &principal_id,
        ).await.map_err(|e| {
            if e.to_string().contains("not found") {
                Status::not_found(format!("Service not found: {}", req.id))
            } else {
                error!("Failed to update service: {}", e);
                Status::internal(format!("Failed to update service: {}", e))
            }
        })?;
        
        // Convert to API response
        let response = ServiceResponse {
            service: Some(service.into()),
        };
        
        Ok(Response::new(response))
    }

    async fn delete_service(
        &self,
        request: Request<DeleteServiceRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/service",
            "delete",
        ).await?;
        
        // Delete service
        self.admin_manager.delete_service(
            &req.id,
            &principal_id,
        ).await.map_err(|e| {
            if e.to_string().contains("not found") {
                Status::not_found(format!("Service not found: {}", req.id))
            } else {
                error!("Failed to delete service: {}", e);
                Status::internal(format!("Failed to delete service: {}", e))
            }
        })?;
        
        // Revoke all tokens for this service
        if let Ok(manager) = TokenRotationManager::get_global().await {
            if let Err(e) = manager.revoke_all_tokens(
                &req.id,
                "service",
                "Service deleted",
                Some(&principal_id),
            ).await {
                warn!("Failed to revoke tokens for deleted service: {}", e);
            }
        }
        
        // Deregister from service mesh
        if let Err(e) = service_mesh::deregister_service(&req.id, "*").await {
            warn!("Failed to deregister service from service mesh: {}", e);
        }
        
        Ok(Response::new(()))
    }

    async fn list_services(
        &self,
        request: Request<ListServicesRequest>,
    ) -> Result<Response<ListServicesResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/service",
            "list",
        ).await?;
        
        // List services
        let (services, next_page_token, total_count) = self.admin_manager.list_services(
            req.clone(),
        ).await.map_err(|e| {
            error!("Failed to list services: {}", e);
            Status::internal(format!("Failed to list services: {}", e))
        })?;
        
        // Convert to API response
        let response = ListServicesResponse {
            services: services.into_iter().map(|service| service.into()).collect(),
            next_page_token,
            total_count,
        };
        
        Ok(Response::new(response))
    }

    // Certificate management
    async fn generate_service_certificate(
        &self,
        request: Request<GenerateCertRequest>,
    ) -> Result<Response<CertificateResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/certificate",
            "create",
        ).await?;
        
        // Generate certificate
        let cert = self.admin_manager.generate_service_certificate(
            &req.service_id,
            req.valid_days.map(|d| d as u32),
            if req.san_dns.is_empty() { None } else { Some(req.san_dns.clone()) },
            &principal_id,
        ).await.map_err(|e| {
            error!("Failed to generate certificate: {}", e);
            Status::internal(format!("Failed to generate certificate: {}", e))
        })?;
        
        Ok(Response::new(cert))
    }

    async fn revoke_certificate(
        &self,
        request: Request<RevokeCertRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/certificate",
            "revoke",
        ).await?;
        
        // Revoke certificate
        self.admin_manager.revoke_certificate(
            &req.cert_id,
            &req.reason,
            &principal_id,
        ).await.map_err(|e| {
            if e.to_string().contains("not found") {
                Status::not_found(format!("Certificate not found: {}", req.cert_id))
            } else {
                error!("Failed to revoke certificate: {}", e);
                Status::internal(format!("Failed to revoke certificate: {}", e))
            }
        })?;
        
        Ok(Response::new(()))
    }

    async fn get_cert_status(
        &self,
        request: Request<GetCertStatusRequest>,
    ) -> Result<Response<CertStatusResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/certificate",
            "read",
        ).await?;
        
        // Get certificate information
        let cert_info = self.storage.get_entity::<crate::certificates::CertificateInfo>(&req.cert_id).await
            .map_err(|e| {
                if e.to_string().contains("not found") {
                    Status::not_found(format!("Certificate not found: {}", req.cert_id))
                } else {
                    error!("Failed to get certificate: {}", e);
                    Status::internal(format!("Failed to get certificate: {}", e))
                }
            })?;
            
        // Check if revoked
        let is_revoked = self.cert_manager.is_certificate_revoked(&cert_info.serial_number).await;
        
        // Check if expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        let is_expired = cert_info.not_after < now;
        
        // Determine status
        let status = if cert_info.revoked {
            CertStatus::Revoked
        } else if is_expired {
            CertStatus::Expired
        } else {
            CertStatus::Valid
        };
        
        // Create response
        let response = CertStatusResponse {
            cert_id: cert_info.id,
            status: status as i32,
            revocation_reason: cert_info.revocation_reason,
            revocation_time: cert_info.revoked_at,
            not_before: cert_info.not_before,
            not_after: cert_info.not_after,
            is_expired,
        };
        
        Ok(Response::new(response))
    }

    // Audit logging
    async fn get_audit_logs(
        &self,
        request: Request<GetAuditLogsRequest>,
    ) -> Result<Response<GetAuditLogsResponse>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check - Audit logs are sensitive
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/audit",
            "read",
        ).await?;
        
        // Get audit logs
        let (logs, next_page_token, total_count) = self.admin_manager.get_audit_logs(
            req.clone(),
        ).await.map_err(|e| {
            error!("Failed to get audit logs: {}", e);
            Status::internal(format!("Failed to get audit logs: {}", e))
        })?;
        
        // Convert logs to API format
        let api_logs: Vec<AuditLog> = logs.into_iter()
            .map(|log| AuditLog {
                id: log.id,
                event_type: log.event_type.to_string(),
                actor: log.principal_id,
                action: log.action,
                resource_type: log.resource.clone(),
                resource_id: log.resource,
                service_id: log.service_id.unwrap_or_else(|| "auth-service".to_string()),
                metadata: log.metadata.clone(),
                status: log.outcome.to_string(),
                timestamp: log.timestamp.timestamp(),
                client_ip: log.source_ip.unwrap_or_default(),
                session_id: log.request_id.clone(),
                request_id: log.request_id,
            })
            .collect();
            
        // Create response
        let response = GetAuditLogsResponse {
            logs: api_logs,
            next_page_token,
            total_count,
        };
        
        Ok(Response::new(response))
    }

    async fn push_audit_log(
        &self,
        request: Request<PushAuditLogRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.get_ref();
        
        // Security check: This API can only be called with authentication
        let (principal_id, principal_type, _) = self.get_authenticated_principal(&request).await?;
        
        // Permission check
        self.check_principal_permission(
            &principal_id,
            &principal_type,
            "auth/audit",
            "write",
        ).await?;
        
        // Create metadata
        let mut metadata = req.metadata.clone();
        
        // Enrich with authenticated principal
        metadata.insert("logged_by".to_string(), principal_id.clone());
        
        // Log the event through audit manager
        audit::get_manager().await.map_err(|e| {
            error!("Failed to get audit manager: {}", e);
            Status::internal("Audit service unavailable")
        })?.log_event(
            EventType::from(req.event_type.as_str()),
            &req.actor,
            if req.actor.starts_with("service-") { "service" } else { "user" },
            &req.resource_type,
            &req.action,
            Outcome::from(req.status.as_str()),
            &req.metadata.get("message").unwrap_or(&req.action),
            Some(metadata.clone()),
            None,
            req.client_ip.as_deref(),
            None,
            req.request_id.as_deref(),
        ).await.map_err(|e| {
            error!("Failed to log audit event: {}", e);
            Status::internal("Failed to log audit event")
        })?;
        
        Ok(Response::new(()))
    }

    // Health check
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        // This is a basic health check endpoint that doesn't require authentication
        
        // Check dependencies
        let storage_healthy = self.storage.is_healthy().await;
        
        let token_manager_healthy = true; // Internal component
        
        let secrets_healthy = if let Some(client) = &self.secrets_client {
            client.is_healthy().await
        } else {
            false // Secrets service not available
        };
        
        // Determine overall status
        let status = if storage_healthy && token_manager_healthy {
            // Core components are healthy
            if secrets_healthy {
                "SERVING"
            } else {
                "SERVING_WITH_ISSUES" // Can operate but with limited functionality
            }
        } else {
            "NOT_SERVING"
        };
        
        // Create response with details
        let mut details = HashMap::new();
        details.insert("storage".to_string(), if storage_healthy { "healthy" } else { "unhealthy" }.to_string());
        details.insert("token_manager".to_string(), if token_manager_healthy { "healthy" } else { "unhealthy" }.to_string());
        details.insert("secrets".to_string(), if secrets_healthy { "healthy" } else { "unhealthy" }.to_string());
        
        // Return response
        let response = HealthCheckResponse {
            status: status.to_string(),
            details,
        };
        
        Ok(Response::new(response))
    }
}