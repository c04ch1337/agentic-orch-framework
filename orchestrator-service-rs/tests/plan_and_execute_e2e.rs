use std::collections::HashMap;
use std::time::Duration;

use tonic::{
    transport::Channel,
    Code,
};

/// gRPC E2E tests for OrchestratorService::PlanAndExecute.
///
/// These tests exercise the real orchestration path exposed only over gRPC:
///
/// OrchestratorService.PlanAndExecute -> Data Router ->
///   LLM Service / Soul KB / Safety Service / (optional) Context Manager.
///
/// Important architectural limitations documented here and in the assertions:
///
/// 1. There is currently *no* HTTP path from /api/v1/execute to
///    PlanAndExecute; the API Gateway always calls ProcessRequest with
///    service="orchestrator".
/// 2. Orchestrator does not instantiate or call ToolsService at all, so
///    no E2E path exists today for "Orchestrator -> Data Router ->
///    ToolsService -> Orchestrator".
/// 3. Failures in downstream services (LLM, Soul KB, Safety, etc.) are
///    surfaced as gRPC Status::internal errors; they are *not* mapped to
///    graceful AgiResponse payloads with friendly final_answer text.
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    orchestrator_service_client::OrchestratorServiceClient,
    Request as ProtoRequest,
};

/// Create a gRPC client to the running orchestrator-service instance.
///
/// By default this connects to http://127.0.0.1:50051, which matches the
/// orchestrator bind address from config_rs::get_bind_address("ORCHESTRATOR", 50051).
///
/// The target can be overridden for tests by setting ORCHESTRATOR_E2E_ADDR,
/// e.g.:
///   ORCHESTRATOR_E2E_ADDR=http://orchestrator:50051 cargo test -p orchestrator-service-rs --test plan_and_execute_e2e
async fn create_orchestrator_client(
) -> Result<OrchestratorServiceClient<Channel>, Box<dyn std::error::Error>> {
    let addr = std::env::var("ORCHESTRATOR_E2E_ADDR")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());

    let channel = Channel::from_shared(addr.clone())?
        .connect_timeout(Duration::from_secs(5))
        .connect()
        .await?;

    Ok(OrchestratorServiceClient::new(channel))
}

#[tokio::test]
async fn plan_and_execute_e2e_success() -> Result<(), Box<dyn std::error::Error>> {
    // This test exercises the successful orchestration path.
    //
    // It is intentionally PlanAndExecute-only over gRPC; the HTTP
    // /api/v1/execute route now conditionally calls PlanAndExecute based on
    // method/metadata, but that wiring is validated in the HTTP E2E tests.
    let mut client = create_orchestrator_client().await?;

    let user_query = "What is the primary function of the Data Router service, and then write a simple Python function to log a message.";

    let request = ProtoRequest {
        id: "final-e2e-plan-001".to_string(),
        service: "".to_string(),
        // method is informational only; orchestrator uses the payload as raw UTF-8.
        method: "plan_and_execute".to_string(),
        payload: user_query.as_bytes().to_vec(),
        metadata: HashMap::new(),
    };

    let response = client.plan_and_execute(request).await?;
    let agi = response.into_inner();

    // Basic invariants
    assert_eq!(agi.phoenix_session_id, "final-e2e-plan-001", "phoenix_session_id should echo the Request.id");
    assert_eq!(agi.routed_service, "llm-service", "PlanAndExecute should ultimately route to llm-service for this query");

    // The final answer should be non-empty and plausibly answer both parts of the query.
    assert!(
        !agi.final_answer.trim().is_empty(),
        "final_answer must be non-empty"
    );

    let final_lower = agi.final_answer.to_lowercase();
    assert!(
        final_lower.contains("data router"),
        "final_answer should mention the Data Router service; got: {}",
        agi.final_answer
    );
    assert!(
        agi.final_answer.contains("def "),
        "final_answer should contain a Python function definition (look for 'def '); got: {}",
        agi.final_answer
    );

    // Execution plan should contain the structured plan header and routing info
    assert!(
        agi.execution_plan.contains("Execution Plan") || agi.execution_plan.contains("Plan:"),
        "execution_plan should contain a recognizable plan header; got: {}",
        agi.execution_plan
    );
    assert!(
        agi.execution_plan.contains("Status:"),
        "execution_plan should contain 'Status:' as constructed in plan_and_execute; got: {}",
        agi.execution_plan
    );
    assert!(
        agi.execution_plan.contains("Routed To:"),
        "execution_plan should contain 'Routed To:' to document the final target service; got: {}",
        agi.execution_plan
    );

    Ok(())
}

#[tokio::test]
async fn plan_and_execute_e2e_failure_returns_graceful_agi_response(
) -> Result<(), Box<dyn std::error::Error>> {
    // This negative test documents the updated failure behavior of PlanAndExecute.
    //
    // To truly exercise a downstream failure, you must intentionally
    // misconfigure or stop a dependency (for example, bring down the LLM
    // service, Tools service, or point the orchestrator's DATA_ROUTER address
    // at an unreachable endpoint) *before* running this test.
    //
    // To avoid making the default test run depend on a broken environment,
    // this test is gated behind the ORCHESTRATOR_E2E_FAILURE_TEST=1
    // environment variable. When unset, the test becomes a no-op and passes.
    if std::env::var("ORCHESTRATOR_E2E_FAILURE_TEST") != Ok("1".to_string()) {
        eprintln!(
            "Skipping failure-mode PlanAndExecute E2E test; set ORCHESTRATOR_E2E_FAILURE_TEST=1 to enable."
        );
        return Ok(());
    }

    let mut client = create_orchestrator_client().await?;

    let user_query =
        "Trigger a downstream failure in Data Router, Tools Service, or one of their targets.";

    let request = ProtoRequest {
        id: "final-e2e-plan-failure-001".to_string(),
        service: "".to_string(),
        method: "plan_and_execute_failure".to_string(),
        payload: user_query.as_bytes().to_vec(),
        metadata: HashMap::new(),
    };

    let result = client.plan_and_execute(request).await;

    // In the updated implementation, downstream failures are surfaced as a
    // graceful AgiResponse with a user-friendly final_answer and an
    // execution_plan that documents the failing stage and service.
    let response = match result {
        Ok(resp) => resp,
        Err(status) => {
            panic!(
                "Expected PlanAndExecute to return a graceful AgiResponse on downstream failure, \
but it returned a gRPC error instead: code={:?}, message={}",
                status.code(),
                status.message()
            );
        }
    };

    let agi = response.into_inner();

    assert!(
        !agi.final_answer.trim().is_empty(),
        "final_answer must be non-empty even in failure cases"
    );

    assert!(
        agi.execution_plan.contains("FAILED at stage")
            || agi.execution_plan.contains("failed at stage")
            || agi.execution_plan.to_lowercase().contains("failed"),
        "execution_plan should mention the failing orchestration stage and/or service; got: {}",
        agi.execution_plan
    );

    assert!(
        !agi.routed_service.trim().is_empty(),
        "routed_service should reflect the failing dependency (e.g., llm-service, tools-service)"
    );

    assert_eq!(
        agi.phoenix_session_id, "final-e2e-plan-failure-001",
        "phoenix_session_id should echo the Request.id even in failure cases"
    );

    Ok(())
}