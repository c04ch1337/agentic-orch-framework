//! # Config Update Service
//!
//! Handles downloading and deploying LLM adapters (LoRA) and configuration updates
//! with cryptographic signature verification.

use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use ring::signature::{self, UnparsedPublicKey, VerificationAlgorithm};
use serde::{Deserialize, Serialize};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;

/// Configuration for config update service
#[derive(Debug, Clone)]
pub struct ConfigUpdateConfig {
    pub enabled: bool,
    pub adapter_download_url: String,
    pub signature_verification_enabled: bool,
    pub public_key_path: PathBuf,
    pub update_check_interval_secs: u64,
}

impl ConfigUpdateConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("CONFIG_UPDATE_ENABLED")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(true);

        let adapter_download_url = std::env::var("CONFIG_UPDATE_ADAPTER_URL")
            .unwrap_or_else(|_| "https://updates.phoenix-orch.example.com/adapters".to_string());

        let signature_verification_enabled = std::env::var("CONFIG_UPDATE_VERIFY_SIGNATURES")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(true);

        let public_key_path = std::env::var("CONFIG_UPDATE_PUBLIC_KEY")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./certs/update_public_key.pem"));

        let update_check_interval_secs = std::env::var("CONFIG_UPDATE_CHECK_INTERVAL_SECS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(86400); // 24 hours

        Self {
            enabled,
            adapter_download_url,
            signature_verification_enabled,
            public_key_path,
            update_check_interval_secs,
        }
    }
}

/// Adapter metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterMetadata {
    pub adapter_id: String,
    pub version: String,
    pub model_name: String,
    pub download_url: String,
    pub signature: String,
    pub file_size: u64,
    pub checksum: String,
    pub description: Option<String>,
}

/// Configuration update metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateMetadata {
    pub update_id: String,
    pub version: String,
    pub download_url: String,
    pub signature: String,
    pub file_size: u64,
    pub checksum: String,
    pub description: Option<String>,
}

/// Main config update service
pub struct ConfigUpdateService {
    config: ConfigUpdateConfig,
    http_client: reqwest::Client,
    public_key: Option<Vec<u8>>,
}

impl ConfigUpdateService {
    pub fn new(config: ConfigUpdateConfig) -> Result<Self, ConfigUpdateError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| ConfigUpdateError::Http(e.to_string()))?;

        let public_key = if config.signature_verification_enabled && config.public_key_path.exists() {
            Some(fs::read(&config.public_key_path)
                .map_err(|e| ConfigUpdateError::Io(format!("Failed to read public key: {}", e)))?)
        } else {
            None
        };

        Ok(Self {
            config,
            http_client,
            public_key,
        })
    }

    pub fn new_default() -> Result<Self, ConfigUpdateError> {
        Self::new(ConfigUpdateConfig::from_env())
    }

    /// Download and verify an adapter
    pub async fn download_adapter(
        &self,
        metadata: &AdapterMetadata,
        output_path: &Path,
    ) -> Result<(), ConfigUpdateError> {
        if !self.config.enabled {
            return Err(ConfigUpdateError::Disabled);
        }

        log::info!("Downloading adapter {} from {}", metadata.adapter_id, metadata.download_url);

        // Download adapter file
        let response = self
            .http_client
            .get(&metadata.download_url)
            .send()
            .await
            .map_err(|e| ConfigUpdateError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ConfigUpdateError::Http(format!(
                "Download failed with status: {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await
            .map_err(|e| ConfigUpdateError::Http(e.to_string()))?;

        // Verify signature if enabled
        if self.config.signature_verification_enabled {
            self.verify_signature(&bytes, &metadata.signature)?;
        }

        // Verify checksum
        let computed_checksum = sha256_checksum(&bytes);
        if computed_checksum != metadata.checksum {
            return Err(ConfigUpdateError::ChecksumMismatch {
                expected: metadata.checksum.clone(),
                actual: computed_checksum,
            });
        }

        // Write to output path
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| ConfigUpdateError::Io(format!("Failed to create output directory: {}", e)))?;
        }

        let mut file = File::create(output_path).await
            .map_err(|e| ConfigUpdateError::Io(format!("Failed to create output file: {}", e)))?;

        file.write_all(&bytes).await
            .map_err(|e| ConfigUpdateError::Io(format!("Failed to write adapter file: {}", e)))?;

        file.flush().await
            .map_err(|e| ConfigUpdateError::Io(format!("Failed to flush adapter file: {}", e)))?;

        log::info!("Successfully downloaded and verified adapter {}", metadata.adapter_id);
        Ok(())
    }

    /// Push configuration update
    pub async fn push_config_update(
        &self,
        config_path: &Path,
        metadata: &ConfigUpdateMetadata,
    ) -> Result<(), ConfigUpdateError> {
        if !self.config.enabled {
            return Err(ConfigUpdateError::Disabled);
        }

        log::info!("Pushing configuration update {}", metadata.update_id);

        // Read config file
        let config_bytes = fs::read(config_path).await
            .map_err(|e| ConfigUpdateError::Io(format!("Failed to read config file: {}", e)))?;

        // Verify signature if enabled
        if self.config.signature_verification_enabled {
            self.verify_signature(&config_bytes, &metadata.signature)?;
        }

        // Verify checksum
        let computed_checksum = sha256_checksum(&config_bytes);
        if computed_checksum != metadata.checksum {
            return Err(ConfigUpdateError::ChecksumMismatch {
                expected: metadata.checksum.clone(),
                actual: computed_checksum,
            });
        }

        // Backup existing config
        let backup_path = config_path.with_extension("toml.backup");
        if config_path.exists() {
            fs::copy(config_path, &backup_path).await
                .map_err(|e| ConfigUpdateError::Io(format!("Failed to backup config: {}", e)))?;
        }

        // Write new config
        fs::write(config_path, &config_bytes).await
            .map_err(|e| ConfigUpdateError::Io(format!("Failed to write new config: {}", e)))?;

        log::info!("Successfully pushed configuration update {}", metadata.update_id);
        Ok(())
    }

    fn verify_signature(&self, data: &[u8], signature: &str) -> Result<(), ConfigUpdateError> {
        let public_key = self.public_key.as_ref()
            .ok_or_else(|| ConfigUpdateError::Verification("Public key not loaded".to_string()))?;

        let signature_bytes = general_purpose::STANDARD.decode(signature)
            .map_err(|e| ConfigUpdateError::Verification(format!("Invalid signature encoding: {}", e)))?;

        // Parse PEM public key (simplified - in production use proper PEM parser)
        let key = UnparsedPublicKey::new(&signature::ED25519, public_key);
        key.verify(data, &signature_bytes)
            .map_err(|e| ConfigUpdateError::Verification(format!("Signature verification failed: {}", e)))?;

        Ok(())
    }

    /// Check for available updates
    pub async fn check_for_updates(&self) -> Result<Vec<AdapterMetadata>, ConfigUpdateError> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let url = format!("{}/list", self.config.adapter_download_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ConfigUpdateError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ConfigUpdateError::Http(format!(
                "Update check failed with status: {}",
                response.status()
            )));
        }

        let adapters: Vec<AdapterMetadata> = response.json().await
            .map_err(|e| ConfigUpdateError::Http(format!("Failed to parse adapter list: {}", e)))?;

        Ok(adapters)
    }
}

fn sha256_checksum(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigUpdateError {
    #[error("Service is disabled")]
    Disabled,

    #[error("IO error: {0}")]
    Io(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Verification error: {0}")]
    Verification(String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_checksum() {
        let data = b"test data";
        let checksum = sha256_checksum(data);
        assert_eq!(checksum.len(), 64); // SHA256 hex string length
    }
}

