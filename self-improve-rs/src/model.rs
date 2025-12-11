 // self-improve-rs/src/model.rs
 // Internal request/DTO types for the self-improvement engine.
 //
 // These types are primarily used by in-process callers and internal
 // adapters. They are not intended to be a stable public HTTP schema;
 // services may map their own structures into these types or directly
 // into `CriticalFailure` depending on integration needs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

 /// Failure event representation used by internal integrations.
 ///
 /// This is an internal DTO that can be constructed by adapters or
 /// mappers and then converted into a `CriticalFailure`. It is not a
 /// required external HTTP schema and may evolve independently of any
 /// particular service boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureEventRequest {
    /// Stable request or correlation id for the failed orchestration.
    pub request_id: String,
    /// High-level failure type classification (e.g. "CRITICAL_FAILURE").
    pub failure_type: String,
    /// Optional sanitized / redacted original query.
    pub original_query: Option<String>,
    /// Optional summarized execution plan that led to the failure.
    pub plan: Option<Value>,
    /// Optional tool transcripts or per-tool logs (sanitized).
    pub tool_transcripts: Option<Value>,
    /// Final answer that was returned to the user (if any).
    pub final_answer: Option<String>,
    /// Time the failure was detected.
    pub detected_at: DateTime<Utc>,
    /// Optional distributed tracing correlation id.
    pub correlation_id: Option<String>,
}
