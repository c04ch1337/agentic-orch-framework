// auth-service-rs/src/certificates.rs
//
// Certificate management for mutual TLS (mTLS) authentication
// Provides:
// - CA certificate generation and management
// - Service certificate issuance
// - Certificate validation
// - Certificate revocation
// - Automated certificate rotation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc};
use tokio::sync::{RwLock, Mutex};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow, Context};
use tracing::{debug, error, info, warn};
use ring::digest;
use rcgen::{Certificate, CertificateParams, DnType, DistinguishedName, IsCa, KeyPair, KeyUsagePurpose};
use x509_parser::parse_x509_certificate;
use once_cell::sync::Lazy;

use crate::storage::{StorageBackend, Entity};

// Global instance for certificate management
static CERTIFICATE_MANAGER: Lazy<RwLock<Option<Arc<CertificateManager>>>> = Lazy::new(|| {
    RwLock::new(None)
});

/// Certificate information stored in the database
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertificateInfo {
    pub id: String,
    pub subject: String,
    pub issuer: String,
    pub not_before: i64,
    pub not_after: i64,
    pub serial_number: String,
    pub fingerprint: String,
    pub certificate_pem: String, 
    pub certificate_type: CertificateType,
    pub revoked: bool,
    pub revoked_at: Option<i64>,
    pub revocation_reason: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl Entity for CertificateInfo {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_entity_type() -> &'static str {
        "certificate"
    }
}

/// Certificate types
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum CertificateType {
    CA,              // Certificate Authority
    Intermediate,    // Intermediate CA
    Server,          // Server certificate
    Client,          // Client certificate
    Peer,            // Peer certificate for mTLS
}

impl std::fmt::Display for CertificateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertificateType::CA => write!(f, "ca"),
            CertificateType::Intermediate => write!(f, "intermediate"),
            CertificateType::Server => write!(f, "server"),
            CertificateType::Client => write!(f, "client"),
            CertificateType::Peer => write!(f, "peer"),
        }
    }
}

impl From<&str> for CertificateType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ca" => CertificateType::CA,
            "intermediate" => CertificateType::Intermediate,
            "server" => CertificateType::Server,
            "client" => CertificateType::Client,
            "peer" => CertificateType::Peer,
            _ => CertificateType::Server, // Default
        }
    }
}

/// CRL (Certificate Revocation List) entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrlEntry {
    pub serial_number: String,
    pub revoked_at: i64,
    pub reason_code: u8,
    pub reason: String,
}

/// Certificate manager
pub struct CertificateManager {
    storage: Arc<dyn StorageBackend>,
    
    // CA certificate and key
    ca_cert: RwLock<Option<Certificate>>,
    ca_cert_pem: RwLock<Option<String>>,
    ca_key_pem: RwLock<Option<String>>,
    ca_info: RwLock<Option<CertificateInfo>>,
    
    // Cache of issued certificates
    certificates_cache: RwLock<HashMap<String, CertificateInfo>>,
    
    // Revocation list (CRL)
    revocation_list: RwLock<Vec<CrlEntry>>,
    
    // Certificate issuance lock to prevent race conditions
    issuance_lock: Mutex<()>,
}

impl CertificateManager {
    /// Create a new certificate manager
    pub async fn new(storage: Arc<dyn StorageBackend>) -> Result<Self> {
        let manager = Self {
            storage,
            ca_cert: RwLock::new(None),
            ca_cert_pem: RwLock::new(None),
            ca_key_pem: RwLock::new(None),
            ca_info: RwLock::new(None),
            certificates_cache: RwLock::new(HashMap::new()),
            revocation_list: RwLock::new(Vec::new()),
            issuance_lock: Mutex::new(()),
        };
        
        // Initialize the CA certificate
        manager.initialize_ca().await?;
        
        // Load revocation list
        manager.load_revocation_list().await?;
        
        Ok(manager)
    }
    
    /// Initialize the global certificate manager
    pub async fn init_global(storage: Arc<dyn StorageBackend>) -> Result<()> {
        let manager = Self::new(storage).await?;
        
        let mut cert_manager = CERTIFICATE_MANAGER.write().await;
        *cert_manager = Some(Arc::new(manager));
        
        Ok(())
    }
    
    /// Get the global certificate manager
    pub async fn get_global() -> Result<Arc<CertificateManager>> {
        let cert_manager = CERTIFICATE_MANAGER.read().await;
        match &*cert_manager {
            Some(manager) => Ok(manager.clone()),
            None => Err(anyhow!("Certificate manager not initialized")),
        }
    }
    
    /// Initialize the CA certificate
    async fn initialize_ca(&self) -> Result<()> {
        // First check if we already have a CA certificate in the database
        let ca_certs = self.storage
            .query_entities::<CertificateInfo>(
                "certificate_type = 'ca' AND revoked = false"
            ).await
            .context("Failed to query CA certificates")?;
        
        if !ca_certs.is_empty() {
            // Found existing CA certificate
            let ca_info = &ca_certs[0];
            
            // Load private key from secrets service
            let ca_key_pem = self.get_certificate_private_key(&ca_info.id).await?;
            if ca_key_pem.is_none() {
                return Err(anyhow!("CA certificate exists but private key not found"));
            }
            
            // Parse the certificate
            let ca_cert = self.parse_certificate_from_pem(
                &ca_info.certificate_pem,
                &ca_key_pem.unwrap()
            )?;
            
            // Store in memory
            {
                let mut ca_cert_lock = self.ca_cert.write().await;
                *ca_cert_lock = Some(ca_cert);
                
                let mut ca_cert_pem_lock = self.ca_cert_pem.write().await;
                *ca_cert_pem_lock = Some(ca_info.certificate_pem.clone());
                
                let mut ca_key_pem_lock = self.ca_key_pem.write().await;
                *ca_key_pem_lock = ca_key_pem;
                
                let mut ca_info_lock = self.ca_info.write().await;
                *ca_info_lock = Some(ca_info.clone());
            }
            
            info!("Loaded existing CA certificate: {}", ca_info.id);
            
            return Ok(());
        }
        
        // No existing CA certificate, create a new one
        info!("No existing CA certificate found, generating new CA");
        
        // Acquire the issuance lock
        let _lock = self.issuance_lock.lock().await;
        
        // Create CA parameters
        let mut params = CertificateParams::default();
        
        // Set CA properties
        params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Constrained(0));
        
        // Set subject DN
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, "Phoenix ORCH AGI CA");
        distinguished_name.push(DnType::OrganizationName, "Phoenix ORCH AGI");
        params.distinguished_name = distinguished_name;
        
        // Set key usage
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        
        // Set validity (10 years typical for CA)
        params.not_before = SystemTime::now();
        params.not_after = SystemTime::now() + Duration::from_secs(315_360_000); // 10 years
        
        // Generate the CA certificate
        let ca_cert = Certificate::from_params(params).context("Failed to create CA certificate")?;
        
        // Get the certificate as PEM
        let ca_cert_pem = ca_cert.serialize_pem().context("Failed to serialize CA certificate")?;
        
        // Get the private key as PEM
        let ca_key_pem = ca_cert.serialize_private_key_pem();
        
        // Store the CA certificate in the database
        let cert_info = self.create_certificate_info(
            &ca_cert_pem,
            CertificateType::CA,
            None, // No parent for CA
        )?;
        
        self.storage.store_entity(&cert_info).await
            .context("Failed to store CA certificate")?;
        
        // Store the private key in secure storage
        self.store_certificate_private_key(&cert_info.id, &ca_key_pem).await?;
        
        // Store in memory
        {
            let mut ca_cert_lock = self.ca_cert.write().await;
            *ca_cert_lock = Some(ca_cert);
            
            let mut ca_cert_pem_lock = self.ca_cert_pem.write().await;
            *ca_cert_pem_lock = Some(ca_cert_pem);
            
            let mut ca_key_pem_lock = self.ca_key_pem.write().await;
            *ca_key_pem_lock = Some(ca_key_pem);
            
            let mut ca_info_lock = self.ca_info.write().await;
            *ca_info_lock = Some(cert_info);
        }
        
        info!("Generated and stored new CA certificate");
        
        Ok(())
    }
    
    /// Create certificate info from PEM
    fn create_certificate_info(
        &self,
        cert_pem: &str,
        cert_type: CertificateType,
        parent_cert_id: Option<&str>,
    ) -> Result<CertificateInfo> {
        // Parse the certificate
        let (_, cert) = parse_x509_certificate(cert_pem.as_bytes())
            .map_err(|e| anyhow!("Failed to parse certificate: {}", e))?;
        
        // Get certificate properties
        let subject = cert.subject.to_string();
        let issuer = cert.issuer.to_string();
        
        let not_before = cert.validity.not_before.timestamp();
        let not_after = cert.validity.not_after.timestamp();
        
        // Get serial number as hex string
        let serial = cert.serial.to_string();
        
        // Compute SHA-256 fingerprint
        let fingerprint = self.compute_certificate_fingerprint(cert_pem)?;
        
        // Create certificate info
        let cert_id = Uuid::new_v4().to_string();
        
        // Create metadata
        let mut metadata = HashMap::new();
        if let Some(parent_id) = parent_cert_id {
            metadata.insert("parent_id".to_string(), parent_id.to_string());
        }
        
        let cert_info = CertificateInfo {
            id: cert_id,
            subject,
            issuer,
            not_before,
            not_after,
            serial_number: serial,
            fingerprint,
            certificate_pem: cert_pem.to_string(),
            certificate_type: cert_type,
            revoked: false,
            revoked_at: None,
            revocation_reason: None,
            metadata,
        };
        
        Ok(cert_info)
    }
    
    /// Compute SHA-256 fingerprint of a certificate
    fn compute_certificate_fingerprint(&self, cert_pem: &str) -> Result<String> {
        let (_, cert) = parse_x509_certificate(cert_pem.as_bytes())
            .map_err(|e| anyhow!("Failed to parse certificate: {}", e))?;
            
        // Calculate SHA-256 hash of DER encoding
        let binary_der = cert.as_ref();
        
        let digest = digest::digest(&digest::SHA256, binary_der);
        
        // Convert to hex string with colons
        let fingerprint = digest
            .as_ref()
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join(":");
        
        Ok(fingerprint)
    }
    
    /// Store a certificate's private key in secure storage
    async fn store_certificate_private_key(&self, cert_id: &str, key_pem: &str) -> Result<()> {
        // Use the secrets service to store the key
        use crate::secrets_client::{SecretsClient, get_secrets_client};
        
        let secrets = get_secrets_client().await
            .context("Failed to get secrets client")?;
        
        // Store key with the certificate ID as the key name
        let secret_key = format!("cert_key_{}", cert_id);
        secrets.store_secret(&secret_key, key_pem).await?;
        
        Ok(())
    }
    
    /// Get a certificate's private key from secure storage
    async fn get_certificate_private_key(&self, cert_id: &str) -> Result<Option<String>> {
        // Use the secrets service to retrieve the key
        use crate::secrets_client::{SecretsClient, get_secrets_client};
        
        let secrets = get_secrets_client().await
            .context("Failed to get secrets client")?;
        
        // Get key with the certificate ID as the key name
        let secret_key = format!("cert_key_{}", cert_id);
        match secrets.get_secret(&secret_key).await {
            Ok(key) => Ok(Some(key)),
            Err(_) => Ok(None),
        }
    }
    
    /// Parse certificate from PEM
    fn parse_certificate_from_pem(&self, cert_pem: &str, key_pem: &str) -> Result<Certificate> {
        // Parse using rcgen
        let key_pair = KeyPair::from_pem(key_pem)
            .context("Failed to parse private key PEM")?;
            
        let cert_params = CertificateParams::from_ca_cert_pem(cert_pem, key_pair)
            .context("Failed to parse certificate PEM")?;
            
        let cert = Certificate::from_params(cert_params)
            .context("Failed to create certificate from params")?;
            
        Ok(cert)
    }
    
    /// Get the CA certificate
    pub async fn get_ca_certificate(&self) -> Result<String> {
        let ca_cert_pem = self.ca_cert_pem.read().await;
        
        match &*ca_cert_pem {
            Some(pem) => Ok(pem.clone()),
            None => Err(anyhow!("CA certificate not initialized")),
        }
    }
    
    /// Get or create a certificate for a service
    pub async fn get_or_create_service_certificate(
        &self,
        service_id: &str,
        valid_days: Option<u32>,
        alt_names: Option<Vec<String>>,
    ) -> Result<(String, String, Option<String>)> {
        // First check if the service already has a valid certificate
        let query = format!(
            "certificate_type = 'server' AND subject LIKE '%CN={}%' AND revoked = false",
            service_id
        );
        
        let existing_certs = self.storage
            .query_entities::<CertificateInfo>(&query)
            .await
            .context("Failed to query service certificates")?;
        
        if !existing_certs.is_empty() {
            // Found existing certificates, check validity
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
                
            for cert in &existing_certs {
                // Check if certificate is still valid (not expired)
                if cert.not_after > now {
                    // Get the private key
                    if let Some(key_pem) = 
                        self.get_certificate_private_key(&cert.id).await? {
                        debug!("Using existing certificate for service {}", service_id);
                        
                        // Get the CA certificate for the chain
                        let ca_cert_pem = self.get_ca_certificate().await?;
                        
                        return Ok((cert.certificate_pem.clone(), key_pem, Some(ca_cert_pem)));
                    }
                }
            }
            
            // If we get here, there are existing certs but none are valid or have keys
            // We'll create a new one
        }
        
        // No valid existing certificate, create a new one
        let _lock = self.issuance_lock.lock().await;
        
        // Make sure we have a CA certificate
        let ca_cert = {
            let ca_cert_guard = self.ca_cert.read().await;
            match &*ca_cert_guard {
                Some(cert) => cert.clone(),
                None => return Err(anyhow!("CA certificate not initialized")),
            }
        };
        
        let ca_info = {
            let ca_info_guard = self.ca_info.read().await;
            match &*ca_info_guard {
                Some(info) => info.clone(),
                None => return Err(anyhow!("CA information not initialized")),
            }
        };
        
        // Create certificate params
        let mut params = CertificateParams::default();
        
        // Set subject
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, service_id);
        distinguished_name.push(DnType::OrganizationName, "Phoenix ORCH AGI");
        params.distinguished_name = distinguished_name;
        
        // Set validity (default 1 year)
        params.not_before = SystemTime::now();
        params.not_after = SystemTime::now() + Duration::from_secs(
            (valid_days.unwrap_or(365) as u64) * 86400
        );
        
        // Set key usage
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
            KeyUsagePurpose::KeyAgreement,
        ];
        
        // Set as server certificate
        params.is_ca = IsCa::NoCa;
        
        // Add subject alternative names
        params.subject_alt_names = vec![
            rcgen::SanType::DnsName(service_id.to_string()),
        ];
        
        if let Some(alt_names_vec) = alt_names {
            for name in alt_names_vec {
                params.subject_alt_names.push(rcgen::SanType::DnsName(name));
            }
        }
        
        // Add localhost and IP addresses for local development
        params.subject_alt_names.push(rcgen::SanType::DnsName("localhost".to_string()));
        params.subject_alt_names.push(rcgen::SanType::IpAddress("127.0.0.1".parse().unwrap()));
        
        // Generate the certificate
        let server_cert = Certificate::from_params(params)
            .context("Failed to create server certificate")?;
        
        // Sign with the CA
        let server_cert_pem = server_cert
            .serialize_pem_with_signer(&ca_cert)
            .context("Failed to sign server certificate")?;
        
        // Get the private key
        let server_key_pem = server_cert.serialize_private_key_pem();
        
        // Store the certificate
        let cert_info = self.create_certificate_info(
            &server_cert_pem,
            CertificateType::Server,
            Some(&ca_info.id),
        )?;
        
        self.storage
            .store_entity(&cert_info)
            .await
            .context("Failed to store server certificate")?;
        
        // Store the private key
        self.store_certificate_private_key(&cert_info.id, &server_key_pem).await?;
        
        // Get the CA certificate for the chain
        let ca_cert_pem = self.get_ca_certificate().await?;
        
        info!("Created new certificate for service {}", service_id);
        
        Ok((server_cert_pem, server_key_pem, Some(ca_cert_pem)))
    }
    
    /// Revoke a certificate
    pub async fn revoke_certificate(
        &self,
        cert_id: &str,
        reason: &str,
    ) -> Result<()> {
        // Get the certificate
        let cert = self.storage
            .get_entity::<CertificateInfo>(cert_id)
            .await
            .context("Failed to get certificate")?;
        
        // Check if already revoked
        if cert.revoked {
            return Err(anyhow!("Certificate already revoked"));
        }
        
        // Update certificate as revoked
        let mut updated_cert = cert.clone();
        updated_cert.revoked = true;
        updated_cert.revoked_at = Some(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64);
        updated_cert.revocation_reason = Some(reason.to_string());
        
        // Store updated certificate
        self.storage
            .store_entity(&updated_cert)
            .await
            .context("Failed to update certificate")?;
        
        // Add to revocation list
        let crl_entry = CrlEntry {
            serial_number: cert.serial_number,
            revoked_at: updated_cert.revoked_at.unwrap(),
            reason_code: 1, // Default reason code (key compromise)
            reason: reason.to_string(),
        };
        
        let mut revocation_list = self.revocation_list.write().await;
        revocation_list.push(crl_entry);
        
        // Remove from cache
        let mut cache = self.certificates_cache.write().await;
        cache.remove(cert_id);
        
        info!("Revoked certificate {} with reason: {}", cert_id, reason);
        
        Ok(())
    }
    
    /// Check if a certificate is revoked
    pub async fn is_certificate_revoked(&self, serial_number: &str) -> bool {
        let revocation_list = self.revocation_list.read().await;
        revocation_list.iter().any(|entry| entry.serial_number == serial_number)
    }
    
    /// Load the revocation list
    async fn load_revocation_list(&self) -> Result<()> {
        // Query all revoked certificates
        let revoked_certs = self.storage
            .query_entities::<CertificateInfo>("revoked = true")
            .await
            .context("Failed to query revoked certificates")?;
        
        let mut crl_entries = Vec::new();
        
        for cert in revoked_certs {
            if let Some(revoked_at) = cert.revoked_at {
                let entry = CrlEntry {
                    serial_number: cert.serial_number,
                    revoked_at,
                    reason_code: 1, // Default reason code
                    reason: cert.revocation_reason.unwrap_or_else(|| "Unknown".to_string()),
                };
                
                crl_entries.push(entry);
            }
        }
        
        // Update the revocation list
        let mut revocation_list = self.revocation_list.write().await;
        *revocation_list = crl_entries;
        
        info!("Loaded {} revoked certificates", revocation_list.len());
        
        Ok(())
    }
    
    /// Generate a CRL in PEM format
    pub async fn generate_crl(&self) -> Result<String> {
        // Not fully implemented - would require additional CRL serialization
        // This is a placeholder that returns a basic CRL
        
        let now = SystemTime::now();
        let now_secs = now.duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        // Get revocation list
        let revocation_list = self.revocation_list.read().await;
        
        // Format for X.509 CRL - simplified version
        let crl = format!(
            "-----BEGIN X509 CRL-----\n\
             Version: 2\n\
             Issuer: CN=Phoenix ORCH AGI CA\n\
             Last Update: {}\n\
             Next Update: {}\n\
             Revoked Certificates: {}\n\
             -----END X509 CRL-----",
            DateTime::<Utc>::from(now).format("%Y-%m-%d %H:%M:%S UTC"),
            DateTime::<Utc>::from(now + Duration::from_secs(86400)).format("%Y-%m-%d %H:%M:%S UTC"),
            revocation_list.len(),
        );
        
        Ok(crl)
    }
    
    /// List all certificates
    pub async fn list_certificates(
        &self,
        filter: Option<&str>,
        include_revoked: bool,
    ) -> Result<Vec<CertificateInfo>> {
        // Build query
        let mut query = String::new();
        
        if !include_revoked {
            query.push_str("revoked = false");
        }
        
        if let Some(f) = filter {
            if !query.is_empty() {
                query.push_str(" AND ");
            }
            query.push_str(f);
        }
        
        // Query certificates
        let certs = self.storage
            .query_entities::<CertificateInfo>(if query.is_empty() { None } else { Some(&query) })
            .await
            .context("Failed to query certificates")?;
            
        Ok(certs)
    }
}

// Convenience functions for global certificate manager
pub async fn get_or_create_service_certificate(
    service_id: &str,
    valid_days: Option<u32>,
    alt_names: Option<Vec<String>>,
) -> Result<(String, String, Option<String>)> {
    let cm = CertificateManager::get_global().await?;
    cm.get_or_create_service_certificate(service_id, valid_days, alt_names).await
}

pub async fn get_ca_certificate() -> Result<String> {
    let cm = CertificateManager::get_global().await?;
    cm.get_ca_certificate().await
}

pub async fn revoke_certificate(cert_id: &str, reason: &str) -> Result<()> {
    let cm = CertificateManager::get_global().await?;
    cm.revoke_certificate(cert_id, reason).await
}

pub async fn is_certificate_revoked(serial_number: &str) -> Result<bool> {
    let cm = CertificateManager::get_global().await?;
    Ok(cm.is_certificate_revoked(serial_number).await)
}

pub async fn generate_crl() -> Result<String> {
    let cm = CertificateManager::get_global().await?;
    cm.generate_crl().await
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::storage::MockStorage;
    
    pub async fn create_test_cert_manager() -> CertificateManager {
        use crate::secrets_client::init_mock_secrets_client;
        
        // Initialize mock secrets client
        init_mock_secrets_client().await;
        
        // Create storage backend
        let storage = Arc::new(MockStorage::new());
        
        // Create certificate manager
        CertificateManager::new(storage).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_certificate_generation() {
        let cm = create_test_cert_manager().await;
        
        // Get CA certificate
        let ca_cert = cm.get_ca_certificate().await.unwrap();
        assert!(!ca_cert.is_empty());
        
        // Create a service certificate
        let (cert_pem, key_pem, ca_chain) = cm
            .get_or_create_service_certificate("test-service", None, None)
            .await
            .unwrap();
            
        assert!(!cert_pem.is_empty());
        assert!(!key_pem.is_empty());
        assert!(ca_chain.is_some());
        
        // Parse the certificate
        let (_, cert) = parse_x509_certificate(cert_pem.as_bytes()).unwrap();
        
        // Check subject
        let subject = cert.subject.to_string();
        assert!(subject.contains("CN=test-service"));
        
        // Create a second certificate and verify it's cached
        let (cert_pem2, key_pem2, _) = cm
            .get_or_create_service_certificate("test-service", None, None)
            .await
            .unwrap();
            
        // Should return the same certificate
        assert_eq!(cert_pem, cert_pem2);
        assert_eq!(key_pem, key_pem2);
    }
    
    #[tokio::test]
    async fn test_certificate_revocation() {
        let cm = create_test_cert_manager().await;
        
        // Create a service certificate
        let (_, _, _) = cm
            .get_or_create_service_certificate("revoke-test", None, None)
            .await
            .unwrap();
            
        // List certificates to find the ID
        let certs = cm.list_certificates(
            Some("subject LIKE '%CN=revoke-test%'"),
            false
        ).await.unwrap();
        
        assert!(!certs.is_empty());
        
        let cert_id = certs[0].id.clone();
        let serial = certs[0].serial_number.clone();
        
        // Revoke the certificate
        cm.revoke_certificate(&cert_id, "Test revocation").await.unwrap();
        
        // Check if it's revoked
        assert!(cm.is_certificate_revoked(&serial).await);
        
        // Verify it's marked as revoked in storage
        let revoked_certs = cm.list_certificates(
            Some("subject LIKE '%CN=revoke-test%'"),
            true
        ).await.unwrap();
        
        assert!(!revoked_certs.is_empty());
        assert!(revoked_certs[0].revoked);
    }
}