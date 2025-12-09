// reflection-service-rs/src/reflection_logic.rs
// Core reflection logic for analyzing actions and outcomes

use log::{info, error, warn, debug};
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionReflection {
    pub request_id: String,
    pub action_description: String,
    pub outcome: String,
    pub success: bool,
    pub analysis: String,
    pub lessons_learned: Vec<String>,
    pub improvements: Vec<String>,
    pub confidence_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonLearned {
    pub id: String,
    pub lesson: String,
    pub context: String,
    pub priority: u8,  // 1-5 scale, 5 being highest
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub id: String,
    pub constraint: String,
    pub context: String,
    pub timestamp: i64,
    pub immediate_use: bool,
}

pub struct ReflectionEngine {
    // Configuration options could be added here
}

impl ReflectionEngine {
    pub fn new() -> Self {
        Self {}
    }

    /// Analyze an action and its outcome to extract lessons
    pub async fn reflect_on_action(
        &self,
        request_id: &str,
        action: &str,
        outcome: &str, 
        success: bool,
        context: &HashMap<String, String>,
    ) -> Result<ActionReflection> {
        debug!("Reflecting on action: {}, outcome: {}, success: {}", action, outcome, success);
        
        // Create reflection container
        let mut reflection = ActionReflection {
            request_id: request_id.to_string(),
            action_description: action.to_string(),
            outcome: outcome.to_string(),
            success,
            analysis: String::new(),
            lessons_learned: Vec::new(),
            improvements: Vec::new(),
            confidence_score: 0.85, // Default confidence
        };
        
        // Main analysis
        reflection.analysis = self.analyze_outcome(action, outcome, success).await?;
        
        // If action failed, generate lessons learned
        if !success {
            info!("Generating lessons from unsuccessful action: {}", action);
            reflection.lessons_learned = self.generate_lessons(action, outcome, context).await?;
            reflection.improvements = self.suggest_improvements(action, outcome, context).await?;
        } else {
            // Even successful actions can have improvements or optimizations
            reflection.improvements = self.suggest_optimizations(action, outcome, context).await?;
        }
        
        Ok(reflection)
    }
    
    /// Analyze outcome vs. expected success
    async fn analyze_outcome(&self, action: &str, outcome: &str, success: bool) -> Result<String> {
        // Pattern matching on the outcome vs success
        let analysis = if success {
            format!("The action '{}' was successful, resulting in: {}. This aligns with expected behavior.", 
                    action, outcome)
        } else {
            format!("The action '{}' was unsuccessful, resulting in: {}. This represents a deviation from expected behavior that requires adaptation.", 
                    action, outcome)
        };
        
        Ok(analysis)
    }
    
    /// Generate lessons from failed actions
    async fn generate_lessons(
        &self, 
        action: &str, 
        outcome: &str,
        context: &HashMap<String, String>
    ) -> Result<Vec<String>> {
        // In a production system, this could use an LLM or more sophisticated analysis
        // For now, we'll implement a rule-based approach
        
        let mut lessons = Vec::new();
        
        // Basic pattern matching for lesson generation
        if outcome.contains("timeout") || outcome.contains("timed out") {
            lessons.push("Improve timeout handling for actions that may take longer to complete".to_string());
        }
        
        if outcome.contains("permission") || outcome.contains("access") {
            lessons.push("Ensure proper permission checks before attempting actions requiring privileged access".to_string());
        }
        
        if outcome.contains("not found") || outcome.contains("missing") {
            lessons.push("Validate resource existence before attempting operations that depend on them".to_string());
        }
        
        // Generate a generic lesson if no specific rules matched
        if lessons.is_empty() {
            let generic_lesson = format!("When executing '{}', unexpected outcomes like '{}' should trigger validation checks", 
                                        action, outcome);
            lessons.push(generic_lesson);
        }
        
        // Add a reflection on the process or the attempted action itself
        let meta_lesson = format!("Review the approach to '{}' to identify potential failure points earlier in the process", action);
        lessons.push(meta_lesson);
        
        Ok(lessons)
    }
    
    /// Suggest improvements for failed actions
    async fn suggest_improvements(
        &self,
        action: &str,
        outcome: &str,
        context: &HashMap<String, String>
    ) -> Result<Vec<String>> {
        // Similar to lessons, but focused on concrete improvements
        let mut improvements = Vec::new();
        
        // Add specific improvements based on action and outcome patterns
        if action.contains("connect") || action.contains("request") {
            improvements.push("Implement exponential backoff and retry logic for network operations".to_string());
        }
        
        if action.contains("validate") || action.contains("verify") {
            improvements.push("Add more comprehensive validation rules to prevent similar failures".to_string());
        }
        
        // Default improvement if no specific rules match
        if improvements.is_empty() {
            improvements.push("Create a more robust error handling strategy, with detailed logging for this operation".to_string());
        }
        
        Ok(improvements)
    }
    
    /// Suggest optimizations for successful actions
    async fn suggest_optimizations(
        &self,
        action: &str,
        outcome: &str,
        context: &HashMap<String, String>
    ) -> Result<Vec<String>> {
        // For successful actions, suggest potential optimizations
        let mut optimizations = Vec::new();
        
        // Generic optimization suggestions
        optimizations.push("Consider if this operation could be cached to improve future performance".to_string());
        
        if action.contains("process") || action.contains("compute") || action.contains("calculate") {
            optimizations.push("Evaluate if this operation could be parallelized for better efficiency".to_string());
        }
        
        Ok(optimizations)
    }
    
    /// Format a lesson for persistent storage
    pub fn format_lesson_for_storage(
        &self,
        lesson: &str,
        context: &str,
        priority: u8,
    ) -> LessonLearned {
        let now = chrono::Utc::now();
        
        LessonLearned {
            id: uuid::Uuid::new_v4().to_string(),
            lesson: lesson.to_string(),
            context: context.to_string(),
            priority: priority.min(5).max(1), // Ensure priority is between 1-5
            timestamp: now.timestamp(),
        }
    }

    /// Generate negative constraint rules based on lessons learned
    pub async fn generate_negative_constraints(
        &self,
        lessons: &[String],
        context: &HashMap<String, String>
    ) -> Vec<String> {
        let mut constraints = Vec::new();
        
        // Convert lessons to more general constraints
        for lesson in lessons {
            // Generate abstract constraint rule from the lesson
            let constraint = format!("Constraint: Avoid actions that could lead to: {}", lesson);
            constraints.push(constraint);
        }

        // Always include at least one constraint if lessons exist
        if !lessons.is_empty() && constraints.is_empty() {
            constraints.push("Constraint: Exercise additional caution when similar conditions arise".to_string());
        }
        
        constraints
    }
    
    /// Format a constraint for storage in Soul KB
    pub fn format_constraint_for_storage(
        &self,
        constraint: &str,
        context: &str
    ) -> Constraint {
        let now = chrono::Utc::now();
        
        Constraint {
            id: uuid::Uuid::new_v4().to_string(),
            constraint: constraint.to_string(),
            context: context.to_string(),
            timestamp: now.timestamp(),
            immediate_use: true, // Mark for immediate use as specified in requirements
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_reflect_on_action_success() {
        let engine = ReflectionEngine::new();
        let context = HashMap::new();
        
        let reflection = engine.reflect_on_action(
            "test-123",
            "fetch user data",
            "user data retrieved successfully",
            true,
            &context
        ).await.unwrap();
        
        assert!(reflection.success);
        assert!(!reflection.analysis.is_empty());
        assert!(!reflection.improvements.is_empty());
    }
    
    #[tokio::test]
    async fn test_reflect_on_action_failure() {
        let engine = ReflectionEngine::new();
        let context = HashMap::new();
        
        let reflection = engine.reflect_on_action(
            "test-456",
            "update database",
            "database connection timed out",
            false,
            &context
        ).await.unwrap();
        
        assert!(!reflection.success);
        assert!(!reflection.analysis.is_empty());
        assert!(!reflection.lessons_learned.is_empty());
        assert!(!reflection.improvements.is_empty());
    }
}