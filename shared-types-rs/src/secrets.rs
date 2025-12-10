use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use thiserror::Error;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::error::ClientError;
use vaultrs::kv2;
use aes_gcm::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Error, Debug)]
pub enum SecretError {
    #[error("Vault client error: {0}")]
    VaultError(#[from] ClientError),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Secret not found: {0}")]
    NotFound(String),
    
    #[error("Secret rotation failed: {0}")]
    RotationError(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SecretData {
    value: String,
    #[serde(default)]
    version: Option<u64>,
}

#[derive(Clone, Debug)]
struct CachedSecret {
    value: String,
    expiry: DateTime<Utc>,
    version: u64,
}

#[derive(Clone)]
struct SecretCache {
    entries: Arc<DashMap<String, CachedSecret>>,
    max_age: Duration,
}

impl SecretCache {
    fn new(max_age: Duration) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            max_age,
        }
    }

    fn get(&self, key: &str) -> Option<CachedSecret> {
        self.entries.get(key).and_then(|entry| {
            let secret = entry.clone();
            if secret.expiry <= Utc::now() {
                self.entries.remove(key);
                None
            } else {
                Some(secret)
            }
        })
    }

    fn set(&self, key: String, value: String, version: u64) {
        let expiry = Utc::now() + chrono::Duration::from_std(self.max_age)
            .unwrap_or_else(|_| chrono::Duration::hours(1));
            
        self.entries.insert(key, CachedSecret {
            value,
            expiry,
            version,
        });
    }
}

pub struct SecretManager {
    client: VaultClient,
    cache: SecretCache,
    retry_config: RetryConfig,
}

#[derive(Clone)]
struct RetryConfig {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(2),
        }
    }
}

impl SecretManager {
    pub async fn new(
        vault_addr: String,
        token: String,
        cache_ttl: Duration,
    ) -> Result<Self, SecretError> {
        info!(
            "Initializing SecretManager with vault address: {}, cache TTL: {}s",
            vault_addr,
            cache_ttl.as_secs()
        );

        let settings = VaultClientSettingsBuilder::default()
            .address(vault_addr)
            .token(token)
            .build()
            .map_err(|e| {
                error!("Failed to build Vault client settings: {}", e);
                SecretError::ConnectionError(e.to_string())
            })?;

        let client = VaultClient::new(settings)
            .map_err(|e| {
                error!("Failed to initialize Vault client: {}", e);
                SecretError::ConnectionError(e.to_string())
            })?;

        debug!("Vault client initialized successfully");
        let manager = Self {
            client,
            cache: SecretCache::new(cache_ttl),
            retry_config: RetryConfig::default(),
        };

        info!("SecretManager initialization complete");
        Ok(manager)
    }

    pub async fn get_secret(&self, path: &str) -> Result<String, SecretError> {
        info!("Requesting secret from path: {}", path);

        if let Some(cached) = self.cache.get(path) {
            debug!("Cache hit for secret: {}", path);
            info!("Retrieved secret from cache for path: {}", path);
            return Ok(cached.value);
        }
        debug!("Cache miss for secret: {}", path);

        let mut attempts = 0;
        let mut delay = self.retry_config.base_delay;

        loop {
            attempts += 1;
            info!("Attempting to fetch secret from Vault (attempt {}/{})",
                  attempts, self.retry_config.max_attempts);

            match self.fetch_from_vault(path).await {
                Ok((value, version)) => {
                    debug!("Successfully fetched secret version {} from path: {}", version, path);
                    self.cache.set(path.to_string(), value.clone(), version);
                    info!("Secret retrieved and cached successfully for path: {}", path);
                    return Ok(value);
                }
                Err(e) => {
                    if attempts >= self.retry_config.max_attempts {
                        error!("Failed to fetch secret after {} attempts: {}", attempts, e);
                        return Err(e);
                    }
                    warn!("Retry attempt {} for path {} failed: {}. Retrying in {:?}",
                          attempts, path, e, delay);
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, self.retry_config.max_delay);
                }
            }
        }
    }

    async fn fetch_from_vault(&self, path: &str) -> Result<(String, u64), SecretError> {
        let (mount, secret_path) = self.split_path(path)?;
        
        let secret: SecretData = kv2::read(&self.client, &mount, &secret_path)
            .await
            .map_err(|e| SecretError::VaultError(e))?;
            
        let version = secret.version.unwrap_or(1);
            
        // Decrypt the value if it's encrypted
        let decrypted = if let Ok(encrypted) = BASE64.decode(secret.value.as_bytes()) {
            self.decrypt(&encrypted)?
        } else {
            secret.value
        };
        
        Ok((decrypted, version))
    }

    pub async fn rotate_secret(&self, path: &str) -> Result<(), SecretError> {
        info!("Starting secret rotation for path: {}", path);
        
        let (mount, secret_path) = self.split_path(path)?;
        debug!("Split path into mount: {} and path: {}", mount, secret_path);
        
        // Get current secret
        info!("Fetching current secret for rotation");
        let current: SecretData = kv2::read(&self.client, &mount, &secret_path)
            .await
            .map_err(|e| {
                error!("Failed to read current secret for rotation: {}", e);
                SecretError::VaultError(e)
            })?;
            
        // Generate new encryption key and encrypt value
        debug!("Generating new encryption key and encrypting value");
        let new_value = self.encrypt(&current.value)?;
        
        // Write back encrypted value with incremented version
        let version = current.version.unwrap_or(1);
            
        info!("Writing new secret version {} for path: {}/{}", version + 1, mount, secret_path);
        let new_secret = SecretData {
            value: BASE64.encode(new_value),
            version: Some(version + 1),
        };
        
        kv2::set(&self.client, &mount, &secret_path, &new_secret)
            .await
            .map_err(|e| {
                error!("Failed to write rotated secret: {}", e);
                SecretError::RotationError(e.to_string())
            })?;
            
        // Invalidate cache
        debug!("Invalidating cache entry for rotated secret");
        self.cache.entries.remove(&format!("{}/{}", mount, secret_path));
        
        info!("Secret rotation completed successfully for path: {}/{}", mount, secret_path);
        Ok(())
    }

    fn split_path(&self, full_path: &str) -> Result<(String, String), SecretError> {
        debug!("Splitting path: {}", full_path);
        
        let parts: Vec<&str> = full_path.splitn(2, '/').collect();
        if parts.len() != 2 {
            error!("Invalid path format: {}", full_path);
            return Err(SecretError::NotFound(
                "Path must be in format 'mount/path'".to_string()
            ));
        }
        
        debug!("Successfully split path into mount: {} and path: {}", parts[0], parts[1]);
        Ok((parts[0].to_string(), parts[1].to_string()))
    }
    
    fn encrypt(&self, value: &str) -> Result<Vec<u8>, SecretError> {
        debug!("Starting encryption of secret value");
        
        let key = self.generate_encryption_key();
        debug!("Generated new encryption key");
        
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| {
                error!("Failed to create cipher: {}", e);
                SecretError::RotationError(e.to_string())
            })?;
            
        let nonce_bytes = self.generate_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);
        debug!("Generated nonce for encryption");
        
        let ciphertext = cipher
            .encrypt(nonce, value.as_bytes())
            .map_err(|e| {
                error!("Encryption failed: {}", e);
                SecretError::RotationError(e.to_string())
            })?;
            
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        debug!("Encryption completed successfully");
        Ok(result)
    }
    
    fn decrypt(&self, encrypted: &[u8]) -> Result<String, SecretError> {
        debug!("Starting decryption of secret value");
        
        if encrypted.len() < 12 {
            error!("Invalid encrypted data: length < 12 bytes");
            return Err(SecretError::RotationError("Invalid encrypted data".to_string()));
        }
        
        let (nonce_bytes, ciphertext) = encrypted.split_at(12);
        debug!("Split encrypted data into nonce and ciphertext");
        
        let key = self.generate_encryption_key();
        debug!("Generated decryption key");
        
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| {
                error!("Failed to create cipher for decryption: {}", e);
                SecretError::RotationError(e.to_string())
            })?;
            
        let nonce = Nonce::from_slice(nonce_bytes);
        debug!("Created nonce from encrypted data");
        
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| {
                error!("Decryption failed: {}", e);
                SecretError::RotationError(e.to_string())
            })?;
            
        let result = String::from_utf8(plaintext)
            .map_err(|e| {
                error!("Failed to convert decrypted data to UTF-8: {}", e);
                SecretError::RotationError(e.to_string())
            })?;
            
        debug!("Decryption completed successfully");
        Ok(result)
    }
    
    fn generate_encryption_key(&self) -> [u8; 32] {
        debug!("Generating new 32-byte encryption key");
        let mut key = [0u8; 32];
        rand::thread_rng().fill(&mut key);
        debug!("Encryption key generated successfully");
        key
    }
    
    fn generate_nonce(&self) -> [u8; 12] {
        debug!("Generating new 12-byte nonce");
        let mut nonce = [0u8; 12];
        rand::thread_rng().fill(&mut nonce);
        debug!("Nonce generated successfully");
        nonce
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_functionality() {
        let cache = SecretCache::new(Duration::from_secs(60));
        
        // Test set and get
        cache.set("test/key".to_string(), "test_value".to_string(), 1);
        let cached = cache.get("test/key");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().value, "test_value");
        
        // Test non-existent key
        assert!(cache.get("non/existent").is_none());
    }
    
    #[tokio::test]
    async fn test_split_path() {
        // Create a mock SecretManager for testing split_path
        // This test verifies the path splitting logic
        let parts: Vec<&str> = "mount/path/to/secret".splitn(2, '/').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "mount");
        assert_eq!(parts[1], "path/to/secret");
    }
}