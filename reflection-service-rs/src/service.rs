// reflection-service-rs/src/service.rs
// Implementation of the ReflectionService gRPC service

use std::collections::HashMap;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use log::{info, error, warn, debug};
use anyhow::{Result, anyhow};
use tokio::sync::Mutex;

// Import generated protobuf code
use crate::agi_core::{
    reflection_service_server::ReflectionService,
    ReflectionRequest, ReflectionResult,
    EvaluationRequest, EvaluationResult,
    MetaCognitiveRequest, MetaCognitiveResult,
};

// Import our modules
use crate::reflection_logic::{ReflectionEngine, LessonLearned};
use crate::soul_kb_client::SoulKBClient;
use crate::logging_client::{LoggingClient, LogLevel};

// Custom error type for reflection service
#[derive(Debug, thiserror::Error)]
pub enum ReflectionError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
    
    #[error("Knowledge base error: {0}")]
    KnowledgeBaseError(String),
    
    #[error("Logging error: {0}")]
    LoggingError(String),
}

// Service implementation
pub struct ReflectionServiceImpl {
    reflection_engine: ReflectionEngine,
    soul_kb_client: Arc<Mutex<SoulKBClient>>,
    logging_client: Arc<Mutex<LoggingClient>>,
}

impl ReflectionServiceImpl {
    // Create a new ReflectionServiceImpl with required dependencies
    pub async fn new() -> Self {
        let soul_kb_client = SoulKBClient::new().await;
        let logging_client = LoggingClient::new().await;
        
        Self {
            reflection_engine: ReflectionEngine::new(),
            soul_kb_client: Arc::new(Mutex::new(soul_kb_client)),
            logging_client: Arc::new(Mutex::new(logging_client)),
        }
    }
    
    // Log and store a learned lesson
    async fn store_lesson(&self,
                         lesson: &str,
                         context: &str,
                         priority: u8,
                         request_id: &str,
                         action: &str) -> Result<(), ReflectionError> {
        // Format the lesson for storage
        let lesson_obj = self.reflection_engine.format_lesson_for_storage(
            lesson, context, priority
        );
        
        // Store the lesson in Soul-KB
        let soul_kb_result = {
            let mut client = self.soul_kb_client.lock().await;
            client.store_lesson(&lesson_obj).await
        };
        
        match soul_kb_result {
            Ok(lesson_id) => {
                info!("Stored lesson learned in Soul-KB with ID: {}", lesson_id);
                
                // Log the improvement event
                let logging_result = {
                    let mut client = self.logging_client.lock().await;
                    client.log_improvement_event(action, lesson, request_id).await
                };
                
                if let Err(e) = logging_result {
                    warn!("Failed to log improvement event: {}", e);
                    return Err(ReflectionError::LoggingError(e.to_string()));
                }
                
                Ok(())
            },
            Err(e) => {
                error!("Failed to store lesson in Soul-KB: {}", e);
                Err(ReflectionError::KnowledgeBaseError(e.to_string()))
            }
        }
    }

    // Store a negative constraint rule
    async fn store_constraint(&self,
                             constraint: &str,
                             context: &str,
                             request_id: &str) -> Result<(), ReflectionError> {
        // Format the constraint for storage
        let constraint_obj = self.reflection_engine.format_constraint_for_storage(
            constraint, context
        );
        
        // Store the constraint in Soul-KB
        let soul_kb_result = {
            let mut client = self.soul_kb_client.lock().await;
            client.store_constraint(&constraint_obj).await
        };
        
        match soul_kb_result {
            Ok(constraint_id) => {
                info!("Stored constraint in Soul-KB with ID: {}", constraint_id);
                
                // Log the constraint event
                let logging_result = {
                    let mut client = self.logging_client.lock().await;
                    client.log_constraint_event(constraint, request_id).await
                };
                
                if let Err(e) = logging_result {
                    warn!("Failed to log constraint event: {}", e);
                    return Err(ReflectionError::LoggingError(e.to_string()));
                }
                
                Ok(())
            },
            Err(e) => {
                error!("Failed to store constraint in Soul-KB: {}", e);
                Err(ReflectionError::KnowledgeBaseError(e.to_string()))
            }
        }
    }
    
    // Validate reflection request
    fn validate_reflection_request(&self, request: &ReflectionRequest) -> Result<(), ReflectionError> {
        if request.request_id.is_empty() {
            return Err(ReflectionError::InvalidRequest("Request ID cannot be empty".to_string()));
        }
        
        if request.action_description.is_empty() {
            return Err(ReflectionError::InvalidRequest("Action description cannot be empty".to_string()));
        }
        
        if request.outcome.is_empty() {
            return Err(ReflectionError::InvalidRequest("Outcome cannot be empty".to_string()));
        }
        
        Ok(())
    }
    
    // Validate evaluation request
    fn validate_evaluation_request(&self, request: &EvaluationRequest) -> Result<(), ReflectionError> {
        if request.request_id.is_empty() {
            return Err(ReflectionError::InvalidRequest("Request ID cannot be empty".to_string()));
        }
        
        if request.proposed_action.is_empty() {
            return Err(ReflectionError::InvalidRequest("Proposed action cannot be empty".to_string()));
        }
        
        if request.goal.is_empty() {
            return Err(ReflectionError::InvalidRequest("Goal cannot be empty".to_string()));
        }
        
        Ok(())
    }
    
    // Validate meta-cognitive request
    fn validate_metacognitive_request(&self, request: &MetaCognitiveRequest) -> Result<(), ReflectionError> {
        if request.request_id.is_empty() {
            return Err(ReflectionError::InvalidRequest("Request ID cannot be empty".to_string()));
        }
        
        if request.topic.is_empty() {
            return Err(ReflectionError::InvalidRequest("Topic cannot be empty".to_string()));
        }
        
        if request.depth < 1 || request.depth > 5 {
            return Err(ReflectionError::InvalidRequest("Depth must be between 1 and 5".to_string()));
        }
        
        Ok(())
    }
}

#[tonic::async_trait]
impl ReflectionService for ReflectionServiceImpl {
    // Reflect on a completed action
    async fn reflect_on_action(
        &self,
        request: Request<ReflectionRequest>,
    ) -> Result<Response<ReflectionResult>, Status> {
        let req = request.into_inner();
        info!("Received ReflectOnAction: request_id={}, action={}", req.request_id, req.action_description);
        
        // Validate request
        if let Err(e) = self.validate_reflection_request(&req) {
            error!("Invalid reflection request: {}", e);
            return Err(Status::invalid_argument(e.to_string()));
        }
        
        // Log the reflection event
        {
            let mut client = self.logging_client.lock().await;
            if let Err(e) = client.log_reflection_event(&req.request_id, req.success, &req.action_description).await {
                warn!("Failed to log reflection event: {}", e);
                // Continue processing despite logging error
            }
        }
        
        // Convert context map
        let context: HashMap<String, String> = req.context.clone();
        
        // Perform reflection with the reflection engine
        match self.reflection_engine.reflect_on_action(
            &req.request_id,
            &req.action_description,
            &req.outcome,
            req.success,
            &context,
        ).await {
            Ok(reflection) => {
                // If the action was unsuccessful, store lessons learned
                if !req.success && !reflection.lessons_learned.is_empty() {
                    for (i, lesson) in reflection.lessons_learned.iter().enumerate() {
                        // Calculate priority based on position (first lessons are higher priority)
                        let priority = 5.min(3 + (reflection.lessons_learned.len() - i) as u8);
                        
                        // Store the lesson with increasing priority
                        if let Err(e) = self.store_lesson(
                            lesson,
                            &req.action_description,
                            priority,
                            &req.request_id,
                            &req.action_description,
                        ).await {
                            warn!("Failed to store lesson: {}", e);
                            // Continue processing despite storage error
                        }
                    }

                    // Generate negative constraint rules from lessons learned
                    let constraints = self.reflection_engine.generate_negative_constraints(
                        &reflection.lessons_learned,
                        &req.context
                    ).await;

                    // Store constraints in Soul-KB
                    for constraint in constraints {
                        if let Err(e) = self.store_constraint(
                            &constraint,
                            &req.action_description,
                            &req.request_id,
                        ).await {
                            warn!("Failed to store constraint: {}", e);
                        }
                    }
                }
                
                // Return the reflection result
                let reply = ReflectionResult {
                    request_id: req.request_id,
                    analysis: reflection.analysis,
                    lessons_learned: reflection.lessons_learned,
                    improvements: reflection.improvements,
                    confidence_score: reflection.confidence_score,
                    metadata: HashMap::new(),
                };
                
                Ok(Response::new(reply))
            },
            Err(e) => {
                error!("Error in reflection process: {}", e);
                Err(Status::internal(format!("Error in reflection process: {}", e)))
            }
        }
    }
    
    // Evaluate a proposed action
    async fn evaluate_action(
        &self,
        request: Request<EvaluationRequest>,
    ) -> Result<Response<EvaluationResult>, Status> {
        let req = request.into_inner();
        info!("Received EvaluateAction: request_id={}, action={}", req.request_id, req.proposed_action);
        
        // Validate request
        if let Err(e) = self.validate_evaluation_request(&req) {
            error!("Invalid evaluation request: {}", e);
            return Err(Status::invalid_argument(e.to_string()));
        }
        
        // For now, provide a basic evaluation - in a real implementation, 
        // this would use a more sophisticated evaluation algorithm
        
        // Dummy implementation for the evaluation logic
        // In a real system, this would use machine learning models or rule-based systems
        let recommended = true; // Default to recommending the action
        let rationale = format!("Action '{}' aligns with the goal '{}'", req.proposed_action, req.goal);
        let risks = vec!["No significant risks identified".to_string()];
        let alternatives = vec!["Continue with the proposed action".to_string()];
        
        // Return the evaluation result
        let reply = EvaluationResult {
            request_id: req.request_id,
            recommended,
            rationale,
            risks,
            alternatives,
            confidence_score: 0.9, // High confidence
            metadata: HashMap::new(),
        };
        
        Ok(Response::new(reply))
    }
    
    // Perform meta-cognitive analysis
    async fn meta_cognition(
        &self,
        request: Request<MetaCognitiveRequest>,
    ) -> Result<Response<MetaCognitiveResult>, Status> {
        let req = request.into_inner();
        info!("Received MetaCognition: request_id={}, topic={}", req.request_id, req.topic);
        
        // Validate request
        if let Err(e) = self.validate_metacognitive_request(&req) {
            error!("Invalid meta-cognitive request: {}", e);
            return Err(Status::invalid_argument(e.to_string()));
        }
        
        // For now, provide a basic meta-cognitive analysis - in a real implementation,
        // this would use a more sophisticated analysis algorithm
        
        // Dummy implementation for the meta-cognitive analysis
        // In a real system, this would use historical data and system performance metrics
        let self_assessment = format!(
            "Meta-cognitive analysis of '{}' at depth level {}", 
            req.topic, req.depth
        );
        
        let strengths = vec![
            "Consistent application of known patterns".to_string(),
            "Strong error detection capabilities".to_string(),
        ];
        
        let weaknesses = vec![
            "Limited historical context for decision-making".to_string(),
            "Potential for overconfidence in pattern recognition".to_string(),
        ];
        
        let growth_areas = vec![
            "Expand analysis to include more contextual factors".to_string(),
            "Develop stronger prediction models based on past outcomes".to_string(),
        ];
        
        // Return the meta-cognitive result
        let reply = MetaCognitiveResult {
            request_id: req.request_id,
            self_assessment,
            strengths,
            weaknesses,
            growth_areas,
            metadata: HashMap::new(),
        };
        
        Ok(Response::new(reply))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_validate_reflection_request() {
        let service = ReflectionServiceImpl::new().await;
        
        // Valid request
        let valid_request = ReflectionRequest {
            request_id: "test-123".to_string(),
            action_description: "test action".to_string(),
            outcome: "test outcome".to_string(),
            success: true,
            context: HashMap::new(),
        };
        
        assert!(service.validate_reflection_request(&valid_request).is_ok());
        
        // Invalid request - missing request_id
        let invalid_request = ReflectionRequest {
            request_id: "".to_string(),
            action_description: "test action".to_string(),
            outcome: "test outcome".to_string(),
            success: true,
            context: HashMap::new(),
        };
        
        assert!(service.validate_reflection_request(&invalid_request).is_err());
    }
}