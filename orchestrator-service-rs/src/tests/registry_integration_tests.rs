use super::*;
use mockall::{mock, predicate};
use tonic::{Request, Response};

// Mock for the AgentRegistryClient
mock! {
    pub AgentRegistryClient {}

    #[async_trait]
    impl AgentRegistryServiceClient<tonic::transport::Channel> for AgentRegistryClient {
        async fn get_agent(
            &mut self,
            request: Request<GetAgentRequest>
        ) -> Result<Response<GetAgentResponse>, Status>;

        // Add other methods from AgentRegistryServiceClient as needed
    }
}

#[tokio::test]
async fn test_find_agent_by_capability_success() {
    // Set up a mock Agent Registry client
    let mut mock_registry = MockAgentRegistryClient::new();

    // Define the expected request and response
    mock_registry
        .expect_get_agent()
        .with(predicate::always())
        .times(1)
        .returning(|request| {
            let req = request.into_inner();
            assert_eq!(req.capability, "test_capability");

            let agent_info = AgentInfo {
                agent_id: "test-id".to_string(),
                name: "test-agent".to_string(),
                port: 8080,
                role: "tester".to_string(),
                capabilities: vec!["test_capability".to_string()],
                status: "ONLINE".to_string(),
                metadata: std::collections::HashMap::new(),
            };

            Ok(Response::new(GetAgentResponse {
                found: true,
                agent: Some(agent_info),
            }))
        });

    // Create an OrchestratorServer with our mock registry
    let server = OrchestratorServer::new();

    // Insert the mock registry
    {
        let mut registry_client = server.agent_registry_client.lock().await;
        *registry_client = Some(mock_registry);
    }

    // Call the method we're testing
    let result = server
        .find_agent_by_capability("test_capability")
        .await
        .unwrap();

    // Verify the result
    assert!(result.is_some());
    let agent = result.unwrap();
    assert_eq!(agent.name, "test-agent");
    assert_eq!(agent.endpoint, "http://localhost:8080");
}

#[tokio::test]
async fn test_find_agent_by_capability_not_found() {
    // Set up a mock Agent Registry client
    let mut mock_registry = MockAgentRegistryClient::new();

    // Define the expected request and response for agent not found
    mock_registry
        .expect_get_agent()
        .with(predicate::always())
        .times(1)
        .returning(|_| {
            Ok(Response::new(GetAgentResponse {
                found: false,
                agent: None,
            }))
        });

    // Create an OrchestratorServer with our mock registry
    let server = OrchestratorServer::new();

    // Insert the mock registry
    {
        let mut registry_client = server.agent_registry_client.lock().await;
        *registry_client = Some(mock_registry);
    }

    // Call the method we're testing
    let result = server
        .find_agent_by_capability("nonexistent_capability")
        .await
        .unwrap();

    // Verify the result is None
    assert!(result.is_none());
}

#[tokio::test]
async fn test_find_agent_by_capability_registry_error() {
    // Set up a mock Agent Registry client
    let mut mock_registry = MockAgentRegistryClient::new();

    // Define the expected request and response for registry error
    mock_registry
        .expect_get_agent()
        .with(predicate::always())
        .times(1)
        .returning(|_| Err(Status::unavailable("Agent Registry service unavailable")));

    // Create an OrchestratorServer with our mock registry
    let server = OrchestratorServer::new();

    // Insert the mock registry
    {
        let mut registry_client = server.agent_registry_client.lock().await;
        *registry_client = Some(mock_registry);
    }

    // Call the method we're testing
    let result = server.find_agent_by_capability("test_capability").await;

    // Verify we get an error with unavailable status
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err.code(), tonic::Code::Unavailable);
}

#[tokio::test]
async fn test_route_with_capability() {
    // Set up a mock Agent Registry client
    let mut mock_registry = MockAgentRegistryClient::new();

    // Define the expected request and response
    mock_registry
        .expect_get_agent()
        .with(predicate::always())
        .times(1)
        .returning(|_| {
            let agent_info = AgentInfo {
                agent_id: "test-id".to_string(),
                name: "test-agent".to_string(),
                port: 8080,
                role: "tester".to_string(),
                capabilities: vec!["test_capability".to_string()],
                status: "ONLINE".to_string(),
                metadata: std::collections::HashMap::new(),
            };

            Ok(Response::new(GetAgentResponse {
                found: true,
                agent: Some(agent_info),
            }))
        });

    // Create an OrchestratorServer with our mock registry
    let server = OrchestratorServer::new();

    // Insert the mock registry
    {
        let mut registry_client = server.agent_registry_client.lock().await;
        *registry_client = Some(mock_registry);
    }

    // Create a route request with a capability
    let request = Request::new(RouteRequest {
        target_service: "capability:test_capability".to_string(),
        request: Some(ProtoRequest {
            id: "test-request".to_string(),
            service: "test-service".to_string(),
            method: "test-method".to_string(),
            payload: vec![],
            metadata: std::collections::HashMap::new(),
        }),
    });

    // Call the route method
    let result = server.route(request).await;

    // Verify the result
    assert!(result.is_ok());
    let response = result.unwrap().into_inner();
    assert_eq!(response.routed_to, "test-agent");
}
