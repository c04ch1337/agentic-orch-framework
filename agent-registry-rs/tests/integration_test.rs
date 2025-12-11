use std::time::Duration;
use tokio;
use tonic::{Request, Response};

use agent_registry::{
    AgentRegistryClient, AgentRegistryService, GetAvailableCapabilitiesRequest, ListAgentsRequest,
};

use orchestrator::{OrchestratorClient, OrchestratorService, RequestData};

struct TestAgent {
    name: String,
    capabilities: Vec<String>,
}

impl TestAgent {
    fn new(name: &str, capabilities: Vec<&str>) -> Self {
        Self {
            name: name.to_string(),
            capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
        }
    }

    async fn serve(&self, port: u16) {
        // Implementation of test agent service
        // This would implement the gRPC health service and agent capabilities
    }
}

#[tokio::test]
async fn test_complete_agent_verification_flow() {
    // 1. Start Agent Registry
    let registry = AgentRegistryService::new();
    let registry_port = 50063;
    tokio::spawn(registry.serve(registry_port));

    // 2. Start test agents with health checks
    let agent1 = TestAgent::new("BLUE_TEAM", vec!["analyze_threat"]);
    let agent2 = TestAgent::new("RED_TEAM", vec!["simulate_attack"]);

    tokio::spawn(agent1.serve(50064));
    tokio::spawn(agent2.serve(50065));

    // 3. Start Orchestrator
    let orchestrator = OrchestratorService::new(registry_port);
    tokio::spawn(orchestrator.serve(50051));

    // Allow services to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 4. Verify Registry health checks
    let registry_client =
        AgentRegistryClient::connect(format!("http://localhost:{}", registry_port))
            .await
            .unwrap();

    let agents = registry_client
        .list_agents(ListAgentsRequest::default())
        .await
        .unwrap()
        .into_inner()
        .agents;

    // Should only include verified agents
    assert_eq!(agents.len(), 2);
    assert!(agents.iter().all(|a| a.verified));

    // 5. Test Orchestrator routing
    let orchestrator_client = OrchestratorClient::connect("http://localhost:50051")
        .await
        .unwrap();

    let response = orchestrator_client
        .process_request(Request::new(RequestData {
            capability: "analyze_threat".to_string(),
            // other fields...
        }))
        .await
        .unwrap();

    assert!(response.into_inner().success);

    // 6. Verify capability aggregation
    let capabilities = registry_client
        .get_available_capabilities(GetAvailableCapabilitiesRequest::default())
        .await
        .unwrap()
        .into_inner();

    assert!(capabilities
        .capabilities
        .contains(&"analyze_threat".to_string()));
    assert!(capabilities
        .capabilities
        .contains(&"simulate_attack".to_string()));
}
