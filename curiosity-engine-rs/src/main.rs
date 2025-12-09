use curiosity_engine::curiosity_engine_server::{CuriosityEngine, CuriosityEngineServer};
use curiosity_engine::{KnowledgeGap, ScheduledTask};
use tonic::{Request, Response, Status};

pub mod curiosity_engine {
    tonic::include_proto!("curiosity_engine");
}

#[derive(Debug, Default)]
pub struct CuriosityEngineService {}

#[tonic::async_trait]
impl CuriosityEngine for CuriosityEngineService {
    async fn generate_research_task(
        &self,
        request: Request<KnowledgeGap>,
    ) -> Result<Response<ScheduledTask>, Status> {
        let gap = request.into_inner();
        
        // Generate research task based on knowledge gap
        let task_description = format!("Research: {}", gap.description);
        
        // Set high priority (8/10) as per requirements
        let priority = 8;
        
        Ok(Response::new(ScheduledTask {
            id: gap.id,
            description: task_description,
            priority,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::]:50076".parse()?;
    let service = CuriosityEngineService::default();
    
    tonic::transport::Server::builder()
        .add_service(CuriosityEngineServer::new(service))
        .serve(addr)
        .await?;
    
    Ok(())
}