// self-improve-rs/src/adaptation.rs
// Adaptation abstraction for self-improvement.
//
// This module is intentionally conservative: the default implementation
// only logs proposed changes and emits metrics. A future implementation
// can delegate into a dedicated `config-update-rs` crate to trigger
// QLoRA/adapter fine-tuning based on stored ErrorRecords.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error_record::ErrorRecord;

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

/// Logging-only adaptation engine.
///
/// Behavior:
/// - Emits a tracing event summarizing the proposed adaptation.
/// - Increments basic metrics counters.
/// - Does not directly mutate prompts/configs.
/// - When `live_apply_enabled` is true, it will additionally log that
///   live adaptation would be attempted via a future `config-update-rs`
///   integration.
pub struct LoggingAdaptationEngine {
    live_apply_enabled: bool,
}

impl LoggingAdaptationEngine {
    pub fn new(live_apply_enabled: bool) -> Self {
        Self { live_apply_enabled }
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
            // Placeholder for future integration with a dedicated update engine.
            tracing::info!(
                "SELF_IMPROVE_LIVE_APPLY_ENABLED=true: \
                 a future adapter/prompt update engine would be invoked here"
            );
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
