// auth-service-rs/src/rbac.rs
//
// Role-Based Access Control (RBAC) system with fine-grained permissions
// Provides:
// - Role definition and management
// - Permission assignment and checking
// - Hierarchical roles
// - Resource-based authorization
// - Action-based permissions

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow, Context};
use async_trait::async_trait;
use uuid::Uuid;
use regex::Regex;
use log::{debug, error, warn, info};
use chrono::{DateTime, Utc};

use crate::storage::{StorageBackend, StorageBackendExt, Entity};

/// Permission defines an action that can be performed on a resource
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Permission {
    pub resource_pattern: String,      // Resource pattern (supports wildcards and regex)
    pub actions: HashSet<String>,      // Allowed actions (read, write, delete, etc.)
    #[serde(default)]
    pub attributes: HashMap<String, String>,  // Additional attributes/constraints
    pub effect: PermissionEffect,      // Allow or deny
}

/// Permission effect types (allow or deny)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PermissionEffect {
    Allow,
    Deny,
}

impl Default for PermissionEffect {
    fn default() -> Self {
        PermissionEffect::Allow
    }
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: String,
    pub permissions: Vec<Permission>,
    pub parent_roles: Vec<String>,     // For role inheritance
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for Role {
    fn get_id(&self) -> String {
        self.id.clone()
    }
    
    fn get_entity_type() -> &'static str {
        "role"
    }
}

/// Role assignment for a principal (user or service)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAssignment {
    pub id: String,
    pub principal_id: String,          // User or service ID
    pub principal_type: PrincipalType, // User or service
    pub role_id: String,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: String,           // User or service that assigned the role
    pub expires_at: Option<DateTime<Utc>>, // Optional expiration
}

impl Entity for RoleAssignment {
    fn get_id(&self) -> String {
        self.id.clone()
    }
    
    fn get_entity_type() -> &'static str {
        "role_assignment"
    }
}

/// Principal types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PrincipalType {
    User,
    Service,
}

impl std::fmt::Display for PrincipalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrincipalType::User => write!(f, "user"),
            PrincipalType::Service => write!(f, "service"),
        }
    }
}

impl From<&str> for PrincipalType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "user" => PrincipalType::User,
            "service" => PrincipalType::Service,
            _ => PrincipalType::User, // Default to user for unknown types
        }
    }
}

/// Permission decision result
#[derive(Debug, Clone, Serialize)]
pub struct PermissionDecision {
    pub allowed: bool,
    pub resource: String,
    pub action: String,
    pub reason: String,
    pub role_id: Option<String>,
    pub permission_index: Option<usize>,
}

/// RBAC Manager - handles role and permission management
pub struct RbacManager {
    storage: Arc<dyn StorageBackend>,
    
    // Cache to reduce database access
    roles_cache: Arc<RwLock<HashMap<String, Role>>>, 
    assignments_cache: Arc<RwLock<HashMap<String, Vec<RoleAssignment>>>>,
    compiled_patterns: Arc<RwLock<HashMap<String, Regex>>>,
}

impl RbacManager {
    /// Create a new RBAC manager
    pub async fn new(storage: Arc<dyn StorageBackend>) -> Result<Self> {
        let rbac = Self {
            storage,
            roles_cache: Arc::new(RwLock::new(HashMap::new())),
            assignments_cache: Arc::new(RwLock::new(HashMap::new())),
            compiled_patterns: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Initialize system roles if they don't exist
        rbac.initialize_system_roles().await?;
        
        Ok(rbac)
    }
    
    /// Initialize system roles if they don't exist
    async fn initialize_system_roles(&self) -> Result<()> {
        // Define the system roles
        let system_roles = vec![
            Role {
                id: "admin".to_string(),
                name: "Administrator".to_string(),
                description: "System administrator with full access".to_string(),
                permissions: vec![
                    Permission {
                        resource_pattern: "*".to_string(),
                        actions: ["*"].iter().map(|s| s.to_string()).collect(),
                        attributes: HashMap::new(),
                        effect: PermissionEffect::Allow,
                    },
                ],
                parent_roles: vec![],
                metadata: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Role {
                id: "user".to_string(),
                name: "User".to_string(),
                description: "Standard user with limited access".to_string(),
                permissions: vec![
                    Permission {
                        resource_pattern: "user/{id}".to_string(),
                        actions: ["read", "update"].iter().map(|s| s.to_string()).collect(),
                        attributes: HashMap::new(),
                        effect: PermissionEffect::Allow,
                    },
                ],
                parent_roles: vec![],
                metadata: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Role {
                id: "service".to_string(),
                name: "Service".to_string(),
                description: "Base service role".to_string(),
                permissions: vec![
                    Permission {
                        resource_pattern: "service/{id}".to_string(),
                        actions: ["read"].iter().map(|s| s.to_string()).collect(),
                        attributes: HashMap::new(),
                        effect: PermissionEffect::Allow,
                    },
                ],
                parent_roles: vec![],
                metadata: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];
        
        // Check if each role exists, and create it if it doesn't
        for role in system_roles {
            match self.get_role(&role.id).await {
                Ok(_) => {
                    // Role already exists
                    debug!("System role {} already exists", role.id);
                }
                Err(_) => {
                    // Create the role
                    debug!("Creating system role: {}", role.id);
                    self.create_role(role).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Create a new role
    pub async fn create_role(&self, role: Role) -> Result<String> {
        // Store the role in the database
        self.storage.store_entity(&role).await?;
        
        // Update the cache
        let mut cache = self.roles_cache.write().await;
        cache.insert(role.id.clone(), role.clone());
        
        Ok(role.id)
    }
    
    /// Get a role by ID
    pub async fn get_role(&self, role_id: &str) -> Result<Role> {
        // Check the cache first
        {
            let cache = self.roles_cache.read().await;
            if let Some(role) = cache.get(role_id) {
                return Ok(role.clone());
            }
        }
        
        // Not in cache, get from storage
        let role = self.storage.get_entity::<Role>(role_id).await?;
        
        // Update the cache
        let mut cache = self.roles_cache.write().await;
        cache.insert(role_id.to_string(), role.clone());
        
        Ok(role)
    }
    
    /// Update an existing role
    pub async fn update_role(&self, role_id: &str, mut role: Role) -> Result<()> {
        // Ensure the ID matches
        if role.id != role_id {
            return Err(anyhow!("Role ID mismatch"));
        }
        
        // Update the timestamp
        role.updated_at = Utc::now();
        
        // Update in storage
        self.storage.store_entity(&role).await?;
        
        // Update the cache
        let mut cache = self.roles_cache.write().await;
        cache.insert(role_id.to_string(), role);
        
        Ok(())
    }
    
    /// Delete a role
    pub async fn delete_role(&self, role_id: &str) -> Result<()> {
        // Delete from storage
        self.storage.delete_entity::<Role>(role_id).await?;
        
        // Remove from cache
        let mut cache = self.roles_cache.write().await;
        cache.remove(role_id);
        
        Ok(())
    }
    
    /// List all roles
    pub async fn list_roles(&self) -> Result<Vec<Role>> {
        // Get all roles from storage
        let roles = self.storage.list_entities::<Role>().await?;
        
        // Update the cache
        let mut cache = self.roles_cache.write().await;
        for role in &roles {
            cache.insert(role.id.clone(), role.clone());
        }
        
        Ok(roles)
    }
    
    /// Assign a role to a principal (user or service)
    pub async fn assign_role(
        &self,
        principal_id: &str,
        principal_type: PrincipalType,
        role_id: &str,
        assigned_by: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<String> {
        // Verify the role exists
        self.get_role(role_id).await?;
        
        // Create a new role assignment
        let assignment = RoleAssignment {
            id: Uuid::new_v4().to_string(),
            principal_id: principal_id.to_string(),
            principal_type,
            role_id: role_id.to_string(),
            assigned_at: Utc::now(),
            assigned_by: assigned_by.to_string(),
            expires_at,
        };
        
        // Store the assignment
        self.storage.store_entity(&assignment).await?;
        
        // Update the cache
        let key = format!("{}:{}", assignment.principal_type, assignment.principal_id);
        let mut cache = self.assignments_cache.write().await;
        
        if let Some(assignments) = cache.get_mut(&key) {
            assignments.push(assignment.clone());
        } else {
            cache.insert(key, vec![assignment.clone()]);
        }
        
        info!(
            "Role {} assigned to {} {} by {}",
            role_id, principal_type, principal_id, assigned_by
        );
        
        Ok(assignment.id)
    }
    
    /// Revoke a role from a principal
    pub async fn revoke_role(
        &self,
        principal_id: &str,
        principal_type: PrincipalType,
        role_id: &str,
    ) -> Result<()> {
        // Find the assignment
        let assignments = self.get_principal_role_assignments(principal_id, &principal_type).await?;
        
        for assignment in assignments {
            if assignment.role_id == role_id {
                // Delete the assignment
                self.storage.delete_entity::<RoleAssignment>(&assignment.id).await?;
                
                // Update the cache
                let key = format!("{}:{}", principal_type, principal_id);
                let mut cache = self.assignments_cache.write().await;
                
                if let Some(cached_assignments) = cache.get_mut(&key) {
                    cached_assignments.retain(|a| a.id != assignment.id);
                }
                
                info!(
                    "Role {} revoked from {} {}",
                    role_id, principal_type, principal_id
                );
                
                return Ok(());
            }
        }
        
        Err(anyhow!("Role assignment not found"))
    }
    
    /// Get all role assignments for a principal
    pub async fn get_principal_role_assignments(
        &self,
        principal_id: &str,
        principal_type: &PrincipalType,
    ) -> Result<Vec<RoleAssignment>> {
        // Check the cache first
        let key = format!("{}:{}", principal_type, principal_id);
        
        {
            let cache = self.assignments_cache.read().await;
            if let Some(assignments) = cache.get(&key) {
                return Ok(assignments.clone());
            }
        }
        
        // Query the database
        let query = format!("principal_type = '{}' AND principal_id = '{}'", 
                           principal_type, principal_id);
        
        let assignments = self.storage
            .query_entities::<RoleAssignment>(&query)
            .await
            .context("Failed to query role assignments")?;
        
        // Filter out expired assignments
        let now = Utc::now();
        let valid_assignments: Vec<RoleAssignment> = assignments
            .into_iter()
            .filter(|a| a.expires_at.map_or(true, |exp| exp > now))
            .collect();
        
        // Update the cache
        let mut cache = self.assignments_cache.write().await;
        cache.insert(key, valid_assignments.clone());
        
        Ok(valid_assignments)
    }
    
    /// Get all roles for a principal, including those from parent roles
    pub async fn get_principal_roles(
        &self,
        principal_id: &str,
        principal_type: &PrincipalType,
    ) -> Result<Vec<Role>> {
        // Get the direct role assignments
        let assignments = self
            .get_principal_role_assignments(principal_id, principal_type)
            .await?;
        
        let mut result = Vec::new();
        let mut processed_roles = HashSet::new();
        
        // Process each assigned role
        for assignment in assignments {
            if !processed_roles.contains(&assignment.role_id) {
                self.get_role_with_parents(&assignment.role_id, &mut result, &mut processed_roles)
                    .await?;
            }
        }
        
        Ok(result)
    }
    
    // Recursively get a role and its parent roles
    async fn get_role_with_parents(
        &self,
        role_id: &str,
        result: &mut Vec<Role>,
        processed_roles: &mut HashSet<String>,
    ) -> Result<()> {
        // Skip if already processed
        if processed_roles.contains(role_id) {
            return Ok(());
        }
        
        // Mark as processed
        processed_roles.insert(role_id.to_string());
        
        // Get the role
        let role = self.get_role(role_id).await?;
        
        // Add the role to the result
        result.push(role.clone());
        
        // Process parent roles
        for parent_id in &role.parent_roles {
            self.get_role_with_parents(parent_id, result, processed_roles).await?;
        }
        
        Ok(())
    }
    
    /// Check if a principal has permission to perform an action on a resource
    pub async fn check_permission(
        &self,
        principal_id: &str,
        principal_type: &PrincipalType,
        resource: &str,
        action: &str,
        context: Option<&HashMap<String, String>>,
    ) -> Result<PermissionDecision> {
        let roles = self.get_principal_roles(principal_id, principal_type).await?;
        
        if roles.is_empty() {
            return Ok(PermissionDecision {
                allowed: false,
                resource: resource.to_string(),
                action: action.to_string(),
                reason: "No roles assigned".to_string(),
                role_id: None,
                permission_index: None,
            });
        }
        
        // Check each role for the permission
        let mut deny_decision = None;
        
        for role in &roles {
            for (i, permission) in role.permissions.iter().enumerate() {
                if self.resource_matches(&permission.resource_pattern, resource).await? {
                    let action_match = permission.actions.contains("*") ||
                                       permission.actions.contains(action);
                    
                    if action_match {
                        // Check attributes if available
                        if let Some(ctx) = context {
                            if !self.check_attributes(&permission.attributes, ctx) {
                                continue;
                            }
                        }
                        
                        // Action is in the allowed actions list
                        match permission.effect {
                            PermissionEffect::Allow => {
                                return Ok(PermissionDecision {
                                    allowed: true,
                                    resource: resource.to_string(),
                                    action: action.to_string(),
                                    reason: format!("Allowed by role {} permission {}", role.id, i),
                                    role_id: Some(role.id.clone()),
                                    permission_index: Some(i),
                                });
                            }
                            PermissionEffect::Deny => {
                                // Store the deny decision, but continue checking
                                // (explicit deny takes precedence over allow)
                                deny_decision = Some(PermissionDecision {
                                    allowed: false,
                                    resource: resource.to_string(),
                                    action: action.to_string(),
                                    reason: format!("Explicitly denied by role {} permission {}", role.id, i),
                                    role_id: Some(role.id.clone()),
                                    permission_index: Some(i),
                                });
                            }
                        }
                    }
                }
            }
        }
        
        // If we have a deny decision, return it
        if let Some(decision) = deny_decision {
            return Ok(decision);
        }
        
        // No matching permission found
        Ok(PermissionDecision {
            allowed: false,
            resource: resource.to_string(),
            action: action.to_string(),
            reason: "No matching permission found".to_string(),
            role_id: None,
            permission_index: None,
        })
    }
    
    /// Check if a resource matches a pattern
    async fn resource_matches(&self, pattern: &str, resource: &str) -> Result<bool> {
        // Exact match
        if pattern == resource {
            return Ok(true);
        }
        
        // Wildcard match
        if pattern == "*" {
            return Ok(true);
        }
        
        // Path prefix with trailing wildcard
        if pattern.ends_with("/*") {
            let prefix = &pattern[0..pattern.len() - 1]; // Remove the '*'
            return Ok(resource.starts_with(prefix));
        }
        
        // Path pattern with {id} placeholders
        if pattern.contains('{') {
            // Convert the pattern to a regex
            let pattern_regex = self.get_or_create_regex(pattern).await?;
            return Ok(pattern_regex.is_match(resource));
        }
        
        Ok(false)
    }
    
    /// Get or create a compiled regex for a pattern
    async fn get_or_create_regex(&self, pattern: &str) -> Result<Regex> {
        // Check cache first
        {
            let cache = self.compiled_patterns.read().await;
            if let Some(regex) = cache.get(pattern) {
                return Ok(regex.clone());
            }
        }
        
        // Convert the pattern to a regex
        let mut regex_str = "^".to_string();
        let mut chars = pattern.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '{' {
                let mut id_name = String::new();
                
                // Read until closing brace
                while let Some(id_char) = chars.next() {
                    if id_char == '}' {
                        break;
                    }
                    id_name.push(id_char);
                }
                
                // Replace with a regex group
                regex_str.push_str("([^/]+)");
            } else if c == '*' {
                // Replace with a regex wildcard
                regex_str.push_str(".*");
            } else if ['\\', '.', '(', ')', '[', ']', '{', '}', '?', '+', '^', '$']
                .contains(&c) {
                // Escape regex special characters
                regex_str.push('\\');
                regex_str.push(c);
            } else {
                regex_str.push(c);
            }
        }
        
        regex_str.push('$');
        
        // Compile the regex
        let regex = Regex::new(&regex_str)
            .with_context(|| format!("Failed to compile regex from pattern: {}", pattern))?;
        
        // Update the cache
        let mut cache = self.compiled_patterns.write().await;
        cache.insert(pattern.to_string(), regex.clone());
        
        Ok(regex)
    }
    
    /// Check if attributes match the context
    fn check_attributes(&self, attributes: &HashMap<String, String>, context: &HashMap<String, String>) -> bool {
        // All attributes in the permission must match the context
        for (key, value) in attributes {
            if let Some(ctx_value) = context.get(key) {
                if value != ctx_value && value != "*" {
                    return false;
                }
            } else {
                // Attribute not found in context
                return false;
            }
        }
        
        true
    }
    
    /// Get all resources a principal has access to for a given action
    pub async fn get_accessible_resources(
        &self,
        principal_id: &str,
        principal_type: &PrincipalType,
        action: &str,
        resource_prefix: Option<&str>,
    ) -> Result<Vec<String>> {
        let roles = self.get_principal_roles(principal_id, principal_type).await?;
        
        if roles.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut allowed_patterns = Vec::new();
        let mut denied_patterns = Vec::new();
        
        // Collect allowed and denied resource patterns
        for role in &roles {
            for permission in &role.permissions {
                let action_match = permission.actions.contains("*") ||
                                  permission.actions.contains(action);
                
                if action_match {
                    match permission.effect {
                        PermissionEffect::Allow => {
                            // Filter by prefix if provided
                            if let Some(prefix) = resource_prefix {
                                if permission.resource_pattern.starts_with(prefix) {
                                    allowed_patterns.push(permission.resource_pattern.clone());
                                }
                            } else {
                                allowed_patterns.push(permission.resource_pattern.clone());
                            }
                        }
                        PermissionEffect::Deny => {
                            denied_patterns.push(permission.resource_pattern.clone());
                        }
                    }
                }
            }
        }
        
        // Filter out resources that match denied patterns
        // Note: This is a simplistic approach, as in real systems you'd need
        // to resolve resource patterns to actual resource URIs
        let mut result = Vec::new();
        
        for allowed in &allowed_patterns {
            let mut is_denied = false;
            
            for denied in &denied_patterns {
                if denied == "*" || denied == allowed {
                    is_denied = true;
                    break;
                }
            }
            
            if !is_denied {
                result.push(allowed.clone());
            }
        }
        
        Ok(result)
    }
    
    /// Clear all caches
    pub async fn clear_caches(&self) {
        let mut roles_cache = self.roles_cache.write().await;
        roles_cache.clear();
        
        let mut assignments_cache = self.assignments_cache.write().await;
        assignments_cache.clear();
        
        let mut patterns_cache = self.compiled_patterns.write().await;
        patterns_cache.clear();
        
        info!("RBAC caches cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MockStorage;
    
    // Helper to create a test RBAC manager
    async fn create_test_rbac_manager() -> RbacManager {
        let storage = Arc::new(MockStorage::new());
        let rbac = RbacManager::new(storage).await.unwrap();
        /* We would initialize test data here, but the manager already
           initializes system roles in its constructor */
        rbac
    }
    
    #[tokio::test]
    async fn test_rbac_role_crud() {
        let rbac = create_test_rbac_manager().await;
        
        // Create a new role
        let role = Role {
            id: Uuid::new_v4().to_string(),
            name: "TestRole".to_string(),
            description: "A test role".to_string(),
            permissions: vec![
                Permission {
                    resource_pattern: "test/*".to_string(),
                    actions: ["read", "write"].iter().map(|s| s.to_string()).collect(),
                    attributes: HashMap::new(),
                    effect: PermissionEffect::Allow,
                },
            ],
            parent_roles: vec![],
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let role_id = rbac.create_role(role.clone()).await.unwrap();
        
        // Get the role
        let retrieved_role = rbac.get_role(&role_id).await.unwrap();
        assert_eq!(retrieved_role.name, "TestRole");
        
        // Update the role
        let mut updated_role = retrieved_role.clone();
        updated_role.description = "Updated description".to_string();
        rbac.update_role(&role_id, updated_role).await.unwrap();
        
        // Get the updated role
        let retrieved_updated = rbac.get_role(&role_id).await.unwrap();
        assert_eq!(retrieved_updated.description, "Updated description");
        
        // Delete the role
        rbac.delete_role(&role_id).await.unwrap();
        
        // Verify it's deleted
        assert!(rbac.get_role(&role_id).await.is_err());
    }
    
    #[tokio::test]
    async fn test_role_assignment() {
        let rbac = create_test_rbac_manager().await;
        
        // Assign the admin role to a test user
        let assignment_id = rbac.assign_role(
            "test-user-1",
            PrincipalType::User,
            "admin",
            "system",
            None, // No expiration
        ).await.unwrap();
        
        // Get the role assignments
        let assignments = rbac
            .get_principal_role_assignments("test-user-1", &PrincipalType::User)
            .await
            .unwrap();
            
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].role_id, "admin");
        
        // Revoke the role
        rbac.revoke_role("test-user-1", PrincipalType::User, "admin")
            .await
            .unwrap();
            
        // Verify it's revoked
        let assignments_after = rbac
            .get_principal_role_assignments("test-user-1", &PrincipalType::User)
            .await
            .unwrap();
            
        assert_eq!(assignments_after.len(), 0);
    }
    
    #[tokio::test]
    async fn test_permission_check() {
        let rbac = create_test_rbac_manager().await;
        
        // Create a test role with specific permissions
        let role = Role {
            id: "test-role".to_string(),
            name: "Test Role".to_string(),
            description: "Role for testing permission checks".to_string(),
            permissions: vec![
                Permission {
                    resource_pattern: "article/*".to_string(),
                    actions: ["read"].iter().map(|s| s.to_string()).collect(),
                    attributes: HashMap::new(),
                    effect: PermissionEffect::Allow,
                },
                Permission {
                    resource_pattern: "article/secret/*".to_string(),
                    actions: ["read"].iter().map(|s| s.to_string()).collect(),
                    attributes: HashMap::new(),
                    effect: PermissionEffect::Deny,
                },
                Permission {
                    resource_pattern: "user/{id}".to_string(),
                    actions: ["read", "update"].iter().map(|s| s.to_string()).collect(),
                    attributes: HashMap::new(),
                    effect: PermissionEffect::Allow,
                },
            ],
            parent_roles: vec![],
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        rbac.create_role(role).await.unwrap();
        
        // Assign the role to a test user
        rbac.assign_role(
            "test-user-2",
            PrincipalType::User,
            "test-role",
            "system",
            None,
        ).await.unwrap();
        
        // Check permissions
        
        // Should be allowed (article/* pattern)
        let decision1 = rbac
            .check_permission(
                "test-user-2",
                &PrincipalType::User,
                "article/123", 
                "read",
                None,
            )
            .await
            .unwrap();
            
        assert!(decision1.allowed);
        
        // Should be denied (article/secret/* pattern)
        let decision2 = rbac
            .check_permission(
                "test-user-2",
                &PrincipalType::User,
                "article/secret/456",
                "read",
                None,
            )
            .await
            .unwrap();
            
        assert!(!decision2.allowed);
        
        // Should be allowed (user/{id} pattern with id=test-user-2)
        let decision3 = rbac
            .check_permission(
                "test-user-2",
                &PrincipalType::User,
                "user/test-user-2",
                "read",
                None,
            )
            .await
            .unwrap();
            
        assert!(decision3.allowed);
        
        // Should be denied (action not in allowed actions)
        let decision4 = rbac
            .check_permission(
                "test-user-2",
                &PrincipalType::User,
                "article/123",
                "delete",
                None,
            )
            .await
            .unwrap();
            
        assert!(!decision4.allowed);
    }
}
