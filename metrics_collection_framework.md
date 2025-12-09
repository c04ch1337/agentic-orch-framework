# Metrics Collection Framework for Phoenix Orchestrator

## Overview

This document details the metrics collection framework for the Phoenix Orchestrator platform, providing a standardized approach for monitoring and analyzing service performance, business metrics, and system health.

## Metrics Architecture

![Metrics Collection Architecture](https://mermaid.ink/img/pako:eNqNVEty2zAQvAqLLLKlrGQnceKsq1SqksqLs3LOCSRBEjEIMAgIyjHFg-Qo-cjJRIxEGYlTlWSJeT09PdMDepSZEkg7qYQ0-2KhJJ0cZM5FhuJxpfK8AL0Vz0O8FZgrwTrUuNKVU0qCwhH6SnKhtGv2LWtftEOVGw9O8EbiB3HfMTYkH9uazITZ2pMDr2wdcCUvHNKVk6Tpj2TvN0K50tS0A9_jw0INrWccz9z81ZXO3eBGcGvAknkMWzRKgTVZOARn8lk6UL5jE2XacG7Qzc1IIQW2RTRc4bHBMiDqlQHvLh_A0VHYUyhx-fRxs0n2r-53kzz39VDp7aBo73pYAQ3aMeOQC55V-7NJu5Md3h_pZm_5l634gC1-PQw7c2Bffc-sLSTNVVmWIWPp1h7LvGJyPPB1f77H6Z3dkf-82d28jYm8r7DmVE-xVNpV7qP7d9OyzwzHbf8Ky_vZz7q53bXDGD8fTiZwUZXnWYIvO8Y70V75U-JVAevlcpwXLzhOPjCBrU67ZrGbQkLiUTB2LYozTK54R9nwupE3jwZ1EG-nnK-9H1LPLvMSJ8nJ68yiavKsHgYKnvgHbFuJV2h8Md1rK9dGQ75XqE-wKI4LmRnaxVhwTc58Ge67lz4gLcVUXt9reyMFl_z8_-5FKZHe76hU5sj0sKnLm-HWmBsE5f8GnhX0GQ)

The metrics collection system is built on these core components:

1. **Application Metrics Generation**
   - Standardized instrumentation across all Rust services
   - Prometheus client libraries for exposing metrics
   - Consistent metric types and naming conventions

2. **Metrics Collection Infrastructure**
   - Prometheus server for scraping and storing metrics
   - OpenTelemetry collector for metrics forwarding and processing
   - Service discovery for dynamic target configuration

3. **Analysis and Visualization**
   - Grafana dashboards for visualization
   - AlertManager for notifications
   - Recording rules for complex metric calculations

## Standard Metric Types

### 1. Counters
Monotonically increasing values that represent cumulative counts.

**Naming Pattern:** `<namespace>_<subsystem>_<name>_total`

**Examples:**
- `phoenix_requests_total` - Total number of requests processed
- `phoenix_errors_total` - Total number of errors encountered
- `phoenix_llm_tokens_total` - Total number of tokens processed by the LLM service

### 2. Gauges
Values that can increase or decrease and represent a current state.

**Naming Pattern:** `<namespace>_<subsystem>_<name>`

**Examples:**
- `phoenix_connections_active` - Current number of active connections
- `phoenix_circuit_breaker_state` - Current state of circuit breakers (0=closed, 1=open, 2=half-open)
- `phoenix_kb_entries_count` - Number of entries in a knowledge base

### 3. Histograms
Observations bucketed by value ranges with count and sum.

**Naming Pattern:** `<namespace>_<subsystem>_<name>_<unit>`

**Examples:**
- `phoenix_request_duration_seconds` - Request duration in seconds
- `phoenix_response_size_bytes` - Response size in bytes
- `phoenix_llm_generation_duration_seconds` - LLM generation time

### 4. Summaries
Similar to histograms but with quantiles rather than buckets.

**Naming Pattern:** `<namespace>_<subsystem>_<name>_<unit>`

**Examples:**
- `phoenix_api_latency_seconds` - API request latency with quantiles

## Service-Specific KPIs

Each service in the Phoenix Orchestrator platform will expose metrics relevant to its function.

### Orchestrator Service

**Core Metrics:**
- Request volume: `phoenix_orchestrator_requests_total{method="<method>", status="<status>"}`
- Latency: `phoenix_orchestrator_request_duration_seconds{method="<method>"}`
- Plan and execute errors: `phoenix_orchestrator_errors_total{method="plan_and_execute", error_type="<type>"}`
- Circuit breaker status: `phoenix_orchestrator_circuit_breaker_state{service="<service>"}`

**Business Metrics:**
- Plans generated: `phoenix_orchestrator_plans_total{status="<status>"}`
- Ethics checks: `phoenix_orchestrator_ethics_checks_total{result="<allowed|denied>"}`
- Reflection operations: `phoenix_orchestrator_reflection_operations_total`

### Data Router Service

**Core Metrics:**
- Routing requests: `phoenix_data_router_requests_total{target_service="<service>", status="<status>"}`
- Routing latency: `phoenix_data_router_route_duration_seconds{target_service="<service>"}`
- Circuit breaker trips: `phoenix_data_router_circuit_breaker_trips_total{service="<service>"}`

**Business Metrics:**
- Service route distribution: `phoenix_data_router_routes_total{service="<service>"}`
- Language detection counts: `phoenix_data_router_language_detection_total{language="<language>"}`

### LLM Service

**Core Metrics:**
- Requests: `phoenix_llm_requests_total{model="<model>", status="<status>"}`
- Tokens processed: `phoenix_llm_tokens_total{model="<model>", operation="<operation>"}`
- Generation time: `phoenix_llm_generation_duration_seconds{model="<model>"}`
- Queue depth: `phoenix_llm_queue_depth{priority="<priority>"}`

**Business Metrics:**
- Cost tracking: `phoenix_llm_cost_dollars_total{model="<model>"}`
- Token usage ratio: `phoenix_llm_token_usage_ratio{model="<model>"}` (prompt tokens vs. completion tokens)
- Cache hit ratio: `phoenix_llm_cache_hit_ratio`

### Knowledge Base Services

**Core Metrics:**
- Queries: `phoenix_kb_queries_total{kb="<kb_name>", operation="<operation>", status="<status>"}`
- Query latency: `phoenix_kb_query_duration_seconds{kb="<kb_name>", operation="<operation>"}`
- Vector store operations: `phoenix_kb_vector_operations_total{operation="<operation>"}`

**Business Metrics:**
- KB sizes: `phoenix_kb_size_entries{kb="<kb_name>"}`
- Memory pruning operations: `phoenix_kb_pruning_operations_total{kb="<kb_name>"}`
- Vector similarity metrics: `phoenix_kb_similarity_score`

### Safety Service

**Core Metrics:**
- Validation requests: `phoenix_safety_requests_total{operation="<operation>", status="<status>"}`
- Processing time: `phoenix_safety_processing_duration_seconds{operation="<operation>"}`

**Business Metrics:**
- Blocked requests: `phoenix_safety_blocked_requests_total{reason="<reason>"}`
- Risk levels: `phoenix_safety_risk_level_total{level="<level>"}`
- False positives (when known): `phoenix_safety_false_positives_total`

### Executor Service

**Core Metrics:**
- Execution requests: `phoenix_executor_requests_total{status="<status>"}`
- Execution time: `phoenix_executor_duration_seconds{operation="<operation>"}`
- Container operations: `phoenix_executor_container_operations_total{operation="<operation>", status="<status>"}`

**Business Metrics:**
- Resource usage: `phoenix_executor_resource_usage{resource="<resource>", unit="<unit>"}`
- Sandbox violations: `phoenix_executor_sandbox_violations_total{type="<violation_type>"}`

## Implementation Patterns

### 1. Prometheus Client Integration

Each Rust service will use the `prometheus` crate for metrics exposure:

```toml
[dependencies]
prometheus = "0.13"
lazy_static = "1.4"
```

Standard initializations for metrics:

```rust
use lazy_static::lazy_static;
use prometheus::{register_counter, register_gauge, register_histogram, Counter, Gauge, Histogram};

// Define metrics at the module level
lazy_static! {
    static ref REQUESTS_TOTAL: Counter = register_counter!(
        "phoenix_service_requests_total",
        "Total number of requests received",
        &["method", "status"]
    ).unwrap();

    static ref ACTIVE_CONNECTIONS: Gauge = register_gauge!(
        "phoenix_service_connections_active",
        "Number of currently active connections"
    ).unwrap();
    
    static ref REQUEST_DURATION: Histogram = register_histogram!(
        "phoenix_service_request_duration_seconds",
        "Request duration in seconds",
        &["method"]
    ).unwrap();
}
```

### 2. Metrics Exposure Endpoint

Each service will expose a `/metrics` endpoint that Prometheus can scrape:

```rust
use prometheus::{Encoder, TextEncoder};
use warp::{Filter, Reply, Rejection};

// Metrics endpoint for Prometheus scraping
async fn metrics_handler() -> Result<impl Reply, Rejection> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    
    Ok(warp::reply::with_header(
        String::from_utf8(buffer).unwrap(),
        "content-type",
        "text/plain; version=0.0.4",
    ))
}

// Add to server routes
let metrics_route = warp::path("metrics").and_then(metrics_handler);
```

### 3. OpenTelemetry Integration

For services using OpenTelemetry, configure the Prometheus exporter:

```rust
use opentelemetry::metrics::MeterProvider;
use opentelemetry_prometheus::PrometheusExporter;

// Create a prometheus exporter pipeline
let prometheus_exporter = opentelemetry_prometheus::exporter()
    .with_registry(prometheus::Registry::new())
    .build();

// Create a new MeterProvider and configure it to use the exporter
let provider = opentelemetry_sdk::metrics::MeterProvider::builder()
    .with_reader(prometheus_exporter.clone())
    .build();

// Create a meter from the provider
let meter = provider.meter("my-service");

// Create instruments
let request_counter = meter
    .u64_counter("requests")
    .with_description("Number of requests")
    .init();
```

### 4. Standard Middleware for HTTP Services

Create middleware that automatically tracks request metrics:

```rust
async fn metrics_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<Response<hyper::Body>, std::convert::Infallible> {
    let path = req.uri().path().to_string();
    let method = req.method().as_str();
    
    // Start timer
    let start = std::time::Instant::now();
    
    // Track request
    REQUESTS_TOTAL.with_label_values(&[method, "in_progress"]).inc();
    ACTIVE_CONNECTIONS.inc();
    
    // Process request
    let response = next.run(req).await;
    
    // Record duration
    let duration = start.elapsed().as_secs_f64();
    REQUEST_DURATION.with_label_values(&[method]).observe(duration);
    
    // Record status
    let status = response.status().as_u16().to_string();
    REQUESTS_TOTAL.with_label_values(&[method, &status]).inc();
    
    // Decrement active connections
    ACTIVE_CONNECTIONS.dec();
    
    Ok(response)
}
```

### 5. Circuit Breaker Metrics Integration

Extend the existing circuit breaker with standardized metrics:

```rust
impl CircuitBreaker {
    fn record_metrics(&self, service_name: &str, state: CircuitState) {
        // Update gauge for circuit state
        static ref CIRCUIT_STATE: Gauge = register_gauge!(
            "phoenix_circuit_breaker_state",
            "Circuit breaker state (0=closed, 1=open, 2=half-open)",
            &["service"]
        ).unwrap();
        
        let state_value = match state {
            CircuitState::Closed => 0.0,
            CircuitState::Open => 1.0,
            CircuitState::HalfOpen => 2.0,
        };
        
        CIRCUIT_STATE.with_label_values(&[service_name]).set(state_value);
        
        // Record state transitions
        static ref CIRCUIT_TRANSITIONS: Counter = register_counter!(
            "phoenix_circuit_breaker_transitions_total",
            "Number of circuit breaker state transitions",
            &["service", "from_state", "to_state"]
        ).unwrap();
        
        if let Some(old_state) = self.previous_state(service_name) {
            if old_state != state {
                CIRCUIT_TRANSITIONS.with_label_values(&[
                    service_name,
                    old_state.as_str(),
                    state.as_str()
                ]).inc();
            }
        }
        
        // ... other circuit breaker metrics
    }
}
```

## Prometheus Configuration

### scrape_configs in prometheus.yml

```yaml
scrape_configs:
  # Service Discovery for Core Services
  - job_name: 'phoenix-services'
    scrape_interval: 15s
    metrics_path: '/metrics'
    dns_sd_configs:
      - names:
        - 'orchestrator-service'
        - 'data-router'
        - 'llm-service'
        - 'tools-service'
        - 'safety-service'
        - 'logging-service'
        - 'executor'
        - 'mind-kb'
        - 'body-kb'
        - 'heart-kb'
        - 'social-kb'
        - 'soul-kb'
        - 'context-manager'
        - 'reflection'
        type: 'A'
        port: 8080  # Metrics port

  # Node exporter for host metrics
  - job_name: 'node'
    scrape_interval: 10s
    static_configs:
      - targets: ['node-exporter:9100']

  # cAdvisor for container metrics
  - job_name: 'cadvisor'
    scrape_interval: 10s
    static_configs:
      - targets: ['cadvisor:8080']
```

### Recording Rules for Complex Metrics

```yaml
groups:
  - name: phoenix_service_rules
    rules:
    # Error rate recording rule
    - record: phoenix:error_rate:ratio_rate5m
      expr: sum(rate(phoenix_service_requests_total{status=~"5.."}[5m])) by (service) / sum(rate(phoenix_service_requests_total[5m])) by (service)
      
    # Apdex score (application performance index)
    - record: phoenix:apdex:ratio
      expr: (sum(rate(phoenix_service_request_duration_seconds_bucket{le="0.3"}[5m])) by (service) + sum(rate(phoenix_service_request_duration_seconds_bucket{le="1.2"}[5m])) by (service)) / 2 / sum(rate(phoenix_service_request_duration_seconds_count[5m])) by (service)
      
    # SLO: availability
    - record: phoenix:availability:ratio
      expr: sum(rate(phoenix_service_requests_total{status!~"5.."}[1h])) by (service) / sum(rate(phoenix_service_requests_total[1h])) by (service)
```

## Alert Rules

```yaml
groups:
  - name: phoenix_alerts
    rules:
    # High error rate alert
    - alert: HighErrorRate
      expr: phoenix:error_rate:ratio_rate5m > 0.05
      for: 5m
      labels:
        severity: warning
      annotations:
        summary: "High error rate on {{ $labels.service }}"
        description: "Error rate is {{ $value | humanizePercentage }} for the last 5 minutes (threshold: 5%)"

    # Circuit breaker open alert
    - alert: CircuitBreakerOpen
      expr: phoenix_circuit_breaker_state{} == 1
      for: 5m
      labels:
        severity: warning
      annotations:
        summary: "Circuit breaker open on {{ $labels.service }}"
        description: "Circuit breaker for {{ $labels.service }} has been open for at least 5 minutes"

    # Service response time degradation
    - alert: SlowResponseTime
      expr: histogram_quantile(0.95, sum(rate(phoenix_service_request_duration_seconds_bucket[5m])) by (service, le)) > 2
      for: 10m
      labels:
        severity: warning
      annotations:
        summary: "Slow response time on {{ $labels.service }}"
        description: "P95 response time for {{ $labels.service }} is above 2 seconds for 10 minutes"
```

## Service-Specific Implementation Examples

### 1. Orchestrator Service Metrics

```rust
// Orchestrator specific metrics
lazy_static! {
    static ref PLAN_GENERATION_DURATION: Histogram = register_histogram!(
        "phoenix_orchestrator_plan_generation_duration_seconds",
        "Time taken to generate an execution plan",
        prometheus::exponential_buckets(0.1, 2.0, 10).unwrap()
    ).unwrap();

    static ref ETHICS_CHECK_RESULTS: Counter = register_counter!(
        "phoenix_orchestrator_ethics_checks_total",
        "Total number of ethics checks performed",
        &["result"]
    ).unwrap();

    static ref PLAN_STEPS_COUNT: Histogram = register_histogram!(
        "phoenix_orchestrator_plan_steps",
        "Number of steps in generated plans",
        prometheus::linear_buckets(1.0, 1.0, 20).unwrap()
    ).unwrap();
}

// In plan_and_execute implementation
async fn plan_and_execute(&self, request: Request<ProtoRequest>) -> Result<Response<ProtoResponse>, Status> {
    let start_time = Instant::now();
    
    // ... existing implementation ...
    
    // Record plan generation time
    let plan_time = start_time.elapsed();
    PLAN_GENERATION_DURATION.observe(plan_time.as_secs_f64());
    
    // Record ethics check result
    let ethics_result = if ethics_resp.allowed { "allowed" } else { "denied" };
    ETHICS_CHECK_RESULTS.with_label_values(&[ethics_result]).inc();
    
    // ... rest of implementation ...
    
    // Count plan steps (example parsing JSON plan)
    if let Ok(plan) = serde_json::from_str::<Vec<serde_json::Value>>(&plan_text) {
        PLAN_STEPS_COUNT.observe(plan.len() as f64);
    }
    
    // ... complete implementation ...
}
```

### 2. LLM Service Metrics

```rust
// Token counting middleware for LLM requests
async fn track_token_usage(model: &str, prompt_tokens: usize, completion_tokens: usize) {
    static ref TOKEN_USAGE: Counter = register_counter!(
        "phoenix_llm_tokens_total",
        "Total tokens used by LLM service",
        &["model", "token_type"]
    ).unwrap();

    TOKEN_USAGE.with_label_values(&[model, "prompt"]).inc_by(prompt_tokens as f64);
    TOKEN_USAGE.with_label_values(&[model, "completion"]).inc_by(completion_tokens as f64);
    
    // Track cost (assuming pricing constants are defined elsewhere)
    static ref COST_DOLLARS: Counter = register_counter!(
        "phoenix_llm_cost_dollars_total",
        "Estimated cost of LLM API calls in dollars",
        &["model"]
    ).unwrap();
    
    let prompt_cost = prompt_tokens as f64 * MODEL_PROMPT_COST_PER_TOKEN.get(model).unwrap_or(&0.0);
    let completion_cost = completion_tokens as f64 * MODEL_COMPLETION_COST_PER_TOKEN.get(model).unwrap_or(&0.0);
    
    COST_DOLLARS.with_label_values(&[model]).inc_by(prompt_cost + completion_cost);
}
```

### 3. Knowledge Base Query Metrics

```rust
// Vector search metrics for KB services
async fn measure_vector_search(kb_name: &str, query_vector: &[f32], results: &SearchResults) {
    static ref VECTOR_SEARCH_DURATION: Histogram = register_histogram!(
        "phoenix_kb_vector_search_duration_seconds",
        "Time taken for vector similarity search",
        &["kb_name"]
    ).unwrap();
    
    static ref VECTOR_SEARCH_RESULTS: Histogram = register_histogram!(
        "phoenix_kb_search_results_count",
        "Number of results returned from KB searches",
        &["kb_name"]
    ).unwrap();
    
    static ref TOP_MATCH_SIMILARITY: Histogram = register_histogram!(
        "phoenix_kb_top_match_similarity",
        "Similarity score of the top match in vector search",
        &["kb_name"]
    ).unwrap();
    
    // Record the results
    VECTOR_SEARCH_RESULTS.with_label_values(&[kb_name])
        .observe(results.items.len() as f64);
        
    // Record top similarity if available
    if let Some(top_result) = results.items.first() {
        TOP_MATCH_SIMILARITY.with_label_values(&[kb_name])
            .observe(top_result.similarity);
    }
}
```

## Dashboards

The metrics above will feed into Grafana dashboards organized as follows:

### 1. System Overview Dashboard

- Service health status
- Error rates across all services
- Request volume 
- Resource utilization
- Circuit breaker status

### 2. Service Performance Dashboard

- Request latency histograms
- Throughput by service
- Error rates by endpoint
- Resource usage
- Database performance

### 3. Business Metrics Dashboard

- LLM token usage and cost
- Vector search effectiveness
- Plan generation statistics
- Safety checks statistics

### 4. SLO Monitoring Dashboard

- Availability metrics
- Latency SLO compliance
- Error budget consumption
- Apdex scores

## Custom Metadata Labels

To enhance metrics-based observability, we'll adopt consistent metadata labeling:

1. **Service Identification**
   - `service`: Service name
   - `instance`: Unique instance identifier
   - `version`: Service version

2. **Request Classification**
   - `method`: Operation being performed
   - `status`: Status code or result
   - `error_type`: Type of error when applicable

3. **Business Context**
   - `plan_type`: Type of execution plan
   - `model`: LLM model used
   - `kb_name`: Knowledge base name

## Implementation Plan

1. **Phase 1: Core Infrastructure**
   - Deploy Prometheus, Grafana, AlertManager
   - Configure service discovery
   - Set up baseline recording and alerting rules

2. **Phase 2: Service Instrumentation**
   - Add metrics to core services: Orchestrator, Data Router
   - Implement standardized middleware
   - Create baseline dashboards

3. **Phase 3: Advanced Metrics**
   - Add business-specific metrics
   - Develop SLO monitoring
   - Implement cost tracking

4. **Phase 4: Tuning & Optimization**
   - Review cardinality and storage requirements
   - Optimize recording rules
   - Fine-tune alert thresholds based on real traffic