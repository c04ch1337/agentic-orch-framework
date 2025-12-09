// reflection-rs/src/main.rs
// Reflection Service - Self-reflection and action evaluation
// Port 50065

use tonic::{transport::Server, Request, Response, Status};
use std::sync::Arc;
use std::time::Instant;
use std::net::SocketAddr;
use std::env;
use std::collections::HashMap;
use once_cell::sync::Lazy;

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    reflection_service_server::{ReflectionService, ReflectionServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    ReflectionRequest,
    ReflectionResult,
    EvaluationRequest,
    EvaluationResult,
    MetaCognitiveRequest,
    MetaCognitiveResult,
    HealthRequest,
    HealthResponse,
};

#[derive(Debug, Default)]
pub struct ReflectionServer {
    // Could hold LLM client for deeper analysis
    reflection_count: std::sync::atomic::AtomicU64,
}

impl ReflectionServer {
    fn increment_reflection_count(&self) -> u64 {
        self.reflection_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

#[tonic::async_trait]
impl ReflectionService for ReflectionServer {
    async fn reflect_on_action(
        &self,
        request: Request<ReflectionRequest>,
    ) -> Result<Response<ReflectionResult>, Status> {
        let req = request.into_inner();
        let count = self.increment_reflection_count();
        
        log::info!("ReflectOnAction #{}: action='{}', success={}", 
            count, req.action_description, req.success);

        // Analyze the action and its outcome
        let analysis = if req.success {
            format!(
                "The action '{}' completed successfully with outcome: {}. \
                 This demonstrates effective execution of the intended goal.",
                req.action_description, req.outcome
            )
        } else {
            format!(
                "The action '{}' did not achieve the intended outcome: {}. \
                 Analysis suggests reviewing the approach and identifying failure points.",
                req.action_description, req.outcome
            )
        };

        // Generate lessons learned based on success/failure
        let lessons_learned = if req.success {
            vec![
                "Approach was effective for this type of task".to_string(),
                "Similar strategies can be applied to related problems".to_string(),
            ]
        } else {
            vec![
                "Consider alternative approaches for similar tasks".to_string(),
                "Validate assumptions before executing actions".to_string(),
                "Build in checkpoints for early failure detection".to_string(),
            ]
        };

        // Suggest improvements
        let improvements = vec![
            "Document the decision-making process for future reference".to_string(),
            "Consider edge cases that may not have been addressed".to_string(),
        ];

        let mut metadata = HashMap::new();
        metadata.insert("reflection_id".to_string(), format!("ref-{}", count));
        metadata.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());

        Ok(Response::new(ReflectionResult {
            request_id: req.request_id,
            analysis,
            lessons_learned,
            improvements,
            confidence_score: if req.success { 0.85 } else { 0.75 },
            metadata,
        }))
    }

    async fn evaluate_action(
        &self,
        request: Request<EvaluationRequest>,
    ) -> Result<Response<EvaluationResult>, Status> {
        let req = request.into_inner();
        let count = self.increment_reflection_count();
        
        log::info!("EvaluateAction #{}: action='{}', goal='{}'", 
            count, req.proposed_action, req.goal);

        // Analyze the proposed action
        let has_constraints = !req.constraints.is_empty();
        
        // Simple heuristic evaluation
        let (recommended, rationale) = if req.proposed_action.to_lowercase().contains("delete") 
            || req.proposed_action.to_lowercase().contains("destroy") {
            (false, "Action involves potentially destructive operations. Recommend careful review before proceeding.".to_string())
        } else if req.proposed_action.to_lowercase().contains("backup")
            || req.proposed_action.to_lowercase().contains("verify") {
            (true, "Action involves safety-oriented operations. Recommended to proceed.".to_string())
        } else {
            (true, format!(
                "Action '{}' appears aligned with goal '{}'. Proceed with standard caution.",
                req.proposed_action, req.goal
            ))
        };

        // Identify potential risks
        let risks = if has_constraints {
            vec![
                format!("Must operate within {} constraint(s)", req.constraints.len()),
                "Unexpected side effects possible".to_string(),
            ]
        } else {
            vec!["No explicit constraints defined - consider adding guardrails".to_string()]
        };

        // Suggest alternatives
        let alternatives = vec![
            "Consider a phased approach with checkpoints".to_string(),
            "Implement rollback capability before execution".to_string(),
        ];

        let mut metadata = HashMap::new();
        metadata.insert("evaluation_id".to_string(), format!("eval-{}", count));
        metadata.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());

        Ok(Response::new(EvaluationResult {
            request_id: req.request_id,
            recommended,
            rationale,
            risks,
            alternatives,
            confidence_score: 0.80,
            metadata,
        }))
    }

    async fn meta_cognition(
        &self,
        request: Request<MetaCognitiveRequest>,
    ) -> Result<Response<MetaCognitiveResult>, Status> {
        let req = request.into_inner();
        let count = self.increment_reflection_count();
        
        log::info!("MetaCognition #{}: topic='{}', depth={}", 
            count, req.topic, req.depth);

        // Self-assessment based on topic
        let self_assessment = format!(
            "Self-analysis of '{}' at depth level {}/5: \
             Current capabilities are functional with room for enhancement. \
             System operates reliably within defined parameters.",
            req.topic, req.depth.min(5).max(1)
        );

        // Identify strengths
        let strengths = vec![
            "Consistent response patterns across similar requests".to_string(),
            "Reliable execution of defined protocols".to_string(),
            "Effective integration with other system components".to_string(),
        ];

        // Identify weaknesses
        let weaknesses = vec![
            "Limited ability to handle novel situations outside training".to_string(),
            "Dependent on external LLM for deep analysis".to_string(),
        ];

        // Growth areas
        let growth_areas = vec![
            "Expand knowledge base with more domain-specific information".to_string(),
            "Improve adaptive learning from interaction patterns".to_string(),
            "Enhance meta-cognitive depth for complex reflection".to_string(),
        ];

        let mut metadata = HashMap::new();
        metadata.insert("metacog_id".to_string(), format!("meta-{}", count));
        metadata.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        metadata.insert("analysis_depth".to_string(), req.depth.to_string());

        Ok(Response::new(MetaCognitiveResult {
            request_id: req.request_id,
            self_assessment,
            strengths,
            weaknesses,
            growth_areas,
            metadata,
        }))
    }
}

#[tonic::async_trait]
impl HealthService for ReflectionServer {
    async fn get_health(&self, _request: Request<HealthRequest>) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let reflection_count = self.reflection_count.load(std::sync::atomic::Ordering::SeqCst);
        
        let mut dependencies = HashMap::new();
        dependencies.insert("reflection_engine".to_string(), "ACTIVE".to_string());
        dependencies.insert("reflections_processed".to_string(), reflection_count.to_string());
        
        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "reflection-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let addr_str = env::var("REFLECTION_SERVICE_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50065".to_string());
    
    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str.strip_prefix("http://").unwrap_or(&addr_str).parse()?
    } else {
        addr_str.parse()?
    };

    let _ = *START_TIME;

    let reflection_server = Arc::new(ReflectionServer::default());
    let ref_for_health = reflection_server.clone();

    log::info!("Reflection Service starting on {}", addr);
    println!("Reflection Service listening on {}", addr);

    Server::builder()
        .add_service(ReflectionServiceServer::from_arc(reflection_server))
        .add_service(HealthServiceServer::from_arc(ref_for_health))
        .serve(addr)
        .await?;

    Ok(())
}
