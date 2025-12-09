// secrets-service-rs/src/auth.rs
// Authentication and authorization mechanisms for secrets service

use std::collections::HashMap;
use std::{env, sync::Arc};
use tokio::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// Role definition - a named set of permissions
#[derive(Debug, Clone)]
pub struct Role {
    pub name: String,
    pub permissions: Vec<Permission>,
}

// Permission - defines what actions can be performed on which resources
#[derive(Debug, Clone, PartialEq)]
pub struct Permission {
    pub resource_pattern: String, // Resource pattern (supports wildcards)
    pub actions: Vec<String>,     // Allowed actions (read, write, delete, list)
}

// Service credentials for authentication
#[derive(Debug, Clone)]
pub struct ServiceCredentials {
    pub id: String,
    pub secret: String,
    pub roles: Vec<String>, // Role names assigned to this service
}

// Token data for authenticated sessions
#[derive(Debug, Clone)]
pub struct TokenData {
    pub token: String,
    pub service_id: String,
    pub expires_at: u64,
    pub roles: Vec<String>,
}

// Authentication manager
#[derive(Clone)]
pub struct AuthManager {
    roles: Arc<RwLock<HashMap<String, Role>>>,
    services: Arc<RwLock<HashMap<String, ServiceCredentials>>>,
    tokens: Arc<RwLock<HashMap<String, TokenData>>>,
}

impl AuthManager {
    // Create a new AuthManager with default roles and services
    pub async fn new() -> Self {
        let roles = Arc::new(RwLock::new(HashMap::new()));
        let services = Arc::new(RwLock::new(HashMap::new()));
        let tokens = Arc::new(RwLock::new(HashMap::new()));
        
        let auth = Self {
            roles,
            services,
            tokens,
        };
        
        // Initialize default roles and services
        auth.initialize_defaults().await;
        
        auth
    }
    
    // Initialize default roles and services
    async fn initialize_defaults(&self) {
        // Define standard roles
        let admin_role = Role {
            name: "admin".to_string(),
            permissions: vec![
                Permission {
                    resource_pattern: "*".to_string(),
                    actions: vec!["read".to_string(), "write".to_string(), "delete".to_string(), "list".to_string()],
                },
            ],
        };
        
        let reader_role = Role {
            name: "reader".to_string(),
            permissions: vec![
                Permission {
                    resource_pattern: "*".to_string(),
                    actions: vec!["read".to_string(), "list".to_string()],
                },
            ],
        };
        
        let writer_role = Role {
            name: "writer".to_string(),
            permissions: vec![
                Permission {
                    resource_pattern: "*".to_string(),
                    actions: vec!["write".to_string()],
                },
            ],
        };
        
        let llm_service_role = Role {
            name: "llm-service".to_string(),
            permissions: vec![
                Permission {
                    resource_pattern: "llm-api-key/*".to_string(),
                    actions: vec!["read".to_string()],
                },
            ],
        };
        
        let api_gateway_role = Role {
            name: "api-gateway".to_string(),
            permissions: vec![
                Permission {
                    resource_pattern: "api-key/*".to_string(),
                    actions: vec!["read".to_string(), "write".to_string()],
                },
            ],
        };
        
        // Register the roles
        let mut roles = self.roles.write().await;
        roles.insert(admin_role.name.clone(), admin_role);
        roles.insert(reader_role.name.clone(), reader_role);
        roles.insert(writer_role.name.clone(), writer_role);
        roles.insert(llm_service_role.name.clone(), llm_service_role);
        roles.insert(api_gateway_role.name.clone(), api_gateway_role);
        
        // Define service credentials
        // In a production system, these would be securely loaded from environment variables
        // or a dedicated secure store
        let services_config = vec![
            ServiceCredentials {
                id: "llm-service".to_string(),
                secret: env::var("LLM_SERVICE_SECRET").unwrap_or_else(|_| "default-llm-service-secret".to_string()),
                roles: vec!["llm-service".to_string(), "reader".to_string()],
            },
            ServiceCredentials {
                id: "api-gateway".to_string(),
                secret: env::var("API_GATEWAY_SECRET").unwrap_or_else(|_| "default-api-gateway-secret".to_string()),
                roles: vec!["api-gateway".to_string()],
            },
        ];
        
        // Register the services
        let mut services = self.services.write().await;
        for service in services_config {
            services.insert(service.id.clone(), service);
        }
    }
    
    // Authenticate a service and generate a token
    pub async fn authenticate_service(&self, service_id: &str, service_secret: &str) -> Option<TokenData> {
        // Look up service credentials
        let services = self.services.read().await;
        let service = match services.get(service_id) {
            Some(s) => s,
            None => return None, // Service not found
        };
        
        // Check service secret
        if service.secret != service_secret {
            return None; // Invalid secret
        }
        
        // Generate a token with 1-hour expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
            
        let token_data = TokenData {
            token: format!("st.{}", Uuid::new_v4()),
            service_id: service_id.to_string(),
            expires_at: now + 3600, // 1 hour
            roles: service.roles.clone(),
        };
        
        // Store the token
        let mut tokens = self.tokens.write().await;
        tokens.insert(token_data.token.clone(), token_data.clone());
        
        Some(token_data)
    }
    
    // Verify a token
    pub async fn verify_token(&self, token: &str) -> Option<TokenData> {
        // Get token data
        let tokens = self.tokens.read().await;
        let token_data = match tokens.get(token) {
            Some(t) => t.clone(),
            None => return None, // Token not found
        };
        
        // Check expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
            
        if token_data.expires_at < now {
            drop(tokens); // Release read lock
            
            // Remove expired token
            let mut tokens = self.tokens.write().await;
            tokens.remove(token);
            
            return None; // Token expired
        }
        
        Some(token_data)
    }
    
    // Check if a token is authorized for an action on a resource
    pub async fn is_authorized(&self, token: &str, resource: &str, action: &str) -> bool {
        // Verify token
        let token_data = match self.verify_token(token).await {
            Some(t) => t,
            None => return false, // Token invalid or expired
        };
        
        // Get all roles associated with the token
        let roles = self.roles.read().await;
        
        // Check each role for permissions
        for role_name in &token_data.roles {
            if let Some(role) = roles.get(role_name) {
                for permission in &role.permissions {
                    // Check if the permission applies to this resource
                    if resource_matches(&permission.resource_pattern, resource) {
                        // Check if the action is allowed
                        if permission.actions.iter().any(|a| a == action) {
                            return true; // Authorized!
                        }
                    }
                }
            }
        }
        
        // No matching permission found
        false
    }
    
    // Generate a token for a service with specific roles
    pub async fn generate_token(
        &self,
        service_id: &str,
        requested_roles: Vec<String>,
        ttl: u64
    ) -> Option<TokenData> {
        // Verify that the service exists and has these roles
        let services = self.services.read().await;
        let service = match services.get(service_id) {
            Some(s) => s,
            None => return None, // Service not found
        };
        
        // Filter the requested roles to only those the service actually has
        let granted_roles: Vec<String> = requested_roles
            .into_iter()
            .filter(|r| service.roles.contains(r))
            .collect();
            
        if granted_roles.is_empty() {
            return None; // No valid roles requested
        }
        
        // Generate token with the specified TTL
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
            
        // Use the specified TTL or fall back to 1 hour
        let expires_at = now + if ttl > 0 { ttl } else { 3600 };
        
        let token_data = TokenData {
            token: format!("st.{}", Uuid::new_v4()),
            service_id: service_id.to_string(),
            expires_at,
            roles: granted_roles,
        };
        
        // Store the token
        let mut tokens = self.tokens.write().await;
        tokens.insert(token_data.token.clone(), token_data.clone());
        
        Some(token_data)
    }
}

// Helper function to check if a resource matches a pattern
fn resource_matches(pattern: &str, resource: &str) -> bool {
    if pattern == "*" {
        return true; // Wildcard matches everything
    }
    
    if pattern.ends_with("/*") {
        // Path prefix matching
        let prefix = &pattern[0..pattern.len() - 1]; // Remove the trailing "*"
        return resource.starts_with(prefix);
    }
    
    // Exact matching
    pattern == resource
}