// auth-service-rs/src/token_manager.rs
//
// Token management, revocation and rotation
// Provides:
// - Centralized token blacklist management
// - Automated key rotation
// - Token metadata and tracking
// - Token revocation propagation

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;
use anyhow::{Result, anyhow, Context};
use tracing::{debug, error, info, warn};
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};
use jsonwebtoken::{DecodingKey, EncodingKey, Algorithm};

use crate::jwt::TokenManager as JwtManager;
use crate::secrets_client::{SecretsClient, get_secrets_client};
use crate::audit;

// Global token manager instance
static TOKEN_MANAGER: Lazy<RwLock<Option<Arc<TokenRotationManager>>>> = Lazy::new(|| {
    RwLock::new(None)
});

// Constants
const TOKEN_ROTATION_DEFAULT_INTERVAL: u64 = 24 * 60 * 60; // 24 hours in seconds
const TOKEN_ROTATION_MIN_INTERVAL: u64 = 60 * 60; // 1 hour in seconds
const KEY_ROTATION_DEFAULT_INTERVAL: u64 = 7 * 24 * 60 * 60; // 7 days in seconds
const TOKEN_BLACKLIST_CLEANUP_INTERVAL: u64 = 60 * 60; // 1 hour in seconds

/// Token metadata for tracking purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub token_id: String,
    pub user_id: Option<String>,
    pub service_id: Option<String>,
    pub issued_at: i64,
    pub expires_at: i64,
    pub issuer: String,
    pub client_id: Option<String>,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub scopes: Vec<String>,
    pub is_revoked: bool,
    pub revoked_at: Option<i64>,
    pub revoked_by: Option<String>,
    pub revocation_reason: Option<String>,
    pub key_id: String,
}

/// Token blacklist entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlacklistEntry {
    token_id: String,
    expires_at: i64,
    revoked_at: i64,
    reason: String,
}

/// Token rotation manager
pub struct TokenRotationManager {
    // Core JWT token manager
    jwt_manager: Arc<JwtManager>,
    
    // Token blacklist - memory cache of revoked tokens
    blacklist: RwLock<HashMap<String, BlacklistEntry>>,
    
    // Rotation configuration
    token_rotation_interval: Duration,
    key_rotation_interval: Duration,
    
    // Current key information
    current_key_id: RwLock<String>,
    key_created_at: RwLock<i64>,
}

impl TokenRotationManager {
    /// Create a new token rotation manager
    pub async fn new(
        jwt_manager: Arc<JwtManager>,
        token_rotation_interval_secs: Option<u64>,
        key_rotation_interval_secs: Option<u64>,
    ) -> Result<Self> {
        // Set rotation intervals
        let token_rotation_interval = Duration::from_secs(
            token_rotation_interval_secs
                .unwrap_or(TOKEN_ROTATION_DEFAULT_INTERVAL)
                .max(TOKEN_ROTATION_MIN_INTERVAL) // Enforce minimum interval
        );
        
        let key_rotation_interval = Duration::from_secs(
            key_rotation_interval_secs
                .unwrap_or(KEY_ROTATION_DEFAULT_INTERVAL)
        );
        
        // Get current key ID
        let current_key_id = tokio::task::spawn_blocking(move || {
            // This is a hack to get the current key ID, since we don't expose it directly
            // In a real implementation, we would provide a proper API for this
            let temp_key_id = uuid::Uuid::new_v4().to_string();
            temp_key_id
        }).await.map_err(|e| anyhow!("Failed to get current key ID: {}", e))?;
        
        // Create manager
        let manager = Self {
            jwt_manager,
            blacklist: RwLock::new(HashMap::new()),
            token_rotation_interval,
            key_rotation_interval,
            current_key_id: RwLock::new(current_key_id),
            key_created_at: RwLock::new(SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64),
        };
        
        // Load existing blacklist from storage
        manager.load_blacklist().await?;
        
        Ok(manager)
    }
    
    /// Initialize and set the global token rotation manager
    pub async fn init_global(
        jwt_manager: Arc<JwtManager>,
        token_rotation_interval_secs: Option<u64>,
        key_rotation_interval_secs: Option<u64>,
    ) -> Result<()> {
        let manager = Self::new(
            jwt_manager, 
            token_rotation_interval_secs, 
            key_rotation_interval_secs
        ).await?;
        
        // Store in global
        let mut token_manager = TOKEN_MANAGER.write().await;
        *token_manager = Some(Arc::new(manager));
        
        Ok(())
    }
    
    /// Get the global token rotation manager
    pub async fn get_global() -> Result<Arc<TokenRotationManager>> {
        let token_manager = TOKEN_MANAGER.read().await;
        match &*token_manager {
            Some(manager) => Ok(manager.clone()),
            None => Err(anyhow!("Token rotation manager not initialized")),
        }
    }
    
    /// Start rotation background tasks
    pub async fn start_rotation_tasks(self: Arc<Self>) {
        // Start key rotation task
        let manager_clone = self.clone();
        tokio::spawn(async move {
            manager_clone.run_key_rotation_task().await;
        });
        
        // Start token blacklist cleanup task
        let manager_clone = self.clone();
        tokio::spawn(async move {
            manager_clone.run_blacklist_cleanup_task().await;
        });
    }
    
    /// Run key rotation task
    async fn run_key_rotation_task(&self) {
        let mut interval = interval(self.key_rotation_interval);
        
        loop {
            interval.tick().await;
            
            info!("Running scheduled key rotation");
            
            // Check if key needs rotation
            let key_created_at = *self.key_created_at.read().await;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
                
            let key_age = now - key_created_at;
            
            if key_age >= self.key_rotation_interval.as_secs() as i64 {
                // Rotate key
                match self.rotate_keys().await {
                    Ok(_) => info!("Successfully rotated signing keys"),
                    Err(e) => error!("Failed to rotate signing keys: {}", e),
                }
            } else {
                debug!("Key rotation not needed yet");
            }
        }
    }
    
    /// Run blacklist cleanup task
    async fn run_blacklist_cleanup_task(&self) {
        let mut interval = interval(Duration::from_secs(TOKEN_BLACKLIST_CLEANUP_INTERVAL));
        
        loop {
            interval.tick().await;
            
            debug!("Running scheduled blacklist cleanup");
            
            // Cleanup expired entries
            match self.cleanup_blacklist().await {
                Ok(removed) => {
                    if removed > 0 {
                        debug!("Removed {} expired blacklist entries", removed);
                    }
                },
                Err(e) => error!("Failed to clean up blacklist: {}", e),
            }
        }
    }
    
    /// Rotate signing keys
    pub async fn rotate_keys(&self) -> Result<String> {
        info!("Rotating JWT signing keys");
        
        // Call JWT manager to rotate keys
        let new_key_id = self.jwt_manager.rotate_keys(None).await?;
        
        // Update current key ID
        let mut current_key_id = self.current_key_id.write().await;
        *current_key_id = new_key_id.clone();
        
        // Update key creation time
        let mut key_created_at = self.key_created_at.write().await;
        *key_created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Log key rotation
        audit::log_system_event(
            audit::EventType::KeyRotation,
            &format!("Rotated JWT signing keys, new key ID: {}", new_key_id),
            None,
        ).await.ok();
        
        Ok(new_key_id)
    }
    
    /// Load token blacklist from storage
    async fn load_blacklist(&self) -> Result<()> {
        // Get secrets client
        let secrets_client = get_secrets_client().await
            .context("Failed to get secrets client")?;
            
        // List all blacklisted tokens
        let blacklist_keys = secrets_client
            .list_secrets(Some("tokens:blacklist:"))
            .await
            .map_err(|e| anyhow!("Failed to list blacklisted tokens: {}", e))?;
            
        // Load each blacklist entry
        let mut blacklist = self.blacklist.write().await;
        
        for key in blacklist_keys {
            // Get blacklist entry
            let entry_json = secrets_client
                .get_secret(&key)
                .await
                .map_err(|e| anyhow!("Failed to get blacklist entry {}: {}", key, e))?;
                
            // Parse entry
            let entry: BlacklistEntry = serde_json::from_str(&entry_json)
                .context("Failed to parse blacklist entry")?;
                
            // Add to in-memory blacklist
            blacklist.insert(entry.token_id.clone(), entry);
        }
        
        info!("Loaded {} blacklisted tokens", blacklist.len());
        
        Ok(())
    }
    
    /// Cleanup expired blacklist entries
    async fn cleanup_blacklist(&self) -> Result<usize> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        let mut removed = 0;
        
        // Clean up in-memory blacklist
        {
            let mut blacklist = self.blacklist.write().await;
            
            // Find expired entries
            let expired_tokens: Vec<String> = blacklist.iter()
                .filter(|(_, entry)| entry.expires_at < now)
                .map(|(token_id, _)| token_id.clone())
                .collect();
                
            // Remove expired entries
            for token_id in &expired_tokens {
                blacklist.remove(token_id);
            }
            
            removed = expired_tokens.len();
            
            // Clean up storage
            if !expired_tokens.is_empty() {
                // Get secrets client
                match get_secrets_client().await {
                    Ok(secrets_client) => {
                        for token_id in expired_tokens {
                            let key = format!("tokens:blacklist:{}", token_id);
                            
                            if let Err(e) = secrets_client.delete_secret(&key).await {
                                warn!("Failed to delete expired blacklist entry from storage: {}", e);
                            }
                        }
                    },
                    Err(e) => {
                        warn!("Failed to get secrets client for blacklist cleanup: {}", e);
                    }
                }
            }
        }
        
        Ok(removed)
    }
    
    /// Revoke a token
    pub async fn revoke_token(
        &self,
        token_id: &str,
        reason: &str,
        revoked_by: Option<&str>,
        token_metadata: Option<TokenMetadata>,
    ) -> Result<()> {
        // Check if token is already revoked
        {
            let blacklist = self.blacklist.read().await;
            if blacklist.contains_key(token_id) {
                return Err(anyhow!("Token already revoked"));
            }
        }
        
        // Get token metadata if not provided
        let metadata = match token_metadata {
            Some(meta) => meta,
            None => {
                // Get metadata for this token from storage
                let secrets_client = get_secrets_client().await
                    .context("Failed to get secrets client")?;
                    
                let meta_key = format!("tokens:metadata:{}", token_id);
                
                match secrets_client.get_secret(&meta_key).await {
                    Ok(meta_json) => {
                        // Parse metadata
                        serde_json::from_str(&meta_json)
                            .context("Failed to parse token metadata")?
                    },
                    Err(_) => {
                        // No metadata available, create minimal entry
                        TokenMetadata {
                            token_id: token_id.to_string(),
                            user_id: None,
                            service_id: None,
                            issued_at: 0,
                            expires_at: 0, // We'll update this from JWT manager
                            issuer: "unknown".to_string(),
                            client_id: None,
                            device_info: None,
                            ip_address: None,
                            scopes: Vec::new(),
                            is_revoked: false,
                            revoked_at: None,
                            revoked_by: None,
                            revocation_reason: None,
                            key_id: "unknown".to_string(),
                        }
                    }
                }
            }
        };
        
        // Revoke the token with JWT manager
        // This updates the internal revocation store
        self.jwt_manager.revoke_token(token_id).await?;
        
        // Create blacklist entry
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        let entry = BlacklistEntry {
            token_id: token_id.to_string(),
            expires_at: metadata.expires_at,
            revoked_at: now,
            reason: reason.to_string(),
        };
        
        // Add to in-memory blacklist
        {
            let mut blacklist = self.blacklist.write().await;
            blacklist.insert(token_id.to_string(), entry.clone());
        }
        
        // Store in persistent blacklist
        let secrets_client = get_secrets_client().await
            .context("Failed to get secrets client")?;
            
        let key = format!("tokens:blacklist:{}", token_id);
        let entry_json = serde_json::to_string(&entry)
            .context("Failed to serialize blacklist entry")?;
            
        secrets_client.store_secret(&key, &entry_json).await
            .map_err(|e| anyhow!("Failed to store blacklist entry: {}", e))?;
            
        // Update token metadata
        let mut updated_metadata = metadata.clone();
        updated_metadata.is_revoked = true;
        updated_metadata.revoked_at = Some(now);
        updated_metadata.revoked_by = revoked_by.map(|s| s.to_string());
        updated_metadata.revocation_reason = Some(reason.to_string());
        
        // Store updated metadata
        let meta_key = format!("tokens:metadata:{}", token_id);
        let meta_json = serde_json::to_string(&updated_metadata)
            .context("Failed to serialize token metadata")?;
            
        secrets_client.store_secret(&meta_key, &meta_json).await
            .map_err(|e| anyhow!("Failed to update token metadata: {}", e))?;
            
        // Log revocation
        audit::log_system_event(
            audit::EventType::TokenRevoked,
            &format!("Token {} revoked: {}", token_id, reason),
            Some(HashMap::from([
                ("token_id".to_string(), token_id.to_string()),
                ("reason".to_string(), reason.to_string()),
                ("revoked_by".to_string(), revoked_by.unwrap_or("system").to_string()),
            ])),
        ).await.ok();
        
        info!("Revoked token {} with reason: {}", token_id, reason);
        
        Ok(())
    }
    
    /// Check if a token is revoked
    pub async fn is_token_revoked(&self, token_id: &str) -> bool {
        // Check in-memory blacklist first (fast)
        let blacklist = self.blacklist.read().await;
        if blacklist.contains_key(token_id) {
            return true;
        }
        
        // If not found, check with JWT manager (handles token DB for validation)
        // This is a fallback in case the in-memory cache is incomplete
        match self.jwt_manager.check_token_revoked(token_id).await {
            Ok(()) => false, // Not revoked
            Err(_) => true,  // Revoked or error
        }
    }
    
    /// Revoke all tokens for a user or service
    pub async fn revoke_all_tokens(
        &self,
        principal_id: &str,
        principal_type: &str,
        reason: &str,
        revoked_by: Option<&str>,
    ) -> Result<usize> {
        // Get all tokens for this principal
        let secrets_client = get_secrets_client().await
            .context("Failed to get secrets client")?;
            
        // List all token metadata
        let meta_keys = secrets_client
            .list_secrets(Some("tokens:metadata:"))
            .await
            .map_err(|e| anyhow!("Failed to list token metadata: {}", e))?;
            
        let mut revoked_count = 0;
        
        // Check each token to see if it belongs to the principal
        for meta_key in meta_keys {
            let meta_json = match secrets_client.get_secret(&meta_key).await {
                Ok(json) => json,
                Err(_) => continue, // Skip if error
            };
            
            let metadata: TokenMetadata = match serde_json::from_str(&meta_json) {
                Ok(meta) => meta,
                Err(_) => continue, // Skip if error
            };
            
            // Check if token belongs to the principal
            let is_match = match principal_type {
                "user" => metadata.user_id.as_deref() == Some(principal_id),
                "service" => metadata.service_id.as_deref() == Some(principal_id),
                _ => false,
            };
            
            if is_match && !metadata.is_revoked {
                // Token belongs to the principal and is not already revoked
                if let Err(e) = self.revoke_token(
                    &metadata.token_id,
                    reason,
                    revoked_by,
                    Some(metadata),
                ).await {
                    warn!("Failed to revoke token {}: {}", metadata.token_id, e);
                } else {
                    revoked_count += 1;
                }
            }
        }
        
        // Log mass revocation
        if revoked_count > 0 {
            audit::log_system_event(
                audit::EventType::TokenRevoked,
                &format!("Revoked {} tokens for {} {}: {}", 
                         revoked_count, principal_type, principal_id, reason),
                Some(HashMap::from([
                    ("principal_id".to_string(), principal_id.to_string()),
                    ("principal_type".to_string(), principal_type.to_string()),
                    ("reason".to_string(), reason.to_string()),
                    ("count".to_string(), revoked_count.to_string()),
                    ("revoked_by".to_string(), revoked_by.unwrap_or("system").to_string()),
                ])),
            ).await.ok();
        }
        
        info!("Revoked {} tokens for {} {}", revoked_count, principal_type, principal_id);
        
        Ok(revoked_count)
    }
    
    /// Store token metadata
    pub async fn store_token_metadata(&self, metadata: &TokenMetadata) -> Result<()> {
        // Get secrets client
        let secrets_client = get_secrets_client().await
            .context("Failed to get secrets client")?;
            
        // Serialize metadata
        let meta_json = serde_json::to_string(metadata)
            .context("Failed to serialize token metadata")?;
            
        // Store metadata
        let meta_key = format!("tokens:metadata:{}", metadata.token_id);
        
        secrets_client.store_secret(&meta_key, &meta_json).await
            .map_err(|e| anyhow!("Failed to store token metadata: {}", e))?;
            
        Ok(())
    }
    
    /// Get token metadata
    pub async fn get_token_metadata(&self, token_id: &str) -> Result<TokenMetadata> {
        // Get secrets client
        let secrets_client = get_secrets_client().await
            .context("Failed to get secrets client")?;
            
        // Get metadata
        let meta_key = format!("tokens:metadata:{}", token_id);
        
        let meta_json = secrets_client.get_secret(&meta_key).await
            .map_err(|e| anyhow!("Failed to get token metadata: {}", e))?;
            
        // Parse metadata
        let metadata = serde_json::from_str(&meta_json)
            .context("Failed to parse token metadata")?;
            
        Ok(metadata)
    }
    
    /// List all revoked tokens
    pub async fn list_revoked_tokens(&self) -> Vec<BlacklistEntry> {
        let blacklist = self.blacklist.read().await;
        blacklist.values().cloned().collect()
    }
}

// Convenience functions for global token manager
pub async fn revoke_token(
    token_id: &str,
    reason: &str,
    revoked_by: Option<&str>,
) -> Result<()> {
    let manager = TokenRotationManager::get_global().await?;
    manager.revoke_token(token_id, reason, revoked_by, None).await
}

pub async fn is_token_revoked(token_id: &str) -> Result<bool> {
    let manager = TokenRotationManager::get_global().await?;
    Ok(manager.is_token_revoked(token_id).await)
}

pub async fn rotate_keys() -> Result<String> {
    let manager = TokenRotationManager::get_global().await?;
    manager.rotate_keys().await
}

pub async fn revoke_all_tokens(
    principal_id: &str,
    principal_type: &str,
    reason: &str,
    revoked_by: Option<&str>,
) -> Result<usize> {
    let manager = TokenRotationManager::get_global().await?;
    manager.revoke_all_tokens(principal_id, principal_type, reason, revoked_by).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwt::tests::create_test_token_manager;
    
    // Create a test token rotation manager
    async fn create_test_token_rotation_manager() -> Arc<TokenRotationManager> {
        // Create JWT manager
        let jwt_manager = Arc::new(create_test_token_manager().await);
        
        // Create token rotation manager
        let manager = TokenRotationManager::new(
            jwt_manager.clone(),
            Some(60),  // Short rotation interval for tests
            Some(120), // Short key rotation interval for tests
        ).await.unwrap();
        
        Arc::new(manager)
    }
    
    #[tokio::test]
    async fn test_token_revocation() {
        // Initialize mock secrets client for testing
        crate::secrets_client::init_mock_secrets_client().await.unwrap();
        
        let manager = create_test_token_rotation_manager().await;
        
        // Create a token to revoke
        let token_id = "test-token-123";
        
        // Create token metadata
        let metadata = TokenMetadata {
            token_id: token_id.to_string(),
            user_id: Some("test-user".to_string()),
            service_id: None,
            issued_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            expires_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64 + 3600,
            issuer: "test-issuer".to_string(),
            client_id: None,
            device_info: None,
            ip_address: None,
            scopes: Vec::new(),
            is_revoked: false,
            revoked_at: None,
            revoked_by: None,
            revocation_reason: None,
            key_id: "test-key-id".to_string(),
        };
        
        // Store token metadata
        manager.store_token_metadata(&metadata).await.unwrap();
        
        // Revoke the token
        manager.revoke_token(
            token_id,
            "Test revocation",
            Some("test-admin"),
            Some(metadata),
        ).await.unwrap();
        
        // Check if token is revoked
        assert!(manager.is_token_revoked(token_id).await);
        
        // Verify metadata is updated
        let updated_metadata = manager.get_token_metadata(token_id).await.unwrap();
        assert!(updated_metadata.is_revoked);
        assert!(updated_metadata.revoked_at.is_some());
        assert_eq!(updated_metadata.revoked_by, Some("test-admin".to_string()));
        assert_eq!(updated_metadata.revocation_reason, Some("Test revocation".to_string()));
    }
    
    #[tokio::test]
    async fn test_key_rotation() {
        let manager = create_test_token_rotation_manager().await;
        
        // Get initial key ID
        let initial_key_id = manager.current_key_id.read().await.clone();
        
        // Rotate keys
        let new_key_id = manager.rotate_keys().await.unwrap();
        
        // Verify key ID changed
        assert_ne!(initial_key_id, new_key_id);
        
        // Verify current key ID is updated
        let current_key_id = manager.current_key_id.read().await.clone();
        assert_eq!(current_key_id, new_key_id);
    }
}