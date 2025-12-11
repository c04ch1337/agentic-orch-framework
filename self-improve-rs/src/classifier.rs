// self-improve-rs/src/classifier.rs
// Heuristic failure classification for self-improvement records.

use serde::{Deserialize, Serialize};

use crate::CriticalFailure;

/// Classification output used to enrich an ErrorRecord.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureClassification {
    pub error_category: String,
    pub contributing_tools: Vec<String>,
    pub suspected_prompts_or_configs: Vec<String>,
    pub proposed_corrections: Vec<String>,
}

/// Strategy interface for failure classification.
///
/// Implementations may be heuristic-only or backed by an LLM or
/// external service in the future.
pub trait FailureClassifier {
    fn classify(&self, failure: &CriticalFailure) -> FailureClassification;
}

/// Simple heuristic classifier that infers categories and hints from
/// failure_type strings, target service, and tool transcripts.
#[derive(Debug, Default)]
pub struct HeuristicFailureClassifier;

impl FailureClassifier for HeuristicFailureClassifier {
    fn classify(&self, failure: &CriticalFailure) -> FailureClassification {
        let mut error_category = "unknown".to_string();
        let mut contributing_tools = Vec::new();
        let mut suspected_prompts_or_configs = Vec::new();
        let mut proposed_corrections = Vec::new();

        let ft_lower = failure.failure_type.to_ascii_lowercase();

        if ft_lower.contains("safety") || ft_lower.contains("policy") {
            error_category = "safety_violation".to_string();
            proposed_corrections.push(
                "Tighten safety policy checks and expand blocked pattern set for high-risk actions"
                    .to_string(),
            );
        } else if ft_lower.contains("tool") || ft_lower.contains("execution") {
            error_category = "tool_execution_failure".to_string();
            proposed_corrections.push(
                "Harden tool parameter validation and add retries / circuit-breaking for flaky tools"
                    .to_string(),
            );
        } else if ft_lower.contains("timeout") {
            error_category = "timeout_or_unavailable_dependency".to_string();
            proposed_corrections.push(
                "Review timeout budgets and fallback behavior for downstream dependencies"
                    .to_string(),
            );
        } else if ft_lower.contains("critical") {
            error_category = "critical_failure".to_string();
            proposed_corrections.push(
                "Add targeted tests and guardrails around this orchestration path".to_string(),
            );
        }

        // Try to infer contributing tools from metadata and transcripts.
        if let Some(service) = &failure.target_service {
            if service.to_ascii_lowercase().contains("tools") {
                contributing_tools.push("tools-service".to_string());
            }
        }

        // Look for tool names in metadata (e.g. `tool_name`, `last_tool`)
        for (k, v) in &failure.metadata {
            let key = k.to_ascii_lowercase();
            if key.contains("tool") && !v.is_empty() {
                contributing_tools.push(v.clone());
            }
        }

        // Use simple substring search on transcripts for common tool identifiers.
        if let Some(transcripts) = &failure.tool_transcripts {
            if let Some(raw) = transcripts.as_str() {
                push_if_contains(raw, "code_exec", "code_exec", &mut contributing_tools);
                push_if_contains(raw, "shell_tool", "shell_tool", &mut contributing_tools);
                push_if_contains(raw, "browser_tool", "browser_tool", &mut contributing_tools);
            } else if let Ok(text) = serde_json::to_string(transcripts) {
                push_if_contains(&text, "code_exec", "code_exec", &mut contributing_tools);
                push_if_contains(&text, "shell_tool", "shell_tool", &mut contributing_tools);
                push_if_contains(
                    &text,
                    "browser_tool",
                    "browser_tool",
                    &mut contributing_tools,
                );
            }
        }

        // Very lightweight prompt/config suspicion based on failure type & stage.
        if let Some(stage) = &failure.stage {
            if stage.to_ascii_lowercase().contains("planning") {
                suspected_prompts_or_configs.push("llm_planning_prompt".to_string());
            } else if stage.to_ascii_lowercase().contains("tools") {
                suspected_prompts_or_configs.push("tool_routing_prompt".to_string());
            }
        }

        if error_category == "safety_violation" {
            suspected_prompts_or_configs.push("safety_guardrails_prompt".to_string());
        }

        FailureClassification {
            error_category,
            contributing_tools,
            suspected_prompts_or_configs,
            proposed_corrections,
        }
    }
}

fn push_if_contains(haystack: &str, needle: &str, label: &str, out: &mut Vec<String>) {
    if haystack.contains(needle) && !out.iter().any(|v| v == label) {
        out.push(label.to_string());
    }
}
