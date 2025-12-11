// self-improve-rs/src/adaptation.rs
// Adaptation abstraction for self-improvement.
//
// Integrates with config-update-rs to trigger QLoRA/adapter fine-tuning
// and configuration updates based on stored ErrorRecords.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error_record::ErrorRecord;
use config_update::{ConfigUpdateService, ConfigUpdateConfig, AdapterMetadata, ConfigUpdateMetadata};

/// Result of running an adaptation engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationResult {
    /// Whether any concrete adaptation was applied.
    pub applied: bool,
    /// Human-readable summary of what would or did change.
    pub summary: String,
}

/// Error type for adaptation engines.
#[derive(Debug, thiserror::Error)]
pub enum AdaptationError {
    #[error("internal adaptation error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait AdaptationEngine {
    async fn apply(&self, record: &ErrorRecord) -> Result<AdaptationResult, AdaptationError>;
}

/// Adaptation engine with config-update-rs integration.
///
/// Behavior:
/// - Emits a tracing event summarizing the proposed adaptation.
/// - Increments basic metrics counters.
/// - When `live_apply_enabled` is true, triggers adapter/config updates via config-update-rs.
pub struct LoggingAdaptationEngine {
    live_apply_enabled: bool,
    config_update_service: Option<Arc<ConfigUpdateService>>,
}

impl LoggingAdaptationEngine {
    pub fn new(live_apply_enabled: bool) -> Self {
        let config_update_service = if live_apply_enabled {
            match ConfigUpdateService::new(ConfigUpdateConfig::from_env()) {
                Ok(service) => {
                    tracing::info!("ConfigUpdateService initialized for self-improvement");
                    Some(Arc::new(service))
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize ConfigUpdateService: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            live_apply_enabled,
            config_update_service,
        }
    }
}

#[async_trait]
impl AdaptationEngine for LoggingAdaptationEngine {
    async fn apply(&self, record: &ErrorRecord) -> Result<AdaptationResult, AdaptationError> {
        // Basic metrics – names chosen to align with repo-wide conventions.
        metrics::increment_counter!("self_improve_example_records_total");
        metrics::increment_counter!(
            "self_improve_adaptations_total",
            "category" => record.error_category.clone()
        );

        let summary = format!(
            "record_id={} request_id={} category={} contributing_tools={:?}",
            record.id, record.request_id, record.error_category, record.contributing_tools,
        );

        tracing::info!(
            error_record.id = %record.id,
            error_record.request_id = %record.request_id,
            error_record.category = %record.error_category,
            ?record.contributing_tools,
            ?record.proposed_corrections,
            "self-improvement adaptation (logging-only)"
        );

        if self.live_apply_enabled {
            if let Some(update_service) = &self.config_update_service {
                // Trigger adapter update if error category suggests model improvement needed
                if record.error_category.contains("model") || record.error_category.contains("prompt") {
                    tracing::info!(
                        "Triggering adapter update for error category: {}",
                        record.error_category
                    );

                    // Check for available adapters
                    match update_service.check_for_updates().await {
                        Ok(adapters) => {
                            if !adapters.is_empty() {
                                // Download the first available adapter (in production, use smarter selection)
                                let adapter = &adapters[0];
                                let adapter_path = Path::new(&format!("./adapters/{}.bin", adapter.adapter_id));
                                
                                if let Err(e) = update_service.download_adapter(adapter, adapter_path).await {
                                    tracing::warn!("Failed to download adapter: {}", e);
                                } else {
                                    tracing::info!("Successfully downloaded adapter: {}", adapter.adapter_id);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to check for adapter updates: {}", e);
                        }
                    }
                }

                // Trigger config update if error category suggests configuration issue
                if record.error_category.contains("config") || record.error_category.contains("parameter") {
                    tracing::info!(
                        "Triggering config update for error category: {}",
                        record.error_category
                    );

                    // In production, this would fetch config update metadata from a remote source
                    // For now, we log the intent
                    tracing::info!(
                        "Config update triggered for record_id={}, category={}",
                        record.id,
                        record.error_category
                    );
                }
            } else {
                tracing::warn!(
                    "SELF_IMPROVE_LIVE_APPLY_ENABLED=true but ConfigUpdateService not available"
                );
            }
        }

        Ok(AdaptationResult {
            applied: false,
            summary,
        })
    }
}

    /// Helper to construct the default adaptation engine.
    ///
    /// This indirection makes it easier to switch to a different
    /// implementation later (e.g., delegating into `config-update-rs`).
    pub fn default_engine(live_apply_enabled: bool) -> Arc<dyn AdaptationEngine + Send + Sync> {
        Arc::new(LoggingAdaptationEngine::new(live_apply_enabled))
    }
