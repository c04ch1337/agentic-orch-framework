// auth-service-rs/src/admin.rs
//
// Administrative APIs and interfaces for the auth service
// Provides:
// - User management
// - Role and permission management
// - Service registration
// - Token management
// - Certificate management

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};
use anyhow::{Result, anyhow, Context};
use tracing::{debug, error, info, warn};

use crate::proto::auth_service::*;
use crate::rbac::{RbacManager, Role, Permission, PermissionEffect, PrincipalType};
use crate::certificates::{CertificateManager, CertificateType};
use crate::jwt::TokenManager;
use crate::audit;
use crate::delegation::TokenDelegator;
use crate::storage::{StorageBackend, Entity};

// User entity for admin API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub status: UserStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_login: Option<i64>,
    pub metadata: HashMap<String, String>,
}

impl Entity for User {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_entity_type() -> &'static str {
        "user"
    }
}

// User status enum
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UserStatus {
    Active,
    Inactive,
    Locked,
    Pending,
}

impl From<i32> for UserStatus {
    fn from(val: i32) -> Self {
        match val {
            0 => UserStatus::Active,
            1 => UserStatus::Inactive,
            2 => UserStatus::Locked,
            3 => UserStatus::Pending,
            _ => UserStatus::Inactive,
        }
    }
}

impl From<UserStatus> for i32 {
    fn from(status: UserStatus) -> Self {
        match status {
            UserStatus::Active => 0,
            UserStatus::Inactive => 1,
            UserStatus::Locked => 2,
            UserStatus::Pending => 3,
        }
    }
}

// Service entity for admin API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: ServiceStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub allowed_origins: Vec<String>,
    pub allowed_redirect_urls: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl Entity for Service {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_entity_type() -> &'static str {
        "service"
    }
}

// Service status enum
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Active,
    Inactive,
    Maintenance,
    Deprecated,
}

impl From<i32> for ServiceStatus {
    fn from(val: i32) -> Self {
        match val {
            0 => ServiceStatus::Active,
            1 => ServiceStatus::Inactive,
            2 => ServiceStatus::Maintenance,
            3 => ServiceStatus::Deprecated,
            _ => ServiceStatus::Inactive,
        }
    }
}

impl From<ServiceStatus> for i32 {
    fn from(status: ServiceStatus) -> Self {
        match status {
            ServiceStatus::Active => 0,
            ServiceStatus::Inactive => 1,
            ServiceStatus::Maintenance => 2,
            ServiceStatus::Deprecated => 3,
        }
    }
}

/// Administrative API manager
pub struct AdminManager {
    // Core services
    rbac_manager: Arc<RbacManager>,
    token_manager: Arc<TokenManager>,
    cert_manager: Arc<CertificateManager>,
    storage: Arc<dyn StorageBackend>,
    
    // Optional token delegator
    token_delegator: Option<Arc<TokenDelegator>>,
    
    // Password hashing (when not using an external identity provider)
    password_hasher: Arc<Mutex<argon2::Argon2<'static>>>,
}

impl AdminManager {
    /// Create a new admin manager
    pub async fn new(
        rbac_manager: Arc<RbacManager>,
        token_manager: Arc<TokenManager>,
        cert_manager: Arc<CertificateManager>,
        storage: Arc<dyn StorageBackend>,
        token_delegator: Option<Arc<TokenDelegator>>,
    ) -> Result<Self> {
        // Initialize password hasher
        let argon2_config = argon2::Argon2::default();
        
        Ok(Self {
            rbac_manager,
            token_manager,
            cert_manager,
            storage,
            token_delegator,
            password_hasher: Arc::new(Mutex::new(argon2_config)),
        })
    }
    
    /// Create a new user
    pub async fn create_user(
        &self,
        request: CreateUserRequest,
        admin_id: &str,
    ) -> Result<User> {
        // Check for duplicate usernames
        let query = format!("username = '{}'", request.username);
        let existing = self.storage
            .query_entities::<User>(&query)
            .await
            .context("Failed to query existing users")?;
            
        if !existing.is_empty() {
            return Err(anyhow!("Username already exists"));
        }
        
        // Hash the password if provided
        let password_hash = if !request.password.is_empty() {
            let hasher = self.password_hasher.lock().await;
            let mut salt = [0u8; 16];
            rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut salt);
            
            let mut hash = [0u8; 32]; // 32 bytes for Argon2id output
            hasher.hash_password_into(
                request.password.as_bytes(),
                &salt,
                &mut hash,
            ).map_err(|e| anyhow!("Password hashing failed: {}", e))?;
            
            // Format as salted hash
            let mut salted_hash = Vec::with_capacity(salt.len() + hash.len());
            salted_hash.extend_from_slice(&salt);
            salted_hash.extend_from_slice(&hash);
            
            Some(base64::encode(salted_hash))
        } else {
            None
        };
        
        // Create user record
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        let user_id = uuid::Uuid::new_v4().to_string();
        
        let mut metadata = HashMap::new();
        if let Some(hash) = password_hash {
            metadata.insert("password_hash".to_string(), hash);
        }
        
        let user = User {
            id: user_id.clone(),
            username: request.username.clone(),
            email: request.email.clone(),
            status: UserStatus::from(request.status),
            created_at: now,
            updated_at: now,
            last_login: None,
            metadata,
        };
        
        // Store user
        self.storage.store_entity(&user).await
            .context("Failed to store user")?;
            
        // Assign roles
        for role_id in request.roles {
            self.rbac_manager
                .assign_role(&user_id, PrincipalType::User, &role_id, admin_id, None)
                .await
                .context("Failed to assign role")?;
                
            // Log the role assignment
            audit::log_role_assigned(&user_id, "user", &role_id, admin_id).await.ok();
        }
        
        // Log user creation
        audit::log_system_event(
            audit::EventType::UserCreated,
            &format!("User {} created by {}", user.username, admin_id),
            Some(HashMap::from([
                ("user_id".to_string(), user.id.clone()),
                ("created_by".to_string(), admin_id.to_string()),
            ])),
        ).await.ok();
        
        info!("User {} created by {}", user.username, admin_id);
        
        Ok(user)
    }
    
    /// Get a user by ID
    pub async fn get_user(&self, user_id: &str) -> Result<User> {
        self.storage
            .get_entity::<User>(user_id)
            .await
            .context("Failed to get user")
    }
    
    /// Update a user
    pub async fn update_user(
        &self,
        request: UpdateUserRequest,
        admin_id: &str,
    ) -> Result<User> {
        // Get existing user
        let mut user = self.storage
            .get_entity::<User>(&request.id)
            .await
            .context("Failed to get user")?;
            
        // Update fields
        let mut updated = false;
        
        if !request.username.is_empty() && request.username != user.username {
            // Check for username conflicts
            let query = format!("username = '{}'", request.username);
            let existing = self.storage
                .query_entities::<User>(&query)
                .await
                .context("Failed to query existing users")?;
                
            if !existing.is_empty() {
                return Err(anyhow!("Username already exists"));
            }
            
            user.username = request.username;
            updated = true;
        }
        
        if !request.email.is_empty() && request.email != user.email {
            user.email = request.email;
            updated = true;
        }
        
        if request.status != 0 {
            user.status = UserStatus::from(request.status);
            updated = true;
        }
        
        // Update password if provided
        if !request.password.is_empty() {
            let hasher = self.password_hasher.lock().await;
            let mut salt = [0u8; 16];
            rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut salt);
            
            let mut hash = [0u8; 32];
            hasher.hash_password_into(
                request.password.as_bytes(),
                &salt,
                &mut hash,
            ).map_err(|e| anyhow!("Password hashing failed: {}", e))?;
            
            // Format as salted hash
            let mut salted_hash = Vec::with_capacity(salt.len() + hash.len());
            salted_hash.extend_from_slice(&salt);
            salted_hash.extend_from_slice(&hash);
            
            user.metadata.insert(
                "password_hash".to_string(), 
                base64::encode(salted_hash)
            );
            updated = true;
        }
        
        if updated {
            // Update timestamp
            user.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
                
            // Store updated user
            self.storage.store_entity(&user).await
                .context("Failed to update user")?;
                
            // Log update
            audit::log_system_event(
                audit::EventType::UserUpdated,
                &format!("User {} updated by {}", user.username, admin_id),
                Some(HashMap::from([
                    ("user_id".to_string(), user.id.clone()),
                    ("updated_by".to_string(), admin_id.to_string()),
                ])),
            ).await.ok();
            
            info!("User {} updated by {}", user.username, admin_id);
        }
        
        // Update roles if provided
        if !request.roles.is_empty() {
            // Get current role assignments
            let current_roles = self.rbac_manager
                .get_principal_roles(&user.id, &PrincipalType::User)
                .await
                .context("Failed to get user roles")?;
                
            // Calculate roles to add and remove
            let current_role_ids: Vec<String> = current_roles.iter()
                .map(|r| r.id.clone())
                .collect();
                
            let new_role_ids: Vec<String> = request.roles.into_iter()
                .collect();
                
            // Roles to add (in new but not in current)
            let roles_to_add: Vec<String> = new_role_ids.iter()
                .filter(|id| !current_role_ids.contains(id))
                .cloned()
                .collect();
                
            // Roles to remove (in current but not in new)
            let roles_to_remove: Vec<String> = current_role_ids.iter()
                .filter(|id| !new_role_ids.contains(id))
                .cloned()
                .collect();
                
            // Add new roles
            for role_id in roles_to_add {
                self.rbac_manager
                    .assign_role(&user.id, PrincipalType::User, &role_id, admin_id, None)
                    .await
                    .context("Failed to assign role")?;
                    
                audit::log_role_assigned(&user.id, "user", &role_id, admin_id).await.ok();
            }
            
            // Remove roles
            for role_id in roles_to_remove {
                self.rbac_manager
                    .revoke_role(&user.id, PrincipalType::User, &role_id)
                    .await
                    .context("Failed to revoke role")?;
                    
                audit::log_role_revoked(&user.id, "user", &role_id, admin_id).await.ok();
            }
        }
        
        Ok(user)
    }
    
    /// Delete a user
    pub async fn delete_user(
        &self,
        user_id: &str,
        admin_id: &str,
    ) -> Result<()> {
        // Get user first (to ensure it exists and to get username for logging)
        let user = self.storage
            .get_entity::<User>(user_id)
            .await
            .context("Failed to get user")?;
            
        // Revoke all roles
        let roles = self.rbac_manager
            .get_principal_roles(user_id, &PrincipalType::User)
            .await
            .context("Failed to get user roles")?;
            
        for role in roles {
            self.rbac_manager
                .revoke_role(user_id, PrincipalType::User, &role.id)
                .await
                .context("Failed to revoke role")?;
        }
        
        // Delete user
        self.storage
            .delete_entity::<User>(user_id)
            .await
            .context("Failed to delete user")?;
            
        // Log deletion
        audit::log_system_event(
            audit::EventType::UserDeleted,
            &format!("User {} deleted by {}", user.username, admin_id),
            Some(HashMap::from([
                ("user_id".to_string(), user_id.to_string()),
                ("deleted_by".to_string(), admin_id.to_string()),
            ])),
        ).await.ok();
        
        info!("User {} deleted by {}", user.username, admin_id);
        
        Ok(())
    }
    
    /// List users with pagination and filtering
    pub async fn list_users(
        &self,
        request: ListUsersRequest,
    ) -> Result<(Vec<User>, Option<String>, i32)> {
        // Build query from filter
        let query = if !request.filter.is_empty() {
            Some(request.filter.as_str())
        } else {
            None
        };
        
        // Get page size with default
        let page_size = if request.page_size > 0 {
            request.page_size as usize
        } else {
            100 // Default page size
        };
        
        // Calculate offset from page token
        let offset = if !request.page_token.is_empty() {
            // Page tokens are offsets
            request.page_token.parse::<usize>().unwrap_or(0)
        } else {
            0
        };
        
        // Query users
        let users = self.storage
            .query_entities_paged::<User>(
                query,
                page_size,
                offset,
                Some("username ASC"),
            )
            .await
            .context("Failed to query users")?;
            
        // Get total count (for pagination info)
        let total_count = self.storage
            .count_entities::<User>(query)
            .await
            .context("Failed to count users")?;
            
        // Calculate next page token
        let next_page_token = if users.len() == page_size && offset + page_size < total_count {
            Some((offset + page_size).to_string())
        } else {
            None
        };
        
        Ok((users, next_page_token, total_count as i32))
    }
    
    /// Create a new role
    pub async fn create_role(
        &self,
        request: CreateRoleRequest,
        admin_id: &str,
    ) -> Result<Role> {
        // Check for duplicate role names
        let query = format!("name = '{}'", request.name);
        let existing = self.rbac_manager
            .list_roles_by_query(&query)
            .await
            .context("Failed to query existing roles")?;
            
        if !existing.is_empty() {
            return Err(anyhow!("Role name already exists"));
        }
        
        // Convert permissions from proto to internal model
        let mut permissions = Vec::new();
        for perm in request.permissions {
            // Extract permission components (format: resource:action)
            let parts: Vec<&str> = perm.split(':').collect();
            if parts.len() < 2 {
                return Err(anyhow!("Invalid permission format: {}", perm));
            }
            
            let resource = parts[0];
            let action = parts[1];
            
            permissions.push(Permission {
                resource_pattern: resource.to_string(),
                actions: [action.to_string()].iter().cloned().collect(),
                attributes: HashMap::new(),
                effect: PermissionEffect::Allow,
            });
        }
        
        // Create role
        let role_id = uuid::Uuid::new_v4().to_string();
        let role = Role {
            id: role_id,
            name: request.name.clone(),
            description: request.description.clone(),
            permissions,
            parent_roles: Vec::new(),  // No parent roles for new roles
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        // Store role
        self.rbac_manager
            .create_role(role.clone())
            .await
            .context("Failed to create role")?;
            
        // Log role creation
        audit::log_system_event(
            audit::EventType::Other,
            &format!("Role {} created by {}", role.name, admin_id),
            Some(HashMap::from([
                ("role_id".to_string(), role.id.clone()),
                ("created_by".to_string(), admin_id.to_string()),
            ])),
        ).await.ok();
        
        info!("Role {} created by {}", role.name, admin_id);
        
        Ok(role)
    }
    
    /// Update a role
    pub async fn update_role(
        &self,
        request: UpdateRoleRequest,
        admin_id: &str,
    ) -> Result<Role> {
        // Get existing role
        let mut role = self.rbac_manager
            .get_role(&request.id)
            .await
            .context("Failed to get role")?;
            
        // Update fields
        let mut updated = false;
        
        if !request.name.is_empty() && request.name != role.name {
            // Check for name conflicts
            let query = format!("name = '{}'", request.name);
            let existing = self.rbac_manager
                .list_roles_by_query(&query)
                .await
                .context("Failed to query existing roles")?;
                
            if !existing.is_empty() && existing[0].id != role.id {
                return Err(anyhow!("Role name already exists"));
            }
            
            role.name = request.name;
            updated = true;
        }
        
        if !request.description.is_empty() && request.description != role.description {
            role.description = request.description;
            updated = true;
        }
        
        // Update permissions if provided
        if !request.permissions.is_empty() {
            // Convert permissions from proto to internal model
            let mut permissions = Vec::new();
            for perm in request.permissions {
                // Extract permission components
                let parts: Vec<&str> = perm.split(':').collect();
                if parts.len() < 2 {
                    return Err(anyhow!("Invalid permission format: {}", perm));
                }
                
                let resource = parts[0];
                let action = parts[1];
                
                permissions.push(Permission {
                    resource_pattern: resource.to_string(),
                    actions: [action.to_string()].iter().cloned().collect(),
                    attributes: HashMap::new(),
                    effect: PermissionEffect::Allow,
                });
            }
            
            role.permissions = permissions;
            updated = true;
        }
        
        if updated {
            // Update timestamp
            role.updated_at = chrono::Utc::now();
            
            // Update role
            self.rbac_manager
                .update_role(&request.id, role.clone())
                .await
                .context("Failed to update role")?;
                
            // Log update
            audit::log_system_event(
                audit::EventType::Other,
                &format!("Role {} updated by {}", role.name, admin_id),
                Some(HashMap::from([
                    ("role_id".to_string(), role.id.clone()),
                    ("updated_by".to_string(), admin_id.to_string()),
                ])),
            ).await.ok();
            
            info!("Role {} updated by {}", role.name, admin_id);
        }
        
        Ok(role)
    }
    
    /// Delete a role
    pub async fn delete_role(
        &self,
        role_id: &str,
        admin_id: &str,
    ) -> Result<()> {
        // Get role first (for logging)
        let role = self.rbac_manager
            .get_role(role_id)
            .await
            .context("Failed to get role")?;
            
        // Delete role
        self.rbac_manager
            .delete_role(role_id)
            .await
            .context("Failed to delete role")?;
            
        // Log deletion
        audit::log_system_event(
            audit::EventType::Other,
            &format!("Role {} deleted by {}", role.name, admin_id),
            Some(HashMap::from([
                ("role_id".to_string(), role_id.to_string()),
                ("deleted_by".to_string(), admin_id.to_string()),
            ])),
        ).await.ok();
        
        info!("Role {} deleted by {}", role.name, admin_id);
        
        Ok(())
    }
    
    /// List roles with filtering
    pub async fn list_roles(
        &self,
        request: ListRolesRequest,
    ) -> Result<(Vec<Role>, Option<String>, i32)> {
        // Build query from filter
        let query = if !request.filter.is_empty() {
            Some(request.filter.as_str())
        } else {
            None
        };
        
        // Query roles
        let roles = if query.is_some() {
            self.rbac_manager
                .list_roles_by_query(query.unwrap())
                .await
                .context("Failed to query roles")?
        } else {
            self.rbac_manager
                .list_roles()
                .await
                .context("Failed to list roles")?
        };
        
        // Simple pagination for roles (in-memory)
        let page_size = if request.page_size > 0 {
            request.page_size as usize
        } else {
            100 // Default
        };
        
        let offset = if !request.page_token.is_empty() {
            request.page_token.parse::<usize>().unwrap_or(0)
        } else {
            0
        };
        
        let total_count = roles.len();
        
        let paged_roles = if offset < roles.len() {
            let end = (offset + page_size).min(roles.len());
            roles[offset..end].to_vec()
        } else {
            Vec::new()
        };
        
        // Calculate next page token
        let next_page_token = if offset + page_size < total_count {
            Some((offset + page_size).to_string())
        } else {
            None
        };
        
        Ok((paged_roles, next_page_token, total_count as i32))
    }
    
    /// Add a user to a role
    pub async fn add_user_to_role(
        &self,
        user_id: &str,
        role_id: &str,
        admin_id: &str,
    ) -> Result<()> {
        // Assign the role
        self.rbac_manager
            .assign_role(user_id, PrincipalType::User, role_id, admin_id, None)
            .await
            .context("Failed to assign role")?;
            
        // Log the assignment
        audit::log_role_assigned(user_id, "user", role_id, admin_id).await.ok();
        
        Ok(())
    }
    
    /// Remove a user from a role
    pub async fn remove_user_from_role(
        &self,
        user_id: &str,
        role_id: &str,
        admin_id: &str,
    ) -> Result<()> {
        // Revoke the role
        self.rbac_manager
            .revoke_role(user_id, PrincipalType::User, role_id)
            .await
            .context("Failed to revoke role")?;
            
        // Log the revocation
        audit::log_role_revoked(user_id, "user", role_id, admin_id).await.ok();
        
        Ok(())
    }
    
    /// Register a service
    pub async fn register_service(
        &self,
        request: RegisterServiceRequest,
        admin_id: &str,
    ) -> Result<Service> {
        // Check for duplicate service names
        let query = format!("name = '{}'", request.name);
        let existing = self.storage
            .query_entities::<Service>(&query)
            .await
            .context("Failed to query existing services")?;
            
        if !existing.is_empty() {
            return Err(anyhow!("Service name already exists"));
        }
        
        // Create service record
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        let service_id = uuid::Uuid::new_v4().to_string();
        
        let service = Service {
            id: service_id.clone(),
            name: request.name.clone(),
            description: request.description.clone(),
            status: ServiceStatus::Active, // Default to active
            created_at: now,
            updated_at: now,
            allowed_origins: request.allowed_origins.clone(),
            allowed_redirect_urls: request.allowed_redirect_urls.clone(),
            metadata: HashMap::new(),
        };
        
        // Store service
        self.storage.store_entity(&service).await
            .context("Failed to store service")?;
            
        // Assign roles
        for role_id in request.roles {
            self.rbac_manager
                .assign_role(&service_id, PrincipalType::Service, &role_id, admin_id, None)
                .await
                .context("Failed to assign role")?;
                
            audit::log_role_assigned(&service_id, "service", &role_id, admin_id).await.ok();
        }
        
        // Generate certificate for the service if mTLS is enabled
        let cert_result = self.cert_manager
            .get_or_create_service_certificate(&service_id, None, None)
            .await;
            
        if let Ok((cert_pem, key_pem, _)) = cert_result {
            // Store certificate information in metadata for reference
            let mut service_update = service.clone();
            service_update.metadata.insert("has_certificate".to_string(), "true".to_string());
            
            // Store updated service
            self.storage.store_entity(&service_update).await.ok();
            
            // Log certificate generation
            info!("Generated mTLS certificate for service {}", service_id);
        }
        
        // Log service registration
        audit::log_system_event(
            audit::EventType::ServiceRegistered,
            &format!("Service {} registered by {}", service.name, admin_id),
            Some(HashMap::from([
                ("service_id".to_string(), service_id.clone()),
                ("registered_by".to_string(), admin_id.to_string()),
            ])),
        ).await.ok();
        
        info!("Service {} registered by {}", service.name, admin_id);
        
        Ok(service)
    }
    
    /// Update a service
    pub async fn update_service(
        &self,
        request: UpdateServiceRequest,
        admin_id: &str,
    ) -> Result<Service> {
        // Get existing service
        let mut service = self.storage
            .get_entity::<Service>(&request.id)
            .await
            .context("Failed to get service")?;
            
        // Update fields
        let mut updated = false;
        
        if !request.name.is_empty() && request.name != service.name {
            // Check for name conflicts
            let query = format!("name = '{}'", request.name);
            let existing = self.storage
                .query_entities::<Service>(&query)
                .await
                .context("Failed to query existing services")?;
                
            if !existing.is_empty() && existing[0].id != service.id {
                return Err(anyhow!("Service name already exists"));
            }
            
            service.name = request.name;
            updated = true;
        }
        
        if !request.description.is_empty() && request.description != service.description {
            service.description = request.description;
            updated = true;
        }
        
        if request.status != 0 && ServiceStatus::from(request.status) != service.status {
            service.status = ServiceStatus::from(request.status);
            updated = true;
        }
        
        if !request.allowed_origins.is_empty() {
            service.allowed_origins = request.allowed_origins;
            updated = true;
        }
        
        if !request.allowed_redirect_urls.is_empty() {
            service.allowed_redirect_urls = request.allowed_redirect_urls;
            updated = true;
        }
        
        if updated {
            // Update timestamp
            service.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
                
            // Store updated service
            self.storage.store_entity(&service).await
                .context("Failed to update service")?;
                
            // Log update
            audit::log_system_event(
                audit::EventType::Other,
                &format!("Service {} updated by {}", service.name, admin_id),
                Some(HashMap::from([
                    ("service_id".to_string(), service.id.clone()),
                    ("updated_by".to_string(), admin_id.to_string()),
                ])),
            ).await.ok();
            
            info!("Service {} updated by {}", service.name, admin_id);
        }
        
        // Update roles if provided
        if !request.roles.is_empty() {
            // Get current role assignments
            let current_roles = self.rbac_manager
                .get_principal_roles(&service.id, &PrincipalType::Service)
                .await
                .context("Failed to get service roles")?;
                
            // Calculate roles to add and remove
            let current_role_ids: Vec<String> = current_roles.iter()
                .map(|r| r.id.clone())
                .collect();
                
            let new_role_ids: Vec<String> = request.roles.into_iter()
                .collect();
                
            // Roles to add
            let roles_to_add: Vec<String> = new_role_ids.iter()
                .filter(|id| !current_role_ids.contains(id))
                .cloned()
                .collect();
                
            // Roles to remove
            let roles_to_remove: Vec<String> = current_role_ids.iter()
                .filter(|id| !new_role_ids.contains(id))
                .cloned()
                .collect();
                
            // Add new roles
            for role_id in roles_to_add {
                self.rbac_manager
                    .assign_role(&service.id, PrincipalType::Service, &role_id, admin_id, None)
                    .await
                    .context("Failed to assign role")?;
                    
                audit::log_role_assigned(&service.id, "service", &role_id, admin_id).await.ok();
            }
            
            // Remove roles
            for role_id in roles_to_remove {
                self.rbac_manager
                    .revoke_role(&service.id, PrincipalType::Service, &role_id)
                    .await
                    .context("Failed to revoke role")?;
                    
                audit::log_role_revoked(&service.id, "service", &role_id, admin_id).await.ok();
            }
        }
        
        Ok(service)
    }
    
    /// Delete a service
    pub async fn delete_service(
        &self,
        service_id: &str,
        admin_id: &str,
    ) -> Result<()> {
        // Get service first
        let service = self.storage
            .get_entity::<Service>(service_id)
            .await
            .context("Failed to get service")?;
            
        // Revoke all roles
        let roles = self.rbac_manager
            .get_principal_roles(service_id, &PrincipalType::Service)
            .await
            .context("Failed to get service roles")?;
            
        for role in roles {
            self.rbac_manager
                .revoke_role(service_id, PrincipalType::Service, &role.id)
                .await
                .context("Failed to revoke role")?;
        }
        
        // Delete service
        self.storage
            .delete_entity::<Service>(service_id)
            .await
            .context("Failed to delete service")?;
            
        // Log deletion
        audit::log_system_event(
            audit::EventType::ServiceDeleted,
            &format!("Service {} deleted by {}", service.name, admin_id),
            Some(HashMap::from([
                ("service_id".to_string(), service_id.to_string()),
                ("deleted_by".to_string(), admin_id.to_string()),
            ])),
        ).await.ok();
        
        info!("Service {} deleted by {}", service.name, admin_id);
        
        Ok(())
    }
    
    /// List services with pagination and filtering
    pub async fn list_services(
        &self,
        request: ListServicesRequest,
    ) -> Result<(Vec<Service>, Option<String>, i32)> {
        // Build query from filter
        let query = if !request.filter.is_empty() {
            Some(request.filter.as_str())
        } else {
            None
        };
        
        // Get page size with default
        let page_size = if request.page_size > 0 {
            request.page_size as usize
        } else {
            100 // Default page size
        };
        
        // Calculate offset from page token
        let offset = if !request.page_token.is_empty() {
            request.page_token.parse::<usize>().unwrap_or(0)
        } else {
            0
        };
        
        // Query services
        let services = self.storage
            .query_entities_paged::<Service>(
                query,
                page_size,
                offset,
                Some("name ASC"),
            )
            .await
            .context("Failed to query services")?;
            
        // Get total count
        let total_count = self.storage
            .count_entities::<Service>(query)
            .await
            .context("Failed to count services")?;
            
        // Calculate next page token
        let next_page_token = if services.len() == page_size && offset + page_size < total_count {
            Some((offset + page_size).to_string())
        } else {
            None
        };
        
        Ok((services, next_page_token, total_count as i32))
    }
    
    /// Generate service certificate
    pub async fn generate_service_certificate(
        &self,
        service_id: &str,
        valid_days: Option<u32>,
        san_dns: Option<Vec<String>>,
        admin_id: &str,
    ) -> Result<CertificateResponse> {
        // Check if service exists
        let service = self.storage
            .get_entity::<Service>(service_id)
            .await
            .context("Failed to get service")?;
            
        // Generate certificate
        let (cert_pem, key_pem, ca_cert_pem) = self.cert_manager
            .get_or_create_service_certificate(
                service_id,
                valid_days,
                san_dns,
            )
            .await
            .context("Failed to generate certificate")?;
            
        // Update service metadata
        let mut updated_service = service.clone();
        updated_service.metadata.insert("has_certificate".to_string(), "true".to_string());
        updated_service.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        self.storage.store_entity(&updated_service).await
            .context("Failed to update service metadata")?;
            
        // Log certificate generation
        audit::log_system_event(
            audit::EventType::Other,
            &format!("Certificate generated for service {} by {}", service_id, admin_id),
            Some(HashMap::from([
                ("service_id".to_string(), service_id.to_string()),
                ("generated_by".to_string(), admin_id.to_string()),
            ])),
        ).await.ok();
        
        info!("Certificate generated for service {} by {}", service_id, admin_id);
        
        // Create response
        let cert_resp = CertificateResponse {
            cert_pem,
            key_pem,
            cert_id: "".to_string(), // Not returned for security
            not_before: 0,
            not_after: 0,
            subject: format!("CN={}", service_id),
            issuer: "Phoenix ORCH AGI CA".to_string(),
            san_dns: Vec::new(),
            san_ips: Vec::new(),
            fingerprint: "".to_string(),
        };
        
        Ok(cert_resp)
    }
    
    /// Revoke certificate
    pub async fn revoke_certificate(
        &self,
        cert_id: &str,
        reason: &str,
        admin_id: &str,
    ) -> Result<()> {
        // Revoke certificate
        self.cert_manager
            .revoke_certificate(cert_id, reason)
            .await
            .context("Failed to revoke certificate")?;
            
        // Log revocation
        audit::log_system_event(
            audit::EventType::Other,
            &format!("Certificate {} revoked by {}: {}", cert_id, admin_id, reason),
            Some(HashMap::from([
                ("cert_id".to_string(), cert_id.to_string()),
                ("revoked_by".to_string(), admin_id.to_string()),
                ("reason".to_string(), reason.to_string()),
            ])),
        ).await.ok();
        
        info!("Certificate {} revoked by {}: {}", cert_id, admin_id, reason);
        
        Ok(())
    }
    
    /// Get audit logs
    pub async fn get_audit_logs(
        &self,
        request: GetAuditLogsRequest,
    ) -> Result<(Vec<audit::AuditLogEntry>, Option<String>, i32)> {
        // Build query
        let mut query_parts = Vec::new();
        
        if !request.filter.is_empty() {
            query_parts.push(request.filter.clone());
        }
        
        if request.start_time > 0 {
            query_parts.push(format!("timestamp >= {}", request.start_time));
        }
        
        if request.end_time > 0 {
            query_parts.push(format!("timestamp <= {}", request.end_time));
        }
        
        let query = if query_parts.is_empty() {
            None
        } else {
            Some(query_parts.join(" AND "))
        };
        
        // Get page size with default
        let page_size = if request.page_size > 0 {
            request.page_size as usize
        } else {
            100 // Default page size
        };
        
        // Calculate offset from page token
        let offset = if !request.page_token.is_empty() {
            request.page_token.parse::<usize>().unwrap_or(0)
        } else {
            0
        };
        
        // Determine sort order
        let sort = if request.sort_desc {
            "timestamp DESC"
        } else {
            "timestamp ASC"
        };
        
        // Query logs
        let audit_logs = self.storage
            .query_entities_paged::<audit::AuditLogEntry>(
                query.as_deref(),
                page_size,
                offset,
                Some(sort),
            )
            .await
            .context("Failed to query audit logs")?;
            
        // Get total count
        let total_count = self.storage
            .count_entities::<audit::AuditLogEntry>(query.as_deref())
            .await
            .context("Failed to count audit logs")?;
            
        // Calculate next page token
        let next_page_token = if audit_logs.len() == page_size && offset + page_size < total_count {
            Some((offset + page_size).to_string())
        } else {
            None
        };
        
        // Log the audit log access (meta-audit)
        audit::log_system_event(
            audit::EventType::LogAccessed,
            &format!("Audit logs accessed with filter: {}", request.filter),
            Some(HashMap::from([
                ("filter".to_string(), request.filter),
                ("count".to_string(), audit_logs.len().to_string()),
            ])),
        ).await.ok();
        
        Ok((audit_logs, next_page_token, total_count as i32))
    }
    
    /// Safe shutdown of the admin manager
    pub async fn shutdown(&self) -> Result<()> {
        // Flush any pending audit logs
        if let Ok(audit_manager) = audit::get_manager() {
            audit_manager.flush().await?;
        }
        
        info!("Admin manager shutdown complete");
        
        Ok(())
    }
}

// Convert internal user model to proto response
impl From<User> for user_response::User {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            roles: Vec::new(), // Roles need to be populated separately
            status: user.status.into(),
            created_at: user.created_at,
            updated_at: user.updated_at,
            last_login: user.last_login.unwrap_or(0),
        }
    }
}

// Convert internal role model to proto response
impl From<Role> for role_response::Role {
    fn from(role: Role) -> Self {
        // Convert permissions to string format
        let permissions: Vec<String> = role.permissions.iter()
            .flat_map(|p| {
                p.actions.iter().map(move |a| {
                    format!("{}:{}", p.resource_pattern, a)
                })
            })
            .collect();
        
        Self {
            id: role.id,
            name: role.name,
            description: role.description,
            permissions,
            users: Vec::new(), // Users need to be populated separately
            is_system_role: role.metadata.get("system_role").map_or(false, |v| v == "true"),
            created_at: role.created_at.timestamp(),
            updated_at: role.updated_at.timestamp(),
        }
    }
}

// Convert internal service model to proto response
impl From<Service> for service_response::Service {
    fn from(service: Service) -> Self {
        Self {
            id: service.id,
            name: service.name,
            description: service.description,
            roles: Vec::new(), // Roles need to be populated separately
            status: service.status.into(),
            allowed_redirect_urls: service.allowed_redirect_urls,
            allowed_origins: service.allowed_origins,
            created_at: service.created_at,
            updated_at: service.updated_at,
        }
    }
}