// reflection-service-rs/src/tests.rs
// Tests for the Reflection Service

#[cfg(test)]
mod tests {
    use crate::agi_core::{
        reflection_service_server::ReflectionService, EvaluationRequest, EvaluationResult,
        MetaCognitiveRequest, MetaCognitiveResult, ReflectionRequest, ReflectionResult,
    };
    use crate::service::ReflectionServiceImpl;
    use std::collections::HashMap;
    use tonic::{Request, Response, Status};

    // Test reflection on successful action
    #[tokio::test]
    async fn test_reflect_on_successful_action() {
        let service = ReflectionServiceImpl::new().await;

        // Create a test request with a successful action
        let mut context = HashMap::new();
        context.insert("system".to_string(), "test_system".to_string());

        let request = Request::new(ReflectionRequest {
            request_id: "test-123".to_string(),
            action_description: "fetch user data from database".to_string(),
            outcome: "user data successfully retrieved".to_string(),
            success: true,
            context: context.clone(),
        });

        // Call the service method
        let response = service.reflect_on_action(request).await.unwrap();
        let result = response.into_inner();

        // Verify the result
        assert_eq!(result.request_id, "test-123");
        assert!(!result.analysis.is_empty());
        assert!(result.improvements.len() > 0);
        assert!(result.confidence_score > 0.0);
    }

    // Test reflection on failed action
    #[tokio::test]
    async fn test_reflect_on_failed_action() {
        let service = ReflectionServiceImpl::new().await;

        // Create a test request with a failed action
        let mut context = HashMap::new();
        context.insert("system".to_string(), "test_system".to_string());

        let request = Request::new(ReflectionRequest {
            request_id: "test-456".to_string(),
            action_description: "update user profile".to_string(),
            outcome: "database connection timed out".to_string(),
            success: false,
            context: context.clone(),
        });

        // Call the service method
        let response = service.reflect_on_action(request).await.unwrap();
        let result = response.into_inner();

        // Verify the result
        assert_eq!(result.request_id, "test-456");
        assert!(!result.analysis.is_empty());
        assert!(result.lessons_learned.len() > 0);
        assert!(result.improvements.len() > 0);
        assert!(result.confidence_score > 0.0);
    }

    // Test validation of reflection request
    #[tokio::test]
    async fn test_validate_reflection_request() {
        let service = ReflectionServiceImpl::new().await;

        // Create an invalid request (missing action description)
        let request = Request::new(ReflectionRequest {
            request_id: "test-789".to_string(),
            action_description: "".to_string(), // Empty action description
            outcome: "some outcome".to_string(),
            success: true,
            context: HashMap::new(),
        });

        // Call the service method - should return an error
        let response = service.reflect_on_action(request).await;
        assert!(response.is_err());

        // Check that the error is an invalid argument error
        match response {
            Err(status) => {
                assert_eq!(status.code(), tonic::Code::InvalidArgument);
                assert!(status.message().contains("Action description"));
            }
            _ => panic!("Expected an error response"),
        }
    }

    // Test action evaluation
    #[tokio::test]
    async fn test_evaluate_action() {
        let service = ReflectionServiceImpl::new().await;

        // Create a test evaluation request
        let mut constraints = Vec::new();
        constraints.push("must be completed within 5 seconds".to_string());

        let mut context = HashMap::new();
        context.insert("system".to_string(), "test_system".to_string());

        let request = Request::new(EvaluationRequest {
            request_id: "eval-123".to_string(),
            proposed_action: "query the database for user information".to_string(),
            goal: "retrieve user profile data".to_string(),
            constraints,
            context,
        });

        // Call the service method
        let response = service.evaluate_action(request).await.unwrap();
        let result = response.into_inner();

        // Verify the result
        assert_eq!(result.request_id, "eval-123");
        assert!(!result.rationale.is_empty());
        assert!(result.risks.len() > 0);
        assert!(result.alternatives.len() > 0);
        assert!(result.confidence_score > 0.0);
    }

    // Test meta-cognition
    #[tokio::test]
    async fn test_meta_cognition() {
        let service = ReflectionServiceImpl::new().await;

        // Create a test meta-cognitive request
        let request = Request::new(MetaCognitiveRequest {
            request_id: "meta-123".to_string(),
            topic: "decision making process".to_string(),
            depth: 3, // Medium depth
        });

        // Call the service method
        let response = service.meta_cognition(request).await.unwrap();
        let result = response.into_inner();

        // Verify the result
        assert_eq!(result.request_id, "meta-123");
        assert!(!result.self_assessment.is_empty());
        assert!(result.strengths.len() > 0);
        assert!(result.weaknesses.len() > 0);
        assert!(result.growth_areas.len() > 0);
    }
}
