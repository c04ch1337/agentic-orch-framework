# Troubleshooting Guide

This guide addresses common issues that developers might encounter when using the Tool SDK, along with their causes and solutions.

## Table of Contents

- [Authentication Issues](#authentication-issues)
- [Rate Limiting Problems](#rate-limiting-problems)
- [Connection Errors](#connection-errors)
- [Timeout Issues](#timeout-issues)
- [Circuit Breaker Tripping](#circuit-breaker-tripping)
- [Parsing and Serialization Errors](#parsing-and-serialization-errors)
- [Configuration Problems](#configuration-problems)
- [Performance Issues](#performance-issues)
- [Debugging Techniques](#debugging-techniques)
- [Common Error Codes](#common-error-codes)

## Authentication Issues

### Invalid API Key

**Error**: `ServiceError::Authentication("Invalid API key provided")`

**Causes**:
- Incorrect API key provided
- API key has been revoked or expired
- API key doesn't have the required permissions

**Solutions**:
1. Double-check the API key for accuracy
2. Ensure the API key has not expired
3. Verify the key has appropriate permissions
4. Generate a new API key if necessary
5. Check if the API key needs to be prefixed (e.g., "Bearer ")

### Missing API Key

**Error**: `ServiceError::Authentication("No API key set for ... client")`

**Causes**:
- API key not provided to client
- Environment variable not set or misspelled

**Solutions**:
1. Ensure the API key is provided when building the client
2. Check that environment variables are set correctly:
   ```bash
   # Check if the environment variable is set
   echo $PHOENIX_OPENAI_API_KEY
   ```
3. Set the API key through code:
   ```rust
   let client = openai_client()
       .api_key("your-api-key")
       .build()?;
   ```

### Organization ID Issues

**Error**: `ServiceError::Authentication("Organization not found")`

**Causes**:
- Incorrect organization ID
- Your account doesn't belong to specified organization

**Solutions**:
1. Verify the organization ID
2. Check your account membership
3. Set the correct organization ID:
   ```rust
   let client = openai_client()
       .api_key(api_key)
       .org_id(org_id)
       .build()?;
   ```

## Rate Limiting Problems

### Rate Limit Exceeded

**Error**: `ServiceError::RateLimit("Rate limit exceeded: Too many requests")`

**Causes**:
- Too many requests in a short period
- Rate limit is too restrictive
- Shared API key being used across multiple services

**Solutions**:
1. Implement exponential backoff retry:
   ```rust
   let retry_config = RetryConfig {
       max_retries: 5,
       initial_interval: Duration::from_millis(500),
       multiplier: 2.0,
       ..RetryConfig::default()
   };
   
   let client = openai_client()
       .api_key(api_key)
       .retry(retry_config)
       .build()?;
   ```

2. Reduce the frequency of requests
3. Implement request queuing
4. Consider using a higher-tier API plan
5. Use separate API keys for different services

### Custom Rate Limit Configuration

**Problem**: Default rate limits don't match service requirements

**Solution**:
```rust
// Configure custom rate limiting
let mut client = openai_client()
    .api_key(api_key)
    .build()?;

// Set 100 requests per minute rate limit
client.configure_rate_limit(100, Duration::from_secs(60));
```

## Connection Errors

### Connection Refused

**Error**: `ServiceError::Network("Connection refused")`

**Causes**:
- Service is down
- Incorrect base URL
- Network connectivity issue
- Firewall blocking connection

**Solutions**:
1. Verify service status (check status pages)
2. Check base URL is correct:
   ```rust
   let client = openai_client()
       .api_key(api_key)
       .base_url("https://api.openai.com/v1")
       .build()?;
   ```
3. Test network connectivity (ping, curl)
4. Check firewall rules
5. Implement retry with backoff logic

### DNS Resolution Failure

**Error**: `ServiceError::Network("Failed to resolve host...")`

**Causes**:
- DNS resolution issues
- Incorrect hostname
- Network misconfiguration

**Solutions**:
1. Verify hostname is correct
2. Check DNS settings
3. Try using IP address directly if applicable
4. Implement retry logic with longer initial delay

### SSL/TLS Errors

**Error**: `ServiceError::Network("SSL error: certificate verification failed")`

**Causes**:
- Invalid SSL certificate
- Expired certificate
- Self-signed certificate not trusted
- Intermediate certificate missing

**Solutions**:
1. Verify certificate is from a trusted authority
2. Update certificate if expired
3. Add custom root certificates if necessary:
   ```rust
   let mut http_builder = reqwest::ClientBuilder::new()
       .add_root_certificate(certificate);
       
   // Note: This approach should be used cautiously, especially in production
   ```

## Timeout Issues

### Request Timeout

**Error**: `ServiceError::Timeout("Request timed out after 30 seconds")`

**Causes**:
- Service is slow to respond
- Request is too large
- Default timeout is too short
- Network latency

**Solutions**:
1. Increase the timeout:
   ```rust
   let client = openai_client()
       .api_key(api_key)
       .timeout(60) // 60 seconds
       .build()?;
   ```
2. Break large requests into smaller chunks
3. Check service health status
4. Optimize request payload size

### Read Timeout

**Error**: `ServiceError::Timeout("Read timed out")`

**Causes**:
- Service started processing but took too long
- Response is too large

**Solutions**:
1. Increase timeout duration
2. Limit response size where applicable
3. Consider implementing streaming for large responses

## Circuit Breaker Tripping

### Circuit Breaker Open

**Error**: `ServiceError::Service("Circuit breaker is open, rejecting requests for X more seconds")`

**Causes**:
- Too many failures in a short period
- Service health degraded
- Circuit breaker thresholds too sensitive

**Solutions**:
1. Check service health
2. Wait for circuit to close automatically (after reset timeout)
3. Manually reset the circuit breaker in testing:
   ```rust
   // Get access to the resilience object
   resilience.reset_circuit_breaker();
   ```
4. Adjust circuit breaker configuration:
   ```rust
   let cb_config = CircuitBreakerConfig {
       failure_threshold: a10, // More failures before opening
       reset_timeout: Duration::from_secs(5), // Shorter reset timeout
       success_threshold: 1, // Fewer successes needed to close
       ..CircuitBreakerConfig::default()
   };
   
   let client = openai_client()
       .api_key(api_key)
       .circuit_breaker(cb_config)
       .build()?;
   ```

### Half-Open Circuit Failing

**Problem**: Circuit breaker transitions to half-open but immediately trips back to open

**Causes**:
- Service still unhealthy
- Success threshold too high

**Solutions**:
1. Verify service is actually healthy before attempting reset
2. Lower success threshold:
   ```rust
   let cb_config = CircuitBreakerConfig {
       success_threshold: 1, // Only one success needed to close
       ..CircuitBreakerConfig::default()
   };
   ```
3. Implement health checks before attempting operations

## Parsing and Serialization Errors

### Request Serialization Failure

**Error**: `ServiceError::Validation("Failed to serialize request: Expected string, got null")`

**Causes**:
- Invalid data in request object
- Missing required fields
- Type mismatch

**Solutions**:
1. Validate request data before sending
2. Check for null or undefined values
3. Use the correct types for fields
4. Implement request validation:
   ```rust
   fn validate_request(&self, request: &ChatCompletionRequest) -> Result<()> {
       if request.messages.is_empty() {
           return Err(ServiceError::validation("Messages cannot be empty"));
       }
       // More validation...
       Ok(())
   }
   ```

### Response Parsing Failure

**Error**: `ServiceError::Parsing("Failed to parse response: Invalid JSON")`

**Causes**:
- Service returned invalid JSON
- Unexpected response format
- Response model doesn't match actual response

**Solutions**:
1. Log the raw response for debugging
2. Update response model to match actual response
3. Check API version compatibility
4. Look for changes in the service's documentation
5. Add more flexible parsing logic:
   ```rust
   #[derive(Deserialize)]
   struct FlexibleResponse {
       #[serde(default)]
       results: Vec<Result>,
       #[serde(default)]
       data: Option<Vec<Result>>,
       // Add alternative fields that might contain results
   }
   ```

## Configuration Problems

### Missing Configuration

**Error**: `ServiceError::Configuration("Base URL is required")`

**Causes**:
- Required configuration not provided
- Configuration loading failure

**Solutions**:
1. Verify all required configurations are provided
2. Check environment variables are set correctly
3. Provide explicit configuration:
   ```rust
   let client = openai_client()
       .api_key(api_key)
       .base_url("https://api.openai.com/v1")
       .build()?;
   ```
4. Use a configuration provider:
   ```rust
   let config_provider = EnvConfigProvider::new()
       .with_prefix("PHOENIX")
       .with_namespace("OPENAI");
   
   let api_key = config_provider.get_string("API_KEY")?;
   let base_url = config_provider.get_string_or(
       "BASE_URL", 
       "https://api.openai.com/v1"
   );
   ```

### Environment Variable Issues

**Problem**: Environment variables not being loaded correctly

**Causes**:
- Variable name mismatch
- Variables not set in environment
- Prefix or namespace confusion

**Solutions**:
1. Double-check variable names and formats
2. Print environment variables for debugging:
   ```rust
   for var in ["PHOENIX_OPENAI_API_KEY", "PHOENIX_OPENAI_BASE_URL"] {
       match std::env::var(var) {
           Ok(val) => println!("{}: {}", var, val),
           Err(_) => println!("{} not set", var),
       }
   }
   ```
3. Use direct configuration instead of environment variables

## Performance Issues

### Slow Response Times

**Problem**: API calls take too long to complete

**Causes**:
- Network latency
- Service under high load
- Inefficient request patterns
- Large response sizes

**Solutions**:
1. Use profiling to identify bottlenecks
2. Implement request batching where applicable
3. Use caching for frequently accessed data:
   ```rust
   // Simple in-memory cache example
   struct CachedClient<T: Clone> {
       client: Arc<dyn ServiceClient>,
       cache: Mutex<HashMap<String, (T, Instant)>>,
       ttl: Duration,
   }
   
   impl<T: Clone> CachedClient<T> {
       async fn get(&self, key: &str, fetch: impl Future<Output = Result<T>>) -> Result<T> {
           // Check cache first
           if let Some((value, time)) = self.cache.lock().unwrap().get(key) {
               if time.elapsed() < self.ttl {
                   return Ok(value.clone());
               }
           }
           
           // Fetch new value
           let value = fetch.await?;
           
           // Update cache
           self.cache.lock().unwrap().insert(
               key.to_string(),
               (value.clone(), Instant::now())
           );
           
           Ok(value)
       }
   }
   ```
4. Optimize request payload size
5. Consider client-side connection pooling

### Memory Usage

**Problem**: High memory usage

**Causes**:
- Large response data
- Creating too many client instances
- Memory leaks in response handling

**Solutions**:
1. Reuse client instances:
   ```rust
   // Create once and share
   let client = Arc::new(openai_client().api_key(api_key).build()?);
   ```
2. Stream large responses where applicable
3. Limit response sizes or paginate
4. Release large objects when done processing
5. Use profiling tools to identify memory leaks

### Too Many Connections

**Problem**: "Too many open files" or connection pool exhausted

**Causes**:
- Creating new clients for each request
- Not closing connections
- Connection leaks

**Solutions**:
1. Reuse client instances
2. Use a connection pool with appropriate limits
3. Check for resource leaks in error paths
4. Set appropriate keep-alive settings

## Debugging Techniques

### Enable Debug Logging

```rust
// Set up debug logging
env_logger::Builder::new()
    .filter(None, log::LevelFilter::Info)
    .filter(Some("tool_sdk"), log::LevelFilter::Debug)
    .init();
```

### Log Request/Response Details

```rust
log::debug!("Sending request to {}: {:?}", endpoint, request);

match client.execute(endpoint, request).await {
    Ok(response) => {
        log::debug!("Received response: {:?}", response);
        // Process response
    },
    Err(e) => {
        log::error!("Request failed: {}", e);
        // Handle error
    }
}
```

### Inspect Network Requests

Use tools like Wireshark or Fiddler to inspect the raw HTTP requests and responses.

### Use Correlation IDs

```rust
// Generate a correlation ID
let correlation_id = format!("{}", uuid::Uuid::new_v4());

// Add to request context
let context = ContextEnricher::new("my_service")
    .with_request_id(correlation_id.clone());

// Log with correlation ID
log::info!("[{}] Starting request", correlation_id);

// Use in error context
match client.execute(endpoint, request).await {
    Ok(response) => response,
    Err(e) => {
        Err(e.with_context_value("correlation_id", correlation_id))
    }
}
```

### Add Request Tracking

```rust
async fn tracked_request<T, R>(
    client: &impl RequestExecutor,
    endpoint: &str,
    request: &T,
    request_id: &str,
) -> Result<R>
where
    T: Serialize + Send + Sync,
    R: for<'de> Deserialize<'de> + Send,
{
    log::info!("[{}] Starting request to {}", request_id, endpoint);
    
    let start = Instant::now();
    let result = client.execute(endpoint, request).await;
    let duration = start.elapsed();
    
    match &result {
        Ok(_) => {
            log::info!(
                "[{}] Request completed in {:?}", 
                request_id, 
                duration
            );
        },
        Err(e) => {
            log::error!(
                "[{}] Request failed after {:?}: {}", 
                request_id, 
                duration, 
                e
            );
        }
    }
    
    result
}
```

## Common Error Codes

### HTTP Status Codes

| Status Code | Meaning                    | Error Type               | Retryable | Solution                                  |
|-------------|----------------------------|--------------------------|-----------|-------------------------------------------|
| 400         | Bad Request                | Validation               | No        | Fix request parameters                    |
| 401         | Unauthorized               | Authentication           | No        | Check API key                             |
| 403         | Forbidden                  | Authorization            | No        | Check permissions                         |
| 404         | Not Found                  | Service                  | No        | Check endpoint or resource ID             |
| 429         | Too Many Requests          | RateLimit                | Yes       | Implement backoff, reduce request rate    |
| 500         | Internal Server Error      | Service                  | Yes       | Retry with backoff                        |
| 502         | Bad Gateway                | Network                  | Yes       | Retry with backoff                        |
| 503         | Service Unavailable        | Service                  | Yes       | Retry with backoff                        |
| 504         | Gateway Timeout            | Timeout                  | Yes       | Increase timeout, retry with backoff      |

### OpenAI Error Types

| Error Type                | Meaning                    | Solution                                     |
|---------------------------|----------------------------|----------------------------------------------|
| `invalid_request_error`   | Invalid request parameters | Check request format and parameters          |
| `authentication_error`    | Auth failed                | Verify API key                               |
| `permission_error`        | Insufficient permissions   | Check API key permissions                    |
| `rate_limit_error`        | Rate limit exceeded        | Implement backoff, reduce request frequency  |
| `server_error`            | OpenAI server error        | Retry with backoff, check OpenAI status      |
| `model_not_found`         | Model doesn't exist        | Check model name and availability            |

### SerpAPI Error Codes

| Error Code | Meaning                    | Solution                                     |
|------------|----------------------------|----------------------------------------------|
| 400        | Invalid parameters         | Check search parameters                      |
| 401        | Invalid API key            | Verify API key                               |
| 429        | Rate limit exceeded        | Implement backoff, reduce request frequency  |
| 500        | Internal server error      | Retry with backoff                           |

## Common Implementation Issues

### Creating a New Client for Each Request

**Problem**: Creating a new client for every request causes performance issues

**Bad Practice**:
```rust
// Don't do this
async fn handle_request(prompt: &str) -> Result<String> {
    let client = tool_sdk::openai_client().api_key(get_api_key()).build()?;
    client.simple_completion(prompt, None).await
}
```

**Good Practice**:
```rust
// Create once and reuse
struct MyService {
    client: Arc<OpenAIClient>,
}

impl MyService {
    fn new(api_key: String) -> Result<Self> {
        let client = tool_sdk::openai_client().api_key(api_key).build()?;
        Ok(Self {
            client: Arc::new(client),
        })
    }
    
    async fn handle_request(&self, prompt: &str) -> Result<String> {
        self.client.simple_completion(prompt, None).await
    }
}
```

### Not Handling Rate Limits Properly

**Problem**: Rate limits cause cascading failures

**Bad Practice**:
```rust
// No handling for rate limits
match client.chat_completion(request).await {
    Ok(response) => handle_response(response),
    Err(e) => {
        log::error!("Request failed: {}", e);
        return Err(MyError::ApiError);
    }
}
```

**Good Practice**:
```rust
match client.chat_completion(request).await {
    Ok(response) => handle_response(response),
    Err(e) if matches!(e, ServiceError::RateLimit(_)) => {
        let retry_after = extract_retry_after(&e).unwrap_or(5);
        log::warn!("Rate limited, retrying after {} seconds", retry_after);
        tokio::time::sleep(Duration::from_secs(retry_after)).await;
        retry_request(request).await
    },
    Err(e) => {
        log::error!("Request failed: {}", e);
        return Err(MyError::ApiError);
    }
}
```

### Ignoring Context in Errors

**Problem**: Error context is lost, making debugging difficult

**Bad Practice**:
```rust
// Context is lost
async fn process_data() -> Result<Data, MyError> {
    match client.fetch_data().await {
        Ok(data) => process(data),
        Err(_) => Err(MyError::FetchFailed),
    }
}
```

**Good Practice**:
```rust
async fn process_data() -> Result<Data, MyError> {
    match client.fetch_data().await {
        Ok(data) => process(data),
        Err(e) => {
            // Preserve context
            log::error!("Fetch failed with context: {:?}", e);
            
            // You can map to your error type while preserving info
            Err(MyError::FetchFailed {
                source: Box::new(e),
                operation: "fetch_data".to_string(),
            })
        }
    }
}
```

### Not Setting Appropriate Timeouts

**Problem**: Requests hang indefinitely

**Bad Practice**:
```rust
// Using default timeout which might be too long
let client = tool_sdk::openai_client()
    .api_key(api_key)
    .build()?;
```

**Good Practice**:
```rust
// Set appropriate timeouts
let client = tool_sdk::openai_client()
    .api_key(api_key)
    .timeout(30) // 30 seconds
    .build()?;
```

## Recovery Strategies

### Graceful Degradation

When a service is unavailable, fall back to alternative behavior:

```rust
match client.chat_completion(request).await {
    Ok(response) => handle_response(response),
    Err(e) => {
        log::warn!("API call failed: {}", e);
        // Fall back to local cache or simpler model
        provide_fallback_response()
    }
}
```

### Circuit Breaker with Fallback

```rust
let client = openai_client()
    .api_key(api_key)
    .circuit_breaker(CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: Duration::from_secs(30),
        ..CircuitBreakerConfig::default()
    })
    .build()?;

// Function with fallback
async fn get_completion_with_fallback(prompt: &str) -> String {
    match client.simple_completion(prompt, None).await {
        Ok(response) => response,
        Err(e) => {
            log::warn!("Using fallback due to error: {}", e);
            // Fallback to a predefined response
            "I'm sorry, I can't provide a response right now.".to_string()
        }
    }
}
```

### Retry with Progressive Backoff

```rust
async fn retry_with_backoff<F, Fut, T>(
    operation: F,
    max_retries: usize,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut retries = 0;
    let mut delay = Duration::from_millis(100);
    
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && retries < max_retries => {
                log::warn!("Retryable error: {}. Retry {}/{} in {:?}", 
                          e, retries + 1, max_retries, delay);
                
                tokio::time::sleep(delay).await;
                retries += 1;
                delay *= 2; // Exponential backoff
            },
            Err(e) => return Err(e),
        }
    }
}
```

## Conclusion

This guide covers the most common issues you might encounter when using the Tool SDK. If you're experiencing a problem not covered here, consider:

1. Enabling debug logging to gather more information
2. Checking the raw request and response data
3. Looking for similar issues in the project repository
4. Consulting the API documentation for the specific service
5. Reaching out to the PHOENIX ORCH project team for assistance

Remember that the Tool SDK is designed to handle many common failure scenarios automatically through its resilience patterns. However, understanding these issues and their solutions will help you use the SDK more effectively and troubleshoot problems when they arise.