use log_analyzer::log_analyzer_server::{LogAnalyzer, LogAnalyzerServer};
use log_analyzer::{ExecutionLog, FailureReport, FailureReport_Severity};
use tonic::{Request, Response, Status};

pub mod log_analyzer {
    tonic::include_proto!("log_analyzer");
}

#[derive(Debug, Default)]
pub struct LogAnalyzerService {}

#[tonic::async_trait]
impl LogAnalyzer for LogAnalyzerService {
    async fn analyze_execution_outcome(
        &self,
        request: Request<ExecutionLog>,
    ) -> Result<Response<FailureReport>, Status> {
        let log = request.into_inner();
        
        // Simple NLP logic using regex to detect common error patterns
        let severity = if log.raw_log.contains("error") || log.raw_log.contains("fail") {
            FailureReport_Severity::Critical
        } else if log.raw_log.contains("warn") {
            FailureReport_Severity::Ambiguous
        } else {
            FailureReport_Severity::Success
        };
        
        // Extract root cause summary (simplified)
        let root_cause_summary = if severity != FailureReport_Severity::Success {
            "Failure detected in service execution".to_string()
        } else {
            "Execution successful".to_string()
        };
        
        // Service ID extraction (simplified)
        let service_id = if let Some(line) = log.raw_log.lines().find(|l| l.contains("service")) {
            line.split_whitespace().nth(1).unwrap_or("unknown").to_string()
        } else {
            "unknown".to_string()
        };
        
        Ok(Response::new(FailureReport {
            severity: severity.into(),
            root_cause_summary,
            service_id,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::]:50075".parse()?;
    let service = LogAnalyzerService::default();
    
    tonic::transport::Server::builder()
        .add_service(LogAnalyzerServer::new(service))
        .serve(addr)
        .await?;
    
    Ok(())
}