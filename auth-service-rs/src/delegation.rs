// auth-service-rs/src/delegation.rs
//
// Token delegation for cross-service operations
// Provides:
// - Creation of delegated tokens with scoped permissions
// - Chain of custody tracking for delegated actions
// - Validation of delegated tokens
// - Auditing of delegation events

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow, Context};
use tracing::{debug, error, info, warn};
use once_cell::sync::Lazy;

use crate::jwt::{TokenManager, Claims};
use crate::audit;
use crate::storage::{StorageBackend, Entity};

// Global delegator instance
static TOKEN_DELEGATOR: Lazy<RwLock<Option<Arc<TokenDelegator>>>> = Lazy::new(|| {
    RwLock::new(None)
});

/// Delegation record stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRecord {
    pub id: String,
    pub parent_token_id: String,  // The ID of the token that created this delegation
    pub delegate_token_id: String, // The ID of the delegated token
    pub delegator: String,         // Who delegated (service ID)
    pub delegate: String,          // Who received the delegation (service ID)
    pub permissions: Vec<String>,  // What permissions were delegated
    pub resources: Vec<String>,    // What resources the delegation applies to
    pub created_at: i64,
    pub expires_at: i64,
    pub revoked: bool,
    pub revoked_at: Option<i64>,
    pub metadata: HashMap<String, String>,
}

impl Entity for DelegationRecord {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_entity_type() -> &'static str {
        "token_delegation"
    }
}

/// Delegation options for creating delegated tokens
#[derive(Debug, Clone)]
pub struct DelegationOptions {
    pub permissions: Vec<String>,     // What permissions to delegate
    pub resources: Vec<String>,       // What resources the delegation applies to
    pub expires_in_seconds: u64,      // How long the delegation is valid
    pub max_hops: Option<u32>,        // Max number of further delegations allowed
    pub metadata: Option<HashMap<String, String>>, // Additional metadata
}

/// Token delegator for creating and validating delegated tokens
pub struct TokenDelegator {
    // Token manager for token operations
    token_manager: Arc<TokenManager>,
    
    // Storage for delegation records
    storage: Arc<dyn StorageBackend>,
    
    // Cache of active delegations
    delegations_cache: RwLock<HashMap<String, DelegationRecord>>,
}

impl TokenDelegator {
    /// Create a new token delegator
    pub async fn new(
        token_manager: Arc<TokenManager>,
        storage: Arc<dyn StorageBackend>,
    ) -> Result<Self> {
        let delegator = Self {
            token_manager,
            storage,
            delegations_cache: RwLock::new(HashMap::new()),
        };
        
        Ok(delegator)
    }
    
    /// Initialize global token delegator
    pub async fn init_global(
        token_manager: Arc<TokenManager>,
        storage: Arc<dyn StorageBackend>,
    ) -> Result<()> {
        let delegator = Self::new(token_manager, storage).await?;
        
        let mut global_delegator = TOKEN_DELEGATOR.write().await;
        *global_delegator = Some(Arc::new(delegator));
        
        Ok(())
    }
    
    /// Get the global token delegator
    pub async fn get_global() -> Result<Arc<TokenDelegator>> {
        let delegator = TOKEN_DELEGATOR.read().await;
        match &*delegator {
            Some(d) => Ok(d.clone()),
            None => Err(anyhow!("Token delegator not initialized")),
        }
    }
    
    /// Create a delegated token from a parent token
    pub async fn create_delegated_token(
        &self,
        parent_token: &str,
        delegate_service_id: &str,
        options: DelegationOptions,
    ) -> Result<String> {
        // Validate the parent token
        let parent_claims = self.token_manager.validate_token(
            parent_token,
            None,  // No audience check for delegation
            true,  // Validate expiration
        ).await?;
        
        // Get the parent token ID (jti claim)
        let parent_token_id = parent_claims.jti.clone();
        
        // Check delegation depth
        let current_depth = parent_claims.custom_claims
            .get("delegation_depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
            
        if let Some(max_hops) = options.max_hops {
            if current_depth >= max_hops as u64 {
                return Err(anyhow!("Maximum delegation depth exceeded"));
            }
        }
        
        // Check if the delegator has the permissions they're trying to delegate
        for permission in &options.permissions {
            // Skip permission check for admin role
            if parent_claims.roles.contains(&"admin".to_string()) {
                break;
            }
            
            // Check if parent token has this permission or has a wildcard
            if !parent_claims.scopes.as_ref().map_or(false, |scopes| {
                scopes.contains(&permission.to_string()) || 
                scopes.contains(&"*".to_string())
            }) {
                return Err(anyhow!(
                    "Cannot delegate permission '{}' that parent token doesn't have", 
                    permission
                ));
            }
        }
        
        // Create custom claims for the delegation
        let mut custom_claims = HashMap::new();
        
        // Set the delegation depth
        custom_claims.insert(
            "delegation_depth".to_string(), 
            serde_json::to_value(current_depth + 1)?
        );
        
        // Set the parent token ID
        custom_claims.insert(
            "parent_token_id".to_string(), 
            serde_json::to_value(parent_token_id.clone())?
        );
        
        // Set the delegator (service that created this delegation)
        custom_claims.insert(
            "delegator".to_string(), 
            serde_json::to_value(parent_claims.sub.clone())?
        );
        
        // Add any additional metadata
        if let Some(meta) = options.metadata {
            for (key, value) in meta {
                custom_claims.insert(
                    format!("meta_{}", key), 
                    serde_json::to_value(value)?
                );
            }
        }
        
        // Create the delegated token
        let (delegate_token, delegate_claims) = self.token_manager.generate_token(
            delegate_service_id,
            "delegation",  // Audience
            "service",     // Token type
            options.expires_in_seconds,
            parent_claims.roles.clone(),
            Some(options.permissions.clone()),
            Some(custom_claims),
        ).await?;
        
        // Create a delegation record
        let delegation = DelegationRecord {
            id: Uuid::new_v4().to_string(),
            parent_token_id,
            delegate_token_id: delegate_claims.jti.clone(),
            delegator: parent_claims.sub.clone(),
            delegate: delegate_service_id.to_string(),
            permissions: options.permissions,
            resources: options.resources,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            expires_at: delegate_claims.exp as i64,
            revoked: false,
            revoked_at: None,
            metadata: HashMap::new(),
        };
        
        // Store the delegation record
        self.storage
            .store_entity(&delegation)
            .await
            .context("Failed to store delegation record")?;
            
        // Update cache
        let mut cache = self.delegations_cache.write().await;
        cache.insert(delegate_claims.jti.clone(), delegation.clone());
        
        // Audit the delegation
        audit::log_system_event(
            audit::EventType::TokenIssued,
            &format!("Delegated token issued to {} by {}", 
                    delegate_service_id, parent_claims.sub),
            Some(HashMap::from([
                ("delegator".to_string(), parent_claims.sub),
                ("delegate".to_string(), delegate_service_id.to_string()),
                ("token_id".to_string(), delegate_claims.jti),
                ("parent_token_id".to_string(), parent_token_id),
            ])),
        ).await
        .ok(); // Ignore audit errors
        
        info!("Created delegated token for {} by {}", 
              delegate_service_id, parent_claims.sub);
        
        Ok(delegate_token)
    }
    
    /// Validate a delegated token
    pub async fn validate_delegated_token(
        &self,
        token: &str,
        required_permissions: Option<&[&str]>,
    ) -> Result<Claims> {
        // First, validate the token with the token manager
        let claims = self.token_manager.validate_token(token, None, true).await?;
        
        // Check if this is a delegated token
        let delegation_depth = claims.custom_claims
            .get("delegation_depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
            
        if delegation_depth == 0 {
            return Err(anyhow!("Not a delegated token"));
        }
        
        // Get the parent token ID
        let parent_token_id = claims.custom_claims
            .get("parent_token_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Delegated token missing parent token ID"))?;
            
        // Check if the delegation has been revoked
        let is_revoked = self.is_delegation_revoked(&claims.jti).await?;
        if is_revoked {
            return Err(anyhow!("Delegation has been revoked"));
        }
        
        // Check required permissions
        if let Some(perms) = required_permissions {
            let token_scopes = claims.scopes.as_ref()
                .ok_or_else(|| anyhow!("Token has no permission scopes"))?;
                
            for perm in perms {
                if !token_scopes.contains(&perm.to_string()) && 
                   !token_scopes.contains(&"*".to_string()) {
                    return Err(anyhow!("Token missing required permission: {}", perm));
                }
            }
        }
        
        // Token is valid
        Ok(claims)
    }
    
    /// Check if a delegation has been revoked
    pub async fn is_delegation_revoked(&self, token_id: &str) -> Result<bool> {
        // Check cache first
        {
            let cache = self.delegations_cache.read().await;
            if let Some(delegation) = cache.get(token_id) {
                return Ok(delegation.revoked);
            }
        }
        
        // Not in cache, check storage
        let query = format!("delegate_token_id = '{}' AND revoked = true", token_id);
        let revoked = self.storage
            .query_entities::<DelegationRecord>(&query)
            .await
            .context("Failed to check delegation revocation")?;
            
        Ok(!revoked.is_empty())
    }
    
    /// Revoke a delegated token
    pub async fn revoke_delegation(&self, token_id: &str, reason: &str) -> Result<()> {
        // First check if the delegation exists
        let query = format!("delegate_token_id = '{}'", token_id);
        let delegations = self.storage
            .query_entities::<DelegationRecord>(&query)
            .await
            .context("Failed to query delegation")?;
            
        if delegations.is_empty() {
            return Err(anyhow!("Delegation not found for token ID: {}", token_id));
        }
        
        let mut delegation = delegations[0].clone();
        
        // Check if already revoked
        if delegation.revoked {
            return Err(anyhow!("Delegation already revoked"));
        }
        
        // Update as revoked
        delegation.revoked = true;
        delegation.revoked_at = Some(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64);
        delegation.metadata.insert("revocation_reason".to_string(), reason.to_string());
        
        // Update in storage
        self.storage
            .store_entity(&delegation)
            .await
            .context("Failed to update delegation record")?;
            
        // Update in cache
        let mut cache = self.delegations_cache.write().await;
        cache.insert(token_id.to_string(), delegation.clone());
        
        // Revoke the token itself
        self.token_manager.revoke_token(token_id).await?;
        
        // Audit the revocation
        audit::log_system_event(
            audit::EventType::TokenRevoked,
            &format!("Delegated token revoked: {}", token_id),
            Some(HashMap::from([
                ("token_id".to_string(), token_id.to_string()),
                ("reason".to_string(), reason.to_string()),
            ])),
        ).await
        .ok(); // Ignore audit errors
        
        info!("Revoked delegated token {} with reason: {}", token_id, reason);
        
        Ok(())
    }
    
    /// List active delegations for a service
    pub async fn list_service_delegations(
        &self,
        service_id: &str,
        as_delegator: bool,  // If true, list tokens delegated by this service
                            // If false, list tokens delegated to this service
    ) -> Result<Vec<DelegationRecord>> {
        let field = if as_delegator { "delegator" } else { "delegate" };
        let query = format!("{} = '{}' AND revoked = false", field, service_id);
        
        let delegations = self.storage
            .query_entities::<DelegationRecord>(&query)
            .await
            .context("Failed to query delegations")?;
            
        Ok(delegations)
    }
    
    /// Create a chain of delegations from a parent token
    pub async fn create_delegation_chain(
        &self,
        parent_token: &str,
        service_chain: &[(&str, &[&str])], // (service_id, permissions) pairs
        expires_in_seconds: u64,
    ) -> Result<Vec<String>> {
        // Check if there are services in the chain
        if service_chain.is_empty() {
            return Err(anyhow!("Empty service chain"));
        }
        
        let mut tokens = Vec::new();
        let mut current_token = parent_token.to_string();
        
        // Create delegated tokens for each service in the chain
        for (i, (service_id, permissions)) in service_chain.iter().enumerate() {
            // Configure delegation options
            let options = DelegationOptions {
                permissions: permissions.iter().map(|p| p.to_string()).collect(),
                resources: vec![],  // No specific resources
                expires_in_seconds,
                max_hops: Some((service_chain.len() - i) as u32),  // Remaining chain length
                metadata: None,
            };
            
            // Create delegated token
            let token = self.create_delegated_token(&current_token, service_id, options).await?;
            
            tokens.push(token.clone());
            current_token = token;
        }
        
        Ok(tokens)
    }
}

// Convenience functions for global token delegator
pub async fn create_delegated_token(
    parent_token: &str,
    delegate_service_id: &str,
    options: DelegationOptions,
) -> Result<String> {
    let delegator = TokenDelegator::get_global().await?;
    delegator.create_delegated_token(parent_token, delegate_service_id, options).await
}

pub async fn validate_delegated_token(
    token: &str,
    required_permissions: Option<&[&str]>,
) -> Result<Claims> {
    let delegator = TokenDelegator::get_global().await?;
    delegator.validate_delegated_token(token, required_permissions).await
}

pub async fn revoke_delegation(token_id: &str, reason: &str) -> Result<()> {
    let delegator = TokenDelegator::get_global().await?;
    delegator.revoke_delegation(token_id, reason).await
}

pub async fn list_service_delegations(
    service_id: &str,
    as_delegator: bool,
) -> Result<Vec<DelegationRecord>> {
    let delegator = TokenDelegator::get_global().await?;
    delegator.list_service_delegations(service_id, as_delegator).await
}

pub async fn create_delegation_chain(
    parent_token: &str,
    service_chain: &[(&str, &[&str])],
    expires_in_seconds: u64,
) -> Result<Vec<String>> {
    let delegator = TokenDelegator::get_global().await?;
    delegator.create_delegation_chain(parent_token, service_chain, expires_in_seconds).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MockStorage;
    use crate::jwt::tests::create_test_token_manager;
    
    async fn create_test_token_delegator() -> TokenDelegator {
        let token_manager = Arc::new(create_test_token_manager().await);
        let storage = Arc::new(MockStorage::new());
        
        TokenDelegator::new(token_manager, storage).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_delegation() {
        let delegator = create_test_token_delegator().await;
        
        // Create a parent token
        let (parent_token, parent_claims) = delegator.token_manager
            .generate_token(
                "service1",
                "test",
                "service",
                3600, // 1 hour
                vec!["service".to_string()],
                Some(vec!["read".to_string(), "write".to_string()]),
                None,
            )
            .await
            .unwrap();
            
        // Create delegation options
        let options = DelegationOptions {
            permissions: vec!["read".to_string()],
            resources: vec![],
            expires_in_seconds: 1800, // 30 minutes
            max_hops: Some(2),
            metadata: None,
        };
        
        // Create delegated token
        let delegated_token = delegator
            .create_delegated_token(&parent_token, "service2", options)
            .await
            .unwrap();
            
        // Validate delegated token
        let delegate_claims = delegator
            .validate_delegated_token(&delegated_token, Some(&["read"]))
            .await
            .unwrap();
            
        // Check properties
        assert_eq!(delegate_claims.sub, "service2");
        
        let delegation_depth = delegate_claims.custom_claims
            .get("delegation_depth")
            .and_then(|v| v.as_u64())
            .unwrap();
            
        assert_eq!(delegation_depth, 1);
        
        // Check parent token ID
        let parent_token_id = delegate_claims.custom_claims
            .get("parent_token_id")
            .and_then(|v| v.as_str())
            .unwrap();
            
        assert_eq!(parent_token_id, parent_claims.jti);
        
        // Check delegator
        let delegator_id = delegate_claims.custom_claims
            .get("delegator")
            .and_then(|v| v.as_str())
            .unwrap();
            
        assert_eq!(delegator_id, "service1");
        
        // Revoke delegation
        delegator
            .revoke_delegation(&delegate_claims.jti, "Test revocation")
            .await
            .unwrap();
            
        // Validate should fail now
        let result = delegator
            .validate_delegated_token(&delegated_token, None)
            .await;
            
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_delegation_chain() {
        let delegator = create_test_token_delegator().await;
        
        // Create a parent token with all permissions
        let (parent_token, _) = delegator.token_manager
            .generate_token(
                "service-root",
                "test",
                "service",
                3600,
                vec!["admin".to_string()],
                Some(vec!["*".to_string()]),
                None,
            )
            .await
            .unwrap();
            
        // Create a service chain
        let service_chain = [
            ("service-a", ["read", "write"]),
            ("service-b", ["read"]),
            ("service-c", ["execute"]),
        ];
        
        // Create delegation chain
        let tokens = delegator
            .create_delegation_chain(&parent_token, &service_chain, 1800)
            .await
            .unwrap();
            
        assert_eq!(tokens.len(), 3);
        
        // Validate each token
        let claims_a = delegator
            .validate_delegated_token(&tokens[0], Some(&["read", "write"]))
            .await
            .unwrap();
            
        let claims_b = delegator
            .validate_delegated_token(&tokens[1], Some(&["read"]))
            .await
            .unwrap();
            
        let claims_c = delegator
            .validate_delegated_token(&tokens[2], Some(&["execute"]))
            .await
            .unwrap();
            
        assert_eq!(claims_a.sub, "service-a");
        assert_eq!(claims_b.sub, "service-b");
        assert_eq!(claims_c.sub, "service-c");
        
        // Check delegation depths
        let depth_a = claims_a.custom_claims
            .get("delegation_depth")
            .and_then(|v| v.as_u64())
            .unwrap();
            
        let depth_b = claims_b.custom_claims
            .get("delegation_depth")
            .and_then(|v| v.as_u64())
            .unwrap();
            
        let depth_c = claims_c.custom_claims
            .get("delegation_depth")
            .and_then(|v| v.as_u64())
            .unwrap();
            
        assert_eq!(depth_a, 1);
        assert_eq!(depth_b, 2);
        assert_eq!(depth_c, 3);
    }
}