use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::adaptation::{AdaptationEngine, LoggingAdaptationEngine};
use crate::classifier::FailureClassification;
use crate::error_record::ErrorRecord;
use crate::{CriticalFailure, SelfImproveConfig, SelfImprover};

fn make_failure() -> CriticalFailure {
    CriticalFailure {
        request_id: "req-123".to_string(),
        failure_type: "CRITICAL_FAILURE".to_string(),
        stage: Some("reflection_action".to_string()),
        target_service: Some("reflection-service-rs".to_string()),
        error_type: Some("action_failure".to_string()),
        error_message: Some("tool execution failed".to_string()),
        original_query: Some("original query".to_string()),
        plan_json: None,
        tool_transcripts: None,
        final_answer: Some("fallback answer".to_string()),
        metadata: {
            let mut m = HashMap::new();
            m.insert("tool_name".to_string(), "example_tool".to_string());
            m
        },
    }
}

fn make_classification() -> FailureClassification {
    FailureClassification {
        error_category: "tool_execution_failure".to_string(),
        contributing_tools: vec!["example_tool".to_string()],
        suspected_prompts_or_configs: vec!["tool_routing_prompt".to_string()],
        proposed_corrections: vec!["tighten validation".to_string()],
    }
}

#[test]
fn error_record_from_failure_maps_fields() {
    let failure = make_failure();
    let classification = make_classification();

    let record = ErrorRecord::from_failure(&failure, &classification);

    assert_eq!(record.request_id, failure.request_id);
    assert_eq!(record.failure_type, failure.failure_type);
    assert_eq!(record.error_category, classification.error_category);
    assert_eq!(record.error_stage, failure.stage);
    assert_eq!(record.target_service, failure.target_service);
    assert_eq!(record.error_type, failure.error_type);
    assert_eq!(record.error_message, failure.error_message);
    assert_eq!(
        record.contributing_tools,
        classification.contributing_tools
    );
    assert_eq!(
        record.suspected_prompts_or_configs,
        classification.suspected_prompts_or_configs
    );
    assert_eq!(
        record.proposed_corrections,
        classification.proposed_corrections
    );
    assert_eq!(record.original_query_snapshot, failure.original_query);
    assert_eq!(record.plan_summary, failure.plan_json);
    assert_eq!(record.tool_error_summaries, failure.tool_transcripts);
}

#[tokio::test]
async fn logging_adaptation_engine_basic_behavior() {
    let failure = make_failure();
    let classification = make_classification();
    let record = ErrorRecord::from_failure(&failure, &classification);

    let engine = LoggingAdaptationEngine::new(false);
    let result = engine
        .apply(&record)
        .await
        .expect("adaptation should succeed");

    assert!(!result.applied, "logging engine should not apply live changes");
    assert!(
        result.summary.contains(&record.request_id),
        "summary should reference request_id"
    );
}

#[tokio::test]
async fn process_failure_is_noop_when_disabled() {
    // Point repository at a temp path we control.
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let store_path: PathBuf = tmp_dir.path().join("error_records_disabled.ndjson");
    unsafe {
        env::set_var(
            "SELF_IMPROVE_STORE_PATH",
            store_path.to_string_lossy().to_string(),
        );
    }

    let cfg = SelfImproveConfig {
        enabled: false,
        live_apply_enabled: false,
    };

    let engine = SelfImprover::new(cfg).expect("engine construction should succeed");
    engine
        .process_failure(make_failure())
        .await
        .expect("process_failure should succeed when disabled");

    // When disabled, we should not have created the backing file.
    assert!(
        !store_path.exists(),
        "repository file should not be created when self-improve is disabled"
    );
}

#[tokio::test]
async fn process_failure_persists_record_when_enabled() {
    // Point repository at a temp path we control.
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let store_path: PathBuf = tmp_dir.path().join("error_records_enabled.ndjson");
    unsafe {
        env::set_var(
            "SELF_IMPROVE_STORE_PATH",
            store_path.to_string_lossy().to_string(),
        );
    }

    let cfg = SelfImproveConfig {
        enabled: true,
        live_apply_enabled: false,
    };

    let engine = SelfImprover::new(cfg).expect("engine construction should succeed");
    engine
        .process_failure(make_failure())
        .await
        .expect("process_failure should succeed when enabled");

    // File should exist and contain at least one non-empty line.
    let contents = fs::read_to_string(&store_path).expect("repository file should be readable");
    let non_empty_lines: Vec<_> = contents.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        !non_empty_lines.is_empty(),
        "repository file should contain at least one record line"
    );
}

#[test]
fn critical_failure_from_reflection_failure_sets_expected_fields() {
    let mut meta = HashMap::new();
    meta.insert("custom_key".to_string(), "custom_value".to_string());

    let failure = CriticalFailure::from_reflection_failure(
        "req-abc".to_string(),
        "test action".to_string(),
        "failed badly".to_string(),
        false,
        meta.clone(),
    );

    assert_eq!(failure.request_id, "req-abc");
    assert_eq!(failure.failure_type, "REFLECTION_ACTION_FAILURE");
    assert_eq!(failure.stage.as_deref(), Some("reflection_action"));
    assert_eq!(failure.target_service.as_deref(), Some("reflection-service-rs"));
    assert_eq!(failure.error_type.as_deref(), Some("action_failure"));
    assert_eq!(failure.error_message.as_deref(), Some("failed badly"));

    // Metadata should contain both original and enriched keys.
    assert_eq!(
        failure
            .metadata
            .get("reflection_action_description")
            .map(String::as_str),
        Some("test action")
    );
    assert_eq!(
        failure
            .metadata
            .get("reflection_success")
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        failure.metadata.get("custom_key").map(String::as_str),
        Some("custom_value")
    );
}