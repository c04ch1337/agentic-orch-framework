# Error Handling and Resilience

This document describes the error handling system and resilience patterns implemented in the Tool SDK.

## Error Handling System

The Tool SDK implements a comprehensive error handling system designed to:

1. Provide detailed and contextual error information
2. Categorize errors by type for better handling
3. Identify retryable vs. permanent errors
4. Map service-specific error formats to a consistent format
5. Support debugging and troubleshooting

### Core Error Types

#### ServiceError

The central error type in the Tool SDK is `ServiceError`, which categorizes errors into specific types:

```rust
pub enum ServiceError {
    /// Network or connection errors
    Network(String),
    
    /// Authentication errors
    Authentication(String),
    
    /// Authorization errors (permission issues)
    Authorization(String),
    
    /// Rate limiting errors
    RateLimit(String),
    
    /// Service-specific errors
    Service(String),
    
    /// Request validation errors
    Validation(String),
    
    /// Response parsing errors
    Parsing(String),
    
    /// Configuration errors
    Configuration(String),
    
    /// Timeout errors
    Timeout(String),
    
    /// Unexpected or internal errors
    Internal(String),
    
    /// Errors with additional context
    WithContext {
        inner: Box<ServiceError>,
        context: ErrorContext,
    },
}
```

The structure allows for errors to be categorized by type, making it easier to handle specific kinds of errors differently.

#### Result Type

The SDK provides a specialized `Result` type alias:

```rust
pub type Result<T> = std::result::Result<T, ServiceError>;
```

This simplifies function signatures and makes it clear that functions return a `ServiceError` on failure.

### Error Context

The `ErrorContext` struct adds rich contextual information to errors:

```rust
pub struct ErrorContext {
    /// Service that generated the error
    pub service: String,
    
    /// Request timestamp
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    
    /// HTTP status code if applicable
    pub status_code: Option<u16>,
    
    /// Service-specific error code
    pub error_code: Option<String>,
    
    /// Request ID for tracing
    pub request_id: Option<String>,
    
    /// Endpoint that was called
    pub endpoint: Option<String>,
    
    /// Additional context data
    pub data: HashMap<String, String>,
}
```

This context is crucial for debugging, especially in distributed systems where errors may occur across service boundaries.

### Creating and Using Errors

#### Creating Errors

The SDK provides convenient factory methods for creating errors:

```rust
// Create a network error
let error = ServiceError::network("Connection refused");

// Create an authentication error
let error = ServiceError::authentication("Invalid API key");

// Create a rate limit error
let error = ServiceError::rate_limit("Too many requests");
```

#### Adding Context

Context can be added to errors for richer information:

```rust
// Add context to an error
let error = ServiceError::network("Connection timeout")
    .with_context(
        ErrorContext::for_service("openai")
            .status_code(408)
            .request_id("req-12345")
            .endpoint("completions")
    );

// Add a single context value
let error = ServiceError::network("Connection timeout")
    .with_context_value("attempt", "3");
```

#### Checking Error Types

Errors can be examined to determine how to handle them:

```rust
match result {
    Ok(value) => {
        // Handle success case
    },
    Err(e) if e.is_retryable() => {
        // Handle retryable errors (network, timeout, rate limits)
        println!("Transient error: {}", e);
        // Retry the operation
    },
    Err(e) => {
        // Handle permanent errors
        println!("Permanent error: {}", e);
        // Don't retry
    }
}
```

### Error Mapping

The SDK provides error mapping from external API errors to the unified `ServiceError` format. This mapping happens in the service-specific implementations.

For example, OpenAI API errors are mapped like this:

```rust
// Example of how OpenAI errors are mapped
fn map_openai_error(status: u16, error_data: Value) -> ServiceError {
    let error_message = error_data["error"]["message"]
        .as_str()
        .unwrap_or("Unknown OpenAI error");

    let error_type = error_data["error"]["type"]
        .as_str()
        .unwrap_or("unknown_error");

    let context = ErrorContext::for_service("openai")
        .status_code(status)
        .error_code(error_type);

    match (status, error_type) {
        (401, _) => ServiceError::authentication(error_message),
        (403, _) => ServiceError::authorization(error_message),
        (429, _) => ServiceError::rate_limit(error_message),
        (400, _) => ServiceError::validation(error_message),
        (404, _) => ServiceError::service(format!("Resource not found: {}", error_message)),
        (500..=599, _) => ServiceError::service(format!("OpenAI server error: {}", error_message)),
        _ => ServiceError::service(format!("OpenAI API error: {}", error_message)),
    }
    .with_context(context)
}
```

This mapping ensures that service-specific errors are normalized to the SDK's error model, making them easier to handle in a consistent way.

### Retryable vs. Permanent Errors

The SDK distinguishes between retryable (transient) and permanent errors:

```rust
impl ServiceError {
    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        match self {
            ServiceError::Network(_) => true,
            ServiceError::Timeout(_) => true,
            ServiceError::RateLimit(_) => true,
            ServiceError::WithContext { inner, .. } => inner.is_retryable(),
            _ => false,
        }
    }
    
    /// Check if this is a permanent error (not retryable)
    pub fn is_permanent(&self) -> bool {
        !self.is_retryable()
    }
}
```

This distinction is important for implementing resilience patterns:
- **Retryable errors**: Network issues, timeouts, rate limits
- **Permanent errors**: Authentication failures, validation issues, service errors

## Resilience Patterns

The SDK implements resilience patterns to handle transient failures and prevent cascading failures. These patterns are designed to make external service calls more robust.

### Retry Pattern

The retry pattern automatically retries failed operations with an exponential backoff strategy.

#### RetryConfig

The retry behavior is configurable through the `RetryConfig` struct:

```rust
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    
    /// Initial interval between retries
    pub initial_interval: Duration,
    
    /// Maximum interval between retries
    pub max_interval: Duration,
    
    /// Multiplier for exponential backoff
    pub multiplier: f64,
    
    /// Randomization factor to avoid thundering herd
    pub randomization_factor: f64,
    
    /// Maximum total time to spend retrying
    pub max_elapsed_time: Option<Duration>,
}
```

#### Usage

```rust
// Configure retry with 3 attempts, starting at 100ms
let retry_config = RetryConfig {
    max_retries: 3,
    initial_interval: Duration::from_millis(100),
    max_interval: Duration::from_secs(2),
    multiplier: 2.0,
    randomization_factor: 0.1,
    max_elapsed_time: Some(Duration::from_secs(10)),
};

// Create client with retry configuration
let client = openai_client()
    .api_key("your-api-key")
    .retry(retry_config)
    .build()?;
```

### Circuit Breaker Pattern

The circuit breaker pattern prevents cascading failures by failing fast when a service is experiencing issues.

#### CircuitBreakerConfig

The circuit breaker behavior is configurable through the `CircuitBreakerConfig` struct:

```rust
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: usize,
    
    /// Duration to wait before transitioning to half-open
    pub reset_timeout: Duration,
    
    /// Number of successful requests needed to close the circuit
    pub success_threshold: usize,
    
    /// Size of the sliding window for error rate calculation
    pub sliding_window_size: usize,
    
    /// Error rate threshold to open the circuit (0.0-1.0)
    pub error_threshold_percentage: f64,
}
```

#### Circuit Breaker States

The circuit breaker has three states:

1. **Closed**: Requests flow normally
2. **Open**: All requests are rejected immediately
3. **Half-Open**: A limited number of test requests are allowed through

```
┌────────────┐   failures >= threshold   ┌──────────┐
│   Closed   │─────────────────────────→│   Open   │
└────────────┘                           └──────────┘
      ↑                                       │
      │                                       │
      │                                       │ timeout elapsed
      │                                       │
      │                                       ▼
      │                                  ┌───────────┐
      └──────────────────────────────────│ Half-Open │
           successes >= threshold        └───────────┘
```

#### Usage

```rust
// Configure circuit breaker
let cb_config = CircuitBreakerConfig {
    failure_threshold: 5,
    reset_timeout: Duration::from_secs(30),
    success_threshold: 2,
    ..CircuitBreakerConfig::default()
};

// Create client with circuit breaker
let client = openai_client()
    .api_key("your-api-key")
    .circuit_breaker(cb_config)
    .build()?;
```

### Resilience Facade

The SDK provides a unified `Resilience` facade that composes multiple resilience patterns:

```rust
// Create a resilience facade with custom configurations
let resilience = Resilience::new(retry_config, circuit_breaker_config);

// Use the resilience facade with any async operation
let result = resilience.execute(|| async {
    // Your operation here
    client.some_api_call().await
}).await;
```

The facade applies both retry and circuit breaker patterns to the operation:

1. First checks if the circuit is closed
2. Executes the operation with retries if it's retryable
3. Records failures and successes for the circuit breaker
4. Manages circuit breaker state transitions

## Best Practices for Error Handling

### Catching and Handling Errors

```rust
match client.chat_completion(request).await {
    Ok(response) => {
        // Process successful response
        println!("Received response: {:?}", response);
    },
    Err(e) => {
        match e {
            ServiceError::Authentication(_) | ServiceError::Authorization(_) => {
                // Handle auth errors (check credentials)
                log_error("Authentication failed, check API key", &e);
            },
            ServiceError::RateLimit(_) => {
                // Handle rate limit (back off and retry)
                log_error("Rate limited, will retry after delay", &e);
                tokio::time::sleep(Duration::from_secs(5)).await;
                // Retry the request
            },
            ServiceError::Network(_) | ServiceError::Timeout(_) => {
                // Handle network/timeout (retry with backoff)
                log_error("Network issue, will retry", &e);
                // Retry with backoff
            },
            _ if e.is_retryable() => {
                // Handle other retryable errors
                log_error("Retryable error occurred", &e);
                // Implement retry logic
            },
            _ => {
                // Handle permanent errors
                log_error("Permanent error, cannot proceed", &e);
                // Report error to user
            }
        }
    }
}
```

### Propagating Errors with Context

When propagating errors up the call stack, add context at each level:

```rust
async fn fetch_data() -> Result<Data> {
    match client.fetch_raw_data().await {
        Ok(raw) => {
            // Process raw data
            Ok(processed_data)
        },
        Err(e) => {
            // Add context and propagate
            Err(e.with_context_value("operation", "fetch_data"))
        }
    }
}
```

### Designing for Resilience

1. **Always check for retryability**: Use `is_retryable()` to determine if an operation should be retried
2. **Configure timeouts appropriately**: Set reasonable timeouts to avoid hanging
3. **Use the resilience facade**: Leverage the built-in retry and circuit breaker patterns
4. **Handle rate limits gracefully**: Back off and retry when rate limited
5. **Log with context**: Include error context in logs for better debugging

## Extending the Error System

### Adding New Error Types

To add a new error type, extend the `ServiceError` enum:

```rust
// Example of how you might extend ServiceError
pub enum ServiceError {
    // ... existing variants ...
    
    /// New error type for quota exceeded
    QuotaExceeded(String),
}

impl ServiceError {
    /// Factory method for quota exceeded errors
    pub fn quota_exceeded(message: impl Into<String>) -> Self {
        ServiceError::QuotaExceeded(message.into())
    }
    
    /// Update is_retryable to handle new type
    pub fn is_retryable(&self) -> bool {
        match self {
            // ... existing cases ...
            ServiceError::QuotaExceeded(_) => false, // Not retryable
            _ => false,
        }
    }
}
```

### Custom Error Mapping

To add mapping for a new service, create a mapping function:

```rust
// Example of mapping for a new service
fn map_custom_service_error(status: u16, body: &str) -> ServiceError {
    // Parse the error response
    let error_data = match serde_json::from_str::<Value>(body) {
        Ok(data) => data,
        Err(_) => {
            return ServiceError::parsing(format!(
                "Failed to parse error response: {}", body
            ));
        }
    };
    
    let error_message = error_data["error"]["message"]
        .as_str()
        .unwrap_or("Unknown error");
        
    let context = ErrorContext::for_service("custom-service")
        .status_code(status);
        
    // Map based on status code and/or error message
    match status {
        401 => ServiceError::authentication(error_message),
        403 => ServiceError::authorization(error_message),
        // ... other mappings ...
        _ => ServiceError::service(format!("Service error: {}", error_message)),
    }
    .with_context(context)
}
```

## Troubleshooting Common Errors

### Authentication Errors

**Error**: `Authentication error: Invalid API key`

**Solutions**:
- Check that the API key is correct
- Verify that the API key has the necessary permissions
- Check for environment variable misspellings
- Verify that the API key format is correct for the service

### Rate Limit Errors

**Error**: `Rate limit exceeded: Too many requests`

**Solutions**:
- Implement exponential backoff
- Reduce request frequency
- Use retry with longer delays
- Consider upgrading your API tier

### Network Errors

**Error**: `Network error: Connection refused`

**Solutions**:
- Check network connectivity
- Verify that the service is available
- Check for correct base URL
- Verify proxy settings if applicable

### Timeout Errors

**Error**: `Timeout error: Request timed out after 30 seconds`

**Solutions**:
- Increase timeout setting for long-running operations
- Check service health/status
- Verify that the operation isn't too heavy
- Consider breaking into smaller operations

### Parsing Errors

**Error**: `Parsing error: JSON error: expected value at line 1 column 1`

**Solutions**:
- Check for malformed response data
- Verify service compatibility
- Validate request format
- Check for API version mismatches

## Conclusion

The error handling and resilience systems in the Tool SDK provide a robust foundation for working with external services. By leveraging these patterns, developers can create reliable applications that gracefully handle various failure scenarios.

The key recommendations for working with the error system:

1. **Use the contextual error information**: The rich context provided with errors helps with debugging and troubleshooting.
2. **Leverage resilience patterns**: Use retry and circuit breaker patterns for better reliability.
3. **Distinguish between error types**: Handle different types of errors appropriately.
4. **Add context when propagating errors**: Enrich errors with context as they propagate up the call stack.
5. **Log with detailed context**: Include error context in logs for better debugging.