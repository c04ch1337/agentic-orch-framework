# Integration Guide for Services

This guide provides step-by-step instructions for integrating the Tool SDK into Phoenix ORCH services, along with best practices, common patterns, and troubleshooting tips.

## Table of Contents

- [Getting Started](#getting-started)
- [Basic Integration Steps](#basic-integration-steps)
- [Configuration](#configuration)
- [Using Service Clients](#using-service-clients)
- [Implementing Resilience](#implementing-resilience)
- [Error Handling](#error-handling)
- [Observability](#observability)
- [Common Integration Patterns](#common-integration-patterns)
- [Anti-Patterns to Avoid](#anti-patterns-to-avoid)
- [Troubleshooting](#troubleshooting)

## Getting Started

### Prerequisites

- Rust 1.65 or later
- Tokio async runtime
- Access to required API keys for external services

### Adding the Dependency

In your service's `Cargo.toml`, add the Tool SDK as a dependency:

```toml
[dependencies]
tool-sdk = { path = "../tool-sdk" }
```

For production use, you may want to use a specific version from your organization's cargo registry:

```toml
[dependencies]
tool-sdk = { version = "0.1", registry = "phoenix-orch" }
```

## Basic Integration Steps

### 1. Import the SDK

```rust
// Import the base SDK
use tool_sdk::{ServiceClient, Result as ToolResult};

// Import service-specific modules
use tool_sdk::services::openai::{self, ChatCompletionRequest, ChatMessage};
```

### 2. Create Client Instances

```rust
// Create an OpenAI client
let openai_client = tool_sdk::openai_client()
    .api_key(api_key) // or read from environment variables
    .timeout(60) // seconds
    .build()?;
```

### 3. Make API Calls

```rust
// Create a request
let request = ChatCompletionRequest {
    model: "gpt-3.5-turbo".to_string(),
    messages: vec![
        ChatMessage {
            role: "user".to_string(),
            content: "Hello, how can I help?".to_string(),
            name: None,
        },
    ],
    temperature: Some(0.7),
    ..Default::default()
};

// Make the API call
let response = openai_client.chat_completion(request).await?;

// Process the response
if let Some(choice) = response.choices.first() {
    if let Some(content) = &choice.message.content {
        println!("Response: {}", content);
    }
}
```

## Configuration

### Environment Variables

The Tool SDK uses environment variables with the following convention:

```
PHOENIX_<SERVICE>_<PARAMETER>
```

For example:
- `PHOENIX_OPENAI_API_KEY` - API key for OpenAI
- `PHOENIX_OPENAI_BASE_URL` - Base URL for OpenAI API
- `PHOENIX_SERPAPI_API_KEY` - API key for SerpAPI

### Configuration Provider

You can use the built-in configuration provider to load settings:

```rust
use tool_sdk::config::{EnvConfigProvider, ConfigProvider};

// Create a config provider for environment variables
let config_provider = EnvConfigProvider::new()
    .with_prefix("PHOENIX")
    .with_namespace("OPENAI");

// Load a specific value
let api_key = config_provider.get_string_or("API_KEY", "");
let base_url = config_provider.get_string_or("BASE_URL", "https://api.openai.com/v1");
```

### Custom Configuration

You can also configure clients programmatically:

```rust
// Create a client with custom settings
let client = tool_sdk::openai_client()
    .api_key(api_key)
    .base_url("https://custom-proxy.example.com")
    .timeout(30) // seconds
    .build()?;
```

## Using Service Clients

### OpenAI Client

```rust
// Create an OpenAI client
let openai_client = tool_sdk::openai_client()
    .api_key(config.openai_api_key.clone())
    .build()?;

// Simple completion
let response = openai_client.simple_completion(
    "Summarize the following text: ...",
    Some("gpt-4")
).await?;

// Generate embeddings
let embeddings = openai_client.embed_text(
    "This is a sample text to embed.", 
    Some("text-embedding-ada-002")
).await?;
```

### SerpAPI Client

```rust
// Create a SerpAPI client
let serpapi_client = tool_sdk::serpapi_client()
    .api_key(config.serpapi_api_key.clone())
    .build()?;

// Perform a Google search
let search_results = serpapi_client.google_search(GoogleSearchParams {
    q: "Phoenix ORCH project".to_string(),
    num: Some(5),
    ..Default::default()
}).await?;
```

## Implementing Resilience

### Retry Configuration

Configure retry behavior for transient failures:

```rust
use tool_sdk::resilience::{RetryConfig, Resilience};
use std::time::Duration;

// Configure custom retry behavior
let retry_config = RetryConfig {
    max_retries: 3,
    initial_interval: Duration::from_millis(100),
    max_interval: Duration::from_secs(5),
    multiplier: 2.0,
    ..RetryConfig::default()
};

// Create client with custom retry
let client = tool_sdk::openai_client()
    .api_key(api_key)
    .retry(retry_config)
    .build()?;
```

### Circuit Breaker

Configure circuit breakers to prevent cascading failures:

```rust
use tool_sdk::resilience::{CircuitBreakerConfig, Resilience};

// Configure circuit breaker
let cb_config = CircuitBreakerConfig {
    failure_threshold: 5,
    reset_timeout: Duration::from_secs(30),
    success_threshold: 2,
    ..CircuitBreakerConfig::default()
};

// Create client with circuit breaker
let client = tool_sdk::openai_client()
    .api_key(api_key)
    .circuit_breaker(cb_config)
    .build()?;
```

### Manual Resilience

Create a resilience facade for manual operations:

```rust
// Create a resilience facade
let resilience = Resilience::new(
    RetryConfig::default(),
    CircuitBreakerConfig::default()
);

// Use resilience with any async operation
let result = resilience.execute(|| async {
    // Your custom operation here
    api_client.some_custom_call().await
}).await?;
```

## Error Handling

### Handling Service Errors

```rust
use tool_sdk::error::{ServiceError, Result};

match client.chat_completion(request).await {
    Ok(response) => {
        // Process successful response
    },
    Err(e) => {
        // Handle different error types
        match e {
            ServiceError::Authentication(_) => {
                // Handle authentication errors
                log::error!("Authentication failed: {}", e);
                return Err(MyServiceError::AuthError(e.to_string()));
            },
            ServiceError::RateLimit(_) => {
                // Handle rate limiting
                log::warn!("Rate limited: {}", e);
                return Err(MyServiceError::RateLimitError);
            },
            _ if e.is_retryable() => {
                // Handle other retryable errors
                log::warn!("Retryable error: {}", e);
                return Err(MyServiceError::TemporaryError);
            },
            _ => {
                // Handle permanent errors
                log::error!("Permanent error: {}", e);
                return Err(MyServiceError::PermanentError(e.to_string()));
            }
        }
    }
}
```

### Adding Context to Errors

```rust
// Add context to errors for better debugging
match client.chat_completion(request).await {
    Ok(response) => Ok(response),
    Err(e) => {
        Err(e.with_context_value("request_id", request_id)
             .with_context_value("user_id", user_id))
    }
}
```

### Mapping to Service-Specific Errors

```rust
// Example of mapping Tool SDK errors to your service's errors
impl From<ServiceError> for MyServiceError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::Authentication(_) => MyServiceError::AuthError(err.to_string()),
            ServiceError::RateLimit(_) => MyServiceError::RateLimitError,
            _ if err.is_retryable() => MyServiceError::TemporaryError,
            _ => MyServiceError::PermanentError(err.to_string()),
        }
    }
}
```

## Observability

### Collecting Metrics

```rust
// Get metrics from a client
let metrics = client.metrics();
if let Some(metrics_map) = metrics {
    for (key, value) in metrics_map {
        println!("{}: {}", key, value);
        // Push to your metrics system
        metrics_system.record_gauge(&key, value.parse::<f64>().unwrap_or(0.0));
    }
}
```

### Health Checks

```rust
// Perform a health check
let is_healthy = client.health_check().await?;
if !is_healthy {
    log::warn!("OpenAI service is not healthy");
    // Take appropriate action
}
```

## Common Integration Patterns

### Service Factory Pattern

Create a factory for managing service clients:

```rust
// Service factory manages creation and caching of service clients
struct ServiceFactory {
    openai_client: Option<Arc<OpenAIClient>>,
    serpapi_client: Option<Arc<SerpAPIClient>>,
    config: ServiceConfig,
}

impl ServiceFactory {
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            openai_client: None,
            serpapi_client: None,
            config,
        }
    }

    pub fn openai_client(&mut self) -> Result<Arc<OpenAIClient>> {
        if self.openai_client.is_none() {
            let client = tool_sdk::openai_client()
                .api_key(self.config.openai_api_key.clone())
                .build()?;
            self.openai_client = Some(Arc::new(client));
        }
        Ok(Arc::clone(self.openai_client.as_ref().unwrap()))
    }

    // Similar methods for other clients...
}
```

### Repository Pattern

Wrap SDK clients with a repository for business logic:

```rust
// Repository pattern adds business logic on top of the SDK
struct AIRepository {
    client: Arc<OpenAIClient>,
}

impl AIRepository {
    pub fn new(client: Arc<OpenAIClient>) -> Self {
        Self { client }
    }

    // Business-specific method
    pub async fn generate_response(&self, prompt: &str) -> Result<String> {
        // Add business logic here
        let response = self.client.simple_completion(prompt, None).await?;
        
        // Post-process the response as needed
        let processed_response = post_process(response);
        
        Ok(processed_response)
    }
}
```

### Service Proxy Pattern

Create a proxy to add cross-cutting concerns:

```rust
// Proxy adds logging, metrics, etc. to client calls
struct OpenAIServiceProxy {
    client: Arc<OpenAIClient>,
    logger: Logger,
    metrics: MetricsCollector,
}

impl OpenAIServiceProxy {
    // Wrap client methods with cross-cutting concerns
    pub async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let start = Instant::now();
        let request_id = generate_request_id();
        
        self.logger.debug("Starting chat completion request", json!({
            "request_id": request_id,
            "model": request.model,
        }));
        
        let result = self.client.chat_completion(request).await;
        
        let duration = start.elapsed();
        self.metrics.record_duration("openai.chat_completion", duration);
        
        match &result {
            Ok(response) => {
                self.logger.debug("Completed chat completion request", json!({
                    "request_id": request_id,
                    "duration_ms": duration.as_millis(),
                    "token_count": response.usage.total_tokens,
                }));
            },
            Err(e) => {
                self.logger.error("Failed chat completion request", json!({
                    "request_id": request_id,
                    "error": e.to_string(),
                    "duration_ms": duration.as_millis(),
                }));
            }
        }
        
        result
    }
}
```

## Anti-Patterns to Avoid

### Creating Multiple Client Instances

**Bad Practice**:
```rust
// Don't do this - creates a new client for every request
async fn handle_request(prompt: String) -> Result<String> {
    let client = tool_sdk::openai_client().api_key(get_api_key()).build()?;
    client.simple_completion(&prompt, None).await
}
```

**Good Practice**:
```rust
// Create a single client and reuse it
struct Service {
    openai_client: Arc<OpenAIClient>,
}

impl Service {
    pub fn new(api_key: String) -> Result<Self> {
        let client = tool_sdk::openai_client().api_key(api_key).build()?;
        Ok(Self {
            openai_client: Arc::new(client),
        })
    }
    
    async fn handle_request(&self, prompt: String) -> Result<String> {
        self.openai_client.simple_completion(&prompt, None).await
    }
}
```

### Ignoring Retryable Errors

**Bad Practice**:
```rust
// Don't treat all errors the same way
match client.chat_completion(request).await {
    Ok(response) => handle_response(response),
    Err(e) => {
        log::error!("API call failed: {}", e);
        return Err(MyError::ApiError);
    }
}
```

**Good Practice**:
```rust
// Handle retryable errors differently
match client.chat_completion(request).await {
    Ok(response) => handle_response(response),
    Err(e) if e.is_retryable() => {
        log::warn!("Temporary error, will retry: {}", e);
        // Implement retry logic
        retry_later(request);
        return Err(MyError::RetryableError);
    },
    Err(e) => {
        log::error!("Permanent API error: {}", e);
        return Err(MyError::PermanentError);
    }
}
```

### Hardcoding Credentials

**Bad Practice**:
```rust
// Don't hardcode credentials
let client = tool_sdk::openai_client()
    .api_key("sk-1234567890abcdef")
    .build()?;
```

**Good Practice**:
```rust
// Load from environment or secure configuration
let config_provider = EnvConfigProvider::new()
    .with_prefix("PHOENIX")
    .with_namespace("OPENAI");

let api_key = config_provider.get_string("API_KEY")?;
let client = tool_sdk::openai_client()
    .api_key(api_key)
    .build()?;
```

### Not Configuring Timeouts

**Bad Practice**:
```rust
// Using default timeout which might be too long
let client = tool_sdk::openai_client()
    .api_key(api_key)
    .build()?;
```

**Good Practice**:
```rust
// Set appropriate timeouts based on expected response time
let client = tool_sdk::openai_client()
    .api_key(api_key)
    .timeout(10) // 10 seconds for user-facing requests
    .build()?;
```

## Configuration Examples

### Basic Service Configuration

```toml
# config/service.toml
[external_services]
openai_api_key = "${PHOENIX_OPENAI_API_KEY}"
openai_base_url = "https://api.openai.com/v1"
openai_timeout_seconds = 30

serpapi_api_key = "${PHOENIX_SERPAPI_API_KEY}"
serpapi_base_url = "https://serpapi.com"
serpapi_timeout_seconds = 10

[resilience]
retry_max_attempts = 3
retry_initial_interval_ms = 100
retry_max_interval_ms = 5000
retry_multiplier = 2.0

circuit_breaker_failure_threshold = 5
circuit_breaker_reset_timeout_seconds = 30
circuit_breaker_success_threshold = 2
```

### Docker Environment Variables

```yaml
# docker-compose.yml
services:
  my-service:
    image: phoenix-orch/my-service:latest
    environment:
      - PHOENIX_OPENAI_API_KEY=${OPENAI_API_KEY}
      - PHOENIX_OPENAI_BASE_URL=https://api.openai.com/v1
      - PHOENIX_OPENAI_TIMEOUT_SECONDS=30
      - PHOENIX_SERPAPI_API_KEY=${SERPAPI_API_KEY}
```

### Kubernetes ConfigMap and Secret

```yaml
# kubernetes/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: my-service-config
data:
  PHOENIX_OPENAI_BASE_URL: "https://api.openai.com/v1"
  PHOENIX_OPENAI_TIMEOUT_SECONDS: "30"
  PHOENIX_SERPAPI_BASE_URL: "https://serpapi.com"
  PHOENIX_SERPAPI_TIMEOUT_SECONDS: "10"
---
# kubernetes/secret.yaml
apiVersion: v1
kind: Secret
metadata:
  name: my-service-secrets
type: Opaque
data:
  PHOENIX_OPENAI_API_KEY: <base64-encoded-key>
  PHOENIX_SERPAPI_API_KEY: <base64-encoded-key>
```

## Troubleshooting

### Common Integration Issues

#### Authentication Failures

**Problem**: `ServiceError::Authentication("Invalid API key provided")`

**Solutions**:
- Verify the API key is correct
- Check environment variable names (PHOENIX_OPENAI_API_KEY vs OPENAI_API_KEY)
- Verify the API key has the necessary permissions
- Check for whitespace in the API key
- Verify that the API key is properly set in the environment

#### Timeout Issues

**Problem**: `ServiceError::Timeout("Request timed out after 30 seconds")`

**Solutions**:
- Increase the timeout for the client
- Check network connectivity
- Verify service is not overloaded
- Consider using a proxy closer to the API endpoint
- Break large requests into smaller chunks

#### Rate Limit Errors

**Problem**: `ServiceError::RateLimit("Rate limit exceeded")`

**Solutions**:
- Implement exponential backoff retry
- Reduce request frequency
- Monitor rate limit headers in responses
- Consider upgrading API tier
- Add request throttling/queuing

#### Circuit Breaker Open

**Problem**: `ServiceError::Service("Circuit breaker is open, rejecting requests")`

**Solutions**:
- Check logs for prior failures
- Monitor service health
- Wait for the circuit breaker reset timeout
- Verify underlying service is operational
- Consider adjusting circuit breaker parameters

### Debugging Tips

1. **Enable Debug Logging**:
   ```rust
   // Set up logging with debug level for tool-sdk
   env_logger::Builder::new()
       .filter(None, LevelFilter::Info)
       .filter(Some("tool_sdk"), LevelFilter::Debug)
       .init();
   ```

2. **Log Request/Response Details**:
   ```rust
   // Log request details (be careful with sensitive data)
   log::debug!("Sending request: {:?}", request);
   
   // Log response details
   match client.chat_completion(request).await {
       Ok(response) => {
           log::debug!("Received response: token count={}", response.usage.total_tokens);
           // Don't log full response content which might be sensitive
       },
       Err(e) => {
           log::error!("Request failed: {:?}", e);
       }
   }
   ```

3. **Check Environment Variables**:
   ```rust
   // Debug environment variable loading
   for var in ["PHOENIX_OPENAI_API_KEY", "PHOENIX_OPENAI_BASE_URL"] {
       match std::env::var(var) {
           Ok(val) => log::debug!("{} is set with length {}", var, val.len()),
           Err(_) => log::warn!("{} is not set", var),
       }
   }
   ```

### Getting Help

If you encounter issues integrating the Tool SDK, you can:

1. Check the [Troubleshooting Guide](./TROUBLESHOOTING.md) for common issues
2. Review the API documentation with `cargo doc --open`
3. Look at the example code in the `examples/` directory
4. Submit an issue on the internal Phoenix ORCH repository

## Conclusion

This integration guide provides the foundational knowledge to start using the Tool SDK in your Phoenix ORCH services. By following these patterns and best practices, you can create robust services that interface reliably with external APIs.

Remember these key integration principles:

1. **Reuse clients** - Create clients once and reuse them
2. **Handle errors properly** - Distinguish between retryable and permanent errors
3. **Configure resilience** - Use retry and circuit breaker patterns appropriately
4. **Secure credentials** - Never hardcode API keys
5. **Add proper observability** - Collect metrics and implement health checks