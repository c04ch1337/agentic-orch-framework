// self-improve-rs/src/error_record.rs
// Structured error/training record used by the self-improvement engine.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::CriticalFailure;
use crate::classifier::FailureClassification;

/// Persisted representation of a severe orchestration failure suitable for
/// downstream training, analysis, or adaptation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub id: String,
    pub request_id: String,
    pub failure_type: String,
    pub error_category: String,
    pub error_stage: Option<String>,
    pub target_service: Option<String>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub contributing_tools: Vec<String>,
    pub suspected_prompts_or_configs: Vec<String>,
    pub proposed_corrections: Vec<String>,
    pub original_query_snapshot: Option<String>,
    pub plan_summary: Option<serde_json::Value>,
    pub tool_error_summaries: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ErrorRecord {
    /// Derive an ErrorRecord from a high-level CriticalFailure and its
    /// classifier output.
    pub fn from_failure(failure: &CriticalFailure, classification: &FailureClassification) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4().to_string(),
            request_id: failure.request_id.clone(),
            failure_type: failure.failure_type.clone(),
            error_category: classification.error_category.clone(),
            error_stage: failure.stage.clone(),
            target_service: failure.target_service.clone(),
            error_type: failure.error_type.clone(),
            error_message: failure.error_message.clone(),
            contributing_tools: classification.contributing_tools.clone(),
            suspected_prompts_or_configs: classification.suspected_prompts_or_configs.clone(),
            proposed_corrections: classification.proposed_corrections.clone(),
            original_query_snapshot: failure.original_query.clone(),
            plan_summary: failure.plan_json.clone(),
            tool_error_summaries: failure.tool_transcripts.clone(),
            created_at: now,
            updated_at: now,
        }
    }
}
