// auth-service-rs/src/jwt.rs
//
// JWT token implementation for secure authentication
// Provides:
// - Token generation with flexible claims
// - Validation with proper security checks  
// - Refresh token handling
// - Revocation checking
// - Key rotation support

use std::time::{SystemTime, UNIX_EPOCH, Duration};
use jsonwebtoken::{encode, decode, Header, Validation, Algorithm, EncodingKey, DecodingKey, TokenData};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::sync::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use redis::AsyncCommands;
use anyhow::{Result, anyhow, Context};
use tracing::{debug, error, warn, info};
use once_cell::sync::Lazy;
use zeroize::Zeroize;

// JWT token types
const TOKEN_TYPE_ACCESS: &str = "access";
const TOKEN_TYPE_REFRESH: &str = "refresh";
const TOKEN_TYPE_SERVICE: &str = "service";

// JWT key rotation
static SIGNING_KEYS: Lazy<RwLock<HashMap<String, SigningKey>>> = Lazy::new(|| {
    RwLock::new(HashMap::new())
});

// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,            // Subject (user_id or service_id)
    pub iss: String,            // Issuer
    pub aud: String,            // Audience
    pub exp: u64,               // Expiration time (unix timestamp)
    pub nbf: u64,               // Not valid before (unix timestamp)
    pub iat: u64,               // Issued at (unix timestamp)
    pub jti: String,            // JWT ID (unique identifier for the token)
    pub typ: String,            // Token type (access, refresh, service)
    pub roles: Vec<String>,     // User roles or service roles
    
    // Optional claims with custom data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,  // Permission scopes for the token
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,  // For service tokens
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,     // For user tokens
    
    #[serde(flatten)]
    pub custom_claims: HashMap<String, serde_json::Value>,  // Additional custom claims
}

#[derive(Debug)]
struct SigningKey {
    id: String,
    algorithm: Algorithm,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    created_at: u64,
    expires_at: Option<u64>,
}

impl Drop for SigningKey {
    fn drop(&mut self) {
        // Zeroize sensitive key material when dropped
        self.id.zeroize();
        
        // Note: EncodingKey and DecodingKey don't implement Zeroize,
        // but ideally they should be zeroized as well
    }
}

// Token manager struct
pub struct TokenManager {
    issuer: String,
    default_key_id: Arc<AsyncMutex<String>>,
    redis_client: redis::Client,
    redis_connection: Arc<AsyncMutex<Option<redis::aio::ConnectionManager>>>,
}

impl TokenManager {
    pub async fn new(secret: &str, redis_url: &str, issuer: &str) -> Result<Self> {
        // Create initial signing key
        let key_id = Uuid::new_v4().to_string();
        let signing_key = SigningKey {
            id: key_id.clone(),
            algorithm: Algorithm::HS256,
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            created_at: current_timestamp(),
            expires_at: None, // This key doesn't expire until rotated
        };
        
        // Add the signing key to the key store
        let mut keys = SIGNING_KEYS.write().map_err(|_| anyhow!("Failed to acquire write lock on signing keys"))?;
        keys.insert(key_id.clone(), signing_key);
        
        // Connect to Redis
        let redis_client = redis::Client::open(redis_url)
            .context("Failed to create Redis client")?;
        
        let redis_conn = match redis_client.get_async_connection_manager().await {
            Ok(conn) => Some(conn),
            Err(err) => {
                warn!("Failed to connect to Redis: {}. Token revocation will not be available.", err);
                None
            }
        };
        
        Ok(Self {
            issuer: issuer.to_string(),
            default_key_id: Arc::new(AsyncMutex::new(key_id)),
            redis_client,
            redis_connection: Arc::new(AsyncMutex::new(redis_conn)),
        })
    }
    
    // Generate a new access token
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
        let now = current_timestamp();
        let expiration = now + ttl;
        let token_id = Uuid::new_v4().to_string();
        
        // Create claims
        let mut claims = Claims {
            sub: subject.to_string(),
            iss: self.issuer.clone(),
            aud: audience.to_string(),
            exp: expiration,
            nbf: now,
            iat: now,
            jti: token_id,
            typ: token_type.to_string(),
            roles,
            scopes,
            service_name: None,
            user_name: None,
            custom_claims: custom_claims.unwrap_or_default(),
        };
        
        // For service tokens, add the service name as a claim
        if token_type == TOKEN_TYPE_SERVICE {
            claims.service_name = Some(subject.to_string());
        } else {
            // For user tokens, add the username if available
            if claims.custom_claims.contains_key("username") {
                if let Some(serde_json::Value::String(username)) = claims.custom_claims.get("username") {
                    claims.user_name = Some(username.clone());
                }
            }
        }
        
        // Get the current key ID for signing
        let key_id = self.default_key_id.lock().await.clone();
        
        // Get the signing key
        let keys = SIGNING_KEYS.read().map_err(|_| anyhow!("Failed to acquire read lock on signing keys"))?;
        let signing_key = keys.get(&key_id)
            .ok_or_else(|| anyhow!("Signing key not found: {}", key_id))?;
        
        // Create JWT header with key ID
        let mut header = Header::new(signing_key.algorithm);
        header.kid = Some(key_id);
        
        // Encode the token
        let token = encode(
            &header,
            &claims,
            &signing_key.encoding_key
        ).context("Failed to encode JWT token")?;
        
        debug!("Generated {} token for subject {} with expiration {}", token_type, subject, expiration);
        
        Ok((token, claims))
    }
    
    // Validate a token and return the claims if valid
    pub async fn validate_token(
        &self,
        token: &str,
        expected_audience: Option<&str>,
        validate_expiration: bool,
    ) -> Result<Claims> {
        // Decode the token header to get the key ID
        let header = jsonwebtoken::decode_header(token)
            .context("Failed to decode JWT header")?;
        
        let key_id = header.kid.ok_or_else(|| anyhow!("Token missing key ID (kid)"))?;
        
        // Get the signing key
        let keys = SIGNING_KEYS.read().map_err(|_| anyhow!("Failed to acquire read lock on signing keys"))?;
        let signing_key = keys.get(&key_id)
            .ok_or_else(|| anyhow!("Unknown signing key: {}", key_id))?;
        
        // Create validation configuration
        let mut validation = Validation::new(signing_key.algorithm);
        validation.validate_exp = validate_expiration; 
        validation.set_issuer(&[self.issuer.clone()]);
        
        if let Some(aud) = expected_audience {
            validation.set_audience(&[aud]);
        }
        
        // Decode and validate the token
        let token_data: TokenData<Claims> = decode(
            token,
            &signing_key.decoding_key,
            &validation
        ).context("Failed to decode and validate JWT token")?;
        
        let claims = token_data.claims;
        
        // Check if the token has been revoked
        self.check_token_revoked(&claims.jti).await?;
        
        debug!("Validated token for subject {} with id {}", claims.sub, claims.jti);
        
        Ok(claims)
    }
    
    // Generate refresh token
    pub async fn generate_refresh_token(
        &self,
        subject: &str,
        ttl: u64,
        roles: Vec<String>,
    ) -> Result<(String, Claims)> {
        self.generate_token(
            subject,
            &self.issuer, // Audience is the issuer for refresh tokens
            TOKEN_TYPE_REFRESH,
            ttl,
            roles,
            None,
            None,
        ).await
    }
    
    // Refresh an access token using a valid refresh token
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
        audience: &str,
        access_token_ttl: u64,
    ) -> Result<(String, Claims)> {
        // Validate the refresh token
        let refresh_claims = self.validate_token(refresh_token, Some(&self.issuer), true).await?;
        
        // Ensure it's actually a refresh token
        if refresh_claims.typ != TOKEN_TYPE_REFRESH {
            return Err(anyhow!("Invalid token type. Expected refresh token"));
        }
        
        // Generate a new access token with the same subject and roles
        let (access_token, access_claims) = self.generate_token(
            &refresh_claims.sub,
            audience,
            TOKEN_TYPE_ACCESS,
            access_token_ttl,
            refresh_claims.roles,
            refresh_claims.scopes,
            Some(refresh_claims.custom_claims),
        ).await?;
        
        Ok((access_token, access_claims))
    }
    
    // Revoke a specific token
    pub async fn revoke_token(&self, token_id: &str) -> Result<()> {
        let mut conn_guard = self.redis_connection.lock().await;
        
        if let Some(conn) = conn_guard.as_mut() {
            // Store the token ID in the revocation list
            // Using a Redis sorted set with score = expiration time
            // This allows for automatic cleanup of expired revoked tokens
            
            let now = current_timestamp();
            let expiration = now + 86400 * 30; // 30 days retention for revoked tokens
            
            let _: () = conn.zadd("revoked_tokens", token_id, expiration).await
                .context("Failed to store revoked token in Redis")?;
                
            debug!("Revoked token with ID {}", token_id);
            
            Ok(())
        } else {
            warn!("Redis connection not available. Token revocation is not active.");
            Ok(())
        }
    }
    
    // Revoke token by JWT
    pub async fn revoke_token_by_jwt(&self, token: &str) -> Result<()> {
        // Extract just the token ID claim without fully validating the token
        // This allows revoking expired tokens
        let token_data = jsonwebtoken::decode::<Claims>(
            token,
            &DecodingKey::from_secret(&[]), // Dummy key, we're not validating signature
            &Validation::new(Algorithm::HS256)
                .validate_exp(false)
                .validate_nbf(false),
        ).context("Failed to decode token for revocation")?;
        
        self.revoke_token(&token_data.claims.jti).await
    }
    
    // Check if a token has been revoked
    async fn check_token_revoked(&self, token_id: &str) -> Result<()> {
        let mut conn_guard = self.redis_connection.lock().await;
        
        if let Some(conn) = conn_guard.as_mut() {
            // Check if token ID is in the revoked tokens set
            let is_revoked: bool = conn.zscore("revoked_tokens", token_id).await
                .context("Failed to check token revocation status")?;
                
            if is_revoked {
                return Err(anyhow!("Token has been revoked"));
            }
        }
        
        Ok(())
    }
    
    // Rotate signing keys
    pub async fn rotate_keys(&self, algorithm: Option<Algorithm>) -> Result<String> {
        let now = current_timestamp();
        let new_key_id = Uuid::new_v4().to_string();
        
        // Generate a new random secret
        let mut secret = [0u8; 64];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut secret);
        
        // Create new signing key
        let new_key = SigningKey {
            id: new_key_id.clone(),
            algorithm: algorithm.unwrap_or(Algorithm::HS256),
            encoding_key: EncodingKey::from_secret(&secret),
            decoding_key: DecodingKey::from_secret(&secret),
            created_at: now,
            expires_at: None,
        };
        
        // Add the new key to the key store
        {
            let mut keys = SIGNING_KEYS.write().map_err(|_| anyhow!("Failed to acquire write lock on signing keys"))?;
            
            // Set an expiration on the old key if it doesn't have one already
            for (_, key) in keys.iter_mut() {
                if key.expires_at.is_none() {
                    // Key will expire after 24 hours (or other reasonable overlap period)
                    key.expires_at = Some(now + 86400);
                }
            }
            
            // Add the new key
            keys.insert(new_key_id.clone(), new_key);
        }
        
        // Update the default key ID
        let mut default_key = self.default_key_id.lock().await;
        *default_key = new_key_id.clone();
        
        info!("Rotated JWT signing keys. New key ID: {}", new_key_id);
        
        // Clear out expired keys
        self.cleanup_expired_keys().await?;
        
        Ok(new_key_id)
    }
    
    // Clean up expired keys
    async fn cleanup_expired_keys(&self) -> Result<()> {
        let now = current_timestamp();
        let mut expired_keys = Vec::new();
        
        // Find expired keys
        {
            let keys = SIGNING_KEYS.read().map_err(|_| anyhow!("Failed to acquire read lock on signing keys"))?;
            for (key_id, key) in keys.iter() {
                if let Some(expires_at) = key.expires_at {
                    if expires_at < now {
                        expired_keys.push(key_id.clone());
                    }
                }
            }
        }
        
        // Remove expired keys
        if !expired_keys.is_empty() {
            let mut keys = SIGNING_KEYS.write().map_err(|_| anyhow!("Failed to acquire write lock on signing keys"))?;
            for key_id in &expired_keys {
                keys.remove(key_id);
            }
            
            info!("Removed {} expired JWT signing keys", expired_keys.len());
        }
        
        Ok(())
    }
    
    // Get Redis connection for direct access if needed
    pub async fn get_redis_connection(&self) -> Option<redis::aio::ConnectionManager> {
        let conn_guard = self.redis_connection.lock().await;
        conn_guard.clone()
    }
}

// Helper function to get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Helper to create a test token manager
    async fn create_test_token_manager() -> TokenManager {
        TokenManager::new(
            "test-secret-key-for-jwt-signing-not-for-production",
            "redis://127.0.0.1:6379", // This won't actually connect in tests
            "test-issuer"
        ).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_token_generation_and_validation() {
        let manager = create_test_token_manager().await;
        
        let (token, claims) = manager.generate_token(
            "test-user-123",
            "test-audience",
            TOKEN_TYPE_ACCESS,
            3600, // 1 hour
            vec!["user".to_string()],
            Some(vec!["read".to_string(), "write".to_string()]),
            None
        ).await.unwrap();
        
        assert!(!token.is_empty());
        assert_eq!(claims.sub, "test-user-123");
        assert_eq!(claims.aud, "test-audience");
        assert_eq!(claims.typ, TOKEN_TYPE_ACCESS);
        assert_eq!(claims.roles, vec!["user"]);
        assert_eq!(claims.scopes, Some(vec!["read".to_string(), "write".to_string()]));
        
        // Validate the token
        let validated_claims = manager.validate_token(&token, Some("test-audience"), true).await.unwrap();
        
        assert_eq!(validated_claims.sub, claims.sub);
        assert_eq!(validated_claims.jti, claims.jti);
    }
    
    #[tokio::test]
    async fn test_refresh_token_flow() {
        let manager = create_test_token_manager().await;
        
        // Generate a refresh token
        let (refresh_token, refresh_claims) = manager.generate_refresh_token(
            "test-user-123",
            86400, // 24 hours
            vec!["user".to_string()]
        ).await.unwrap();
        
        assert_eq!(refresh_claims.typ, TOKEN_TYPE_REFRESH);
        
        // Use the refresh token to get a new access token
        let (access_token, access_claims) = manager.refresh_access_token(
            &refresh_token,
            "test-audience",
            3600 // 1 hour
        ).await.unwrap();
        
        assert_eq!(access_claims.typ, TOKEN_TYPE_ACCESS);
        assert_eq!(access_claims.sub, "test-user-123");
        assert_eq!(access_claims.aud, "test-audience");
    }
    
    #[tokio::test]
    async fn test_key_rotation() {
        let manager = create_test_token_manager().await;
        
        // Generate a token with the initial key
        let (token1, _) = manager.generate_token(
            "test-user-123",
            "test-audience",
            TOKEN_TYPE_ACCESS,
            3600,
            vec!["user".to_string()],
            None,
            None
        ).await.unwrap();
        
        // Rotate the keys
        let new_key_id = manager.rotate_keys(None).await.unwrap();
        assert!(!new_key_id.is_empty());
        
        // Generate a token with the new key
        let (token2, _) = manager.generate_token(
            "test-user-123",
            "test-audience",
            TOKEN_TYPE_ACCESS,
            3600,
            vec!["user".to_string()],
            None,
            None
        ).await.unwrap();
        
        // Both tokens should be valid
        let claims1 = manager.validate_token(&token1, Some("test-audience"), true).await.unwrap();
        let claims2 = manager.validate_token(&token2, Some("test-audience"), true).await.unwrap();
        
        assert_eq!(claims1.sub, "test-user-123");
        assert_eq!(claims2.sub, "test-user-123");
        
        // But they should have different key IDs
        let header1 = jsonwebtoken::decode_header(&token1).unwrap();
        let header2 = jsonwebtoken::decode_header(&token2).unwrap();
        
        assert_ne!(header1.kid, header2.kid);
        assert_eq!(header2.kid.unwrap(), new_key_id);
    }
}