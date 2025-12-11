// reflection-service-rs/src/logging_client.rs
// Client for interacting with the Logging Service for event logging

use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::env;
use std::time::SystemTime;
use tonic::transport::Channel;
use tonic::Request;

use crate::agi_core::{logging_service_client::LoggingServiceClient, LogEntry, LogResponse};

/// Client for Logging Service operations
pub struct LoggingClient {
    client: Option<LoggingServiceClient<Channel>>,
    service_name: String,
    mock_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LoggingClient {
    /// Create a new Logging Service client
    pub async fn new() -> Self {
        // Get Logging Service address from env or use default
        let addr = env::var("LOGGING_SERVICE_ADDR")
            .unwrap_or_else(|_| "http://logging-service-rs:50056".to_string());

        // Try to connect to the Logging Service
        match LoggingServiceClient::connect(addr.clone()).await {
            Ok(client) => {
                info!("Connected to Logging Service at {}", addr);
                Self {
                    client: Some(client),
                    service_name: "reflection-service".to_string(),
                    mock_mode: false,
                }
            }
            Err(e) => {
                warn!(
                    "Failed to connect to Logging Service at {}: {}. Using mock client.",
                    addr, e
                );
                Self {
                    client: None,
                    service_name: "reflection-service".to_string(),
                    mock_mode: true,
                }
            }
        }
    }

    /// Log an event to the Logging Service
    pub async fn log_event(
        &mut self,
        level: LogLevel,
        message: &str,
        metadata: HashMap<String, String>,
    ) -> Result<String> {
        // Convert log level to string
        let level_str = match level {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };

        // If in mock mode, just log locally
        if self.mock_mode {
            match level {
                LogLevel::Debug => debug!("[MOCK LOG] {}", message),
                LogLevel::Info => info!("[MOCK LOG] {}", message),
                LogLevel::Warn => warn!("[MOCK LOG] {}", message),
                LogLevel::Error => error!("[MOCK LOG] {}", message),
            }
            return Ok("mock-log-id".to_string());
        }

        let client = match self.client.as_mut() {
            Some(client) => client,
            None => return Err(anyhow!("Logging Service client not available")),
        };

        // Get current timestamp in milliseconds
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        // Create log entry
        let log_entry = LogEntry {
            level: level_str.to_string(),
            message: message.to_string(),
            service: self.service_name.clone(),
            metadata,
            timestamp,
        };

        let request = Request::new(log_entry);

        // Send log to Logging Service
        match client.log(request).await {
            Ok(response) => {
                let response = response.into_inner();
                if response.success {
                    debug!("Successfully logged event with ID: {}", response.log_id);
                    Ok(response.log_id)
                } else {
                    warn!("Failed to log event to Logging Service");
                    Err(anyhow!("Failed to log event to Logging Service"))
                }
            }
            Err(e) => {
                warn!("Error logging event to Logging Service: {}", e);
                // Fall back to local logging when remote logging fails
                match level {
                    LogLevel::Debug => debug!("[FALLBACK] {}", message),
                    LogLevel::Info => info!("[FALLBACK] {}", message),
                    LogLevel::Warn => warn!("[FALLBACK] {}", message),
                    LogLevel::Error => error!("[FALLBACK] {}", message),
                }
                Err(anyhow!("Error logging event to Logging Service: {}", e))
            }
        }
    }

    /// Log an improvement event specifically for reflection events
    pub async fn log_improvement_event(
        &mut self,
        action: &str,
        lesson: &str,
        request_id: &str,
    ) -> Result<String> {
        let mut metadata = HashMap::new();
        metadata.insert("request_id".to_string(), request_id.to_string());
        metadata.insert("event_type".to_string(), "self_improvement".to_string());
        metadata.insert("action".to_string(), action.to_string());

        let message = format!(
            "Self-improvement event: Learned \"{}\" from action \"{}\"",
            lesson, action
        );

        self.log_event(LogLevel::Info, &message, metadata).await
    }

    /// Log a reflection event
    pub async fn log_reflection_event(
        &mut self,
        request_id: &str,
        success: bool,
        action: &str,
    ) -> Result<String> {
        let mut metadata = HashMap::new();
        metadata.insert("request_id".to_string(), request_id.to_string());
        metadata.insert("event_type".to_string(), "reflection".to_string());
        metadata.insert("success".to_string(), success.to_string());

        let level = if success {
            LogLevel::Info
        } else {
            LogLevel::Warn
        };
        let message = format!(
            "Reflection event: Action \"{}\" was {}",
            action,
            if success {
                "successful"
            } else {
                "unsuccessful"
            }
        );

        self.log_event(level, &message, metadata).await
    }

    /// Log a constraint event when a negative constraint is stored
    pub async fn log_constraint_event(
        &mut self,
        constraint: &str,
        request_id: &str,
    ) -> Result<String> {
        let mut metadata = HashMap::new();
        metadata.insert("request_id".to_string(), request_id.to_string());
        metadata.insert("event_type".to_string(), "constraint_storage".to_string());

        let message = format!(
            "Constraint stored: \"{}\" for request {}",
            constraint, request_id
        );

        self.log_event(LogLevel::Info, &message, metadata).await
    }

    /// Check if the client is running in mock mode
    pub fn is_mock(&self) -> bool {
        self.mock_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_event_mock() {
        let mut client = LoggingClient::new().await;

        let mut metadata = HashMap::new();
        metadata.insert("test_key".to_string(), "test_value".to_string());

        let result = client
            .log_event(LogLevel::Info, "Test log message", metadata)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_log_improvement_event() {
        let mut client = LoggingClient::new().await;

        let result = client
            .log_improvement_event("test_action", "test_lesson", "test-123")
            .await;

        assert!(result.is_ok());
    }
}
