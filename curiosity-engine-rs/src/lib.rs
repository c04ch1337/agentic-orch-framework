use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod proto {
    tonic::include_proto!("curiosity_engine");
}

use proto::{KnowledgeGap, ScheduledTask};

/// Core Curiosity Engine implementation
pub struct CuriosityEngine {
    // Core state
    state: Arc<RwLock<EngineState>>,
}

/// Internal engine state
#[derive(Debug, Default)]
struct EngineState {
    // Track active research tasks
    active_tasks: Vec<ScheduledTask>,
}

impl CuriosityEngine {
    /// Create a new Curiosity Engine instance
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(EngineState::default())),
        }
    }

    /// Initialize the engine
    pub async fn initialize(&self) -> Result<()> {
        // Initialize state
        let mut state = self.state.write().await;
        state.active_tasks = Vec::new();

        Ok(())
    }

    /// Generate a research task based on a knowledge gap
    pub async fn generate_research_task(&self, gap: KnowledgeGap) -> Result<ScheduledTask> {
        // Generate task description
        let task_description = format!("Research: {}", gap.description);

        // Set high priority (8/10) as per requirements
        let priority = 8;

        // Create the task
        let task = ScheduledTask {
            id: gap.id,
            description: task_description,
            priority,
        };

        // Store in active tasks
        let mut state = self.state.write().await;
        state.active_tasks.push(task.clone());

        Ok(task)
    }

    /// Get all active research tasks
    pub async fn get_active_tasks(&self) -> Result<Vec<ScheduledTask>> {
        let state = self.state.read().await;
        Ok(state.active_tasks.clone())
    }

    /// Get a specific task by ID
    pub async fn get_task(&self, task_id: &str) -> Result<Option<ScheduledTask>> {
        let state = self.state.read().await;
        Ok(state.active_tasks.iter().find(|t| t.id == task_id).cloned())
    }

    /// Remove a completed task
    pub async fn complete_task(&self, task_id: &str) -> Result<()> {
        let mut state = self.state.write().await;
        state.active_tasks.retain(|t| t.id != task_id);
        Ok(())
    }

    /// Get service health status
    pub async fn health_check(&self) -> Result<bool> {
        // Basic health check - verify we can access state
        let _state = self.state.read().await;
        Ok(true)
    }
}

impl Default for CuriosityEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export key types
pub use proto::{KnowledgeGap, ScheduledTask};
