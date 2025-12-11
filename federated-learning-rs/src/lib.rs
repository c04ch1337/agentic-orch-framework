//! # Federated Learning Coordinator
//!
//! Coordinates federated learning cycles:
//! 1. Collects telemetry data from telemetrist-rs
//! 2. Aggregates patterns and generates playbook improvements
//! 3. Triggers adapter/config updates via config-update-rs
//! 4. Manages learning cycles and model versioning

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::time::sleep;

use config_update::{ConfigUpdateService, ConfigUpdateConfig, AdapterMetadata};
use self_improve::{SelfImprover, SelfImproveConfig};
use telemetrist::{Telemetrist, TelemetristConfig, TelemetryEvent};

/// Playbook improvement suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookImprovement {
    pub improvement_id: String,
    pub category: String, // "prompt", "adapter", "config", "workflow"
    pub description: String,
    pub confidence: f32,
    pub suggested_changes: HashMap<String, String>,
    pub evidence_count: usize,
    pub created_at: chrono::DateTime<Utc>,
}

/// Federated learning cycle result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningCycleResult {
    pub cycle_id: String,
    pub start_time: chrono::DateTime<Utc>,
    pub end_time: chrono::DateTime<Utc>,
    pub events_processed: usize,
    pub improvements_generated: usize,
    pub adapters_updated: usize,
    pub configs_updated: usize,
}

/// Configuration for federated learning coordinator
#[derive(Debug, Clone)]
pub struct FederatedLearningConfig {
    pub enabled: bool,
    pub cycle_interval_secs: u64,
    pub min_events_per_cycle: usize,
    pub improvement_threshold: f32,
    pub telemetry_cache_path: PathBuf,
    pub playbook_output_path: PathBuf,
}

impl FederatedLearningConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("FEDERATED_LEARNING_ENABLED")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(true);

        let cycle_interval_secs = std::env::var("FEDERATED_LEARNING_CYCLE_INTERVAL_SECS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(86400); // 24 hours

        let min_events_per_cycle = std::env::var("FEDERATED_LEARNING_MIN_EVENTS")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000);

        let improvement_threshold = std::env::var("FEDERATED_LEARNING_THRESHOLD")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.7);

        let telemetry_cache_path = std::env::var("FEDERATED_LEARNING_TELEMETRY_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/telemetry/cache"));

        let playbook_output_path = std::env::var("FEDERATED_LEARNING_PLAYBOOK_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/federated-learning/playbooks"));

        Self {
            enabled,
            cycle_interval_secs,
            min_events_per_cycle,
            improvement_threshold,
            telemetry_cache_path,
            playbook_output_path,
        }
    }
}

/// Main federated learning coordinator
pub struct FederatedLearningCoordinator {
    config: FederatedLearningConfig,
    telemetrist: Arc<Telemetrist>,
    config_update: Arc<ConfigUpdateService>,
    self_improver: Arc<SelfImprover>,
}

impl FederatedLearningCoordinator {
    pub fn new(
        config: FederatedLearningConfig,
        telemetrist: Telemetrist,
        config_update: ConfigUpdateService,
        self_improver: SelfImprover,
    ) -> Result<Self, FederatedLearningError> {
        // Ensure output directories exist
        if let Some(parent) = config.playbook_output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| FederatedLearningError::Io(format!("Failed to create playbook directory: {}", e)))?;
        }

        Ok(Self {
            config,
            telemetrist: Arc::new(telemetrist),
            config_update: Arc::new(config_update),
            self_improver: Arc::new(self_improver),
        })
    }

    pub fn new_default() -> Result<Self, FederatedLearningError> {
        let config = FederatedLearningConfig::from_env();
        let telemetrist = Telemetrist::new(TelemetristConfig::from_env())
            .map_err(|e| FederatedLearningError::Telemetry(e.to_string()))?;
        let config_update = ConfigUpdateService::new(ConfigUpdateConfig::from_env())
            .map_err(|e| FederatedLearningError::ConfigUpdate(e.to_string()))?;
        let self_improver = SelfImprover::new(SelfImproveConfig::from_env())
            .map_err(|e| FederatedLearningError::SelfImprove(e.to_string()))?;

        Self::new(config, telemetrist, config_update, self_improver)
    }

    /// Run a single learning cycle
    pub async fn run_learning_cycle(&self) -> Result<LearningCycleResult, FederatedLearningError> {
        let cycle_id = uuid::Uuid::new_v4().to_string();
        let start_time = Utc::now();

        log::info!("Starting federated learning cycle: {}", cycle_id);

        // Step 1: Collect telemetry events from cache
        let events = self.collect_telemetry_events().await?;

        if events.len() < self.config.min_events_per_cycle {
            log::info!(
                "Insufficient events for learning cycle: {} < {}",
                events.len(),
                self.config.min_events_per_cycle
            );
            return Err(FederatedLearningError::InsufficientData {
                collected: events.len(),
                required: self.config.min_events_per_cycle,
            });
        }

        // Step 2: Analyze patterns and generate improvements
        let improvements = self.analyze_patterns(&events).await?;

        // Step 3: Apply improvements (adapter/config updates)
        let mut adapters_updated = 0;
        let mut configs_updated = 0;

        for improvement in &improvements {
            if improvement.confidence >= self.config.improvement_threshold {
                match self.apply_improvement(improvement).await {
                    Ok(applied) => {
                        if applied {
                            match improvement.category.as_str() {
                                "adapter" => adapters_updated += 1,
                                "config" => configs_updated += 1,
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to apply improvement {}: {}", improvement.improvement_id, e);
                    }
                }
            }
        }

        // Step 4: Save playbook improvements
        self.save_playbook(&improvements).await?;

        let end_time = Utc::now();
        let result = LearningCycleResult {
            cycle_id,
            start_time,
            end_time,
            events_processed: events.len(),
            improvements_generated: improvements.len(),
            adapters_updated,
            configs_updated,
        };

        log::info!(
            "Learning cycle complete: {} events, {} improvements, {} adapters, {} configs",
            result.events_processed,
            result.improvements_generated,
            result.adapters_updated,
            result.configs_updated
        );

        Ok(result)
    }

    async fn collect_telemetry_events(&self) -> Result<Vec<TelemetryEvent>, FederatedLearningError> {
        // In production, this would read from telemetrist cache or query telemetrist API
        // For now, return empty vector as placeholder
        Ok(Vec::new())
    }

    async fn analyze_patterns(&self, events: &[TelemetryEvent]) -> Result<Vec<PlaybookImprovement>, FederatedLearningError> {
        // Pattern analysis logic:
        // 1. Group events by error type, service, method
        // 2. Identify common failure patterns
        // 3. Generate improvement suggestions based on patterns
        // 4. Calculate confidence scores

        let mut improvements = Vec::new();

        // Example: Analyze execution traces for common failures
        let mut failure_patterns: HashMap<String, usize> = HashMap::new();
        for event in events {
            if let Some(trace) = &event.execution_trace {
                if !trace.success {
                    let pattern_key = format!("{}:{}:{}", trace.service, trace.method, trace.error.as_deref().unwrap_or("unknown"));
                    *failure_patterns.entry(pattern_key).or_insert(0) += 1;
                }
            }
        }

        // Generate improvements for patterns with high frequency
        for (pattern, count) in failure_patterns {
            if count >= 10 {
                let improvement = PlaybookImprovement {
                    improvement_id: uuid::Uuid::new_v4().to_string(),
                    category: "workflow".to_string(),
                    description: format!("High failure rate detected for pattern: {}", pattern),
                    confidence: (count as f32 / events.len() as f32).min(1.0),
                    suggested_changes: {
                        let mut changes = HashMap::new();
                        changes.insert("pattern".to_string(), pattern.clone());
                        changes.insert("frequency".to_string(), count.to_string());
                        changes
                    },
                    evidence_count: count,
                    created_at: Utc::now(),
                };
                improvements.push(improvement);
            }
        }

        Ok(improvements)
    }

    async fn apply_improvement(&self, improvement: &PlaybookImprovement) -> Result<bool, FederatedLearningError> {
        match improvement.category.as_str() {
            "adapter" => {
                // Check for available adapters and download if needed
                let adapters = self.config_update.check_for_updates().await
                    .map_err(|e| FederatedLearningError::ConfigUpdate(e.to_string()))?;

                if !adapters.is_empty() {
                    let adapter = &adapters[0];
                    let adapter_path = std::path::Path::new(&format!("./adapters/{}.bin", adapter.adapter_id));
                    self.config_update.download_adapter(adapter, adapter_path).await
                        .map_err(|e| FederatedLearningError::ConfigUpdate(e.to_string()))?;
                    return Ok(true);
                }
                Ok(false)
            }
            "config" => {
                // Config updates would be handled here
                log::info!("Config improvement suggested: {}", improvement.description);
                Ok(false) // Placeholder
            }
            _ => Ok(false),
        }
    }

    async fn save_playbook(&self, improvements: &[PlaybookImprovement]) -> Result<(), FederatedLearningError> {
        let playbook_file = self.config.playbook_output_path.join(format!("playbook_{}.json", Utc::now().format("%Y%m%d_%H%M%S")));
        
        let playbook_json = serde_json::to_string_pretty(improvements)
            .map_err(|e| FederatedLearningError::Serialization(e.to_string()))?;

        fs::write(&playbook_file, playbook_json).await
            .map_err(|e| FederatedLearningError::Io(format!("Failed to write playbook: {}", e)))?;

        log::info!("Playbook saved to: {:?}", playbook_file);
        Ok(())
    }

    /// Start background learning cycle task
    pub fn start_background_cycles(&self) {
        let config = self.config.clone();
        let coordinator = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.cycle_interval_secs));
            loop {
                interval.tick().await;
                
                if config.enabled {
                    match coordinator.run_learning_cycle().await {
                        Ok(result) => {
                            log::info!("Learning cycle completed: {}", result.cycle_id);
                        }
                        Err(e) => {
                            log::warn!("Learning cycle failed: {}", e);
                        }
                    }
                }
            }
        });
    }
}

impl Clone for FederatedLearningCoordinator {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            telemetrist: Arc::clone(&self.telemetrist),
            config_update: Arc::clone(&self.config_update),
            self_improver: Arc::clone(&self.self_improver),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FederatedLearningError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("Telemetry error: {0}")]
    Telemetry(String),

    #[error("Config update error: {0}")]
    ConfigUpdate(String),

    #[error("Self-improve error: {0}")]
    SelfImprove(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Insufficient data: collected {collected}, required {required}")]
    InsufficientData { collected: usize, required: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_playbook_improvement() {
        let improvement = PlaybookImprovement {
            improvement_id: "test-1".to_string(),
            category: "workflow".to_string(),
            description: "Test improvement".to_string(),
            confidence: 0.8,
            suggested_changes: HashMap::new(),
            evidence_count: 10,
            created_at: Utc::now(),
        };
        assert_eq!(improvement.confidence, 0.8);
    }
}

