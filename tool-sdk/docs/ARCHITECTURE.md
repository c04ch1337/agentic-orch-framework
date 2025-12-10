# Tool SDK Architecture

This document provides a detailed explanation of the Tool SDK's architecture, including core abstractions, component relationships, and implementation patterns.

## Architecture Overview

The Tool SDK is designed as a layered architecture with clear separation of concerns:

```
┌─────────────────────────┐
│  Service-Specific APIs  │ ← OpenAI, SerpAPI, etc.
├─────────────────────────┤
│    Core Abstractions    │ ← ServiceClient, RequestExecutor, etc.
├─────────────────────────┤
│   Resilience Patterns   │ ← Retry, Circuit Breaker
├─────────────────────────┤
│     Error Handling      │ ← ServiceError, ErrorContext
├─────────────────────────┤
│   Config Management     │ ← ConfigProvider
└─────────────────────────┘
```

### Key Design Principles

1. **Trait-based Abstractions**: Core functionality is defined through traits, allowing for flexible implementation and composition
2. **Composable Resilience**: Resilience patterns can be composed and applied to any service client
3. **Rich Error Context**: Errors include detailed context for better troubleshooting
4. **Consistent Client Interface**: All service clients follow a unified interface pattern
5. **Type Safety**: Service-specific types ensure compile-time safety
6. **Builder Pattern**: Fluent interface for client configuration

## Core Abstractions

### ServiceClient

The `ServiceClient` trait is the foundational abstraction for all external service clients. It defines the basic properties and capabilities that all clients must provide.

```rust
#[async_trait]
pub trait ServiceClient: Send + Sync {
    /// The client name/identifier
    fn name(&self) -> &str;
    
    /// The base URL for the service
    fn base_url(&self) -> &str;
    
    /// Service version
    fn version(&self) -> &str;
    
    /// Health check for the service
    async fn health_check(&self) -> Result<bool>;
    
    /// Returns the client's metrics and telemetry if available
    fn metrics(&self) -> Option<HashMap<String, String>>;
}
```

**Responsibility**: Define a common interface for all service clients.

**Usage**: Implemented by all concrete service clients like `OpenAIClient` and `SerpAPIClient`.

### RequestExecutor

The `RequestExecutor` trait defines how service clients execute requests, handling the transport protocol details (typically HTTP).

```rust
#[async_trait]
pub trait RequestExecutor: Send + Sync {
    /// Execute a request that returns a response of type R
    async fn execute<T, R>(&self, endpoint: &str, request: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: DeserializeOwned + Send;
    
    /// Execute a GET request
    async fn get<R>(&self, endpoint: &str, query_params: Option<HashMap<String, String>>) -> Result<R>
    where
        R: DeserializeOwned + Send;
    
    // Other HTTP methods like post, put, delete...
}
```

**Responsibility**: Handle execution of requests to external services.

**Usage**: Implemented by service clients to manage HTTP requests and responses.

### AuthenticatedClient

The `AuthenticatedClient` trait adds authentication capabilities to service clients.

```rust
#[async_trait]
pub trait AuthenticatedClient: Send + Sync {
    /// Authentication type (e.g., "Bearer", "ApiKey")
    fn auth_type(&self) -> &str;
    
    /// Set authentication credentials
    fn set_auth(&mut self, auth: impl Into<String> + Send) -> Result<()>;
    
    /// Check if client is authenticated
    fn is_authenticated(&self) -> bool;
    
    /// Refresh authentication credentials if needed
    async fn refresh_auth(&mut self) -> Result<()>;
    
    /// Add authentication headers to a request
    fn apply_auth(&self, headers: &mut HashMap<String, String>) -> Result<()>;
}
```

**Responsibility**: Manage authentication for service clients.

**Usage**: Implemented by clients that require authentication, like most API clients.

### RateLimited

The `RateLimited` trait adds rate limiting capabilities to service clients.

```rust
#[async_trait]
pub trait RateLimited: Send + Sync {
    /// Get current rate limit status
    fn rate_limit_status(&self) -> Option<RateLimitStatus>;
    
    /// Set rate limiting configuration
    fn configure_rate_limit(&mut self, max_requests: u32, period: Duration);
    
    /// Check if a request would exceed rate limits
    async fn check_rate_limit(&self) -> Result<bool>;
    
    /// Record a request for rate limiting purposes
    fn record_request(&self);
}
```

**Responsibility**: Manage and enforce rate limits for service clients.

**Usage**: Implemented by clients that need to respect API rate limits.

### Telemetry

The `Telemetry` trait adds observability and metrics capabilities to service clients.

```rust
#[async_trait]
pub trait Telemetry: Send + Sync {
    /// Record a request event with timing
    fn record_request(&self, endpoint: &str, status: u16, duration: Duration);
    
    /// Record an error event
    fn record_error(&self, endpoint: &str, error: &str);
    
    /// Get current metrics
    fn metrics(&self) -> HashMap<String, String>;
    
    /// Reset metrics
    fn reset_metrics(&mut self);
}
```

**Responsibility**: Collect and expose metrics and telemetry data.

**Usage**: Implemented by clients that need to track metrics and observability data.

## Resilience Patterns

The SDK implements two primary resilience patterns:

### Retry

The `RetryExecutor` provides retry functionality with configurable backoff strategies.

```rust
pub struct RetryExecutor {
    config: RetryConfig,
}

pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_interval: Duration,
    pub max_interval: Duration,
    pub multiplier: f64,
    pub randomization_factor: f64,
    pub max_elapsed_time: Option<Duration>,
}
```

**Responsibility**: Retry failed operations with exponential backoff.

**Usage**: Used by the `Resilience` facade to handle transient failures.

### Circuit Breaker

The `CircuitBreaker` prevents cascading failures when a service is experiencing issues.

```rust
pub struct CircuitBreaker {
    status: RwLock<CircuitBreakerStatus>,
    opened_at: RwLock<Option<Instant>>,
    failure_count: AtomicUsize,
    success_count: AtomicUsize,
    config: CircuitBreakerConfig,
}

pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,
    pub reset_timeout: Duration,
    pub success_threshold: usize,
    pub sliding_window_size: usize,
    pub error_threshold_percentage: f64,
}
```

**Responsibility**: Prevent cascading failures by failing fast when a service is unhealthy.

**Usage**: Used by the `Resilience` facade to prevent overloading failing services.

### Resilience Facade

The `Resilience` struct provides a unified facade for composing resilience patterns.

```rust
pub struct Resilience {
    retry: RetryExecutor,
    circuit_breaker: Arc<CircuitBreaker>,
}
```

**Responsibility**: Compose multiple resilience patterns and apply them to operations.

**Usage**: Used by service clients to make their operations more resilient.

## Component Relationships

### Client Implementation Pattern

The typical service client follows this implementation pattern:

1. Implement the core `ServiceClient` trait to define basic properties
2. Implement `RequestExecutor` to handle HTTP or other transport requests
3. Implement `AuthenticatedClient` if authentication is needed
4. Implement `RateLimited` if rate limiting is required
5. Implement `Telemetry` for metrics and observability

Example with the OpenAI client:

```rust
pub struct OpenAIClient {
    http_client: Client,
    config: OpenAIConfig,
    resilience: Resilience,
    rate_limits: Arc<Mutex<Option<RateLimitStatus>>>,
    metrics: Mutex<HashMap<String, String>>,
}

impl ServiceClient for OpenAIClient { /* ... */ }
impl RequestExecutor for OpenAIClient { /* ... */ }
impl AuthenticatedClient for OpenAIClient { /* ... */ }
impl RateLimited for OpenAIClient { /* ... */ }
impl Telemetry for OpenAIClient { /* ... */ }
```

### Builder Pattern

Clients use the builder pattern for flexible configuration:

```
OpenAIClient Builder Pattern
┌────────────────────┐       ┌──────────────┐
│ OpenAIClientBuilder│──────→│ OpenAIClient │
└────────────────────┘       └──────────────┘
         ↑                          ↑
         │                          │
┌────────────────────┐       ┌──────────────┐
│  Factory Function  │──────→│Default Config│
└────────────────────┘       └──────────────┘
```

The builder pattern allows for:
- Optional parameters
- Default values
- Method chaining
- Validation before instantiation

## Data Flow

### Request/Response Flow

```
┌──────────┐    ┌───────────────┐    ┌───────────────┐    ┌──────────────┐
│  Client  │───→│Resilience Layer│───→│Request Executor│───→│External API  │
└──────────┘    └───────────────┘    └───────────────┘    └──────────────┘
     ↑                  ↑                    ↑                    │
     │                  │                    │                    │
     └──────────────────┴────────────────────┴────────────────────┘
                            Response Flow
```

1. Client code invokes a method on a service client
2. Request passes through the Resilience layer (retry/circuit breaker)
3. Request Executor adds authentication, builds HTTP request
4. HTTP request is sent to the external API
5. Response flows back through the same layers
6. Errors are enriched with context at each layer

### Error Handling Flow

```
┌──────────────┐    ┌───────────────┐    ┌────────────┐
│External Error│───→│Error Mapping  │───→│ServiceError│
└──────────────┘    └───────────────┘    └────────────┘
                           │                    ↓
                           │            ┌────────────────┐
                           └───────────→│Add Error Context│
                                        └────────────────┘
```

1. External API returns an error
2. Error is mapped to a `ServiceError` type
3. Context is added for debugging and tracing
4. Error is propagated back to the caller with rich context

## Extension Points

The SDK is designed to be extensible in several ways:

1. **New Service Clients**: New service integrations can be added by implementing the core traits
2. **Custom Resilience Patterns**: Additional resilience patterns can be integrated into the Resilience facade
3. **Error Mapping Extensions**: Service-specific error mapping can be added to normalize errors
4. **Configuration Extensions**: Custom configuration providers can be implemented

## Implementation Notes

### Thread Safety

All core abstractions are designed to be thread-safe, using `Send + Sync` bounds and appropriate synchronization primitives (Arc, Mutex, RwLock) where needed.

### Async Runtime

The SDK is designed to work with async Rust, using the `async_trait` crate for trait methods and `tokio` as the async runtime.

### Error Handling

Errors are handled using a custom `ServiceError` type that can be enriched with context, categorized by type, and checked for retryability.

## Memory Management

The SDK uses a mix of ownership, borrowing, and shared ownership (Arc) to manage memory efficiently:

1. **Client instances**: Typically owned by the application code
2. **Request objects**: Typically borrowed for execution
3. **Shared components**: Wrapped in Arc for concurrent access

## Configuration Management

Configuration is managed through the `ConfigProvider` trait, with implementations for:

1. **Environment variables**: `EnvConfigProvider`
2. **In-memory values**: Direct configuration via builder methods
3. **Service-specific configs**: Like `OpenAIConfig` and `SerpAPIConfig`

## Security Considerations

The SDK implements several security best practices:

1. **No hardcoded credentials**: All sensitive data is loaded from configuration
2. **Transport security**: Uses HTTPS by default for all requests
3. **Timeout protection**: Configurable timeouts for all requests
4. **Rate limiting**: Protection against accidental API abuse
5. **Error information protection**: Error details are available for debugging but sanitized for user display

## Performance Considerations

The SDK is designed for optimal performance:

1. **Connection pooling**: HTTP clients reuse connections
2. **Minimal allocations**: Borrows instead of clones where possible
3. **Efficient error handling**: No exceptions/unwinding
4. **Configurable timeouts**: Prevent hanging on slow services
5. **Metrics collection**: Performance can be monitored and optimized