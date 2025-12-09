# Logging Strategy for Phoenix Orchestrator

## Overview

This document outlines the comprehensive logging strategy for the Phoenix Orchestrator platform. The logging framework is designed to provide consistent, structured logs across all services, support correlation with distributed traces, and enable effective troubleshooting and monitoring.

## Logging Architecture

![Logging Architecture](https://mermaid.ink/img/pako:eNqNVMlu2zAQ_RWCS5HEjpM4adpbD0WDBij6OeTGkSORMEUqJGVbMfI1PfXT-pOlZG3NolxikTOcefPejJ4kQo0kR0lUKmVWdXllDu28QAZ5qX-x-KYYJmrWFZ7UuFKFVwo0whgLJbkxpfNVMZ8WcwK_V5wjUZlfcMuE9AvNxBtcWVRoFLkxWDLGJJBDrrHCjHOxrPbvU5Vc21uDXuv7cmnrM6aL5ILx2pEW4gd-QOeUcQs9QdYHWEqt0MnUKLS2tjDvl2LuT8Kfv_IfLQfE-hMTtSZ3QJdCORsVZXqmZaW9vDXMgTmwcKgxL1RaqgK7Nf0-HRSXB_p8lR5-sWaB_QE3z7lQTQNdNHCOhcbS8bpyA7ZWbO90rWO0N3Z0_eGr7vOvdnj3_nhnOuAhC-IYRKmbxsXKrDdHN1MQpHuGm7v5Y7e9ezcbx_jt-nKMZw87Z0SJUjY8aejTxcm7a6zrXSbHG8ljp7vZm26mjWM8P1xM8HIvl2Kd45cd4-1oL4GI_EEvxkZ5Eec9l2yh2U7SPcZfvKbswJrJm3unzWJFlfZjErsRxQGzO74zbYgmeXIv-DgMtUo5S2vrpxSxFXlCTpKj15mRLvJRNQw4HP0Dtt0Ir1D7YgZdKpcG-UJhMMLMGJMyX7BOxoIvdO7rPblQWGtxxHf3Wl-InEse_7-6KiXR-UQEMidihk1c3gy3Rl5jYPMvZQbFOg)

The Phoenix Orchestrator logging architecture consists of these core components:

1. **Service Log Generation**
   - Structured logging with consistent format
   - OpenTelemetry integration for correlation
   - Contextual enrichment
   - Standardized log levels

2. **Log Collection & Aggregation**
   - Centralized log collection with Loki
   - Log shipping via Promtail/Vector
   - Log retention policies

3. **Analysis & Visualization**
   - Grafana for log visualization and dashboards
   - Log-based alerts
   - Integration with trace and metrics views

## Structured Log Format

All services will use a consistent JSON-based structured log format:

```json
{
  "timestamp": "2025-12-09T18:30:45.123Z",
  "level": "INFO",
  "service": "orchestrator-service",
  "instance": "orchestrator-1",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "correlation_id": "abcdef-123456",
  "message": "Request processing started",
  "request_id": "req-123-456",
  "method": "plan_and_execute",
  "path": "/api/v1/process",
  "duration_ms": 45,
  "context": {
    "user_id": "anonymous",
    "client_ip": "10.0.1.123",
    "client_type": "web"
  }
}
```

### Required Fields

1. **Basic Information**
   - `timestamp`: ISO8601 UTC timestamp with millisecond precision
   - `level`: Log level (ERROR, WARN, INFO, DEBUG, TRACE)
   - `service`: Name of the service generating the log
   - `instance`: Specific instance identifier

2. **Correlation Information**
   - `trace_id`: OpenTelemetry trace identifier
   - `span_id`: OpenTelemetry span identifier
   - `correlation_id`: Service correlation identifier (for legacy systems)
   - `request_id`: Unique request identifier

3. **Context Information**
   - `message`: Human-readable log message
   - `method`: Operation being performed
   - `path`: API endpoint or operation path
   - `duration_ms`: Operation duration (when applicable)
   - `context`: Additional contextual information (JSON object)

### Service-Specific Fields

Services may add additional fields relevant to their domain. These should be documented in the service's documentation and follow the same naming conventions.

## Log Levels and Usage Guidelines

The Phoenix Orchestrator platform defines clear guidelines for when to use each log level:

### ERROR

Used for events that cause operations to fail and require immediate attention.

**Examples:**
- Unhandled exceptions
- Database connection failures
- API authentication failures
- Non-recoverable circuit breaker trips

**Usage Pattern:**
```rust
tracing::error!(
    error.code = %status_code,
    error.type = "database_failure",
    error.message = %e.to_string(),
    "Database connection failed during user query"
);
```

### WARN

Used for events that might cause problems but don't prevent the service from functioning.

**Examples:**
- Deprecation notices
- Expected exceptions that are handled
- Slow operations (exceeding thresholds)
- Features disabled due to issues
- Circuit breaker state changes

**Usage Pattern:**
```rust
tracing::warn!(
    circuit = %service_name,
    error_rate = %stats.window.failure_rate(),
    threshold = %self.config.error_threshold,
    "Circuit OPEN: Failure threshold exceeded"
);
```

### INFO

Used for normal operational events and service lifecycle events.

**Examples:**
- Service startup and shutdown
- Request processing (start/end)
- Configuration changes
- Scheduled task execution
- Normal business events

**Usage Pattern:**
```rust
tracing::info!(
    request_id = %req_data.id,
    service = %req_data.service, 
    method = %req_data.method,
    "Received PlanAndExecute request"
);
```

### DEBUG

Used for detailed information useful during development and troubleshooting.

**Examples:**
- Detailed API request/response content
- Algorithm decision steps
- Cache operations (hit/miss)
- Intermediate processing results

**Usage Pattern:**
```rust
tracing::debug!(
    cache_key = %key,
    cache_size = cache.len(),
    hit = hit_status,
    "Cache lookup completed"
);
```

### TRACE

Used for extremely detailed debugging information, typically only enabled during development.

**Examples:**
- Internal function calls with parameters
- Full data payloads
- Step-by-step execution flows
- Memory allocation information

**Usage Pattern:**
```rust
tracing::trace!(
    function = "process_vector_query",
    values = ?vector_values,
    dimension = vector_values.len(),
    "Processing vector query with data"
);
```

## Log Enrichment

Logs should be enriched with contextual information to maximize their value:

### 1. Request Context

```rust
fn log_with_request_context(req: &Request<Body>) -> tracing::Span {
    let span = tracing::info_span!(
        "http_request",
        method = %req.method(),
        path = %req.uri().path(),
        user_agent = ?req.headers().get("user-agent").map(|h| h.to_str().unwrap_or("")),
        content_length = ?req.headers().get("content-length").map(|h| h.to_str().unwrap_or("")),
        client_ip = ?get_client_ip(req),
    );
    
    span.entered()
}
```

### 2. Error Context

```rust
fn log_error_with_context<E: std::error::Error>(error: &E, context: &str) {
    let error_type = std::any::type_name::<E>();
    let error_message = error.to_string();
    
    // Get the error chain
    let mut error_chain = Vec::new();
    let mut current_error: Option<&dyn std::error::Error> = Some(error);
    
    while let Some(err) = current_error {
        error_chain.push(err.to_string());
        current_error = err.source();
    }
    
    tracing::error!(
        error.type = %error_type,
        error.message = %error_message,
        error.context = %context,
        error.chain = ?error_chain,
        "Error encountered during operation"
    );
}
```

### 3. Business Context

```rust
fn log_business_event(event_type: &str, context: HashMap<String, String>) {
    tracing::info!(
        business_event = %event_type,
        event.timestamp = %chrono::Utc::now().to_rfc3339(),
        event.context = ?context,
        "Business event occurred"
    );
}
```

## Integration with OpenTelemetry

The logging system will be integrated with OpenTelemetry to ensure logs can be correlated with traces and metrics:

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use opentelemetry::trace::{TraceContextExt, Tracer};

// Initialize the tracing subscriber with OpenTelemetry
fn init_tracing(service_name: &str) {
    // Create OpenTelemetry tracer
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(service_name)
        .install_simple()
        .unwrap();
    
    // Create a tracing layer with the configured tracer
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    
    // Create JSON formatter for logs
    let json_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true);
    
    // Create a subscriber with multiple layers
    tracing_subscriber::registry()
        .with(telemetry)
        .with(json_layer)
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    tracing::info!(service = %service_name, "Logging initialized with OpenTelemetry integration");
}
```

## Correlation ID Propagation

To ensure logs can be correlated across services, correlation IDs will be propagated:

```rust
// Extract correlation ID from request headers
fn extract_correlation_id(headers: &HeaderMap) -> Option<String> {
    // Try traceparent header first (OpenTelemetry W3C format)
    if let Some(traceparent) = headers.get("traceparent") {
        if let Ok(traceparent_str) = traceparent.to_str() {
            let parts: Vec<&str> = traceparent_str.split('-').collect();
            if parts.len() >= 2 {
                return Some(parts[1].to_string());
            }
        }
    }
    
    // Fall back to custom header
    headers.get("x-correlation-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

// Set correlation ID in thread-local storage
fn with_correlation_id<F, R>(correlation_id: &str, f: F) -> R
where
    F: FnOnce() -> R
{
    let span = tracing::info_span!("traced_operation", correlation_id = %correlation_id);
    span.in_scope(f)
}
```

## Log Collection and Aggregation

### Promtail Configuration

The Promtail agent will collect logs from all services:

```yaml
# promtail-config.yaml
server:
  http_listen_port: 9080
  grpc_listen_port: 0

positions:
  filename: /tmp/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: phoenix_logs
    static_configs:
      - targets:
          - localhost
        labels:
          job: phoenix_logs
          environment: production
          __path__: /var/log/phoenix/*/*log

    pipeline_stages:
      - json:
          expressions:
            timestamp: timestamp
            level: level
            service: service
            trace_id: trace_id
            span_id: span_id
            correlation_id: correlation_id
            message: message
            
      # Set timestamp from the parsed field
      - timestamp:
          source: timestamp
          format: RFC3339Nano
          
      # Add labels from parsed fields
      - labels:
          level:
          service:
          trace_id:
          span_id:
          correlation_id:
```

### Loki Configuration

```yaml
# loki-config.yaml
auth_enabled: false

server:
  http_listen_port: 3100
  grpc_listen_port: 9096

ingester:
  lifecycler:
    address: 127.0.0.1
    ring:
      kvstore:
        store: inmemory
      replication_factor: 1
    final_sleep: 0s
  chunk_idle_period: 5m
  chunk_retain_period: 30s

schema_config:
  configs:
    - from: 2020-10-24
      store: boltdb-shipper
      object_store: filesystem
      schema: v11
      index:
        prefix: index_
        period: 24h

storage_config:
  boltdb_shipper:
    active_index_directory: /loki/index
    cache_location: /loki/cache
    cache_ttl: 24h
    shared_store: filesystem
  filesystem:
    directory: /loki/chunks

limits_config:
  enforce_metric_name: false
  reject_old_samples: true
  reject_old_samples_max_age: 168h
  retention_period: 30d

compactor:
  working_directory: /loki/compactor
  shared_store: filesystem
  compaction_interval: 10m
  retention_enabled: true
  retention_delete_delay: 2h
  retention_delete_worker_count: 150

analytics:
  reporting_enabled: false
```

## Log Retention and Archiving

The platform will implement a tiered log retention strategy:

1. **Hot Storage (Loki)**
   - ERROR logs: 90 days
   - WARN logs: 60 days
   - INFO logs: 30 days
   - DEBUG logs: 7 days (non-production)
   - TRACE logs: 3 days (non-production)

2. **Cold Storage (Object Storage)**
   - ERROR logs: 1 year
   - WARN logs: 180 days
   - Other logs: Based on compliance requirements

3. **Archival Strategy**
   - Daily log archives
   - Indexed by service, date, and log level
   - Compressed storage format
   - Regular backup verification

## Service Implementation Example

### Core Logging Module

```rust
// logging.rs

use tracing::{Subscriber, Level};
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, Registry};
use tracing_opentelemetry as tracing_otel;
use opentelemetry::sdk::trace as otel_trace;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub service_name: String,
    pub log_level: String,
    pub json_format: bool,
    pub include_trace_info: bool,
    pub include_source_info: bool,
    pub file_output: Option<String>,
    pub otlp_endpoint: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            service_name: "unknown-service".to_string(),
            log_level: "info".to_string(),
            json_format: true,
            include_trace_info: true,
            include_source_info: true,
            file_output: None,
            otlp_endpoint: None,
        }
    }
}

pub fn init_logging(config: LoggingConfig) -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_new(&config.log_level)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Create basic subscriber
    let mut layers = Vec::new();

    // Format layer - JSON or plain text
    if config.json_format {
        let json_layer = fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .with_target(true)
            .with_context(move |_| {
                // Add service name and other default fields
                let mut fields = std::collections::HashMap::new();
                fields.insert("service".to_string(), config.service_name.clone());
                fields.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
                fields
            });
        layers.push(json_layer.boxed());
    } else {
        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true);
        layers.push(fmt_layer.boxed());
    }
    
    // Add OpenTelemetry if configured
    if let Some(endpoint) = config.otlp_endpoint {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint)
            )
            .with_trace_config(
                otel_trace::config()
                    .with_resource(opentelemetry::sdk::Resource::new(vec![
                        opentelemetry::KeyValue::new("service.name", config.service_name.clone()),
                    ]))
            )
            .install_batch(opentelemetry::runtime::Tokio)?;
        
        let otel_layer = tracing_otel::layer().with_tracer(tracer);
        layers.push(otel_layer.boxed());
    }
    
    // File output if needed
    if let Some(file_path) = config.file_output {
        use tracing_appender::rolling::{RollingFileAppender, Rotation};
        
        let file_appender = RollingFileAppender::new(
            Rotation::DAILY,
            std::path::Path::new(&file_path).parent().unwrap_or_else(|| std::path::Path::new(".")),
            format!("{}.log", config.service_name),
        );
        
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        
        // Keep the guard alive for the lifetime of the program
        Box::leak(Box::new(_guard));
        
        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false);
            
        layers.push(file_layer.boxed());
    }
    
    // Build and set the subscriber
    let subscriber = Registry::default()
        .with(env_filter);
    
    let subscriber = layers.into_iter().fold(subscriber, |subscriber, layer| {
        subscriber.with(layer)
    });
    
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
```

### Service Initialization with Logging

```rust
// main.rs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging first
    let logging_config = LoggingConfig {
        service_name: "orchestrator-service".to_string(),
        log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        json_format: true,
        include_trace_info: true,
        include_source_info: true,
        file_output: Some("/var/log/phoenix/orchestrator".to_string()),
        otlp_endpoint: std::env::var("OTEL_COLLECTOR_ENDPOINT").ok(),
    };
    
    logging::init_logging(logging_config)?;
    
    // Log startup
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Orchestrator service starting up"
    );
    
    // ... rest of service initialization
    
    Ok(())
}
```

### Middleware for HTTP Request Logging

```rust
// middleware.rs
pub async fn request_logging_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, std::convert::Infallible> {
    let path = req.uri().path().to_string();
    let method = req.method().to_string();
    
    // Extract correlation ID or generate new one
    let correlation_id = extract_correlation_id(req.headers())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    let start = std::time::Instant::now();
    
    // Create span for request context
    let span = tracing::info_span!("http_request",
        method = %method,
        path = %path,
        correlation_id = %correlation_id,
        user_agent = ?req.headers().get("user-agent").and_then(|h| h.to_str().ok()),
        remote_addr = ?get_remote_addr(&req),
    );
    
    tracing::info!(parent: &span, "Request started");
    
    // Process request inside span context
    let response = span.in_scope(|| next.run(req));
    let response = response.await;
    
    // Log completion
    let status = response.status().as_u16();
    let duration_ms = start.elapsed().as_millis() as u64;
    
    // Log differently based on status code
    if status >= 500 {
        tracing::error!(parent: &span,
            status = %status,
            duration_ms = %duration_ms,
            "Request failed with server error"
        );
    } else if status >= 400 {
        tracing::warn!(parent: &span,
            status = %status,
            duration_ms = %duration_ms,
            "Request failed with client error"
        );
    } else {
        tracing::info!(parent: &span,
            status = %status,
            duration_ms = %duration_ms,
            "Request completed successfully"
        );
    }
    
    Ok(response)
}
```

## Log Query Examples in Loki

These examples demonstrate how to query logs in Loki for common scenarios:

### 1. Trace Context Queries

```
{trace_id="4bf92f3577b34da6a3ce929d0e0e4736"} | json
```

### 2. Error Investigation

```
{level="ERROR"} |= "database connection" | json | line_format "{{.timestamp}} {{.message}}: {{.error_message}}"
```

### 3. Performance Analysis

```
{service="orchestrator-service"} | json | duration_ms > 1000 
| line_format "{{.timestamp}} {{.method}} {{.path}} took {{.duration_ms}}ms"
```

### 4. User Journey Analysis

```
{correlation_id="abcdef-123456"} | json 
| line_format "{{.timestamp}} [{{.service}}] {{.message}}"
| sort
```

## Log-Based Alerting Rules

Alerts based on log content will be configured in Loki:

```yaml
groups:
  - name: phoenix_log_alerts
    rules:
      - alert: HighErrorRate
        expr: |
          sum(count_over_time({level="ERROR"}[5m])) by (service)
          > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate in {{ $labels.service }}"
          description: "{{ $labels.service }} has more than 10 errors in the last 5 minutes"

      - alert: AuthenticationFailures
        expr: |
          sum(count_over_time({level=~"ERROR|WARN"} |= "authentication failed" [15m])) by (service)
          > 20
        for: 10m
        labels:
          severity: critical
        annotations:
          summary: "Multiple authentication failures detected"
          description: "{{ $labels.service }} has {{ $value }} authentication failures in the last 15 minutes"

      - alert: CircuitBreakerTripped
        expr: |
          sum(count_over_time({level="WARN"} |= "Circuit OPEN" [5m])) by (service, circuit)
          > 0
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "Circuit breaker tripped in {{ $labels.service }}"
          description: "Circuit {{ $labels.circuit }} in service {{ $labels.service }} has tripped"
```

## Log Analysis Dashboards

Key log analysis dashboards will include:

1. **Operational Health Dashboard**
   - Error rate by service
   - Log volume anomalies
   - Circuit breaker status
   - Authentication failures

2. **Service Performance Dashboard**
   - Slow request logs visualization
   - Error trends over time
   - 4xx vs 5xx error comparison

3. **Security Monitoring Dashboard**
   - Authentication and authorization logs
   - Access pattern anomalies
   - Sensitive operations audit

4. **Business Insights Dashboard**
   - User activity logs
   - Feature usage patterns
   - Error impact on business operations

## Implementation Plan

1. **Phase 1: Core Infrastructure**
   - Set up Loki and Promtail/Vector
   - Configure log retention policies
   - Implement basic log collection

2. **Phase 2: Service Instrumentation**
   - Enhance error-handling-rs with structured logging
   - Add middleware for HTTP/gRPC contexts
   - Standardize log formats

3. **Phase 3: Integration with Tracing**
   - Add correlation IDs to all logs
   - Link logs to traces
   - Build combined dashboards

4. **Phase 4: Advanced Features**
   - Implement log-based alerts
   - Set up archival system
   - Develop advanced log analysis techniques