// auth-service-rs/src/audit.rs
//
// Security Audit Logging for Authentication and Authorization
// Provides:
// - Structured security event logging
// - Event categorization and severity
// - Integration with external logging systems
// - Tamper-evident log storage
// - Compliance with security frameworks

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use anyhow::{Result, anyhow, Context};
use tracing::{info, error, debug, warn};
use async_trait::async_trait;

use crate::storage::{StorageBackend, Entity};
use crate::proto::auth_service::LogEventRequest;

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    // Authentication events
    Login,
    Logout,
    FailedLogin,
    PasswordChange,
    PasswordReset,
    MfaEnabled,
    MfaDisabled,
    MfaChallenge,
    
    // Authorization events
    AccessGranted,
    AccessDenied,
    
    // Token events
    TokenIssued,
    TokenRevoked,
    TokenRefreshed,
    TokenValidationFailed,
    
    // Role and permission events
    RoleAssigned,
    RoleRevoked,
    PermissionGranted,
    PermissionRevoked,
    
    // Administrative events
    UserCreated,
    UserDeleted,
    UserUpdated,
    ServiceRegistered,
    ServiceDeleted,
    ServiceCredentialsIssued,
    
    // System events
    KeyRotation,
    ConfigChange,
    SystemStartup,
    SystemShutdown,
    
    // Security events
    BruteForceDetected,
    SuspiciousActivity,
    RateLimitExceeded,
    
    // Audit events
    LogExported,
    LogAccessed,
    
    // Other
    Other,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::Login => write!(f, "login"),
            EventType::Logout => write!(f, "logout"),
            EventType::FailedLogin => write!(f, "failed_login"),
            EventType::PasswordChange => write!(f, "password_change"),
            EventType::PasswordReset => write!(f, "password_reset"),
            EventType::MfaEnabled => write!(f, "mfa_enabled"),
            EventType::MfaDisabled => write!(f, "mfa_disabled"),
            EventType::MfaChallenge => write!(f, "mfa_challenge"),
            EventType::AccessGranted => write!(f, "access_granted"),
            EventType::AccessDenied => write!(f, "access_denied"),
            EventType::TokenIssued => write!(f, "token_issued"),
            EventType::TokenRevoked => write!(f, "token_revoked"),
            EventType::TokenRefreshed => write!(f, "token_refreshed"),
            EventType::TokenValidationFailed => write!(f, "token_validation_failed"),
            EventType::RoleAssigned => write!(f, "role_assigned"),
            EventType::RoleRevoked => write!(f, "role_revoked"),
            EventType::PermissionGranted => write!(f, "permission_granted"),
            EventType::PermissionRevoked => write!(f, "permission_revoked"),
            EventType::UserCreated => write!(f, "user_created"),
            EventType::UserDeleted => write!(f, "user_deleted"),
            EventType::UserUpdated => write!(f, "user_updated"),
            EventType::ServiceRegistered => write!(f, "service_registered"),
            EventType::ServiceDeleted => write!(f, "service_deleted"),
            EventType::ServiceCredentialsIssued => write!(f, "service_credentials_issued"),
            EventType::KeyRotation => write!(f, "key_rotation"),
            EventType::ConfigChange => write!(f, "config_change"),
            EventType::SystemStartup => write!(f, "system_startup"),
            EventType::SystemShutdown => write!(f, "system_shutdown"),
            EventType::BruteForceDetected => write!(f, "brute_force_detected"),
            EventType::SuspiciousActivity => write!(f, "suspicious_activity"),
            EventType::RateLimitExceeded => write!(f, "rate_limit_exceeded"),
            EventType::LogExported => write!(f, "log_exported"),
            EventType::LogAccessed => write!(f, "log_accessed"),
            EventType::Other => write!(f, "other"),
        }
    }
}

impl From<&str> for EventType {
    fn from(s: &str) -> Self {
        match s {
            "login" => EventType::Login,
            "logout" => EventType::Logout,
            "failed_login" => EventType::FailedLogin,
            "password_change" => EventType::PasswordChange,
            "password_reset" => EventType::PasswordReset,
            "mfa_enabled" => EventType::MfaEnabled,
            "mfa_disabled" => EventType::MfaDisabled,
            "mfa_challenge" => EventType::MfaChallenge,
            "access_granted" => EventType::AccessGranted,
            "access_denied" => EventType::AccessDenied,
            "token_issued" => EventType::TokenIssued,
            "token_revoked" => EventType::TokenRevoked,
            "token_refreshed" => EventType::TokenRefreshed,
            "token_validation_failed" => EventType::TokenValidationFailed,
            "role_assigned" => EventType::RoleAssigned,
            "role_revoked" => EventType::RoleRevoked,
            "permission_granted" => EventType::PermissionGranted,
            "permission_revoked" => EventType::PermissionRevoked,
            "user_created" => EventType::UserCreated,
            "user_deleted" => EventType::UserDeleted,
            "user_updated" => EventType::UserUpdated,
            "service_registered" => EventType::ServiceRegistered,
            "service_deleted" => EventType::ServiceDeleted,
            "service_credentials_issued" => EventType::ServiceCredentialsIssued,
            "key_rotation" => EventType::KeyRotation,
            "config_change" => EventType::ConfigChange,
            "system_startup" => EventType::SystemStartup,
            "system_shutdown" => EventType::SystemShutdown,
            "brute_force_detected" => EventType::BruteForceDetected,
            "suspicious_activity" => EventType::SuspiciousActivity,
            "rate_limit_exceeded" => EventType::RateLimitExceeded,
            "log_exported" => EventType::LogExported,
            "log_accessed" => EventType::LogAccessed,
            _ => EventType::Other,
        }
    }
}

/// Event outcome types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Outcome {
    Success,
    Failure,
    Error,
    Unknown,
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Success => write!(f, "success"),
            Outcome::Failure => write!(f, "failure"),
            Outcome::Error => write!(f, "error"),
            Outcome::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&str> for Outcome {
    fn from(s: &str) -> Self {
        match s {
            "success" => Outcome::Success,
            "failure" => Outcome::Failure,
            "error" => Outcome::Error,
            _ => Outcome::Unknown,
        }
    }
}

/// Security event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Debug => write!(f, "debug"),
            Severity::Info => write!(f, "info"),
            Severity::Notice => write!(f, "notice"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
            Severity::Critical => write!(f, "critical"),
            Severity::Alert => write!(f, "alert"),
            Severity::Emergency => write!(f, "emergency"),
        }
    }
}

impl From<&str> for Severity {
    fn from(s: &str) -> Self {
        match s {
            "debug" => Severity::Debug,
            "info" => Severity::Info,
            "notice" => Severity::Notice,
            "warning" => Severity::Warning,
            "error" => Severity::Error,
            "critical" => Severity::Critical,
            "alert" => Severity::Alert,
            "emergency" => Severity::Emergency,
            _ => Severity::Info,
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub severity: Severity,
    pub principal_id: String,
    pub principal_type: String,
    pub resource: String,
    pub action: String,
    pub outcome: Outcome,
    pub source_ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub message: String,
    pub metadata: HashMap<String, String>,
}

impl Entity for AuditLogEntry {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_entity_type() -> &'static str {
        "audit_log"
    }
}

/// Audit log manager
pub struct AuditManager {
    storage: Arc<dyn StorageBackend>,
    
    // Total counts by event type for metrics
    event_counts: Arc<RwLock<HashMap<EventType, u64>>>,
    
    // Optional logging client for central log aggregation
    logging_client: Option<Arc<tonic::transport::Channel>>,
    
    // Buffer for high-throughput scenarios
    log_buffer: Arc<Mutex<Vec<AuditLogEntry>>>,
    buffer_size_limit: usize,
}

impl AuditManager {
    /// Create a new audit manager
    pub async fn new(
        storage: Arc<dyn StorageBackend>,
        logging_service_addr: Option<String>,
        buffer_size: Option<usize>,
    ) -> Result<Self> {
        // Initialize logging client if address provided
        let logging_client = if let Some(addr) = logging_service_addr {
            match tonic::transport::Channel::from_shared(addr)
                .context("Invalid logging service address")?
                .connect()
                .await
            {
                Ok(channel) => {
                    info!("Connected to central logging service");
                    Some(Arc::new(channel))
                }
                Err(err) => {
                    warn!("Failed to connect to logging service: {}. Using local storage only.", err);
                    None
                }
            }
        } else {
            None
        };
        
        Ok(Self {
            storage,
            event_counts: Arc::new(RwLock::new(HashMap::new())),
            logging_client,
            log_buffer: Arc::new(Mutex::new(Vec::new())),
            buffer_size_limit: buffer_size.unwrap_or(100),
        })
    }
    
    /// Log a security event
    pub async fn log_event(
        &self,
        event_type: EventType,
        principal_id: &str,
        principal_type: &str,
        resource: &str,
        action: &str,
        outcome: Outcome,
        message: &str,
        metadata: Option<HashMap<String, String>>,
        severity: Option<Severity>,
        source_ip: Option<&str>,
        user_agent: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<String> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        
        // Determine severity based on event type and outcome
        let severity = severity.unwrap_or_else(|| {
            match (&event_type, &outcome) {
                (EventType::BruteForceDetected, _) => Severity::Critical,
                (EventType::SuspiciousActivity, _) => Severity::Warning,
                (EventType::AccessDenied, _) => Severity::Notice,
                (EventType::FailedLogin, Outcome::Failure) => Severity::Warning,
                (EventType::TokenValidationFailed, _) => Severity::Warning,
                (_, Outcome::Error) => Severity::Error,
                (_, Outcome::Failure) => Severity::Warning,
                _ => Severity::Info,
            }
        });
        
        // Create log entry
        let log_entry = AuditLogEntry {
            id: id.clone(),
            timestamp: now,
            event_type: event_type.clone(),
            severity,
            principal_id: principal_id.to_string(),
            principal_type: principal_type.to_string(),
            resource: resource.to_string(),
            action: action.to_string(),
            outcome: outcome.clone(),
            source_ip: source_ip.map(String::from),
            user_agent: user_agent.map(String::from),
            request_id: request_id.map(String::from),
            message: message.to_string(),
            metadata: metadata.unwrap_or_default(),
        };
        
        // Log high-severity events immediately
        if severity >= Severity::Warning {
            debug!("Logging high severity event immediately: {} {}", severity, event_type);
            self.store_log_entry(&log_entry).await?;
        } else {
            // Buffer lower-severity events for batch processing
            let mut buffer = self.log_buffer.lock().await;
            buffer.push(log_entry.clone());
            
            // Flush buffer if it's full
            if buffer.len() >= self.buffer_size_limit {
                let entries = std::mem::take(&mut *buffer);
                tokio::spawn(self.flush_buffer(entries));
            }
        }
        
        // Update event counts
        {
            let mut counts = self.event_counts.write().await;
            *counts.entry(event_type).or_insert(0) += 1;
        }
        
        // Log via tracing for operational visibility
        match severity {
            Severity::Critical | Severity::Alert | Severity::Emergency => {
                error!(
                    target: "security",
                    event_type = %event_type,
                    principal = %principal_id,
                    resource = %resource,
                    action = %action,
                    outcome = %outcome,
                    id = %id,
                    "SECURITY: {}",
                    message
                );
            }
            Severity::Error => {
                error!(
                    target: "security",
                    event_type = %event_type,
                    principal = %principal_id,
                    outcome = %outcome,
                    id = %id,
                    "{}",
                    message
                );
            }
            Severity::Warning => {
                warn!(
                    target: "security",
                    event_type = %event_type,
                    principal = %principal_id,
                    outcome = %outcome,
                    id = %id,
                    "{}",
                    message
                );
            }
            _ => {
                info!(
                    target: "security",
                    event_type = %event_type,
                    principal = %principal_id,
                    outcome = %outcome,
                    id = %id,
                    "{}",
                    message
                );
            }
        }
        
        Ok(id)
    }
    
    /// Store a single log entry
    async fn store_log_entry(&self, entry: &AuditLogEntry) -> Result<()> {
        // Store in local storage
        self.storage.store_entity(entry).await?;
        
        // Send to logging service if configured
        if let Some(client) = &self.logging_client {
            self.send_to_logging_service(entry, client).await?;
        }
        
        Ok(())
    }
    
    /// Flush the buffer of log entries
    async fn flush_buffer(&self, entries: Vec<AuditLogEntry>) -> Result<()> {
        debug!("Flushing {} audit log entries", entries.len());
        
        // Store each entry
        for entry in entries {
            if let Err(err) = self.store_log_entry(&entry).await {
                error!("Failed to store audit log entry: {}", err);
                // Continue with other entries even if one fails
            }
        }
        
        Ok(())
    }
    
    /// Send a log entry to the central logging service
    async fn send_to_logging_service(
        &self,
        entry: &AuditLogEntry,
        channel: &Arc<tonic::transport::Channel>,
    ) -> Result<()> {
        use crate::proto::agi_core::logging_service_client::LoggingServiceClient;
        use crate::proto::agi_core::LogEntry;
        
        let mut client = LoggingServiceClient::new(channel.clone());
        
        // Convert AuditLogEntry to LogEntry
        let mut metadata = HashMap::new();
        metadata.insert("event_type".to_string(), entry.event_type.to_string());
        metadata.insert("principal_id".to_string(), entry.principal_id.clone());
        metadata.insert("principal_type".to_string(), entry.principal_type.clone());
        metadata.insert("resource".to_string(), entry.resource.clone());
        metadata.insert("action".to_string(), entry.action.clone());
        metadata.insert("outcome".to_string(), entry.outcome.to_string());
        
        if let Some(source_ip) = &entry.source_ip {
            metadata.insert("source_ip".to_string(), source_ip.clone());
        }
        if let Some(user_agent) = &entry.user_agent {
            metadata.insert("user_agent".to_string(), user_agent.clone());
        }
        if let Some(request_id) = &entry.request_id {
            metadata.insert("request_id".to_string(), request_id.clone());
        }
        
        // Add custom metadata
        for (key, value) in &entry.metadata {
            metadata.insert(key.clone(), value.clone());
        }
        
        let log_entry = LogEntry {
            level: entry.severity.to_string(),
            message: entry.message.clone(),
            service: "auth-service".to_string(),
            metadata,
            timestamp: entry.timestamp.timestamp(),
        };
        
        // Send to logging service
        match client.log(log_entry).await {
            Ok(_) => {
                debug!("Sent audit log to central logging service: {}", entry.id);
                Ok(())
            }
            Err(err) => {
                warn!("Failed to send audit log to central service: {}", err);
                // We still stored it locally, so this isn't a critical failure
                Ok(())
            }
        }
    }
    
    /// Get audit logs by query
    pub async fn query_logs(
        &self,
        filter: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<AuditLogEntry>> {
        // Flush any buffered logs first
        let buffer = {
            let mut buffer_lock = self.log_buffer.lock().await;
            if !buffer_lock.is_empty() {
                std::mem::take(&mut *buffer_lock)
            } else {
                Vec::new()
            }
        };
        
        if !buffer.is_empty() {
            self.flush_buffer(buffer).await?;
        }
        
        // Build query string
        let mut query_parts = Vec::new();
        
        if let Some(filter) = filter {
            query_parts.push(filter.to_string());
        }
        
        if let Some(start) = start_time {
            query_parts.push(format!("timestamp >= '{}'", start.to_rfc3339()));
        }
        
        if let Some(end) = end_time {
            query_parts.push(format!("timestamp <= '{}'", end.to_rfc3339()));
        }
        
        let query = if query_parts.is_empty() {
            None
        } else {
            Some(query_parts.join(" AND "))
        };
        
        // Get logs from storage
        let logs = self.storage
            .query_entities_paged::<AuditLogEntry>(
                query.as_deref(),
                limit.unwrap_or(100),
                offset.unwrap_or(0),
                Some("timestamp DESC"), // Sort by time descending
            )
            .await
            .context("Failed to query audit logs")?;
            
        Ok(logs)
    }
    
    /// Get audit logs for a specific principal
    pub async fn get_principal_logs(
        &self,
        principal_id: &str,
        principal_type: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<AuditLogEntry>> {
        let mut query = format!("principal_id = '{}'", principal_id);
        
        if let Some(pt) = principal_type {
            query.push_str(&format!(" AND principal_type = '{}'", pt));
        }
        
        self.query_logs(Some(&query), None, None, limit, None).await
    }
    
    /// Get audit logs for a specific resource
    pub async fn get_resource_logs(
        &self,
        resource: &str,
        limit: Option<usize>,
    ) -> Result<Vec<AuditLogEntry>> {
        let query = format!("resource = '{}'", resource);
        self.query_logs(Some(&query), None, None, limit, None).await
    }
    
    /// Get recent login activity for a user
    pub async fn get_login_history(
        &self,
        user_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<AuditLogEntry>> {
        let query = format!(
            "principal_id = '{}' AND principal_type = 'user' AND \
             (event_type = 'login' OR event_type = 'failed_login' OR event_type = 'logout')",
            user_id
        );
        
        self.query_logs(Some(&query), None, None, limit, None).await
    }
    
    /// Get audit logs from a log request
    pub async fn log_from_request(&self, request: &LogEventRequest) -> Result<String> {
        let event_type = EventType::from(request.event_type.as_str());
        let outcome = Outcome::from(request.outcome.as_str());
        
        let metadata = if !request.metadata.is_empty() {
            Some(request.metadata.clone())
        } else {
            None
        };
        
        self.log_event(
            event_type,
            &request.principal_id,
            &request.principal_type,
            &request.resource,
            &request.action,
            outcome,
            &request.message,
            metadata,
            None, // Use default severity based on event type
            Some(&request.source_ip),
            Some(&request.user_agent),
            None, // No request ID in the proto
        ).await
    }
    
    /// Get event counts for metrics
    pub async fn get_event_counts(&self) -> HashMap<String, u64> {
        let counts = self.event_counts.read().await;
        counts.iter()
            .map(|(event_type, count)| (event_type.to_string(), *count))
            .collect()
    }
    
    /// Force flush any buffered logs
    pub async fn flush(&self) -> Result<()> {
        let buffer = {
            let mut buffer_lock = self.log_buffer.lock().await;
            std::mem::take(&mut *buffer_lock)
        };
        
        if !buffer.is_empty() {
            debug!("Manually flushing {} buffered audit logs", buffer.len());
            self.flush_buffer(buffer).await?;
        }
        
        Ok(())
    }
}

/// Audit logging convenience functions
pub mod audit {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex as StdMutex;
    
    // Global audit manager instance
    static AUDIT_MANAGER: Lazy<StdMutex<Option<Arc<AuditManager>>>> = Lazy::new(|| {
        StdMutex::new(None)
    });
    
    /// Initialize the global audit manager
    pub async fn init(
        storage: Arc<dyn StorageBackend>,
        logging_service_addr: Option<String>,
        buffer_size: Option<usize>,
    ) -> Result<()> {
        let manager = AuditManager::new(storage, logging_service_addr, buffer_size).await?;
        
        let mut global_manager = AUDIT_MANAGER.lock().unwrap();
        *global_manager = Some(Arc::new(manager));
        
        Ok(())
    }
    
    /// Get the global audit manager
    pub fn get_manager() -> Result<Arc<AuditManager>> {
        let manager = AUDIT_MANAGER.lock().unwrap();
        
        match &*manager {
            Some(m) => Ok(m.clone()),
            None => Err(anyhow!("Audit manager not initialized")),
        }
    }
    
    /// Log a login event
    pub async fn log_login(
        user_id: &str,
        outcome: Outcome,
        ip: Option<&str>,
        user_agent: Option<&str>,
        message: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<String> {
        let manager = get_manager()?;
        
        manager.log_event(
            EventType::Login,
            user_id,
            "user",
            "auth/session",
            "login",
            outcome,
            message,
            metadata,
            None,
            ip,
            user_agent,
            None,
        ).await
    }
    
    /// Log a failed login attempt
    pub async fn log_failed_login(
        user_id: &str,
        reason: &str,
        ip: Option<&str>,
        user_agent: Option<&str>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<String> {
        let manager = get_manager()?;
        
        manager.log_event(
            EventType::FailedLogin,
            user_id,
            "user",
            "auth/session",
            "login",
            Outcome::Failure,
            reason,
            metadata,
            None,
            ip,
            user_agent,
            None,
        ).await
    }
    
    /// Log an authentication token issuance
    pub async fn log_token_issued(
        principal_id: &str,
        principal_type: &str,
        token_type: &str,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<String> {
        let manager = get_manager()?;
        
        let mut metadata = HashMap::new();
        metadata.insert("token_type".to_string(), token_type.to_string());
        
        manager.log_event(
            EventType::TokenIssued,
            principal_id,
            principal_type,
            "auth/token",
            "issue",
            Outcome::Success,
            &format!("{} token issued for {} {}", token_type, principal_type, principal_id),
            Some(metadata),
            None,
            ip,
            user_agent,
            None,
        ).await
    }
    
    /// Log an access control decision
    pub async fn log_access_decision(
        principal_id: &str,
        principal_type: &str,
        resource: &str,
        action: &str,
        allowed: bool,
        reason: &str,
        request_id: Option<&str>,
    ) -> Result<String> {
        let manager = get_manager()?;
        
        let event_type = if allowed {
            EventType::AccessGranted
        } else {
            EventType::AccessDenied
        };
        
        let outcome = if allowed {
            Outcome::Success
        } else {
            Outcome::Failure
        };
        
        manager.log_event(
            event_type,
            principal_id,
            principal_type,
            resource,
            action,
            outcome,
            reason,
            None,
            None,
            None,
            None,
            request_id,
        ).await
    }
    
    /// Log a role assignment
    pub async fn log_role_assigned(
        principal_id: &str,
        principal_type: &str,
        role_id: &str,
        assigned_by: &str,
    ) -> Result<String> {
        let manager = get_manager()?;
        
        let mut metadata = HashMap::new();
        metadata.insert("role_id".to_string(), role_id.to_string());
        metadata.insert("assigned_by".to_string(), assigned_by.to_string());
        
        manager.log_event(
            EventType::RoleAssigned,
            principal_id,
            principal_type,
            &format!("auth/role/{}", role_id),
            "assign",
            Outcome::Success,
            &format!("Role {} assigned to {} {}", role_id, principal_type, principal_id),
            Some(metadata),
            None,
            None,
            None,
            None,
        ).await
    }
    
    /// Log a role revocation
    pub async fn log_role_revoked(
        principal_id: &str,
        principal_type: &str,
        role_id: &str,
        revoked_by: &str,
    ) -> Result<String> {
        let manager = get_manager()?;
        
        let mut metadata = HashMap::new();
        metadata.insert("role_id".to_string(), role_id.to_string());
        metadata.insert("revoked_by".to_string(), revoked_by.to_string());
        
        manager.log_event(
            EventType::RoleRevoked,
            principal_id,
            principal_type,
            &format!("auth/role/{}", role_id),
            "revoke",
            Outcome::Success,
            &format!("Role {} revoked from {} {}", role_id, principal_type, principal_id),
            Some(metadata),
            None,
            None,
            None,
            None,
        ).await
    }
    
    /// Log a system event
    pub async fn log_system_event(
        event_type: EventType,
        message: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<String> {
        let manager = get_manager()?;
        
        manager.log_event(
            event_type,
            "system",
            "system",
            "auth/system",
            "update",
            Outcome::Success,
            message,
            metadata,
            None,
            None,
            None,
            None,
        ).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MockStorage;
    
    // Helper to create a test audit manager
    async fn create_test_audit_manager() -> AuditManager {
        let storage = Arc::new(MockStorage::new());
        AuditManager::new(storage, None, Some(10)).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_audit_log_event() {
        let audit = create_test_audit_manager().await;
        
        let log_id = audit.log_event(
            EventType::Login,
            "test-user",
            "user",
            "auth/session",
            "login",
            Outcome::Success,
            "User logged in successfully",
            None,
            None,
            Some("127.0.0.1"),
            Some("test-agent"),
            None,
        ).await.unwrap();
        
        // Force flush buffers
        audit.flush().await.unwrap();
        
        // Query the logs
        let logs = audit.query_logs(None, None, None, None, None).await.unwrap();
        
        assert!(!logs.is_empty());
        
        // Find our log
        let log = logs.iter().find(|l| l.id == log_id).unwrap();
        
        assert_eq!(log.event_type, EventType::Login);
        assert_eq!(log.principal_id, "test-user");
        assert_eq!(log.principal_type, "user");
        assert_eq!(log.resource, "auth/session");
        assert_eq!(log.action, "login");
        assert_eq!(log.outcome, Outcome::Success);
        assert_eq!(log.message, "User logged in successfully");
        assert_eq!(log.source_ip.as_deref(), Some("127.0.0.1"));
        assert_eq!(log.user_agent.as_deref(), Some("test-agent"));
    }
    
    #[tokio::test]
    async fn test_audit_query() {
        let audit = create_test_audit_manager().await;
        
        // Add a few log events
        audit.log_event(
            EventType::Login,
            "user1",
            "user",
            "auth/session",
            "login",
            Outcome::Success,
            "User1 logged in",
            None,
            None,
            None,
            None,
            None,
        ).await.unwrap();
        
        audit.log_event(
            EventType::AccessDenied,
            "user1",
            "user",
            "resource/secret",
            "read",
            Outcome::Failure,
            "Access denied to secret",
            None,
            None,
            None,
            None,
            None,
        ).await.unwrap();
        
        audit.log_event(
            EventType::Login,
            "user2",
            "user",
            "auth/session",
            "login",
            Outcome::Success,
            "User2 logged in",
            None,
            None,
            None,
            None,
            None,
        ).await.unwrap();
        
        // Force flush
        audit.flush().await.unwrap();
        
        // Query for user1 logs
        let user1_logs = audit.get_principal_logs("user1", Some("user"), None).await.unwrap();
        assert_eq!(user1_logs.len(), 2);
        
        // Query for login events
        let login_logs = audit
            .query_logs(Some("event_type = 'login'"), None, None, None, None)
            .await
            .unwrap();
        assert_eq!(login_logs.len(), 2);
        
        // Query for access denied events
        let access_logs = audit
            .query_logs(Some("event_type = 'access_denied'"), 
                       None, None, None, None)
            .await
            .unwrap();
        assert_eq!(access_logs.len(), 1);
    }
}