 // self-improve-rs/src/lib.rs
 // Library interface for the Self-Improvement Engine.
 //
 // Public API is intentionally minimal and focused on internal service
 // integrations (orchestrator, reflection-service-rs, etc.).
 //
 // Design notes:
 // - This crate is a pure library crate; there is no HTTP server or
 //   standalone binary entrypoint.
 // - The adaptation path is conservative: by default it only logs
 //   proposed changes and writes reviewable artifacts; no automatic
 //   prompt/config mutation is performed unless explicitly enabled.

use std::{env, sync::Arc};

use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

pub mod model;

mod adaptation;
mod classifier;
mod error_record;
mod repository;

#[cfg(test)]
mod tests;

use crate::adaptation::{AdaptationEngine, AdaptationError};
use crate::classifier::{FailureClassification, FailureClassifier, HeuristicFailureClassifier};
use crate::error_record::ErrorRecord;
use crate::repository::ErrorRecordRepository;

/// High-level representation of a critical orchestration failure.
///
/// This struct is designed to be easy to construct from existing failure
/// logging paths (e.g. orchestrator, reflection service) without pulling
/// in concrete service types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalFailure {
    pub request_id: String,
    pub failure_type: String,
    pub stage: Option<String>,
    pub target_service: Option<String>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub original_query: Option<String>,
    pub plan_json: Option<serde_json::Value>,
    pub tool_transcripts: Option<serde_json::Value>,
    pub final_answer: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl CriticalFailure {
    /// Construct a CriticalFailure from a reflection-service-rs action failure.
    ///
    /// This helper is the preferred constructor for reflection-service-rs.
    pub fn from_reflection_failure(
        request_id: String,
        action_description: String,
        outcome: String,
        success: bool,
        mut metadata: std::collections::HashMap<String, String>,
    ) -> Self {
        // Only one failure type for now; can be expanded later.
        let failure_type = if success {
            "REFLECTION_ACTION_SUCCESS".to_string()
        } else {
            "REFLECTION_ACTION_FAILURE".to_string()
        };

        // Enrich metadata with reflection-specific keys.
        metadata.insert(
            "reflection_action_description".to_string(),
            action_description,
        );
        metadata.insert("reflection_success".to_string(), success.to_string());

        CriticalFailure {
            request_id,
            failure_type,
            stage: Some("reflection_action".to_string()),
            target_service: Some("reflection-service-rs".to_string()),
            error_type: Some("action_failure".to_string()),
            error_message: Some(outcome.clone()),
            original_query: None,
            plan_json: None,
            tool_transcripts: None,
            final_answer: None,
            metadata,
        }
    }

    /// Optional helper for orchestrator-service-rs failures.
    ///
    /// This is provided for future use by the orchestrator; it mirrors the
    /// reflection helper but uses orchestrator-specific defaults.
    pub fn from_orchestrator_failure(
        request_id: String,
        failure_type: String,
        error_message: Option<String>,
        mut metadata: std::collections::HashMap<String, String>,
    ) -> Self {
        metadata
            .entry("origin".to_string())
            .or_insert_with(|| "orchestrator".to_string());

        CriticalFailure {
            request_id,
            failure_type,
            stage: Some("orchestrator".to_string()),
            target_service: Some("orchestrator-service-rs".to_string()),
            error_type: Some("orchestrator_failure".to_string()),
            error_message: error_message.clone(),
            original_query: None,
            plan_json: None,
            tool_transcripts: None,
            final_answer: None,
            metadata,
        }
    }
}

/// Result type used by this crate.
pub type Result<T> = std::result::Result<T, SelfImproveError>;

/// Top-level error type for this crate.
#[derive(Debug, thiserror::Error)]
pub enum SelfImproveError {
    #[error("repository error: {0}")]
    Repository(#[from] repository::RepositoryError),

    #[error("adaptation error: {0}")]
    Adaptation(#[from] AdaptationError),
}

/// Configuration flags for the self-improvement engine.
#[derive(Debug, Clone)]
pub struct SelfImproveConfig {
    /// Enable ingestion + record creation.
    pub enabled: bool,
    /// Allow live adaptation (prompt/config mutation).
    /// Must be false by default at the call site.
    pub live_apply_enabled: bool,
}

impl SelfImproveConfig {
    /// Construct configuration from environment variables.
    ///
    /// This helper is intentionally conservative and never panics:
    /// - SELF_IMPROVE_ENABLED: "1", "true", "yes", "on" (case-insensitive) => enabled
    /// - SELF_IMPROVE_LIVE_APPLY: same truthy semantics; defaults to false.
    pub fn from_env() -> Self {
        fn parse_bool_var(name: &str) -> bool {
            match env::var(name) {
                Ok(val) => {
                    let v = val.trim().to_ascii_lowercase();
                    matches!(v.as_str(), "1" | "true" | "yes" | "on")
                }
                Err(_) => false,
            }
        }

        let enabled = parse_bool_var("SELF_IMPROVE_ENABLED");
        let live_apply_enabled = parse_bool_var("SELF_IMPROVE_LIVE_APPLY");

        Self {
            enabled,
            live_apply_enabled,
        }
    }
}

/// Core self-improvement engine.
///
/// Typical usage (inside an async context):
///
/// ```ignore
/// let engine = SelfImprover::new(SelfImproveConfig {
///     enabled: true,
///     live_apply_enabled: false,
/// })?;
///
/// engine.process_failure(failure).await?;
/// ```
pub struct SelfImprover {
    cfg: SelfImproveConfig,
    repo: Arc<dyn ErrorRecordRepository + Send + Sync>,
    classifier: Arc<dyn FailureClassifier + Send + Sync>,
    adaptation: Arc<dyn AdaptationEngine + Send + Sync>,
}

impl SelfImprover {
    /// Construct a new engine instance with default components.
    pub fn new(cfg: SelfImproveConfig) -> Result<Self> {
        let repo: Arc<dyn ErrorRecordRepository + Send + Sync> =
            Arc::new(repository::FileBackedRepository::new_default()?);

        let classifier: Arc<dyn FailureClassifier + Send + Sync> =
            Arc::new(HeuristicFailureClassifier::default());

        let adaptation: Arc<dyn AdaptationEngine + Send + Sync> =
            adaptation::default_engine(cfg.live_apply_enabled);

        Ok(Self {
            cfg,
            repo,
            classifier,
            adaptation,
        })
    }

    /// Process a critical failure:
    /// - Derive an ErrorRecord from the high-level failure details.
    /// - Persist it using the configured repository backend.
    /// - Run the configured adaptation engine (typically logging-only).
    ///
    /// This method is safe to call from orchestrator or reflection paths;
    /// it is designed to never panic and to emit rich tracing.
    #[instrument(
        name = "self_improvement_triggered",
        skip(self, failure),
        fields(
            failure.request_id = %failure.request_id,
            failure.failure_type = %failure.failure_type
        )
    )]
    pub async fn process_failure(&self, failure: CriticalFailure) -> Result<()> {
        if !self.cfg.enabled {
            tracing::debug!("Self-improvement disabled; skipping process_failure");
            return Ok(());
        }

        let classification: FailureClassification = self.classifier.classify(&failure);
        let record: ErrorRecord = ErrorRecord::from_failure(&failure, &classification);

        self.repo.insert(&record).await?;
        let _adaptation_result = self.adaptation.apply(&record).await?;

        Ok(())
    }
}
