# Distributed Tracing Implementation for Phoenix Orchestrator

## Overview

This document details the implementation design for adding distributed tracing to the Phoenix Orchestrator platform using OpenTelemetry. The design focuses on:

1. Enabling correlation IDs across service boundaries
2. Implementing sampling strategies for production traffic
3. Setting up the necessary infrastructure components
4. Standardizing instrumentation across the codebase

## Architecture

![Distributed Tracing Architecture](https://mermaid.ink/img/pako:eNqNVEtv2zAM_iuETgXSIU3WddsOO2zABgzYrdu6U7EDLdGxUFn0JCrNgvz3UU7SvTDsEkQfP34fSZ3JwmokmdSYG79sMI8Kj3JFlnvVPCyUbhrwlX8d4Y3HRmtSDI4a3XljNAgcofFkK1cYR6Yvkf-uQHuPZEUwW-FJ5W2OFJEjFe4E2rLGZvEGXn3xrq0RrCOPTzRqYT1W-6VEb4t2ZdaJd2vZE5sLl0mzYz-qdZEzWXw2La9VchqsgPfJYMY1SsXcI1fgkt0HsKbz6Ezih47vmqoVaJTXhTy_PJ0Oc26o8TgZTyePXpvxeIzznw_bTFiQDZIzGtPXeAKyKJr2DZMmb110P5l3KfbmfMe7H9LgdpucaYW-J5E4X-TKWwm5q9CUeIGTYi8iO1TK8ZiLtpNcbuKCtbgu7_WDtF1-PN4-hHtX-Y7zygov1HCltql2LXpptXvX3LV8ayTm-OP-9IC_f_5yPF70UcZxomieZnI9wWGIF6f1JmLyIXc8wTZpWMt7cU67fQHWH8b2-3D7eDk5vy5UYwVuK2gYkv0PY5P85wZmjsF1Ut6ztG98hKMV1Kyw3OJRnVHFO2X91Z5yc6LjsQlQ02kpfQHmbZ9KvxS4fpOL3LJ0ktMqOUTEbYYOSF4pvsQmfCXFqzjSLHl1jDEufYg7LzPG0MppjC1LGXZGn07uMDhV1qrFD0O-1o5rPn7Mre4Yj0H5vZRcJ7OuaOe_rDQDfnr8BxMJrso)

## Implementation Components

### 1. OpenTelemetry Integration

#### Core Libraries

We'll introduce the following dependencies to all Rust services:

```toml
[dependencies]
opentelemetry = "0.19"
opentelemetry-jaeger = "0.18"
opentelemetry-otlp = "0.12"
tracing = "0.1"
tracing-opentelemetry = "0.19"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

#### Tracer Initialization

Each service will initialize the OpenTelemetry tracer during startup:

```rust
fn init_tracer() -> Result<opentelemetry::sdk::trace::Tracer, opentelemetry::trace::TraceError> {
    let service_name = std::env::var("SERVICE_NAME")
        .unwrap_or_else(|_| "unknown-service".to_string());
    
    // Configure the collector exporter
    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(std::env::var("OTEL_COLLECTOR_ENDPOINT")
            .unwrap_or_else(|_| "http://otel-collector:4317".to_string()));
    
    // Configure the tracer
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .with_trace_config(
            opentelemetry::sdk::trace::config()
                .with_resource(opentelemetry::sdk::Resource::new(vec![
                    opentelemetry::KeyValue::new("service.name", service_name),
                    opentelemetry::KeyValue::new("deployment.environment", 
                        std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string())),
                ]))
                .with_sampler(get_sampler())
        )
        .install_batch(opentelemetry::runtime::Tokio)
}

// Configure appropriate sampling based on environment
fn get_sampler() -> Box<dyn opentelemetry::sdk::trace::Sampler + Send + Sync + 'static> {
    let env = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    
    match env.as_str() {
        "production" => {
            // In production, sample 10% of normal traffic, all errors
            let probability_sampler = opentelemetry::sdk::trace::Sampler::ParentBased(
                Box::new(opentelemetry::sdk::trace::ProbabilityConfig::new(0.1))
            );
            Box::new(probability_sampler)
        },
        // For development/staging, sample everything
        _ => Box::new(opentelemetry::sdk::trace::Sampler::AlwaysOn),
    }
}
```

### 2. Correlation ID Propagation

We'll extend the existing correlation ID mechanism to integrate with OpenTelemetry trace context:

```rust
pub fn with_correlation_id<F, R>(correlation_id: Option<String>, f: F) -> R
where
    F: FnOnce() -> R,
{
    let current_cx = opentelemetry::Context::current();
    let span_cx = match correlation_id {
        Some(id) => {
            // Create span with provided correlation ID
            let mut builder = tracing::info_span!("traced_operation");
            builder = builder.record("correlation_id", &id);
            
            // Store the correlation ID in thread-local storage too for legacy systems
            set_correlation_id(id);
            
            builder.entered().context()
        },
        None => {
            // Extract from current span context if available
            let span = tracing::Span::current();
            let cx = span.context();
            if let Some(trace_id) = cx.span().span_context().trace_id() {
                let id = trace_id.to_string();
                set_correlation_id(id.clone());
                cx
            } else {
                // Generate new ID if none exists
                let id = generate_correlation_id();
                let span = tracing::info_span!("traced_operation", correlation_id = %id);
                span.entered().context()
            }
        },
    };
    
    // Execute with the span context
    opentelemetry::Context::with_current(span_cx, f)
}

// Extract correlation ID from incoming requests (HTTP/gRPC)
pub fn extract_correlation_id_from_request(headers: &HeaderMap) -> Option<String> {
    // Try W3C traceparent header first
    if let Some(trace_parent) = headers.get("traceparent") {
        if let Ok(trace_parent_str) = trace_parent.to_str() {
            // Parse W3C trace context format: 00-<trace-id>-<span-id>-<trace-flags>
            let parts: Vec<&str> = trace_parent_str.split('-').collect();
            if parts.len() >= 3 {
                return Some(parts[1].to_string());
            }
        }
    }
    
    // Fall back to custom correlation ID header
    headers.get("x-correlation-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

// Inject correlation ID into outgoing requests
pub fn inject_correlation_id_to_request(headers: &mut HeaderMap) {
    let ctx = opentelemetry::Context::current();
    let span = ctx.span();
    let span_context = span.span_context();
    
    if span_context.is_valid() {
        // Format as W3C trace context
        let trace_parent = format!(
            "00-{}-{}-{:02x}",
            span_context.trace_id(),
            span_context.span_id(),
            span_context.trace_flags().bits()
        );
        
        headers.insert("traceparent", trace_parent.parse().unwrap());
        
        // Also set our custom header for backward compatibility
        if let Some(correlation_id) = current_correlation_id() {
            headers.insert("x-correlation-id", correlation_id.parse().unwrap());
        }
    }
}
```

### 3. Service Boundary Instrumentation

For each service boundary (HTTP, gRPC), we'll implement consistent tracing patterns:

#### gRPC Tracing Middleware (for tonic)

```rust
// gRPC server interceptor
pub struct TracingInterceptor;

impl tonic::service::Interceptor for TracingInterceptor {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        let metadata = request.metadata();
        
        // Extract correlation ID from metadata
        let correlation_id = if let Some(trace_parent) = metadata.get("traceparent") {
            if let Ok(trace_parent_str) = trace_parent.to_str() {
                let parts: Vec<&str> = trace_parent_str.split('-').collect();
                if parts.len() >= 3 {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else if let Some(corr_id) = metadata.get("x-correlation-id") {
            corr_id.to_str().ok().map(|s| s.to_string())
        } else {
            // Generate new ID if none provided
            Some(generate_correlation_id())
        };
        
        // Create span with correlation ID
        if let Some(id) = correlation_id.clone() {
            set_correlation_id(id);
        }
        
        let method = request.uri().path().to_string();
        tracing::info_span!("grpc_request", 
            %method,
            correlation_id = correlation_id.as_deref().unwrap_or("unknown"),
            otel.kind = "server",
        ).entered();
        
        Ok(request)
    }
}

// gRPC client middleware
pub struct TracingClientInterceptor;

impl<T: Send + 'static> tonic::client::Interceptor<T> for TracingClientInterceptor {
    fn call(&mut self, mut request: tonic::Request<T>) 
        -> Result<tonic::Request<T>, tonic::Status> {
        
        let method = request.uri().path().to_string();
        let span = tracing::info_span!("grpc_client_call", 
            %method,
            peer.service = request.uri().authority().map(|a| a.to_string()),
            otel.kind = "client",
        );
        
        // Inject correlation ID into metadata
        let metadata = request.metadata_mut();
        let ctx = opentelemetry::Context::current();
        let span_context = ctx.span().span_context();
        
        if span_context.is_valid() {
            let trace_parent = format!(
                "00-{}-{}-{:02x}",
                span_context.trace_id(),
                span_context.span_id(),
                span_context.trace_flags().bits()
            );
            
            metadata.insert("traceparent", 
                tonic::metadata::MetadataValue::from_str(&trace_parent).unwrap());
            
            // Also add correlation ID header for backward compatibility
            if let Some(id) = current_correlation_id() {
                metadata.insert("x-correlation-id", 
                    tonic::metadata::MetadataValue::from_str(&id).unwrap());
            }
        }
        
        span.entered();
        Ok(request)
    }
}
```

#### HTTP Tracing Middleware (for warp or other HTTP frameworks)

```rust
async fn trace_request(
    req: Request<Body>,
    next: Next,
) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path().to_string();
    let method = req.method().as_str().to_string();
    
    // Extract correlation ID from headers
    let correlation_id = extract_correlation_id_from_request(req.headers());
    
    // Create span with request information
    let span = tracing::info_span!("http_request",
        %method,
        %path,
        correlation_id = correlation_id.as_deref().unwrap_or("unknown"),
        http.method = %method,
        http.url = %path,
        http.flavor = ?req.version(),
        http.user_agent = ?req.headers().get("user-agent").and_then(|h| h.to_str().ok()),
        otel.kind = "server",
    );
    
    let _guard = span.enter();
    
    // Set correlation ID for this thread
    if let Some(id) = correlation_id {
        set_correlation_id(id);
    }
    
    // Execute the request
    let start = std::time::Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed();
    
    // Record response status
    let status = response.status().as_u16();
    tracing::info!(http.status_code = %status, duration_ms = %duration.as_millis(), "Request completed");
    
    Ok(response)
}

// Client tracing middleware for HTTP requests
pub async fn trace_http_client<F, Fut, T, E>(
    method: &str,
    url: &str,
    f: F,
) -> Result<T, E>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error + 'static,
{
    // Create span for the outgoing request
    let span = tracing::info_span!("http_client_request",
        http.method = %method,
        http.url = %url,
        otel.kind = "client",
    );
    
    // Execute the request within the span context
    let _guard = span.enter();
    let start = std::time::Instant::now();
    let result = f().await;
    let duration = start.elapsed();
    
    // Record result
    match &result {
        Ok(_) => {
            tracing::info!(duration_ms = %duration.as_millis(), "HTTP client request succeeded");
        }
        Err(e) => {
            tracing::error!(duration_ms = %duration.as_millis(), error = %e, "HTTP client request failed");
        }
    }
    
    result
}
```

### 4. Standard Function/Method Instrumentation

For key business logic, we'll implement standardized span creation:

```rust
// Template function for wrapping any business logic with tracing
pub async fn with_traced_operation<F, Fut, T, E>(
    operation_name: &str,
    attributes: Vec<(&str, &dyn std::fmt::Display)>,
    f: F,
) -> Result<T, E>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::error::Error + 'static,
{
    // Create a new span for this operation
    let mut span_builder = tracing::info_span!(operation_name);
    
    // Add all attributes to the span
    for (key, value) in attributes {
        span_builder = span_builder.record(key, format_args!("{}", value));
    }
    
    // Enter the span context
    let _span_guard = span_builder.enter();
    
    // Execute the operation
    let start = std::time::Instant::now();
    let result = f().await;
    let duration = start.elapsed();
    
    // Record result
    match &result {
        Ok(_) => {
            tracing::info!(duration_ms = %duration.as_millis(), "Operation succeeded");
        }
        Err(e) => {
            tracing::error!(duration_ms = %duration.as_millis(), error = %e, "Operation failed");
        }
    }
    
    result
}

// Example usage:
async fn process_request(req: Request) -> Result<Response, Error> {
    with_traced_operation(
        "process_user_request",
        vec![
            ("user_id", &req.user_id),
            ("request_type", &req.request_type),
        ],
        || async {
            // Business logic here
            Ok(Response { /* ... */ })
        }
    ).await
}
```

## Infrastructure Setup

### OpenTelemetry Collector

The collector will be deployed via Docker Compose:

```yaml
otel-collector:
  image: otel/opentelemetry-collector:latest
  container_name: otel-collector
  command: ["--config=/etc/otel/config.yaml"]
  volumes:
    - ./monitoring/otel-collector-config.yaml:/etc/otel/config.yaml
  ports:
    - "4317:4317"  # OTLP gRPC
    - "4318:4318"  # OTLP HTTP
    - "8888:8888"  # Prometheus metrics
    - "8889:8889"  # Health check extension
  networks:
    - agi_network
    - monitoring_network
```

### Collector Configuration

```yaml
# otel-collector-config.yaml
receivers:
  otlp:
    protocols:
      grpc:
      http:

processors:
  batch:
    timeout: 1s
    send_batch_size: 1024
  
  memory_limiter:
    check_interval: 1s
    limit_mib: 1000
  
  # Processor for adding environment info to all spans
  resource:
    attributes:
      - action: insert
        key: deployment.environment
        value: ${ENVIRONMENT}

exporters:
  jaeger:
    endpoint: jaeger:14250
    tls:
      insecure: true

  # Send metrics to Prometheus
  prometheusremotewrite:
    endpoint: "http://prometheus:9090/api/v1/write"
    tls:
      insecure: true
  
  # Send logs to Loki
  loki:
    endpoint: "http://loki:3100/loki/api/v1/push"
    tls:
      insecure: true

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [memory_limiter, batch, resource]
      exporters: [jaeger]
    
    metrics:
      receivers: [otlp]
      processors: [memory_limiter, batch, resource]
      exporters: [prometheusremotewrite]
    
    logs:
      receivers: [otlp]
      processors: [memory_limiter, batch, resource]
      exporters: [loki]

  telemetry:
    logs:
      level: "info"
```

### Jaeger for Trace Visualization

```yaml
jaeger:
  image: jaegertracing/all-in-one:latest
  container_name: jaeger
  ports:
    - "16686:16686"  # UI
    - "14250:14250"  # Model used by otel-collector
  environment:
    - COLLECTOR_OTLP_ENABLED=true
  networks:
    - monitoring_network
```

## Sampling Strategy

### Production Environment

1. **Core Services Sampling:**
   - Orchestrator: 100% of requests (critical path)
   - Data Router: 100% of requests (critical path) 
   - LLM Service: 50% of requests (high volume)
   - Other services: 10% of requests

2. **Error Sampling:**
   - 100% of error cases across all services

3. **Diagnostic Sampling:**
   - Ability to dynamically increase sampling rates for specific services or routes during debugging

### Development/Staging Environments

1. **All Services:** 100% sampling rate

### Sampling Implementation

```rust
fn configure_sampler() -> Box<dyn opentelemetry::sdk::trace::Sampler + Send + Sync + 'static> {
    let env = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    let service = std::env::var("SERVICE_NAME").unwrap_or_else(|_| "unknown".to_string());
    
    match env.as_str() {
        "production" => {
            match service.as_str() {
                "orchestrator-service" | "data-router" => {
                    // Critical path services: sample all requests
                    Box::new(opentelemetry::sdk::trace::Sampler::AlwaysOn)
                },
                "llm-service" => {
                    // High volume service: sample 50%
                    Box::new(opentelemetry::sdk::trace::ProbabilityComposite::new(
                        opentelemetry::sdk::trace::Sampler::ParentBased(
                            Box::new(opentelemetry::sdk::trace::ProbabilityConfig::new(0.5))
                        ),
                        // Always sample errors
                        Box::new(StatusErrorFilter),
                    ))
                },
                _ => {
                    // Default services: sample 10% + errors
                    Box::new(opentelemetry::sdk::trace::ProbabilityComposite::new(
                        opentelemetry::sdk::trace::Sampler::ParentBased(
                            Box::new(opentelemetry::sdk::trace::ProbabilityConfig::new(0.1))
                        ),
                        // Always sample errors
                        Box::new(StatusErrorFilter),
                    ))
                }
            }
        },
        // For non-production: sample everything
        _ => Box::new(opentelemetry::sdk::trace::Sampler::AlwaysOn),
    }
}

// Custom sampler that ensures errors are always sampled
struct StatusErrorFilter;

impl opentelemetry::sdk::trace::Sampler for StatusErrorFilter {
    fn should_sample(
        &self,
        _parent_context: Option<&opentelemetry::Context>,
        _trace_id: opentelemetry::trace::TraceId,
        _name: &str,
        attributes: &[opentelemetry::KeyValue],
    ) -> opentelemetry::sdk::trace::SamplingResult {
        // Check if this is an error response
        for kv in attributes {
            if kv.key.as_str() == "http.status_code" {
                if let Some(status) = kv.value.as_i64() {
                    if status >= 400 {
                        return opentelemetry::sdk::trace::SamplingResult::RecordAndSample;
                    }
                }
            } else if kv.key.as_str() == "error" {
                return opentelemetry::sdk::trace::SamplingResult::RecordAndSample;
            }
        }
        
        // Not an error, defer to other samplers
        opentelemetry::sdk::trace::SamplingResult::NotRecord
    }
}
```

## Request Flow Tracing Example

For a typical request flow through the system:

1. **Client → API Gateway**
   - Generate new trace ID and correlation ID
   - Create root span for the request

2. **API Gateway → Orchestrator**
   - Propagate trace context in gRPC metadata
   - Create child span for orchestrator call

3. **Orchestrator → Data Router → Services**
   - Each service adds its own spans
   - Metadata like service name, request info is attached to spans

4. **Service → Database/External Services**
   - Create child spans for external dependencies
   - Record database query info, external API calls

5. **Response Path**
   - Each service completes its span with status code
   - Records duration and any error information

## Visualization & Analysis

### Jaeger UI Dashboard

- Service dependency graphs
- Trace timeline views
- Span detail inspection
- Performance statistics

### Custom Dashboards in Grafana

- Latency heat maps for critical services
- Error rate by service/endpoint
- Trace volume trends
- Service dependency graphs

## Retention & Storage

- Standard traces: 7 days retention
- Error traces: 30 days retention
- High cardinality data sampling to control storage usage
- Automated archival to cold storage for historical analysis

## Implementation Plan

1. **Phase 1: Core Infrastructure**
   - Deploy OpenTelemetry Collector, Jaeger, Grafana
   - Configure sampling and retention policies

2. **Phase 2: Instrumentation**
   - Update the error-handling-rs crate to support OpenTelemetry
   - Add tracing middleware to service boundaries
   - Instrument core services: Orchestrator, Data Router

3. **Phase 3: Extend Coverage**
   - Instrument remaining services
   - Add custom business spans for key operations
   - Create visualizations and dashboards

4. **Phase 4: Validation & Tuning**
   - Load test to validate trace propagation
   - Optimize sampling rates based on traffic patterns
   - Fine-tune retention policies and storage usage