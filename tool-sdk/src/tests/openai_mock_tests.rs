//! Mock tests for OpenAI service
//!
//! These tests use WireMock to simulate the OpenAI API and verify that the
//! OpenAI client correctly interacts with the API.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;
    
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header, body_json};
    use serde_json::json;
    
    use crate::services::openai::{OpenAIClient, ChatCompletionRequest, ChatMessage, OpenAIClientBuilder};
    use crate::services::openai::{EmbeddingRequest, EmbeddingInput};
    use crate::core::{ServiceClient, AuthenticatedClient};
    use crate::error::{ServiceError};
    
    /// Sets up a mock OpenAI server with base configuration
    async fn setup_mock_server() -> MockServer {
        MockServer::start().await
    }
    
    /// Creates a test OpenAI client configured to use the mock server
    fn create_test_client(mock_server: &MockServer) -> OpenAIClient {
        OpenAIClientBuilder::new()
            .api_key("mock_api_key_for_testing")
            .base_url(mock_server.uri())
            .timeout(5)
            .build()
            .expect("Failed to build OpenAI client")
    }
    
    #[tokio::test]
    async fn test_chat_completion() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Expected request
        let expected_request = ChatCompletionRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello, world!".to_string(),
                name: None,
            }],
            temperature: Some(0.7),
            top_p: None,
            n: None,
            stream: None,
            max_tokens: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
        };
        
        // Expected response from OpenAI
        let mock_response = json!({
            "id": "chatcmpl-mock123",
            "object": "chat.completion",
            "created": 1677858242,
            "model": "gpt-3.5-turbo",
            "usage": {
                "prompt_tokens": 13,
                "completion_tokens": 7,
                "total_tokens": 20
            },
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help you today?"
                    },
                    "finish_reason": "stop",
                    "index": 0
                }
            ]
        });
        
        // Setup the mock
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Authorization", "Bearer mock_api_key_for_testing"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(&expected_request))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Make the request
        let response = client.chat_completion(expected_request).await.unwrap();
        
        // Verify response
        assert_eq!(response.model, "gpt-3.5-turbo");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.role, "assistant");
        assert_eq!(response.choices[0].message.content, Some("Hello! How can I help you today?".to_string()));
        assert_eq!(response.choices[0].finish_reason, Some("stop".to_string()));
    }
    
    #[tokio::test]
    async fn test_simple_completion() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Expected response from OpenAI
        let mock_response = json!({
            "id": "chatcmpl-simple123",
            "object": "chat.completion",
            "created": 1677858242,
            "model": "gpt-3.5-turbo",
            "usage": {
                "prompt_tokens": 13,
                "completion_tokens": 7,
                "total_tokens": 20
            },
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "This is a simple response."
                    },
                    "finish_reason": "stop",
                    "index": 0
                }
            ]
        });
        
        // Setup the mock - since we're using simple_completion, 
        // we can just match on the endpoint and not the exact request body
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Authorization", "Bearer mock_api_key_for_testing"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Make the request
        let response = client.simple_completion("What's the weather?", None).await.unwrap();
        
        // Verify response
        assert_eq!(response, "This is a simple response.");
    }
    
    #[tokio::test]
    async fn test_embeddings() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Expected embedding request
        let embed_request = EmbeddingRequest {
            model: "text-embedding-ada-002".to_string(),
            input: EmbeddingInput::String("This is a test sentence for embedding.".to_string()),
            encoding_format: None,
            user: None,
        };
        
        // Mock response with embedding
        let mock_response = json!({
            "object": "list",
            "data": [
                {
                    "object": "embedding",
                    "embedding": [0.1, 0.2, 0.3, 0.4, 0.5],
                    "index": 0
                }
            ],
            "model": "text-embedding-ada-002",
            "usage": {
                "prompt_tokens": 8,
                "total_tokens": 8
            }
        });
        
        // Setup the mock
        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .and(header("Authorization", "Bearer mock_api_key_for_testing"))
            .and(body_json(&embed_request))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Make the request
        let response = client.embeddings(embed_request).await.unwrap();
        
        // Verify response
        assert_eq!(response.model, "text-embedding-ada-002");
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].embedding, vec![0.1, 0.2, 0.3, 0.4, 0.5]);
    }
    
    #[tokio::test]
    async fn test_embed_text_helper() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response with embedding
        let mock_response = json!({
            "object": "list",
            "data": [
                {
                    "object": "embedding",
                    "embedding": [0.1, 0.2, 0.3, 0.4, 0.5],
                    "index": 0
                }
            ],
            "model": "text-embedding-ada-002",
            "usage": {
                "prompt_tokens": 8,
                "total_tokens": 8
            }
        });
        
        // Setup the mock - use a less strict matcher since we're using the helper
        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .and(header("Authorization", "Bearer mock_api_key_for_testing"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Make the request
        let embedding = client.embed_text("This is a test", None).await.unwrap();
        
        // Verify response
        assert_eq!(embedding, vec![0.1, 0.2, 0.3, 0.4, 0.5]);
    }
    
    #[tokio::test]
    async fn test_list_models() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response with models
        let mock_response = json!({
            "object": "list",
            "data": [
                {
                    "id": "gpt-3.5-turbo",
                    "object": "model",
                    "created": 1677610602,
                    "owned_by": "openai"
                },
                {
                    "id": "gpt-4",
                    "object": "model",
                    "created": 1677649963,
                    "owned_by": "openai"
                }
            ]
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/models"))
            .and(header("Authorization", "Bearer mock_api_key_for_testing"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Make the request
        let response = client.list_models().await.unwrap();
        
        // Verify response
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].id, "gpt-3.5-turbo");
        assert_eq!(response.data[1].id, "gpt-4");
    }
    
    #[tokio::test]
    async fn test_get_model() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response with model
        let mock_response = json!({
            "id": "gpt-3.5-turbo",
            "object": "model",
            "created": 1677610602,
            "owned_by": "openai"
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/models/gpt-3.5-turbo"))
            .and(header("Authorization", "Bearer mock_api_key_for_testing"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Make the request
        let model = client.get_model("gpt-3.5-turbo").await.unwrap();
        
        // Verify response
        assert_eq!(model.id, "gpt-3.5-turbo");
        assert_eq!(model.owned_by, "openai");
    }
    
    #[tokio::test]
    async fn test_authentication_error() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for authentication error
        let mock_response = json!({
            "error": {
                "message": "Incorrect API key provided",
                "type": "invalid_request_error",
                "param": null,
                "code": "invalid_api_key"
            }
        });
        
        // Setup the mock
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client with wrong API key
        let client = OpenAIClientBuilder::new()
            .api_key("wrong_key")
            .base_url(mock_server.uri())
            .build()
            .expect("Failed to build OpenAI client");
        
        // Make the request
        let request = ChatCompletionRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
                name: None,
            }],
            temperature: Some(0.7),
            ..Default::default()
        };
        
        let error = client.chat_completion(request).await.unwrap_err();
        
        // Verify error
        match error {
            ServiceError::Authentication(msg) => {
                assert!(msg.contains("Incorrect API key"));
            }
            _ => panic!("Expected Authentication error, got: {:?}", error),
        }
    }
    
    #[tokio::test]
    async fn test_rate_limit_error() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for rate limit error
        let mock_response = json!({
            "error": {
                "message": "Rate limit exceeded on requests",
                "type": "rate_limit_error",
                "param": null,
                "code": "rate_limit_exceeded"
            }
        });
        
        // Setup the mock with rate limit headers
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_json(&mock_response)
                    .insert_header("retry-after", "30")
                    .insert_header("x-ratelimit-reset-requests", "30")
                    .insert_header("x-ratelimit-remaining-requests", "0")
                    .insert_header("x-ratelimit-limit-requests", "50")
            )
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Make the request
        let request = ChatCompletionRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
                name: None,
            }],
            temperature: Some(0.7),
            ..Default::default()
        };
        
        let error = client.chat_completion(request).await.unwrap_err();
        
        // Verify error
        match error {
            ServiceError::RateLimit(msg) => {
                assert!(msg.contains("Rate limit exceeded"));
            }
            _ => panic!("Expected RateLimit error, got: {:?}", error),
        }
        
        // Check if rate limit status was updated
        let rate_limit = client.rate_limit_status();
        assert!(rate_limit.is_some());
    }
    
    #[tokio::test]
    async fn test_validation_error() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for validation error
        let mock_response = json!({
            "error": {
                "message": "You must provide a model parameter",
                "type": "invalid_request_error",
                "param": "model",
                "code": null
            }
        });
        
        // Setup the mock
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(400)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Make the request with empty model (will be modified by mock)
        let request = ChatCompletionRequest {
            model: "".to_string(), // Empty model
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
                name: None,
            }],
            temperature: Some(0.7),
            ..Default::default()
        };
        
        let error = client.chat_completion(request).await.unwrap_err();
        
        // Verify error
        match error {
            ServiceError::Validation(msg) => {
                assert!(msg.contains("You must provide a model parameter"));
            }
            _ => panic!("Expected Validation error, got: {:?}", error),
        }
    }
    
    #[tokio::test]
    async fn test_context_length_exceeded() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for context length error
        let mock_response = json!({
            "error": {
                "message": "This model's maximum context length is 4097 tokens",
                "type": "invalid_request_error",
                "param": "messages",
                "code": "context_length_exceeded"
            }
        });
        
        // Setup the mock
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(400)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Create a very long message (that would exceed token limit)
        let long_message = "This is a very long message. ".repeat(500);
        let request = ChatCompletionRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: long_message,
                name: None,
            }],
            ..Default::default()
        };
        
        let error = client.chat_completion(request).await.unwrap_err();
        
        // Verify error
        match error {
            ServiceError::Validation(msg) => {
                assert!(msg.contains("maximum context length"));
            }
            _ => panic!("Expected Validation error, got: {:?}", error),
        }
    }
    
    #[tokio::test]
    async fn test_server_error() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for server error
        let mock_response = json!({
            "error": {
                "message": "The server is experiencing high load",
                "type": "server_error",
                "param": null,
                "code": null
            }
        });
        
        // Setup the mock
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(500)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        let request = ChatCompletionRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
                name: None,
            }],
            ..Default::default()
        };
        
        let error = client.chat_completion(request).await.unwrap_err();
        
        // Verify error
        match error {
            ServiceError::Service(msg) => {
                assert!(msg.contains("server is experiencing high load"));
            }
            _ => panic!("Expected Service error, got: {:?}", error),
        }
        
        // Verify error is retryable
        assert!(error.is_retryable());
    }
    
    #[tokio::test]
    async fn test_client_with_organization_id() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Expected response from OpenAI
        let mock_response = json!({
            "id": "chatcmpl-mock123",
            "object": "chat.completion",
            "created": 1677858242,
            "model": "gpt-3.5-turbo",
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help you today?"
                    },
                    "finish_reason": "stop",
                    "index": 0
                }
            ]
        });
        
        // Setup the mock that expects the organization header
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Authorization", "Bearer mock_api_key_for_testing"))
            .and(header("OpenAI-Organization", "org-test-123")) // Check for org ID header
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client with organization ID
        let client = OpenAIClientBuilder::new()
            .api_key("mock_api_key_for_testing")
            .base_url(mock_server.uri())
            .org_id("org-test-123")
            .build()
            .expect("Failed to build OpenAI client");
        
        // Make the request
        let request = ChatCompletionRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
                name: None,
            }],
            ..Default::default()
        };
        
        // This should succeed since we're sending the expected org ID header
        let response = client.chat_completion(request).await.unwrap();
        
        assert_eq!(response.model, "gpt-3.5-turbo");
    }
    
    #[tokio::test]
    async fn test_health_check() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for models list (used for health check)
        let mock_response = json!({
            "object": "list",
            "data": [
                {
                    "id": "gpt-3.5-turbo",
                    "object": "model",
                    "created": 1677610602,
                    "owned_by": "openai"
                }
            ]
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Perform health check
        let is_healthy = client.health_check().await.unwrap();
        
        // Verify health check response
        assert!(is_healthy);
        
        // Setup mock for unhealthy response
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(500)
                .set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;
        
        // Perform health check again
        let is_healthy = client.health_check().await.unwrap();
        
        // Should return false for unhealthy service but not error
        assert!(!is_healthy);
    }
}