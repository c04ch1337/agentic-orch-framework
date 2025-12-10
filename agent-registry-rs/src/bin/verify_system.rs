use tokio;
use tonic::Request;
use std::error::Error;

use agent_registry::{
    AgentRegistryService,
    AgentRegistryClient,
    ListAgentsRequest,
    GetAvailableCapabilitiesRequest,
};

use orchestrator::{
    OrchestratorService,
    OrchestratorClient,
    RequestData,
};

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Start all services
    println!("Starting services...");
    
    // Start Agent Registry
    let registry = AgentRegistryService::new();
    tokio::spawn(registry.serve(50063));

    // Start test agents
    let test_agents = vec![
        ("BLUE_TEAM", 50064, vec!["analyze_threat"]),
        ("RED_TEAM", 50065, vec!["simulate_attack"]),
        ("OFFLINE_AGENT", 50066, vec!["should_not_appear"])
    ];

    for (name, port, caps) in test_agents {
        let agent = TestAgent::new(name, caps);
        tokio::spawn(agent.serve(port));
    }

    // Start Orchestrator
    let orchestrator = OrchestratorService::new(50063);
    tokio::spawn(orchestrator.serve(50051));

    // 2. Run verification tests
    println!("Running verification tests...");
    
    // Test Registry health checks
    verify_registry_health_checks().await?;

    // Test Orchestrator routing
    verify_orchestrator_routing().await?;

    // Test capability aggregation
    verify_capability_aggregation().await?;

    println!("All tests passed!");
    Ok(())
}

async fn verify_registry_health_checks() -> Result<(), Box<dyn Error>> {
    let client = AgentRegistryClient::connect("http://localhost:50063").await?;
    
    // Should only list verified agents
    let agents = client.list_agents(ListAgentsRequest::default()).await?.into_inner().agents;
    assert_eq!(agents.len(), 2); // BLUE_TEAM and RED_TEAM, not OFFLINE_AGENT
    
    Ok(())
}

async fn verify_orchestrator_routing() -> Result<(), Box<dyn Error>> {
    let client = OrchestratorClient::connect("http://localhost:50051").await?;
    
    // Test routing to verified agent
    let response = client
        .process_request(Request::new(RequestData {
            capability: "analyze_threat".to_string(),
            // other fields...
        }))
        .await?;
    assert!(response.into_inner().success);
    
    // Test routing to unverified agent (should fail)
    let response = client
        .process_request(Request::new(RequestData {
            capability: "should_not_appear".to_string(),
            // other fields...
        }))
        .await;
    assert!(response.is_err());
    
    Ok(())
}

async fn verify_capability_aggregation() -> Result<(), Box<dyn Error>> {
    let client = AgentRegistryClient::connect("http://localhost:50063").await?;
    
    let capabilities = client
        .get_available_capabilities(GetAvailableCapabilitiesRequest::default())
        .await?
        .into_inner();
    
    // Should only include capabilities from verified agents
    assert!(capabilities.capabilities.contains(&"analyze_threat".to_string()));
    assert!(capabilities.capabilities.contains(&"simulate_attack".to_string()));
    assert!(!capabilities.capabilities.contains(&"should_not_appear".to_string()));
    
    Ok(())
}