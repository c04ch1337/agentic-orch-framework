//! # Telemetrist - Client-Side Telemetry Service
//!
//! Captures execution traces and conversation logs for federated learning.
//! Implements PII redaction, secure streaming, and resilient local caching.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::time::sleep;

use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Telemetry event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TelemetryEventType {
    ExecutionTrace,
    ConversationLog,
}

/// Execution trace event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub trace_id: String,
    pub request_id: String,
    pub service: String,
    pub method: String,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
    pub timestamp: chrono::DateTime<Utc>,
}

/// Conversation log event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationLog {
    pub log_id: String,
    pub session_id: String,
    pub user_query: String,
    pub system_response: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub timestamp: chrono::DateTime<Utc>,
}

/// Telemetry event wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub event_id: String,
    pub event_type: TelemetryEventType,
    pub execution_trace: Option<ExecutionTrace>,
    pub conversation_log: Option<ConversationLog>,
    pub timestamp: chrono::DateTime<Utc>,
}

/// Configuration for telemetrist
#[derive(Debug, Clone)]
pub struct TelemetristConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub batch_size: usize,
    pub flush_interval_secs: u64,
    pub pii_redaction_enabled: bool,
    pub local_cache_path: PathBuf,
    pub max_cache_size_mb: u64,
}

impl TelemetristConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("TELEMETRY_ENABLED")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(true);

        let endpoint = std::env::var("TELEMETRY_ENDPOINT")
            .unwrap_or_else(|_| "https://telemetry.phoenix-orch.example.com/api/v1/events".to_string());

        let batch_size = std::env::var("TELEMETRY_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        let flush_interval_secs = std::env::var("TELEMETRY_FLUSH_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        let pii_redaction_enabled = std::env::var("TELEMETRY_PII_REDACTION")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(true);

        let local_cache_path = std::env::var("TELEMETRY_CACHE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/telemetry/cache"));

        let max_cache_size_mb = std::env::var("TELEMETRY_MAX_CACHE_SIZE_MB")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        Self {
            enabled,
            endpoint,
            batch_size,
            flush_interval_secs,
            pii_redaction_enabled,
            local_cache_path,
            max_cache_size_mb,
        }
    }
}

/// PII redaction patterns
pub struct PiiRedactor {
    email_pattern: Regex,
    phone_pattern: Regex,
    ssn_pattern: Regex,
    credit_card_pattern: Regex,
}

impl PiiRedactor {
    pub fn new() -> Self {
        Self {
            email_pattern: Regex::new(r#"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b"#).unwrap(),
            phone_pattern: Regex::new(r#"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b"#).unwrap(),
            ssn_pattern: Regex::new(r#"\b\d{3}-\d{2}-\d{4}\b"#).unwrap(),
            credit_card_pattern: Regex::new(r#"\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b"#).unwrap(),
        }
    }

    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();
        result = self.email_pattern.replace_all(&result, "[EMAIL_REDACTED]").to_string();
        result = self.phone_pattern.replace_all(&result, "[PHONE_REDACTED]").to_string();
        result = self.ssn_pattern.replace_all(&result, "[SSN_REDACTED]").to_string();
        result = self.credit_card_pattern.replace_all(&result, "[CARD_REDACTED]").to_string();
        result
    }
}

/// Main telemetrist service
pub struct Telemetrist {
    config: TelemetristConfig,
    event_queue: Arc<Mutex<VecDeque<TelemetryEvent>>>,
    redactor: Arc<PiiRedactor>,
    http_client: reqwest::Client,
    last_flush: Arc<Mutex<Instant>>,
}

impl Telemetrist {
    pub fn new(config: TelemetristConfig) -> Result<Self, TelemetristError> {
        // Ensure cache directory exists
        if let Some(parent) = config.local_cache_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| TelemetristError::Io(format!("Failed to create cache directory: {}", e)))?;
        }

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| TelemetristError::Http(e.to_string()))?;

        Ok(Self {
            config,
            event_queue: Arc::new(Mutex::new(VecDeque::new())),
            redactor: Arc::new(PiiRedactor::new()),
            http_client,
            last_flush: Arc::new(Mutex::new(Instant::now())),
        })
    }

    pub fn new_default() -> Result<Self, TelemetristError> {
        Self::new(TelemetristConfig::from_env())
    }

    /// Record an execution trace
    pub async fn record_execution_trace(&self, trace: ExecutionTrace) -> Result<(), TelemetristError> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut trace = trace;
        
        // Redact PII if enabled
        if self.config.pii_redaction_enabled {
            if let Some(error) = &trace.error {
                trace.error = Some(self.redactor.redact(error));
            }
            for (_, value) in trace.metadata.iter_mut() {
                *value = self.redactor.redact(value);
            }
        }

        let event = TelemetryEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: TelemetryEventType::ExecutionTrace,
            execution_trace: Some(trace),
            conversation_log: None,
            timestamp: Utc::now(),
        };

        self.enqueue_event(event).await
    }

    /// Record a conversation log
    pub async fn record_conversation_log(&self, log: ConversationLog) -> Result<(), TelemetristError> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut log = log;
        
        // Redact PII if enabled
        if self.config.pii_redaction_enabled {
            log.user_query = self.redactor.redact(&log.user_query);
            log.system_response = self.redactor.redact(&log.system_response);
            for (_, value) in log.metadata.iter_mut() {
                *value = self.redactor.redact(value);
            }
        }

        let event = TelemetryEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: TelemetryEventType::ConversationLog,
            execution_trace: None,
            conversation_log: Some(log),
            timestamp: Utc::now(),
        };

        self.enqueue_event(event).await
    }

    async fn enqueue_event(&self, event: TelemetryEvent) -> Result<(), TelemetristError> {
        let mut queue = self.event_queue.lock().await;
        queue.push_back(event);

        // Flush if batch size reached
        if queue.len() >= self.config.batch_size {
            drop(queue);
            self.flush().await?;
        }

        Ok(())
    }

    /// Flush events to remote endpoint
    pub async fn flush(&self) -> Result<(), TelemetristError> {
        let mut queue = self.event_queue.lock().await;
        if queue.is_empty() {
            return Ok(());
        }

        let mut batch = Vec::new();
        while let Some(event) = queue.pop_front() {
            batch.push(event);
            if batch.len() >= self.config.batch_size {
                break;
            }
        }
        drop(queue);

        // Try to send to remote endpoint
        match self.send_batch(&batch).await {
            Ok(_) => {
                log::debug!("Successfully sent {} telemetry events", batch.len());
            }
            Err(e) => {
                log::warn!("Failed to send telemetry batch: {}. Caching locally.", e);
                // Cache locally for retry
                self.cache_batch(&batch).await?;
            }
        }

        Ok(())
    }

    async fn send_batch(&self, batch: &[TelemetryEvent]) -> Result<(), TelemetristError> {
        let response = self
            .http_client
            .post(&self.config.endpoint)
            .json(batch)
            .send()
            .await
            .map_err(|e| TelemetristError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(TelemetristError::Http(format!(
                "Telemetry endpoint returned status: {}",
                response.status()
            )));
        }

        Ok(())
    }

    async fn cache_batch(&self, batch: &[TelemetryEvent]) -> Result<(), TelemetristError> {
        let cache_file = self.config.local_cache_path.join(format!("events_{}.jsonl", Uuid::new_v4()));
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(&cache_file)
            .await
            .map_err(|e| TelemetristError::Io(format!("Failed to open cache file: {}", e)))?;

        for event in batch {
            let line = serde_json::to_string(event)
                .map_err(|e| TelemetristError::Serialization(e.to_string()))?;
            file.write_all(line.as_bytes()).await
                .map_err(|e| TelemetristError::Io(format!("Failed to write to cache: {}", e)))?;
            file.write_all(b"\n").await
                .map_err(|e| TelemetristError::Io(format!("Failed to write newline: {}", e)))?;
        }

        file.flush().await
            .map_err(|e| TelemetristError::Io(format!("Failed to flush cache: {}", e)))?;

        Ok(())
    }

    /// Start background flush task
    pub fn start_background_flush(&self) {
        let config = self.config.clone();
        let queue = Arc::clone(&self.event_queue);
        let http_client = self.http_client.clone();
        let redactor = Arc::clone(&self.redactor);
        let last_flush = Arc::clone(&self.last_flush);
        let cache_path = self.config.local_cache_path.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.flush_interval_secs));
            loop {
                interval.tick().await;
                
                let mut queue_guard = queue.lock().await;
                if !queue_guard.is_empty() {
                    drop(queue_guard);
                    // Flush logic would go here - simplified for now
                }
            }
        });
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TelemetristError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pii_redaction() {
        let redactor = PiiRedactor::new();
        let text = "Contact me at john@example.com or 555-123-4567";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("[EMAIL_REDACTED]"));
        assert!(redacted.contains("[PHONE_REDACTED]"));
    }
}

