//! Mock tests for SerpAPI service
//!
//! These tests use WireMock to simulate the SerpAPI and verify that the
//! SerpAPI client correctly interacts with the API.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, query_param};
    use serde_json::json;
    
    use crate::services::serpapi::{SerpAPIClient, GoogleSearchParams, BingSearchParams, SerpAPIClientBuilder};
    use crate::core::{ServiceClient, AuthenticatedClient};
    use crate::error::{ServiceError};
    
    /// Sets up a mock SerpAPI server
    async fn setup_mock_server() -> MockServer {
        MockServer::start().await
    }
    
    /// Creates a test SerpAPI client configured to use the mock server
    fn create_test_client(mock_server: &MockServer) -> SerpAPIClient {
        SerpAPIClientBuilder::new()
            .api_key("mock_serp_api_key")
            .base_url(mock_server.uri())
            .timeout(5)
            .build()
            .expect("Failed to build SerpAPI client")
    }
    
    #[tokio::test]
    async fn test_google_search() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Create search parameters
        let params = GoogleSearchParams {
            q: "rust programming language".to_string(),
            num: Some(10),
            location: Some("United States".to_string()),
            ..Default::default()
        };
        
        // Mock response for Google search
        let mock_response = json!({
            "search_metadata": {
                "id": "mock_search_id",
                "status": "Success",
                "json_endpoint": "https://serpapi.com/searches/mock_search_id/json",
                "created_at": "2023-01-01 12:00:00 UTC",
                "processed_at": "2023-01-01 12:00:01 UTC",
                "google_url": "https://www.google.com/search?q=rust+programming+language&num=10",
                "raw_html_file": "https://serpapi.com/searches/mock_search_id/raw_html",
                "total_time_taken": 1.85
            },
            "search_parameters": {
                "engine": "google",
                "q": "rust programming language",
                "num": 10,
                "location": "United States"
            },
            "organic_results": [
                {
                    "position": 1,
                    "title": "The Rust Programming Language",
                    "link": "https://www.rust-lang.org/",
                    "snippet": "Rust is a multi-paradigm, high-level, general-purpose programming language ...",
                    "source": "rust-lang.org"
                },
                {
                    "position": 2,
                    "title": "Rust (programming language) - Wikipedia",
                    "link": "https://en.wikipedia.org/wiki/Rust_(programming_language)",
                    "snippet": "Rust is a multi-paradigm, general-purpose programming language...",
                    "source": "wikipedia.org"
                }
            ]
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/search/google"))
            .and(query_param("api_key", "mock_serp_api_key"))
            .and(query_param("q", "rust programming language"))
            .and(query_param("num", "10"))
            .and(query_param("location", "United States"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Execute search request
        let response = client.google_search(params).await.unwrap();
        
        // Verify response
        assert_eq!(response.search_metadata.status, "Success");
        
        // Verify search parameters from response
        assert_eq!(response.search_parameters.engine, "google");
        assert_eq!(response.search_parameters.q, "rust programming language");
        
        // Verify organic results
        assert_eq!(response.organic_results.len(), 2);
        assert_eq!(response.organic_results[0].position, 1);
        assert_eq!(response.organic_results[0].title, "The Rust Programming Language");
        assert_eq!(response.organic_results[0].source, "rust-lang.org");
    }
    
    #[tokio::test]
    async fn test_bing_search() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Create search parameters
        let params = BingSearchParams {
            q: "rust programming language".to_string(),
            count: Some(5),
            ..Default::default()
        };
        
        // Mock response for Bing search
        let mock_response = json!({
            "search_metadata": {
                "id": "mock_search_id_bing",
                "status": "Success",
                "json_endpoint": "https://serpapi.com/searches/mock_search_id_bing/json",
                "created_at": "2023-01-01 12:00:00 UTC",
                "processed_at": "2023-01-01 12:00:01 UTC",
                "bing_url": "https://www.bing.com/search?q=rust+programming+language&count=5",
                "raw_html_file": "https://serpapi.com/searches/mock_search_id_bing/raw_html",
                "total_time_taken": 1.62
            },
            "search_parameters": {
                "engine": "bing",
                "q": "rust programming language",
                "count": 5
            },
            "organic_results": [
                {
                    "position": 1,
                    "title": "Rust Programming Language",
                    "link": "https://www.rust-lang.org/",
                    "snippet": "Rust is a programming language designed for performance and safety...",
                    "about_this_result": {
                        "source": "www.rust-lang.org",
                        "description": "Official website"
                    }
                }
            ]
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/search/bing"))
            .and(query_param("api_key", "mock_serp_api_key"))
            .and(query_param("q", "rust programming language"))
            .and(query_param("count", "5"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Execute search request
        let response = client.bing_search(params).await.unwrap();
        
        // Verify response
        assert_eq!(response.search_metadata.status, "Success");
        
        // Verify search parameters from response
        assert_eq!(response.search_parameters.engine, "bing");
        assert_eq!(response.search_parameters.q, "rust programming language");
        
        // Verify organic results
        assert_eq!(response.organic_results.len(), 1);
        assert_eq!(response.organic_results[0].position, 1);
        assert_eq!(response.organic_results[0].title, "Rust Programming Language");
    }
    
    #[tokio::test]
    async fn test_simple_search() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for simple search
        let mock_response = json!({
            "search_metadata": {
                "id": "mock_simple_search_id",
                "status": "Success"
            },
            "search_parameters": {
                "engine": "google",
                "q": "simple query",
                "num": 10
            },
            "organic_results": [
                {
                    "position": 1,
                    "title": "Simple Result",
                    "link": "https://example.com/result",
                    "snippet": "This is a simple search result"
                }
            ]
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/search/google"))
            .and(query_param("api_key", "mock_serp_api_key"))
            .and(query_param("q", "simple query"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Execute simple search
        let response = client.search("simple query").await.unwrap();
        
        // Verify response
        assert_eq!(response.search_metadata.status, "Success");
        assert_eq!(response.search_parameters.q, "simple query");
        assert_eq!(response.organic_results.len(), 1);
        assert_eq!(response.organic_results[0].title, "Simple Result");
    }
    
    #[tokio::test]
    async fn test_search_with_custom_engine() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for custom engine search
        let mock_response = json!({
            "search_metadata": {
                "id": "mock_custom_search_id",
                "status": "Success"
            },
            "search_parameters": {
                "engine": "duckduckgo",
                "q": "custom engine query"
            },
            "organic_results": [
                {
                    "position": 1,
                    "title": "Custom Engine Result",
                    "link": "https://example.com/custom-result"
                }
            ]
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/search/duckduckgo"))
            .and(query_param("api_key", "mock_serp_api_key"))
            .and(query_param("q", "custom engine query"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Set up parameters for custom engine
        let mut params = HashMap::new();
        params.insert("q".to_string(), "custom engine query".to_string());
        
        // Execute search with custom engine
        let response = client.search_with_engine("duckduckgo", params).await.unwrap();
        
        // Verify response
        assert_eq!(response.search_metadata.status, "Success");
        assert_eq!(response.search_parameters.engine, "duckduckgo");
        assert_eq!(response.search_parameters.q, "custom engine query");
    }
    
    #[tokio::test]
    async fn test_get_account() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for account info
        let mock_response = json!({
            "account_id": "mock_account_id",
            "api_key": "mock_serp_api_key",
            "account_email": "test@example.com",
            "plan_name": "Basic",
            "plan_id": "basic_plan",
            "plan_searches_per_month": 100,
            "plan_searches_left": 87,
            "plan_expires": "2023-12-31"
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/account"))
            .and(query_param("api_key", "mock_serp_api_key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Get account info
        let account_info = client.get_account().await.unwrap();
        
        // Verify response
        assert_eq!(account_info.account_id, "mock_account_id");
        assert_eq!(account_info.account_email, "test@example.com");
        assert_eq!(account_info.plan_name, "Basic");
        assert_eq!(account_info.plan_searches_per_month, 100);
        assert_eq!(account_info.plan_searches_left, 87);
    }
    
    #[tokio::test]
    async fn test_get_search_archive() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for search archive
        let mock_response = json!({
            "searches": [
                {
                    "id": "mock_search_1",
                    "search_parameters": {
                        "engine": "google",
                        "q": "first query"
                    },
                    "created_at": "2023-01-01T12:00:00Z"
                },
                {
                    "id": "mock_search_2",
                    "search_parameters": {
                        "engine": "bing",
                        "q": "second query"
                    },
                    "created_at": "2023-01-02T12:00:00Z"
                }
            ]
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/searches"))
            .and(query_param("api_key", "mock_serp_api_key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Get search archive
        let archive = client.get_search_archive().await.unwrap();
        
        // Verify response
        assert_eq!(archive.searches.len(), 2);
        assert_eq!(archive.searches[0].id, "mock_search_1");
        assert_eq!(archive.searches[0].search_parameters.engine, "google");
        assert_eq!(archive.searches[1].id, "mock_search_2");
        assert_eq!(archive.searches[1].search_parameters.engine, "bing");
    }
    
    #[tokio::test]
    async fn test_authentication_error() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for authentication error
        let mock_response = json!({
            "error": "Invalid API key"
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/search/google"))
            .and(query_param("api_key", "invalid_key"))
            .respond_with(ResponseTemplate::new(401)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client with invalid key
        let client = SerpAPIClientBuilder::new()
            .api_key("invalid_key")
            .base_url(mock_server.uri())
            .build()
            .expect("Failed to build SerpAPI client");
        
        // Execute search with invalid key
        let params = GoogleSearchParams {
            q: "test query".to_string(),
            ..Default::default()
        };
        
        let error = client.google_search(params).await.unwrap_err();
        
        // Verify error
        match error {
            ServiceError::Authentication(msg) => {
                assert!(msg.contains("Invalid API key"));
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
            "error": "You have exceeded the maximum number of searches per month"
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/search/google"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_json(&mock_response)
                    .insert_header("retry-after", "60")
            )
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Execute search
        let params = GoogleSearchParams {
            q: "rate limit test".to_string(),
            ..Default::default()
        };
        
        let error = client.google_search(params).await.unwrap_err();
        
        // Verify error
        match error {
            ServiceError::RateLimit(msg) => {
                assert!(msg.contains("exceeded the maximum"));
            }
            _ => panic!("Expected RateLimit error, got: {:?}", error),
        }
        
        // Verify error is retryable
        assert!(error.is_retryable());
    }
    
    #[tokio::test]
    async fn test_validation_error() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for validation error
        let mock_response = json!({
            "error": "Parameter 'q' is required"
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/search/google"))
            .and(query_param("api_key", "mock_serp_api_key"))
            // Missing "q" parameter
            .respond_with(ResponseTemplate::new(400)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Execute search with missing query parameter
        let params = GoogleSearchParams {
            q: "".to_string(), // Empty query
            ..Default::default()
        };
        
        let error = client.google_search(params).await.unwrap_err();
        
        // Verify error
        match error {
            ServiceError::Validation(msg) => {
                assert!(msg.contains("Parameter 'q' is required"));
            }
            _ => panic!("Expected Validation error, got: {:?}", error),
        }
        
        // Verify error is not retryable
        assert!(!error.is_retryable());
    }
    
    #[tokio::test]
    async fn test_server_error() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for server error
        let mock_response = json!({
            "error": "Internal server error"
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/search/google"))
            .respond_with(ResponseTemplate::new(500)
                .set_body_json(&mock_response))
            .mount(&mock_server)
            .await;
        
        // Create client
        let client = create_test_client(&mock_server);
        
        // Execute search
        let params = GoogleSearchParams {
            q: "server error test".to_string(),
            ..Default::default()
        };
        
        let error = client.google_search(params).await.unwrap_err();
        
        // Verify error
        match error {
            ServiceError::Service(msg) => {
                assert!(msg.contains("Internal server error"));
            }
            _ => panic!("Expected Service error, got: {:?}", error),
        }
        
        // Verify error is retryable
        assert!(error.is_retryable());
    }
    
    #[tokio::test]
    async fn test_health_check() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Mock response for successful account info (used for health check)
        let mock_response = json!({
            "account_id": "mock_account_id",
            "account_email": "test@example.com"
        });
        
        // Setup the mock
        Mock::given(method("GET"))
            .and(path("/account"))
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
            .and(path("/account"))
            .respond_with(ResponseTemplate::new(500)
                .set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;
        
        // Perform health check again
        let is_healthy = client.health_check().await.unwrap();
        
        // Should return false for unhealthy service but not error
        assert!(!is_healthy);
    }
    
    #[tokio::test]
    async fn test_custom_engine_with_builder() {
        // Setup mock server
        let mock_server = setup_mock_server().await;
        
        // Create client with custom default engine
        let client = SerpAPIClientBuilder::new()
            .api_key("mock_serp_api_key")
            .base_url(mock_server.uri())
            .default_engine("custom_engine")
            .build()
            .expect("Failed to build SerpAPI client");
        
        // Should have custom default engine
        assert_eq!(client.config.default_engine, "custom_engine");
    }
}