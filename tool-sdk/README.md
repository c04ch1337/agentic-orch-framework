# Tool SDK

A unified SDK for external service integrations in the Phoenix ORCH project.

## Overview

The Tool SDK provides a standardized abstraction layer for integrating with external services and APIs. It serves as the foundation for all external service interactions within the Phoenix ORCH ecosystem, ensuring consistent approaches to authentication, error handling, resilience patterns, and observability.

The SDK is designed with the following principles:

- **Unified Interface**: Common patterns for interacting with diverse external services
- **Type Safety**: Strongly-typed client interfaces for improved developer experience
- **Resilience First**: Built-in patterns for handling transient failures and service degradation
- **Comprehensive Error Handling**: Detailed error information with rich context
- **Observability**: Integrated telemetry and metrics collection
- **Extensibility**: Easy to extend with new service integrations

## Key Features

- **Core Abstractions**:
  - `ServiceClient`: Base trait for all external service clients
  - `RequestExecutor`: Handles HTTP or other transport mechanism requests
  - `AuthenticatedClient`: Adds authentication capabilities to clients 
  - `RateLimited`: Adds rate limiting capabilities to clients
  - `Telemetry`: Adds observability and metrics capabilities

- **Service-Specific Implementations**:
  - Pre-configured clients for common services (OpenAI, SerpAPI)
  - Type-safe request and response models
  - Service-specific error handling and mapping

- **Resilience Patterns**:
  - Retry with exponential backoff
  - Circuit breaker to prevent cascading failures
  - Unified resilience facade for composing multiple patterns

- **Configuration Management**:
  - Environment variable support
  - Configuration providers and customizable namespacing
  - Runtime configuration updates

- **Error Handling System**:
  - Categorized error types (network, auth, rate limit, etc.)
  - Rich contextual information for debugging
  - Clear distinction between retryable and permanent errors

## Architecture

The Tool SDK is designed around a set of core traits that define the behavior of service clients:

```
ServiceClient (Base trait)
├── RequestExecutor (HTTP handling)
├── AuthenticatedClient (Auth capabilities)
├── RateLimited (Rate limiting)
└── Telemetry (Metrics & observability)
```

These traits are implemented by service-specific clients that handle the details of interacting with particular external APIs. The SDK also provides a unified `Resilience` facade that can be used to add retry and circuit breaker patterns to any service client.

For more detailed architecture documentation, see [ARCHITECTURE.md](./docs/ARCHITECTURE.md).

## Getting Started

### Installation

Add the tool-sdk to your Cargo.toml:

```toml
[dependencies]
tool-sdk = { path = "../tool-sdk" }
```

### Basic Usage

#### OpenAI Client

```rust
use tool_sdk::{openai_client, services::openai::ChatCompletionRequest, services::openai::ChatMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an OpenAI client
    let client = openai_client()
        .api_key("your-api-key")
        .build()?;
    
    // Create a chat completion request
    let request = ChatCompletionRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: "Hello, can you tell me a fun fact about Rust?".to_string(),
                name: None,
            },
        ],
        temperature: Some(0.7),
        ..Default::default()
    };
    
    // Send the request
    let response = client.chat_completion(request).await?;
    
    // Print the response
    if let Some(choice) = response.choices.first() {
        if let Some(content) = &choice.message.content {
            println!("{}", content);
        }
    }
    
    Ok(())
}
```

#### SerpAPI Client

```rust
use tool_sdk::{serpapi_client, services::serpapi::GoogleSearchParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a SerpAPI client
    let client = serpapi_client()
        .api_key("your-api-key")
        .build()?;
    
    // Create search parameters
    let params = GoogleSearchParams {
        q: "Rust programming language".to_string(),
        num: Some(5),
        ..Default::default()
    };
    
    // Execute the search
    let results = client.google_search(params).await?;
    
    // Process the results
    if let Some(organic_results) = results.organic_results {
        for result in organic_results {
            if let Some(title) = result.title {
                println!("Title: {}", title);
            }
        }
    }
    
    Ok(())
}
```

### Using Resilience Patterns

The SDK includes built-in resilience patterns for handling transient failures:

```rust
use tool_sdk::{resilience::{RetryConfig, CircuitBreakerConfig, Resilience}};
use std::time::Duration;

// Configure custom retry behavior
let retry_config = RetryConfig {
    max_retries: 3,
    initial_interval: Duration::from_millis(100),
    max_interval: Duration::from_secs(5),
    multiplier: 2.0,
    ..RetryConfig::default()
};

// Configure circuit breaker
let cb_config = CircuitBreakerConfig {
    failure_threshold: 5,
    reset_timeout: Duration::from_secs(30),
    success_threshold: 2,
    ..CircuitBreakerConfig::default()
};

// Create a resilience facade
let resilience = Resilience::new(retry_config, cb_config);

// Use the resilience facade with any async operation
let result = resilience.execute(|| async {
    // Your operation here
    client.some_api_call().await
}).await;
```

## Configuration

The SDK supports various configuration options:

### Environment Variables

By default, the SDK will look for these environment variables:

- OpenAI: 
  - `PHOENIX_OPENAI_API_KEY`: API key for OpenAI
  - `PHOENIX_OPENAI_BASE_URL`: Base URL (defaults to "https://api.openai.com/v1")
  - `PHOENIX_OPENAI_ORG_ID`: Optional organization ID
  
- SerpAPI:
  - `PHOENIX_SERPAPI_API_KEY`: API key for SerpAPI
  - `PHOENIX_SERPAPI_BASE_URL`: Base URL (defaults to "https://serpapi.com")

### Custom Configuration

You can also configure the clients programmatically:

```rust
use tool_sdk::{
    openai_client,
    config::{EnvConfigProvider, ConfigProvider},
};

// Create a config provider
let config_provider = EnvConfigProvider::new()
    .with_prefix("MY_APP")
    .with_namespace("OPENAI");

// Load API key from config provider
let api_key = config_provider.get_string_or("API_KEY", "");

// Create a client with custom settings
let client = openai_client()
    .api_key(api_key)
    .base_url("https://custom-openai-proxy.example.com")
    .timeout(60) // seconds
    .build()?;
```

## Error Handling

The SDK provides a comprehensive error handling system:

```rust
use tool_sdk::error::{ServiceError, Result};

fn process_data() -> Result<String> {
    // If an API call fails, it will return a ServiceError
    // with specific error information
    let client = openai_client().build()?;
    
    match client.simple_completion("Tell me a joke", None).await {
        Ok(response) => Ok(response),
        Err(e) if e.is_retryable() => {
            // Handle retryable errors (network issues, rate limits)
            println!("Transient error occurred: {}", e);
            Err(e)
        },
        Err(e) => {
            // Handle permanent errors
            println!("Permanent error: {}", e);
            Err(e)
        }
    }
}
```

For more detailed error handling documentation, see [ERROR_HANDLING.md](./docs/ERROR_HANDLING.md).

## Documentation

### Architecture and Design

- [Architecture Documentation](./docs/ARCHITECTURE.md) - Detailed explanation of core abstractions and component relationships
- [Error Handling](./docs/ERROR_HANDLING.md) - Comprehensive guide to the error system

### Usage Guides

- [Integration Guide](./docs/INTEGRATION_GUIDE.md) - Step-by-step guide for integrating the SDK into services
- [Extending the SDK](./docs/EXTENDING.md) - Guide for adding new service integrations
- [Troubleshooting](./docs/TROUBLESHOOTING.md) - Common issues and solutions

### API Documentation

For detailed API documentation, run:

```
cargo doc --open
```

## Examples

Full examples are available in the `examples/` directory:

- `openai_completion.rs`: Shows how to use the OpenAI client for chat completions
- `serpapi_search.rs`: Demonstrates using the SerpAPI client for search
- `resilience_demo.rs`: Demonstrates the resilience patterns in action

Run an example with:

```
PHOENIX_OPENAI_API_KEY=your_api_key cargo run --example openai_completion
```

## Integration with the PHOENIX ORCH Project

This SDK is designed to work seamlessly with the Phoenix ORCH project:

1. Import the tool-sdk in your service's Cargo.toml
2. Use the SDK's client interfaces to communicate with external services
3. Leverage the resilience patterns for robust error handling
4. Configure service-specific settings via environment variables or config files

See the [Integration Guide](./docs/INTEGRATION_GUIDE.md) for detailed steps.

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/my-feature`)
3. Commit your changes (`git commit -am 'Add some feature'`)
4. Push to the branch (`git push origin feature/my-feature`)
5. Create a new Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.