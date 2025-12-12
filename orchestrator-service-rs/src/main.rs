// orchestrator-service-rs/src/main.rs
// Main Entry Point for orchestrator-service-rs
// Implements the OrchestratorService gRPC server

#[cfg(test)]
mod tests {
    pub mod registry_integration_tests;
}

use config_rs;
use once_cell::sync::Lazy;
use prost::Message;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status, transport::Server};
use action_ledger::{ActionLedger, ActionPlanStep, ActionOutcome, ActionOutcomeStatus};
use self_improve::{SelfImprover, SelfImproveConfig, CriticalFailure};
use telemetrist::{Telemetrist, ExecutionTrace, TelemetristConfig};

// Track service start time for uptime reporting
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

// 1. Import the generated code
// This module is created by tonic-prost-build from agi_core.proto
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

// 2. Import the required components from the generated code
use agi_core::{
    AgiResponse, // Added for unified response format
    ContextRequest,
    EthicsCheckRequest,
    EthicsCheckResponse,
    GenerateRequest,
    GetAgentRequest,
    HealthRequest,
    HealthResponse,
    ReflectionRequest,
    Request as ProtoRequest,
    Response as ProtoResponse,
    RouteRequest,
    RouteResponse,
    ToolRequest,
    ToolResponse,
    ValidationRequest,
    ValidationResponse,
    agent_registry_service_client::AgentRegistryServiceClient,
    context_manager_service_client::ContextManagerServiceClient,
    data_router_service_client::DataRouterServiceClient,
    health_service_server::{HealthService, HealthServiceServer},
    orchestrator_service_server::{OrchestratorService, OrchestratorServiceServer},
    reflection_service_client::ReflectionServiceClient,
};

// 3. Orchestration planning and error types

#[derive(Debug, serde::Deserialize)]
struct Plan {
    steps: Vec<PlanStep>,
}

#[derive(Debug, serde::Deserialize)]
struct PlanStep {
    id: String,
    action: String, // "llm", "kb", "tools", "safety", "final"
    description: String,
    #[serde(default)]
    target_service: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_parameters: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
enum OrchestrationStage {
    ContextEnrichment,
    Planning,
    Ethics,
    Safety,
    ToolsExecution,
    Execution,
    Reflection,
}

#[derive(Debug)]
struct OrchestrationError {
    stage: OrchestrationStage,
    target_service: String,
    error_type: String,
    error_message: String,
    retryable: bool,
}

fn classify_status_error(
    stage: OrchestrationStage,
    target_service: &str,
    status: &tonic::Status,
) -> OrchestrationError {
    let code = status.code();
    let retryable = matches!(
        code,
        tonic::Code::Unavailable | tonic::Code::DeadlineExceeded | tonic::Code::ResourceExhausted
    );

    OrchestrationError {
        stage,
        target_service: target_service.to_string(),
        error_type: format!("{:?}", code),
        error_message: status.message().to_string(),
        retryable,
    }
}

#[derive(serde::Serialize)]
struct CriticalFailureLog {
    event_type: String, // always "CRITICAL_FAILURE"
    service: String,    // "orchestrator-service"
    stage: String,      // e.g. "ToolsExecution"
    request_id: String,
    phoenix_session_id: String,
    target_service: String, // "tools-service", "llm-service", etc.
    error_type: String,     // e.g. "DEADLINE_EXCEEDED"
    error_message: String,
    retryable: bool,
    tool_name: Option<String>,
    metadata: std::collections::HashMap<String, String>,
}

impl OrchestratorServer {
    async fn log_critical_failure_and_build_response(
        &self,
        err: OrchestrationError,
        req_data: &ProtoRequest,
        current_tool_name: Option<String>,
    ) -> Result<Response<AgiResponse>, Status> {
        let log_event = CriticalFailureLog {
            event_type: "CRITICAL_FAILURE".to_string(),
            service: "orchestrator-service".to_string(),
            stage: format!("{:?}", err.stage),
            request_id: req_data.id.clone(),
            phoenix_session_id: req_data.id.clone(),
            target_service: err.target_service.clone(),
            error_type: err.error_type.clone(),
            error_message: err.error_message.clone(),
            retryable: err.retryable,
            tool_name: current_tool_name.clone(),
            metadata: req_data.metadata.clone(),
        };

        if let Ok(json) = serde_json::to_string(&log_event) {
            log::error!("{}", json);
        } else {
            log::error!(
                "CRITICAL_FAILURE: stage={:?}, target_service={}, error_type={}, error_message={}",
                err.stage,
                err.target_service,
                err.error_type,
                err.error_message
            );
        }

        // Forward to self-improvement engine
        if let Some(improver_guard) = self.self_improver.lock().await.as_ref() {
            let failure = CriticalFailure::from_orchestrator_failure(
                req_data.id.clone(),
                format!("{:?}", err.stage),
                Some(err.error_message.clone()),
                {
                    let mut meta = req_data.metadata.clone();
                    meta.insert("target_service".to_string(), err.target_service.clone());
                    meta.insert("error_type".to_string(), err.error_type.clone());
                    meta.insert("retryable".to_string(), err.retryable.to_string());
                    if let Some(tool) = &current_tool_name {
                        meta.insert("tool_name".to_string(), tool.clone());
                    }
                    meta
                },
            );
            let improver = std::sync::Arc::clone(improver_guard);
            tokio::spawn(async move {
                if let Err(e) = improver.process_failure(failure).await {
                    log::warn!("Self-improvement engine failed to process failure: {}", e);
                }
            });
        }

    let stage_str = format!("{:?}", err.stage);

    let (final_answer, execution_plan, routed_service) = match err.stage {
        OrchestrationStage::ToolsExecution => {
            let tool_name_display = current_tool_name
                .clone()
                .unwrap_or_else(|| "code_gen".to_string());

            let final_answer = format!(
"I attempted to use the code tools to complete your request, but the code execution environment did not respond in time or reported an internal error. The last tool I called was {}. No unsafe or partial code was executed.

Please try again later or simplify your request so that I can answer without running tools.",
                tool_name_display
            );

            let execution_plan = format!(
                "Execution Plan:
1. Enriched context via Context Manager.
2. Planned steps with LLM via Data Router.
3. Verified ethics via Soul-KB.
4. Validated request with Safety Service.
5. Attempted tools execution [FAILED at stage {:?}, target: {}, error_type: {}].

Status: 502
Routed To: {}
Error: {}",
                err.stage,
                err.target_service,
                err.error_type,
                err.target_service,
                err.error_message
            );

            (final_answer, execution_plan, err.target_service.clone())
        }
        _ => {
            let final_answer = format!(
                "I attempted to complete your request, but a downstream dependency failed during the {} stage while calling {}. Please try again later.",
                stage_str, err.target_service,
            );

            let execution_plan = format!(
                "Execution Plan:
1. Enriched context via Context Manager.
2. Planned steps with LLM via Data Router.
3. Verified ethics via Soul-KB.
4. Validated request with Safety Service.
5. Attempted execution [FAILED at stage {:?}, target: {}, error_type: {}, message: {}].",
                err.stage, err.target_service, err.error_type, err.error_message
            );

            (final_answer, execution_plan, err.target_service.clone())
        }
    };

    let agi_response = AgiResponse {
        final_answer,
        execution_plan,
        routed_service,
        phoenix_session_id: req_data.id.clone(),
        output_artifact_urls: Vec::new(),
    };

    Ok(Response::new(agi_response))
}

}

// 4. Define the Orchestrator Server Structure
// This struct will hold the state and implement the gRPC trait.
/// Agent information returned from find_agent_by_capability
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub name: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub status: String,
}

#[derive(Debug)]
pub struct OrchestratorServer {
    // Client stub for communicating with Data Router Service
    data_router_client: Arc<Mutex<Option<DataRouterServiceClient<tonic::transport::Channel>>>>,
    // Client stub for communicating with Reflection Service
    reflection_client: Arc<Mutex<Option<ReflectionServiceClient<tonic::transport::Channel>>>>,
    // Client stub for Agent Registry Service
    agent_registry_client:
        Arc<Mutex<Option<AgentRegistryServiceClient<tonic::transport::Channel>>>>,
    // Client stub for Context Manager Service
    context_manager_client:
        Arc<Mutex<Option<ContextManagerServiceClient<tonic::transport::Channel>>>>,
    // Action Ledger for audit trail
    action_ledger: Arc<Mutex<Option<ActionLedger>>>,
    // Self-Improvement Engine for failure learning
    self_improver: Arc<Mutex<Option<Arc<SelfImprover>>>>,
    // Telemetrist for execution trace collection
    telemetrist: Arc<Mutex<Option<Arc<Telemetrist>>>>,
}

// Import Log Analyzer client with proper error handling
pub mod log_analyzer {
    #[allow(warnings)]
    tonic::include_proto!("log_analyzer");
}

impl OrchestratorServer {
    /// Create a new OrchestratorServer instance
    pub fn new() -> Self {
        Self {
            data_router_client: Arc::new(Mutex::new(None)),
            reflection_client: Arc::new(Mutex::new(None)),
            agent_registry_client: Arc::new(Mutex::new(None)),
            context_manager_client: Arc::new(Mutex::new(None)),
            action_ledger: Arc::new(Mutex::new(None)),
            self_improver: Arc::new(Mutex::new(None)),
            telemetrist: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize the Telemetrist service
    pub async fn init_telemetrist(&self) -> Result<(), Box<dyn std::error::Error>> {
        match Telemetrist::new(TelemetristConfig::from_env()) {
            Ok(telemetrist) => {
                let mut guard = self.telemetrist.lock().await;
                *guard = Some(Arc::new(telemetrist));
                log::info!("Telemetrist initialized successfully");
                Ok(())
            }
            Err(e) => {
                log::warn!("Failed to initialize Telemetrist: {}", e);
                Err(Box::new(e))
            }
        }
    }

    /// Initialize the Action Ledger
    pub async fn init_action_ledger(&self) -> Result<(), Box<dyn std::error::Error>> {
        match ActionLedger::new_default() {
            Ok(ledger) => {
                let mut guard = self.action_ledger.lock().await;
                *guard = Some(ledger);
                log::info!("Action Ledger initialized successfully");
                Ok(())
            }
            Err(e) => {
                log::warn!("Failed to initialize Action Ledger: {}", e);
                Err(Box::new(e))
            }
        }
    }

    /// Initialize the Self-Improvement Engine
    pub async fn init_self_improver(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cfg = SelfImproveConfig::from_env();
        match SelfImprover::new(cfg) {
            Ok(improver) => {
                let mut guard = self.self_improver.lock().await;
                *guard = Some(Arc::new(improver));
                log::info!("Self-Improvement Engine initialized successfully");
                Ok(())
            }
            Err(e) => {
                log::warn!("Failed to initialize Self-Improvement Engine: {}", e);
                Err(Box::new(e))
            }
        }
    }

    /// Initialize the Context Manager Service client
    pub async fn init_context_manager_client(
        &self,
        addr: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Connecting to Context Manager Service at {}", addr);
        let client = ContextManagerServiceClient::connect(addr.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to connect to Context Manager Service: {}", e);
                e
            })?;
        let mut client_guard = self.context_manager_client.lock().await;
        *client_guard = Some(client);
        log::info!("Successfully connected to Context Manager Service");
        Ok(())
    }

    async fn get_context_manager_client(
        &self,
    ) -> Option<ContextManagerServiceClient<tonic::transport::Channel>> {
        let client_guard = self.context_manager_client.lock().await;
        client_guard.as_ref().cloned()
    }

    /// Initialize the Data Router Service client
    /// Connects to the Data Router Service at the specified address
    pub async fn init_data_router_client(
        &self,
        router_addr: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Connecting to Data Router Service at {}", router_addr);

        let client = DataRouterServiceClient::connect(router_addr.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to connect to Data Router Service: {}", e);
                e
            })?;

        let mut client_guard = self.data_router_client.lock().await;
        *client_guard = Some(client);

        log::info!("Successfully connected to Data Router Service");
        Ok(())
    }

    /// Get a cloned reference to the Data Router client (for internal use)
    /// Note: Tonic clients are cheap to clone and share the underlying connection
    async fn get_data_router_client(
        &self,
    ) -> Result<DataRouterServiceClient<tonic::transport::Channel>, Status> {
        let client_guard = self.data_router_client.lock().await;
        client_guard
            .as_ref()
            .cloned()
            .ok_or_else(|| Status::unavailable("Data Router Service client not initialized"))
    }

    /// Initialize the Reflection Service client
    pub async fn init_reflection_client(
        &self,
        reflection_addr: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Connecting to Reflection Service at {}", reflection_addr);

        let client = ReflectionServiceClient::connect(reflection_addr.clone())
            .await
            .map_err(|e| {
                log::warn!("Failed to connect to Reflection Service (optional): {}", e);
                e
            })?;

        let mut client_guard = self.reflection_client.lock().await;
        *client_guard = Some(client);

        log::info!("Successfully connected to Reflection Service");
        Ok(())
    }

    /// Get a cloned reference to the Reflection client (optional - may not be available)
    async fn get_reflection_client(
        &self,
    ) -> Option<ReflectionServiceClient<tonic::transport::Channel>> {
        let client_guard = self.reflection_client.lock().await;
        client_guard.as_ref().cloned()
    }

    /// Initialize the Agent Registry Service client
    pub async fn init_agent_registry_client(
        &self,
        registry_addr: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Connecting to Agent Registry Service at {}", registry_addr);

        let client = AgentRegistryServiceClient::connect(registry_addr.clone())
            .await
            .map_err(|e| {
                log::warn!("Failed to connect to Agent Registry (optional): {}", e);
                e
            })?;

        let mut client_guard = self.agent_registry_client.lock().await;
        *client_guard = Some(client);

        log::info!("Successfully connected to Agent Registry Service");
        Ok(())
    }

    /// Get a cloned reference to the Agent Registry client
    async fn get_agent_registry_client(
        &self,
    ) -> Option<AgentRegistryServiceClient<tonic::transport::Channel>> {
        let client_guard = self.agent_registry_client.lock().await;
        client_guard.as_ref().cloned()
    }

    /// Find an agent by capability for task delegation
    /// Returns AgentInfo if a verified agent is found with the requested capability
    pub async fn find_agent_by_capability(
        &self,
        capability: &str,
    ) -> Result<Option<AgentInfo>, Status> {
        log::info!("Looking for agent with capability: {}", capability);

        // Get the agent registry client with lock protection
        let registry_client = match tokio::time::timeout(
            std::time::Duration::from_secs(1),
            self.agent_registry_client.lock(),
        )
        .await
        {
            Ok(guard) => guard,
            Err(_) => {
                log::error!("Timeout while acquiring lock on agent registry client");
                return Err(Status::internal("Internal lock timeout"));
            }
        };

        if let Some(client) = &*registry_client {
            // Query Agent Registry for agents with this capability
            let request = tonic::Request::new(GetAgentRequest {
                name: String::new(),
                capability: capability.to_string(),
            });

            // Use a timeout for the registry query
            match tokio::time::timeout(std::time::Duration::from_secs(3), client.get_agent(request))
                .await
            {
                Ok(Ok(response)) => {
                    let resp = response.into_inner();
                    if resp.found {
                        if let Some(agent) = resp.agent {
                            // Agent Registry only returns verified agents
                            log::info!(
                                "Found verified agent '{}' for capability '{}'",
                                agent.name,
                                capability
                            );

                            // Get the host from environment or default to localhost
                            let host = std::env::var("SERVICE_HOST")
                                .unwrap_or_else(|_| "localhost".to_string());

                            return Ok(Some(AgentInfo {
                                name: agent.name,
                                endpoint: format!("http://{}:{}", host, agent.port),
                                capabilities: agent.capabilities,
                                status: agent.status,
                            }));
                        }
                    }

                    log::warn!("No verified agent found with capability: {}", capability);
                    Ok(None)
                }
                Ok(Err(e)) => {
                    log::warning!(
                        "Agent registry returned error for capability {}: {}",
                        capability,
                        e
                    );

                    // Map specific error codes to appropriate statuses
                    match e.code() {
                        tonic::Code::Unavailable => {
                            log::error!("Agent Registry service unavailable: {}", e.message());
                            Err(Status::unavailable(format!(
                                "Agent Registry unavailable: {}",
                                e.message()
                            )))
                        }
                        tonic::Code::DeadlineExceeded => {
                            log::error!("Agent Registry timed out: {}", e.message());
                            Err(Status::deadline_exceeded("Agent Registry timeout"))
                        }
                        _ => {
                            log::error!("Agent Registry error: {}", e.message());
                            Err(Status::internal(format!(
                                "Agent Registry error: {}",
                                e.message()
                            )))
                        }
                    }
                }
                Err(_) => {
                    log::warning!(
                        "Timeout querying Agent Registry for capability {}",
                        capability
                    );
                    Err(Status::deadline_exceeded("Agent Registry query timeout"))
                }
            }
        } else {
            log::warning!("Agent Registry client not initialized");
            Err(Status::failed_precondition(
                "Agent Registry client not initialized",
            ))
        }
    }
}

impl Default for OrchestratorServer {
    fn default() -> Self {
        Self::new()
    }
}

// 4. Implement the OrchestratorService Trait
// This provides the actual logic for the gRPC methods defined in the .proto file.
#[tonic::async_trait]
impl OrchestratorService for OrchestratorServer {
    async fn process_request(
        &self,
        request: Request<ProtoRequest>,
    ) -> Result<Response<AgiResponse>, Status> {
        let req_data = request.into_inner();

        log::info!(
            "Received ProcessRequest: id={}, service={}, method={}",
            req_data.id,
            req_data.service,
            req_data.method
        );

        // Simple implementation for now - in production this would coordinate with services
        let final_answer = format!(
            "Processed request {} for service {} using method {}",
            req_data.id, req_data.service, req_data.method
        );

        let execution_plan = format!(
            "1. Received request\n2. Validated input\n3. Processed via {}\n4. Returned result",
            req_data.service
        );

        let reply = AgiResponse {
            final_answer,
            execution_plan,
            routed_service: req_data.service.clone(),
            phoenix_session_id: req_data.id.clone(),
            output_artifact_urls: Vec::new(),
        };

        Ok(Response::new(reply))
    }

    async fn plan_and_execute(
        &self,
        request: Request<ProtoRequest>,
    ) -> Result<Response<AgiResponse>, Status> {
        let request_start_time = std::time::Instant::now();
        let req_data = request.into_inner();

        log::info!(
            "Received PlanAndExecute request: id={}, service={}, method={}",
            req_data.id,
            req_data.service,
            req_data.method
        );

        // Get the Data Router client
        let mut router_client = self.get_data_router_client().await?;

        // Phase 1: Planning - Use LLM Service to break down the request into sub-tasks
        // If the request payload contains a natural language query, we'll plan it
        let user_query = String::from_utf8_lossy(&req_data.payload);
        log::info!("Planning execution for request: {}", user_query);

        // Step 0: Context Enrichment - Call Context Manager to get enriched context
        let mut enriched_prompt = user_query.clone().to_string();

        if let Some(mut cm_client) = self.get_context_manager_client().await {
            log::info!("Enriching context for request: {}", req_data.id);
            let context_req = ContextRequest {
                request_id: req_data.id.clone(),
                query: user_query.to_string(),
                agent_type: "master".to_string(),
                max_context_tokens: 2000,
                kb_sources: vec![
                    "mind".to_string(),
                    "soul".to_string(),
                    "heart".to_string(),
                    "social".to_string(),
                ],
            };

            match cm_client
                .enrich_context(tonic::Request::new(context_req))
                .await
            {
                Ok(resp) => {
                    let enriched = resp.into_inner();
                    enriched_prompt = enriched.system_prompt;
                    log::info!(
                        "Context enriched. Tokens used: {}",
                        enriched.total_tokens_used
                    );
                }
                Err(e) => {
                    log::warn!(
                        "Context enrichment failed (proceeding with raw query): {}",
                        e
                    );
                }
            }
        }

        // Step 1: Call LLM Service via Data Router to generate a plan
        // Create a request to LLM Service for planning
        let planning_request = ProtoRequest {
            id: format!("{}-plan", req_data.id),
            service: "llm-service".to_string(),
            method: "generate_text".to_string(),
            payload: {
                let generate_req = GenerateRequest {
                    prompt: format!(
                        "Context: {}\n\nTask: Break down this request into actionable steps: {}. Return a JSON list of steps, each with 'action' (llm, tools, kb, safety) and 'description'.",
                        enriched_prompt, user_query
                    ),
                    parameters: std::collections::HashMap::new(),
                };
                let mut buf = Vec::new();
                generate_req.encode(&mut buf).map_err(|e| {
                    Status::internal(format!("Failed to encode GenerateRequest: {}", e))
                })?;
                buf
            },
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("request_type".to_string(), "planning".to_string());
                meta.insert("original_request_id".to_string(), req_data.id.clone());
                meta
            },
        };

        let route_request = RouteRequest {
            target_service: "llm-service".to_string(),
            request: Some(planning_request),
        };

        log::info!("Calling LLM Service for planning via Data Router");
        let planning_response = router_client
            .route(tonic::Request::new(route_request))
            .await;

        let planning_response = match planning_response {
            Ok(resp) => resp,
            Err(status) => {
                let err =
                    classify_status_error(OrchestrationStage::Planning, "llm-service", &status);
                return self.log_critical_failure_and_build_response(err, &req_data, None).await;
            }
        };

        let planning_data = planning_response.into_inner();
        let plan_text = if let Some(plan_resp) = planning_data.response {
            String::from_utf8_lossy(&plan_resp.payload).to_string()
        } else {
            log::warn!("LLM Service returned empty planning response, using direct execution");
            "Direct execution".to_string()
        };

        log::info!("Planning complete. Plan: {}", plan_text);

        let parsed_plan: Option<Plan> = serde_json::from_str(&plan_text).ok();

        struct ExecutionContext {
            kb_notes: Vec<String>,
            tool_results: Vec<String>,
            llm_intermediate_answers: Vec<String>,
        }

        let mut exec_ctx = ExecutionContext {
            kb_notes: Vec::new(),
            tool_results: Vec::new(),
            llm_intermediate_answers: Vec::new(),
        };

        let tool_preference = req_data
            .metadata
            .get("tool_preference")
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_else(|| "auto".to_string());

        // Step 1.5: Ethics Check - Verify plan with Soul-KB
        log::info!("Verifying ethics with Soul-KB");
        let ethics_req = EthicsCheckRequest {
            action: plan_text.clone(),
            context: String::new(),
        };

        let ethics_payload = prost::Message::encode_to_vec(&ethics_req)
            .map_err(|e| Status::internal(format!("Failed to encode EthicsCheckRequest: {}", e)))?;

        let ethics_route_req = RouteRequest {
            target_service: "soul-kb".to_string(),
            request: Some(ProtoRequest {
                id: format!("{}-ethics", req_data.id),
                service: "soul-kb".to_string(),
                method: "CheckEthics".to_string(),
                payload: ethics_payload,
                metadata: std::collections::HashMap::new(),
            }),
        };

        match router_client
            .route(tonic::Request::new(ethics_route_req))
            .await
        {
            Ok(resp) => {
                let inner = resp.into_inner();
                if let Some(response) = inner.response {
                    if response.status_code == 200 {
                        if let Ok(ethics_resp) =
                            EthicsCheckResponse::decode(response.payload.as_slice())
                        {
                            if !ethics_resp.allowed {
                                log::warn!(
                                    "Soul-KB blocked action: {:?}",
                                    ethics_resp.violated_values
                                );
                                return Ok(Response::new(AgiResponse {
                                    final_answer: format!(
                                        "Action blocked by ethical constraints: {:?}",
                                        ethics_resp.violated_values
                                    ),
                                    execution_plan: "Execution halted due to ethical violation"
                                        .to_string(),
                                    routed_service: "soul-kb".to_string(),
                                    phoenix_session_id: req_data.id.clone(),
                                    output_artifact_urls: Vec::new(),
                                }));
                            }
                        }
                    }
                }
            }
            Err(e) => log::warn!("Ethics check failed (proceeding with caution): {}", e),
        }

        log::info!("Planning complete. Plan: {}", plan_text);

        // Phase 2: Safety Check - Validate the plan with Safety Service
        log::info!("Validating plan with Safety Service");
        let safety_request = ProtoRequest {
            id: format!("{}-safety", req_data.id),
            service: "safety-service".to_string(),
            method: "validate_request".to_string(),
            payload: {
                let validation_req = ValidationRequest {
                    request: Some(req_data.clone()),
                    context: {
                        let mut ctx = std::collections::HashMap::new();
                        ctx.insert("plan".to_string(), plan_text.clone());
                        ctx.insert("original_query".to_string(), user_query.to_string());
                        ctx
                    },
                };
                let mut buf = Vec::new();
                validation_req.encode(&mut buf).map_err(|e| {
                    Status::internal(format!("Failed to encode ValidationRequest: {}", e))
                })?;
                buf
            },
            metadata: std::collections::HashMap::new(),
        };

        let safety_route_request = RouteRequest {
            target_service: "safety-service".to_string(),
            request: Some(safety_request),
        };

        let safety_response = router_client
            .route(tonic::Request::new(safety_route_request))
            .await;

        let safety_response = match safety_response {
            Ok(resp) => resp,
            Err(status) => {
                let err =
                    classify_status_error(OrchestrationStage::Safety, "safety-service", &status);
                return self.log_critical_failure_and_build_response(err, &req_data, None).await;
            }
        };

        let safety_data = safety_response.into_inner();
        if let Some(safety_resp) = safety_data.response {
            // Check if the request was approved
            if let Ok(validation_resp) = ValidationResponse::decode(safety_resp.payload.as_slice())
            {
                if !validation_resp.approved {
                    log::warn!(
                        "Safety Service rejected the request: {}",
                        validation_resp.reason
                    );
                    return Ok(Response::new(AgiResponse {
                        final_answer: format!(
                            "Request rejected by Safety Service: {}",
                            validation_resp.reason
                        ),
                        execution_plan: format!(
                            "Safety check failed. Risk level: {}",
                            validation_resp.risk_level
                        ),
                        routed_service: "safety-service".to_string(),
                        phoenix_session_id: req_data.id.clone(),
                        output_artifact_urls: Vec::new(),
                    }));
                }
                log::info!(
                    "Safety Service approved the request (risk level: {})",
                    validation_resp.risk_level
                );
            }
        }

        // Phase 3: Tools execution (optional, driven by plan)
        if let Some(plan) = &parsed_plan {
            for step in &plan.steps {
                if step.action == "tools" && tool_preference != "disable" {
                    let tool_name = step
                        .tool_name
                        .clone()
                        .unwrap_or_else(|| "default_tool".to_string());

                    let mut parameters = step.tool_parameters.clone();
                    parameters.insert("user_query".to_string(), user_query.to_string());
                    parameters.insert("step_description".to_string(), step.description.clone());
                    parameters.insert("request_id".to_string(), req_data.id.clone());

                    // Log pre-execution to action ledger
                    let ledger_entry_id = {
                        if let Some(ledger_guard) = self.action_ledger.lock().await.as_ref() {
                            let action_step = ActionPlanStep {
                                request_id: Some(req_data.id.clone()),
                                actor: "orchestrator".to_string(),
                                tool_or_action_name: tool_name.clone(),
                                parameters_json: serde_json::to_string(&parameters).unwrap_or_default(),
                                user_query_snapshot: Some(user_query.clone()),
                                critical: true,
                                metadata: req_data.metadata.clone(),
                            };
                            ledger_guard.commit_pre_execution(action_step).ok()
                        } else {
                            None
                        }
                    };

                    let tool_request = ToolRequest {
                        tool_name: tool_name.clone(),
                        parameters,
                    };

                    let mut tool_payload = Vec::new();
                    tool_request.encode(&mut tool_payload).map_err(|e| {
                        Status::internal(format!("Failed to encode ToolRequest: {}", e))
                    })?;

                    let mut metadata = req_data.metadata.clone();
                    metadata.insert(
                        "orchestration_stage".to_string(),
                        "tools_execution".to_string(),
                    );
                    metadata.insert("plan_step_id".to_string(), step.id.clone());

                    let tool_proto_request = ProtoRequest {
                        id: format!("{}-tool-{}", req_data.id, step.id),
                        service: "tools-service".to_string(),
                        method: "ExecuteTool".to_string(),
                        payload: tool_payload,
                        metadata,
                    };

                    let route_request = RouteRequest {
                        target_service: "tools-service".to_string(),
                        request: Some(tool_proto_request),
                    };

                    // Record execution trace start
                    let trace_start = std::time::Instant::now();

                    let tool_route_response = router_client
                        .route(tonic::Request::new(route_request))
                        .await;

                    // Record execution trace
                    let trace_duration = trace_start.elapsed().as_millis() as u64;
                    if let Some(telemetrist_guard) = self.telemetrist.lock().await.as_ref() {
                        let trace = ExecutionTrace {
                            trace_id: uuid::Uuid::new_v4().to_string(),
                            request_id: req_data.id.clone(),
                            service: "tools-service".to_string(),
                            method: "ExecuteTool".to_string(),
                            duration_ms: trace_duration,
                            success: tool_route_response.is_ok(),
                            error: tool_route_response.as_ref().err().map(|e| e.message().to_string()),
                            metadata: {
                                let mut meta = std::collections::HashMap::new();
                                meta.insert("tool_name".to_string(), tool_name.clone());
                                meta.insert("step_id".to_string(), step.id.clone());
                                meta
                            },
                            timestamp: chrono::Utc::now(),
                        };
                        let telemetrist = Arc::clone(telemetrist_guard);
                        tokio::spawn(async move {
                            if let Err(e) = telemetrist.record_execution_trace(trace).await {
                                log::warn!("Failed to record execution trace: {}", e);
                            }
                        });
                    }

                    let tool_route_response = match tool_route_response {
                        Ok(resp) => resp,
                        Err(status) => {
                            // Log post-execution failure to action ledger
                            if let (Some(ledger_guard), Some(entry_id)) = (self.action_ledger.lock().await.as_ref(), ledger_entry_id) {
                                let outcome = ActionOutcome {
                                    status: ActionOutcomeStatus::Failed,
                                    result_summary: None,
                                    error_summary: Some(status.message().to_string()),
                                    metadata: std::collections::HashMap::new(),
                                    timestamp: chrono::Utc::now(),
                                };
                                let _ = ledger_guard.commit_post_execution(entry_id, outcome);
                            }

                            let err = classify_status_error(
                                OrchestrationStage::ToolsExecution,
                                "tools-service",
                                &status,
                            );
                            let current_tool_name = Some(tool_name.clone());
                            return self.log_critical_failure_and_build_response(
                                err,
                                &req_data,
                                current_tool_name,
                            ).await;
                        }
                    };

                    let response = tool_route_response
                        .into_inner()
                        .response
                        .ok_or_else(|| Status::internal("Tools Service returned empty response"))?;

                    let tool_response =
                        ToolResponse::decode(response.payload.as_slice()).map_err(|e| {
                            Status::internal(format!("Failed to decode ToolResponse: {}", e))
                        })?;

                    // Log post-execution success to action ledger
                    if let (Some(ledger_guard), Some(entry_id)) = (self.action_ledger.lock().await.as_ref(), ledger_entry_id) {
                        let outcome = ActionOutcome {
                            status: ActionOutcomeStatus::Success,
                            result_summary: Some(tool_response.result.clone()),
                            error_summary: if tool_response.success { None } else { Some(tool_response.error) },
                            metadata: std::collections::HashMap::new(),
                            timestamp: chrono::Utc::now(),
                        };
                        let _ = ledger_guard.commit_post_execution(entry_id, outcome);
                    }

                    exec_ctx.tool_results.push(format!(
                        "Step {} (tools: {}): success ({}).",
                        step.id,
                        tool_name,
                        tool_response
                            .result
                            .split('\n')
                            .next()
                            .unwrap_or("generated result")
                    ));

                    // Also keep full tool result for potential future synthesis usage
                    exec_ctx.tool_results.push(tool_response.result);
                }
            }
        }

        // Phase 4: Execution / Final synthesis
        // Determine target service from request, or use LLM if not specified
        let target_service = if req_data.service.is_empty() {
            // Default to LLM service for general queries
            "llm-service".to_string()
        } else {
            req_data.service.clone()
        };

        log::info!(
            "Executing request via Data Router to target: {} (plan_parsed={})",
            target_service,
            parsed_plan.is_some()
        );

        // Build execution request: enriched LLM prompt when a plan exists, otherwise fallback
        let execution_request = if let Some(plan) = &parsed_plan {
            let mut prompt = String::new();
            prompt.push_str("You are the Orchestrator final synthesis agent.\n\n");
            prompt.push_str("Original user query:\n");
            prompt.push_str(&user_query);
            prompt.push_str("\n\nExecution context:\n");

            if !exec_ctx.kb_notes.is_empty() {
                prompt.push_str("Knowledge base notes:\n");
                for note in &exec_ctx.kb_notes {
                    prompt.push_str("- ");
                    prompt.push_str(note);
                    prompt.push('\n');
                }
                prompt.push('\n');
            }

            if !exec_ctx.llm_intermediate_answers.is_empty() {
                prompt.push_str("Intermediate LLM answers:\n");
                for ans in &exec_ctx.llm_intermediate_answers {
                    prompt.push_str("- ");
                    prompt.push_str(ans);
                    prompt.push('\n');
                }
                prompt.push('\n');
            }

            if !exec_ctx.tool_results.is_empty() {
                prompt.push_str("Tool results:\n");
                for tr in &exec_ctx.tool_results {
                    prompt.push_str("- ");
                    prompt.push_str(tr);
                    prompt.push('\n');
                }
                prompt.push('\n');
            }

            prompt.push_str("Using the above context and tool outputs, provide a clear final answer to the user. ");
            prompt.push_str("If tools executed code, briefly describe what was done and include the final code snippet where appropriate.\n");

            let mut parameters = std::collections::HashMap::new();
            parameters.insert(
                "orchestration_mode".to_string(),
                "plan_and_execute".to_string(),
            );

            let generate_req = GenerateRequest { prompt, parameters };

            let mut buf = Vec::new();
            generate_req.encode(&mut buf).map_err(|e| {
                Status::internal(format!(
                    "Failed to encode GenerateRequest for execution: {}",
                    e
                ))
            })?;

            ProtoRequest {
                id: format!("{}-final", req_data.id),
                service: "llm-service".to_string(),
                method: "generate_text".to_string(),
                payload: buf,
                metadata: req_data.metadata.clone(),
            }
        } else {
            // Fallback to original execution behavior with the raw request
            req_data.clone()
        };

        // Create execution request
        let execution_route_request = RouteRequest {
            target_service: target_service.clone(),
            request: Some(execution_request),
        };

        // Call Data Router Service for execution
        let execution_response = router_client
            .route(tonic::Request::new(execution_route_request))
            .await;

        let execution_response = match execution_response {
            Ok(resp) => resp,
            Err(status) => {
                let err =
                    classify_status_error(OrchestrationStage::Execution, &target_service, &status);
                return self.log_critical_failure_and_build_response(err, &req_data, None).await;
            }
        };

        let execution_data = execution_response.into_inner();

        // Phase 5: Response Aggregation - Build AgiResponse
        let mut output_artifacts = Vec::new();
        let final_answer;
        let execution_plan_details;
        let routed_service = execution_data.routed_to.clone();

        // Record overall request execution trace
        let overall_trace_start = std::time::Instant::now();
        let overall_duration = overall_trace_start.elapsed().as_millis() as u64;
        if let Some(telemetrist_guard) = self.telemetrist.lock().await.as_ref() {
            let trace = ExecutionTrace {
                trace_id: uuid::Uuid::new_v4().to_string(),
                request_id: req_data.id.clone(),
                service: "orchestrator".to_string(),
                method: "PlanAndExecute".to_string(),
                duration_ms: overall_duration,
                success: true,
                error: None,
                metadata: {
                    let mut meta = std::collections::HashMap::new();
                    meta.insert("target_service".to_string(), target_service.clone());
                    meta.insert("plan_parsed".to_string(), parsed_plan.is_some().to_string());
                    meta.insert("tool_preference".to_string(), tool_preference.clone());
                    meta
                },
                timestamp: chrono::Utc::now(),
            };
            let telemetrist = Arc::clone(telemetrist_guard);
            tokio::spawn(async move {
                if let Err(e) = telemetrist.record_execution_trace(trace).await {
                    log::warn!("Failed to record overall execution trace: {}", e);
                }
            });
        }

        if let Some(exec_resp) = execution_data.response {
            log::info!("Execution complete. Response ID: {}", exec_resp.id);

            // Extract the final answer from the execution response
            final_answer = String::from_utf8_lossy(&exec_resp.payload).to_string();

            // Build comprehensive execution plan, including any tool results
            let mut plan_section = format!("Execution Plan:\n{}\n", plan_text);

            if !exec_ctx.tool_results.is_empty() {
                plan_section.push_str("\nTool Results:\n");
                for tr in &exec_ctx.tool_results {
                    plan_section.push_str("- ");
                    plan_section.push_str(tr);
                    plan_section.push('\n');
                }
            }

            execution_plan_details = format!(
                "{}\nStatus: {}\nRouted To: {}\nError: {}",
                plan_section, exec_resp.status_code, routed_service, exec_resp.error
            );

            // Log analyzer integration removed due to undefined ExecutionLog type in updated proto
            // Can be re-added once log_analyzer.proto is updated to match new schema
            log::info!("Execution complete without log analysis");
        } else {
            // Fallback response if execution didn't provide one
            log::warn!("Execution returned empty response, creating fallback");
            final_answer = format!(
                "Orchestrator completed PlanAndExecute for request: {}. Routed to: {}",
                req_data.id, execution_data.routed_to
            );
            execution_plan_details = format!(
                "Plan: {}\nRouted to: {}",
                plan_text, execution_data.routed_to
            );
        }

        // Create the unified AgiResponse
        let reply = AgiResponse {
            final_answer,
            execution_plan: execution_plan_details,
            routed_service,
            phoenix_session_id: req_data.id.clone(),
            output_artifact_urls: output_artifacts,
        };

        // Phase 5: Reflection - Asynchronously call ReflectionService to learn from this execution
        // This is non-blocking and won't delay the response
        if let Some(mut reflection_client) = self.get_reflection_client().await {
            let reflection_req = ReflectionRequest {
                request_id: req_data.id.clone(),
                action_description: format!("PlanAndExecute: {}", user_query),
                outcome: reply.final_answer.clone(),
                success: true, // AgiResponse always indicates success in structure
                context: std::collections::HashMap::new(),
            };

            // Spawn async task to avoid blocking the response
            tokio::spawn(async move {
                match reflection_client
                    .reflect_on_action(tonic::Request::new(reflection_req))
                    .await
                {
                    Ok(resp) => {
                        log::info!(
                            "Reflection complete: confidence={}",
                            resp.into_inner().confidence_score
                        );
                    }
                    Err(e) => {
                        log::debug!("Reflection call failed (non-critical): {}", e);
                    }
                }
            });
        }

        Ok(Response::new(reply))
    }

    async fn route(
        &self,
        request: Request<RouteRequest>,
    ) -> Result<Response<RouteResponse>, Status> {
        let req_data = request.into_inner();
        let request_id = req_data
            .request
            .as_ref()
            .map(|r| r.id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        log::info!(
            "Received Route request: request_id={}, target_service={}",
            request_id,
            req_data.target_service
        );

        // Check if this is a request for a specialty agent by capability
        let target = req_data.target_service.clone();
        if target.starts_with("capability:") {
            let capability = target.trim_start_matches("capability:").trim();
            log::info!("Looking for agent with capability: {}", capability);

            // Find an agent with this capability with error handling
            match self.find_agent_by_capability(capability).await {
                Ok(Some(agent)) => {
                    // Found a verified agent, route the request
                    log::info!(
                        "Routing request {} to agent {} at {}",
                        request_id,
                        agent.name,
                        agent.endpoint
                    );

                    // Actual routing logic would go here...

                    let reply = RouteResponse {
                        response: Some(ProtoResponse {
                            id: request_id.clone(),
                            status_code: 200,
                            payload: format!("Routed to agent: {}", agent.name).into_bytes(),
                            error: String::new(),
                            metadata: std::collections::HashMap::new(),
                        }),
                        routed_to: agent.name,
                    };

                    return Ok(Response::new(reply));
                }
                Ok(None) => {
                    // No verified agent available with this capability
                    log::warn!(
                        "No verified agent available with capability: {}",
                        capability
                    );
                    return Err(Status::unavailable(format!(
                        "No available agent with capability: {}",
                        capability
                    )));
                }
                Err(status) => {
                    // Registry error occurred
                    log::error!(
                        "Agent Registry error while looking for capability {}: {}",
                        capability,
                        status.message()
                    );

                    // Add request context to the error
                    return Err(Status::new(
                        status.code(),
                        format!(
                            "Agent lookup failed for request {}: {}",
                            request_id,
                            status.message()
                        ),
                    ));
                }
            }
        }

        #[cfg(test)]
        pub mod test_utils {
            use crate::agi_core::agent_registry_service_client::AgentRegistryServiceClient;
            use std::sync::Arc;
            use tokio::sync::Mutex;

            // Helper function to create a test orchestrator with a mock registry client
            pub async fn setup_test_orchestrator_with_registry<T>(
                mock_registry: T,
            ) -> crate::OrchestratorServer
            where
                T: Into<Option<AgentRegistryServiceClient<tonic::transport::Channel>>>,
            {
                let server = crate::OrchestratorServer::new();

                {
                    let mut registry_client = server.agent_registry_client.lock().await;
                    *registry_client = mock_registry.into();
                }

                server
            }
        }

        // Standard routing to a named service
        let reply = RouteResponse {
            response: Some(ProtoResponse {
                id: request_id,
                status_code: 200,
                payload: format!("Routed to: {}", req_data.target_service).into_bytes(),
                error: String::new(),
                metadata: std::collections::HashMap::new(),
            }),
            routed_to: req_data.target_service,
        };

        Ok(Response::new(reply))
    }
}

// Implement HealthService for OrchestratorServer
#[tonic::async_trait]
impl HealthService for OrchestratorServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;

        // Check Data Router dependency
        let mut dependencies = std::collections::HashMap::new();
        let data_router_status = {
            let guard = self.data_router_client.lock().await;
            if guard.is_some() {
                "SERVING"
            } else {
                "NOT_SERVING"
            }
        };
        dependencies.insert("data_router".to_string(), data_router_status.to_string());

        let reply = HealthResponse {
            healthy: data_router_status == "SERVING",
            service_name: "orchestrator-service".to_string(),
            uptime_seconds: uptime,
            status: if data_router_status == "SERVING" {
                "SERVING".to_string()
            } else {
                "DEGRADED".to_string()
            },
            dependencies,
        };

        Ok(Response::new(reply))
    }
}

// 5. Main function to start the gRPC server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Initialize start time
    let _ = *START_TIME;

    // Create orchestrator server instance
    let orchestrator_server = Arc::new(OrchestratorServer::new());

    // Initialize Data Router client
    let router_addr = config_rs::get_client_address("DATA_ROUTER", 50060, None);

    if let Err(e) = orchestrator_server
        .init_data_router_client(router_addr.clone())
        .await
    {
        log::warn!(
            "Data Router client init failed: {}. Health will report degraded.",
            e
        );
    } else {
        log::info!("Data Router client initialized: {}", router_addr);
    }

    // Initialize Reflection Service client (optional - continues if unavailable)
    let reflection_addr = config_rs::get_client_address("REFLECTION", 50065, None);

    if let Err(e) = orchestrator_server
        .init_reflection_client(reflection_addr.clone())
        .await
    {
        log::info!("Reflection Service not available (optional): {}", e);
    } else {
        log::info!("Reflection Service client initialized: {}", reflection_addr);
    }

    // Initialize Agent Registry Service client (optional - for task delegation)
    let registry_addr = config_rs::get_client_address("AGENT_REGISTRY", 50070, None);

    if let Err(e) = orchestrator_server
        .init_agent_registry_client(registry_addr.clone())
        .await
    {
        log::info!("Agent Registry Service not available (optional): {}", e);
    } else {
        log::info!(
            "Agent Registry Service client initialized: {}",
            registry_addr
        );
    }

    // Initialize Context Manager Service client
    let context_manager_addr = config_rs::get_client_address("CONTEXT_MANAGER", 50056, None);

    if let Err(e) = orchestrator_server
        .init_context_manager_client(context_manager_addr.clone())
        .await
    {
        log::warn!(
            "Context Manager client init failed: {}. Health will report degraded.",
            e
        );
    } else {
        log::info!("Context Manager client initialized: {}", context_manager_addr);
    }

    // Initialize Action Ledger (optional - continues if unavailable)
    if let Err(e) = orchestrator_server.init_action_ledger().await {
        log::warn!("Action Ledger initialization failed (optional): {}", e);
    }

    // Initialize Self-Improvement Engine (optional - continues if unavailable)
    if let Err(e) = orchestrator_server.init_self_improver().await {
        log::warn!("Self-Improvement Engine initialization failed (optional): {}", e);
    }

    // Initialize Context Manager Service client
    let context_manager_addr = config_rs::get_client_address("CONTEXT_MANAGER", 50056, None);

    if let Err(e) = orchestrator_server
        .init_context_manager_client(context_manager_addr.clone())
        .await
    {
        log::warn!(
            "Context Manager Service not available: {}. Will use raw prompts.",
            e
        );
    } else {
        log::info!(
            "Context Manager Service client initialized: {}",
            context_manager_addr
        );
    }

    // Get bind address from config
    let addr = config_rs::get_bind_address("ORCHESTRATOR", 50051);

    log::info!("OrchestratorService starting on {}", addr);
    println!("OrchestratorService listening on {}", addr);

    // Clone Arc for both services
    let orch_for_health = orchestrator_server.clone();

    Server::builder()
        .add_service(OrchestratorServiceServer::from_arc(orchestrator_server))
        .add_service(HealthServiceServer::from_arc(orch_for_health))
        .serve(addr)
        .await?;

    Ok(())
}
