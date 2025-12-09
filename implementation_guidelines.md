# Implementation Guidelines for Phoenix Orchestrator Monitoring

## Overview

This document provides practical implementation guidelines for the Phoenix Orchestrator monitoring and observability system. It outlines a phased approach, specific implementation steps, and operational recommendations to ensure successful deployment.

## Implementation Phases

The monitoring implementation is divided into logical phases to ensure incremental delivery of value:

![Implementation Phases](https://mermaid.ink/img/pako:eNqNVE1v2zAM_SuCTgWaIk2bbt1hh6HdMGDFMGC3XQqCliibsBz6Yyk1jPz3UU7ibVgLFDhRJB-fHh_FI2lCDQlJRpUw5qopl8_FSlGNRW3-J-K8EMRu124MXIBJL1Gj0nqNNViUQ9RCohFqoMqqJV2VuUXmzw0SpuyC3ORCOxQXB7yvkGvVTl2v8-UyLwgx39Bc2jlf6IaoE8ZGgxJMTnGPn4k1YGwlXrwZ0FmqDFFv9nqIeGpq1GwMilwvSsJLw_KcnO-2OcHfOxlCr9YTvv_Dl8_fyL8V_F2lqXO67whG2_YsG8iQQRY9QNp5WbRhP9kw1Xr0-Mxx25Gru_Qy-NlOHr48j0fqwlmUCa1BV61NUCRWaSu12Sp1Qv9rHxnPbfO0mf7-IHf382kHbJMGOzgZrm91WwTQsgcjZdHa8Ni7Y2jCpY5g0NvGpG0eTqamzQgXHUzgCmNtqTSbtfhX1jrRoLrKydtqhpnHqkTTWiVLB_qv_WkKXkrlORSKjkjIq1LNsXZnWSY5dllBxXEgI7ErsGRCCpL7KWnXuCK7LJNdPsQZjUvhUqxdZnmSZb3HIWjOtS5J6h8ys1KaHvQz1-TBOkm5lJWSL4Z8rgzVrP-ZW921HkOqPZXGpuOuaJd_Mwu19oD_ABG14Nc)

### Phase 1: Foundation (Weeks 1-2)

Set up the core monitoring infrastructure:

1. Deploy monitoring stack components:
   - Prometheus
   - OpenTelemetry Collector
   - Loki
   - Grafana
   - Jaeger

2. Implement core instrumentation library:
   - Create reusable instrumentation crates
   - Basic OpenTelemetry integration
   - Standard metric types and logging patterns

3. Instrument two high-priority services:
   - Orchestrator Service
   - Data Router Service

4. Create system overview dashboard

### Phase 2: Core Capabilities (Weeks 3-4)

Expand monitoring coverage:

1. Extend instrumentation to all critical services:
   - LLM Service
   - Knowledge Base Services
   - Executor Service
   - Safety Service

2. Implement correlation between traces, logs, and metrics

3. Create service-specific dashboards

4. Set up basic alerting rules

### Phase 3: Advanced Features (Weeks 5-6)

Add sophisticated monitoring capabilities:

1. Implement SLO monitoring and error budgets

2. Add business metrics and KPIs

3. Create specialized dashboards for key components

4. Implement comprehensive alerting strategy

### Phase 4: Optimization (Weeks 7-8)

Fine-tune the monitoring system:

1. Optimize sampling strategies based on production data

2. Tune alert thresholds and reduce noise

3. Implement automated dashboard generation

4. Documentation and operational procedures

## Technology Stack Requirements

### Core Components

| Component | Version | Purpose |
|-----------|---------|---------|
| Prometheus | 2.40+ | Metrics collection and storage |
| OpenTelemetry Collector | 0.70+ | Telemetry collection and processing |
| Loki | 2.7+ | Log aggregation and querying |
| Grafana | 9.3+ | Visualization and dashboarding |
| Jaeger | 1.41+ | Distributed tracing visualization |
| AlertManager | 0.25+ | Alert management and notification |

### Client Libraries

| Library | Version | Purpose |
|---------|---------|---------|
| opentelemetry | 0.19+ | OpenTelemetry SDK for Rust |
| opentelemetry-jaeger | 0.18+ | Jaeger exporter for OpenTelemetry |
| opentelemetry-otlp | 0.12+ | OTLP exporter for OpenTelemetry |
| opentelemetry-prometheus | 0.12+ | Prometheus exporter for OpenTelemetry |
| tracing | 0.1+ | Tracing framework for Rust |
| tracing-opentelemetry | 0.19+ | OpenTelemetry integration for tracing |
| prometheus | 0.13+ | Prometheus client for Rust |
| tracing-subscriber | 0.3+ | Subscriber for tracing |

## Monitoring Infrastructure Setup

### Docker Compose Configuration

Update the existing `docker-compose.monitoring.yml` file:

```yaml
version: '3.9'

services:
  # Prometheus for metrics collection
  prometheus:
    image: prom/prometheus:latest
    container_name: prometheus
    volumes:
      - ./monitoring/prometheus:/etc/prometheus
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.enable-lifecycle'
    ports:
      - "${PROMETHEUS_PORT:-9090}:9090"
    depends_on:
      - otel-collector
    networks:
      - monitoring_network
    restart: unless-stopped

  # OpenTelemetry Collector
  otel-collector:
    image: otel/opentelemetry-collector:latest
    container_name: otel-collector
    command: ["--config=/etc/otel/config.yaml"]
    volumes:
      - ./monitoring/otel-collector-config.yaml:/etc/otel/config.yaml
    ports:
      - "${OTEL_COLLECTOR_PORT:-4317}:4317"  # OTLP gRPC
      - "${OTEL_COLLECTOR_HTTP_PORT:-4318}:4318"  # OTLP HTTP
      - "8888:8888"  # Prometheus metrics
    networks:
      - agi_network
      - monitoring_network
    restart: unless-stopped

  # Loki for log aggregation
  loki:
    image: grafana/loki:latest
    container_name: loki
    volumes:
      - ./monitoring/loki-config.yaml:/etc/loki/config.yaml
      - loki_data:/loki
    command: -config.file=/etc/loki/config.yaml
    ports:
      - "${LOKI_PORT:-3100}:3100"
    networks:
      - monitoring_network
    restart: unless-stopped

  # Grafana for visualization
  grafana:
    image: grafana/grafana:latest
    container_name: grafana
    volumes:
      - ./monitoring/grafana/provisioning:/etc/grafana/provisioning
      - ./monitoring/grafana/dashboards:/var/lib/grafana/dashboards
      - grafana_data:/var/lib/grafana
    ports:
      - "${GRAFANA_PORT:-3000}:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_ADMIN_PASSWORD:-secure_password}
      - GF_USERS_ALLOW_SIGN_UP=false
      - GF_INSTALL_PLUGINS=grafana-prometheus-datasource,grafana-loki-datasource,grafana-jaeger-datasource
    depends_on:
      - prometheus
      - loki
      - jaeger
    networks:
      - monitoring_network
    restart: unless-stopped

  # Jaeger for distributed tracing
  jaeger:
    image: jaegertracing/all-in-one:latest
    container_name: jaeger
    ports:
      - "${JAEGER_UI_PORT:-16686}:16686"
      - "${JAEGER_COLLECTOR_PORT:-14250}:14250"
    environment:
      - COLLECTOR_OTLP_ENABLED=true
    depends_on:
      - otel-collector
    networks:
      - monitoring_network
    restart: unless-stopped
    
  # AlertManager for alerts
  alertmanager:
    image: prom/alertmanager:latest
    container_name: alertmanager
    volumes:
      - ./monitoring/alertmanager:/etc/alertmanager
      - alertmanager_data:/alertmanager
    command:
      - '--config.file=/etc/alertmanager/alertmanager.yml'
      - '--storage.path=/alertmanager'
    ports:
      - "${ALERTMANAGER_PORT:-9093}:9093"
    networks:
      - monitoring_network
    restart: unless-stopped

  # Vector for log collection
  vector:
    image: timberio/vector:latest
    container_name: vector
    volumes:
      - ./monitoring/vector.yaml:/etc/vector/vector.yaml
      - /var/log/phoenix:/var/log/phoenix:ro
      - /var/lib/docker/containers:/var/lib/docker/containers:ro
    command: ["--config", "/etc/vector/vector.yaml"]
    depends_on:
      - loki
    networks:
      - agi_network
      - monitoring_network
    restart: unless-stopped

volumes:
  prometheus_data:
  loki_data:
  grafana_data:
  alertmanager_data:

networks:
  monitoring_network:
    name: monitoring_network
    driver: bridge
  agi_network:
    external: true
```

### Configuration Files

Create the following configuration files:

1. **OpenTelemetry Collector Configuration**:
   
   ```bash
   mkdir -p monitoring/
   ```

   Create `monitoring/otel-collector-config.yaml`:

   ```yaml
   receivers:
     otlp:
       protocols:
         grpc:
         http:

   processors:
     batch:
       timeout: 1s
     
     memory_limiter:
       check_interval: 1s
       limit_mib: 1000
     
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

     prometheusremotewrite:
       endpoint: "http://prometheus:9090/api/v1/write"
       tls:
         insecure: true
     
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

2. **Prometheus Configuration**:

   Create `monitoring/prometheus/prometheus.yml`:

   ```yaml
   global:
     scrape_interval: 15s
     evaluation_interval: 15s

   rule_files:
     - "alerts/*.yml"
     - "recording_rules/*.yml"

   alerting:
     alertmanagers:
       - static_configs:
           - targets: ['alertmanager:9093']

   scrape_configs:
     - job_name: 'prometheus'
       static_configs:
         - targets: ['localhost:9090']

     - job_name: 'opentelemetry-collector'
       static_configs:
         - targets: ['otel-collector:8888']

     - job_name: 'phoenix-services'
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
           port: 8080

     - job_name: 'node-exporter'
       static_configs:
         - targets: ['node-exporter:9100']

     - job_name: 'cadvisor'
       static_configs:
         - targets: ['cadvisor:8080']
   ```

3. **AlertManager Configuration**:

   Create `monitoring/alertmanager/alertmanager.yml`:

   ```yaml
   global:
     resolve_timeout: 5m
     smtp_smarthost: 'smtp.example.com:587'
     smtp_from: 'alerts@example.com'
     smtp_auth_username: 'alerts@example.com'
     smtp_auth_password: 'password'

   route:
     group_by: ['alertname', 'service']
     group_wait: 30s
     group_interval: 5m
     repeat_interval: 4h
     receiver: 'slack'
     routes:
     - match:
         severity: critical
       receiver: 'pagerduty'
       continue: true

   receivers:
   - name: 'slack'
     slack_configs:
     - api_url: 'https://example.com/webhook-placeholder-url'
       channel: '#alerts'
       send_resolved: true
       title: >-
         [{{ .Status | toUpper }}] {{ .GroupLabels.alertname }}
       text: >-
         {{ range .Alerts }}
           *Alert:* {{ .Annotations.summary }}
           *Description:* {{ .Annotations.description }}
           *Severity:* {{ .Labels.severity }}
           *Service:* {{ .Labels.service }}
         {{ end }}

   - name: 'pagerduty'
     pagerduty_configs:
     - service_key: '<pagerduty-service-key>'
       send_resolved: true
   ```

4. **Vector Configuration**:

   Create `monitoring/vector.yaml`:

   ```yaml
   data_dir: /var/lib/vector

   sources:
     docker:
       type: docker_logs
       include_containers: 
         - "orchestrator-service"
         - "data-router-service"
         - "llm-service"
         - "tools-service"
         - "safety-service"
         - "executor-service"
         - "mind-kb"
         - "body-kb"
         - "heart-kb"
         - "social-kb"
         - "soul-kb"
       exclude_containers:
         - "vector"
         - "prometheus"
         - "grafana"
     
     file_logs:
       type: file
       include:
         - /var/log/phoenix/*/*.log

   transforms:
     parse_json:
       type: json_parser
       inputs:
         - docker
         - file_logs
       drop_invalid: false
       field: message
     
     extract_metadata:
       type: remap
       inputs:
         - parse_json
       source: |
         .service = .container_name || .service || "unknown"
         .level = .level || "INFO"
         .timestamp = .timestamp || now()
         # Extract trace/span IDs if available
         .trace_id = .trace_id
         .span_id = .span_id

   sinks:
     loki:
       type: loki
       inputs:
         - extract_metadata
       endpoint: http://loki:3100
       encoding:
         codec: json
       labels:
         service: "{{ service }}"
         level: "{{ level }}"
         trace_id: "{{ trace_id }}"
     
     console:
       type: console
       inputs:
         - extract_metadata
       encoding:
         codec: json
   ```

5. **Grafana Provisioning**:

   Create the Grafana provisioning configuration:

   ```bash
   mkdir -p monitoring/grafana/provisioning/datasources
   mkdir -p monitoring/grafana/provisioning/dashboards
   mkdir -p monitoring/grafana/dashboards
   ```

   Create `monitoring/grafana/provisioning/datasources/datasources.yml`:

   ```yaml
   apiVersion: 1

   datasources:
     - name: Prometheus
       type: prometheus
       access: proxy
       url: http://prometheus:9090
       isDefault: true
       editable: false
     
     - name: Loki
       type: loki
       access: proxy
       url: http://loki:3100
       editable: false
     
     - name: Jaeger
       type: jaeger
       access: proxy
       url: http://jaeger:16686
       editable: false
   ```

   Create `monitoring/grafana/provisioning/dashboards/dashboards.yml`:

   ```yaml
   apiVersion: 1

   providers:
     - name: 'Phoenix Orchestrator'
       folder: 'Phoenix Orchestrator'
       type: file
       disableDeletion: false
       updateIntervalSeconds: 30
       options:
         path: /var/lib/grafana/dashboards
         foldersFromFilesStructure: true
   ```

## Service Integration Steps

### 1. Create Instrumentation Libraries

Create reusable instrumentation libraries to standardize monitoring across services:

#### Tracing Library

Create a new crate `phoenix-telemetry` with the following structure:

```
phoenix-telemetry/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── tracing.rs
│   ├── metrics.rs
│   └── logging.rs
```

`Cargo.toml`:

```toml
[package]
name = "phoenix-telemetry"
version = "0.1.0"
edition = "2021"

[dependencies]
opentelemetry = "0.19"
opentelemetry-jaeger = "0.18"
opentelemetry-otlp = "0.12"
opentelemetry-prometheus = "0.12"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-opentelemetry = "0.19"
prometheus = "0.13"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["rt"] }
lazy_static = "1.4"
```

### 2. Service Integration Checklist

For each service, follow this integration checklist:

1. **Add Dependencies**

   Update the service's `Cargo.toml`:

   ```toml
   [dependencies]
   phoenix-telemetry = { path = "../phoenix-telemetry" }
   opentelemetry = "0.19"
   tracing = "0.1"
   tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
   ```

2. **Initialize Telemetry**

   In the service's `main.rs`:

   ```rust
   use phoenix_telemetry::{init_telemetry, TelemetryConfig};

   #[tokio::main]
   async fn main() -> Result<(), Box<dyn std::error::Error>> {
       let config = TelemetryConfig {
           service_name: "orchestrator-service".to_string(), // Change for each service
           otlp_endpoint: std::env::var("OTEL_COLLECTOR_ENDPOINT")
               .unwrap_or_else(|_| "http://otel-collector:4317".to_string()),
           log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
           // Other configuration options
           ..Default::default()
       };

       // Initialize telemetry
       init_telemetry(config)?;
       
       // Service startup code
       // ...
   }
   ```

3. **Add HTTP/gRPC Middleware**

   For HTTP servers (using warp/axum/hyper):

   ```rust
   use phoenix_telemetry::middleware::tracing_middleware;

   let routes = warp::path("endpoint")
       .and(warp::post())
       .and_then(handler)
       .with(tracing_middleware());
   ```

   For gRPC servers (using tonic):

   ```rust
   use phoenix_telemetry::middleware::grpc::TracingInterceptor;

   let service = MyServiceServer::with_interceptor(
       MyServiceImpl::default(),
       TracingInterceptor::default(),
   );
   ```

4. **Instrument Key Functions**

   Add instrumentation to important functions:

   ```rust
   use tracing::{instrument, info, error};

   #[instrument(skip(config), fields(config_name = %config.name))]
   async fn process_request(req_id: String, config: Config) -> Result<Response, MyError> {
       info!(request_id = %req_id, "Processing request");
       
       let start = std::time::Instant::now();
       
       // Function logic
       
       let duration = start.elapsed();
       info!(duration_ms = %duration.as_millis(), "Request processed");
       
       // Return result
   }
   ```

5. **Add Health Metrics Endpoint**

   Add a `/metrics` endpoint for Prometheus scraping:

   ```rust
   async fn metrics_handler() -> Result<impl Reply, Rejection> {
       use prometheus::{Encoder, TextEncoder};
       
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

   // Add to routes
   let metrics_route = warp::path("metrics").and_then(metrics_handler);
   ```

### 3. Service-Specific Integration

Apply specific instrumentation for key services:

#### Orchestrator Service

Focus on tracking plan execution and service routing:

```rust
// Create custom metrics for the orchestrator
lazy_static! {
    static ref PLAN_EXECUTION_DURATION: prometheus::HistogramVec = phoenix_telemetry::metrics::register_histogram_vec(
        "phoenix_orchestrator_plan_execution_duration_seconds",
        "Duration of plan execution in seconds",
        &["status"],
        Some(prometheus::exponential_buckets(0.05, 2.0, 10).unwrap())
    );

    static ref ROUTE_REQUESTS: prometheus::CounterVec = phoenix_telemetry::metrics::register_counter_vec(
        "phoenix_orchestrator_route_requests_total",
        "Number of route requests",
        &["service", "status"]
    );
}

// Use in plan_and_execute method
async fn plan_and_execute(&self, request: Request<ProtoRequest>) -> Result<Response<ProtoResponse>, Status> {
    let start = std::time::Instant::now();
    
    // Existing implementation with added tracing
    
    let status = if result.is_ok() { "success" } else { "failure" };
    PLAN_EXECUTION_DURATION.with_label_values(&[status]).observe(start.elapsed().as_secs_f64());
    
    result
}
```

#### Data Router Service

Focus on routing decisions and service performance:

```rust
// Create custom metrics for data router
lazy_static! {
    static ref ROUTE_LATENCY: prometheus::HistogramVec = phoenix_telemetry::metrics::register_histogram_vec(
        "phoenix_data_router_latency_seconds",
        "Latency of routing requests",
        &["target_service", "status"],
        Some(prometheus::exponential_buckets(0.001, 2.0, 10).unwrap())
    );
}
```

#### LLM Service

Focus on token usage, embedding generation, and model performance:

```rust
// Create custom metrics for LLM service
lazy_static! {
    static ref TOKEN_COUNT: prometheus::CounterVec = phoenix_telemetry::metrics::register_counter_vec(
        "phoenix_llm_tokens_total",
        "Number of tokens processed",
        &["model", "operation"]
    );
    
    static ref EMBEDDING_DIMENSIONS: prometheus::GaugeVec = phoenix_telemetry::metrics::register_gauge_vec(
        "phoenix_llm_embedding_dimensions",
        "Dimensions of generated embeddings",
        &["model"]
    );
}
```

## Testing and Validation Procedures

### 1. Unit Tests for Instrumentation

Create tests to verify instrumentation is working correctly:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::{Registry, Encoder, TextEncoder};

    #[test]
    fn test_metrics_creation() {
        // Create a registry
        let registry = Registry::new();
        
        // Register metrics
        let counter = phoenix_telemetry::metrics::register_counter_with_registry(
            "test_counter",
            "Counter for testing",
            &registry,
        ).unwrap();
        
        // Increment the counter
        counter.inc();
        
        // Gather and verify
        let metric_families = registry.gather();
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        
        assert!(output.contains("test_counter"));
        assert!(output.contains("Counter for testing"));
        assert!(output.contains("1"));
    }
}
```

### 2. Integration Tests

Create integration tests to verify end-to-end monitoring:

```rust
#[tokio::test]
async fn test_tracing_propagation() {
    // Setup tracing for test
    phoenix_telemetry::init_test_tracing();
    
    // Create a traced operation
    let span = tracing::info_span!("test_operation", correlation_id = "test-123");
    let _guard = span.enter();
    
    // Make a request to another service (mocked)
    let client = MockClient::new();
    let response = client.send_request("test data").await;
    
    // Verify headers contain trace context
    assert!(response.headers.contains_key("traceparent"));
    assert!(response.headers.contains_key("x-correlation-id"));
}
```

### 3. Load Testing with Observability

Create load tests to verify monitoring under load:

1. Use `vegeta` or `k6` to generate load
2. Create test script:

```js
// k6 test script
import http from 'k6/http';

export default function() {
  const url = 'http://localhost:8080/api/process';
  const payload = JSON.stringify({
    query: 'test query for monitoring',
    parameters: {}
  });
  
  const params = {
    headers: {
      'Content-Type': 'application/json',
      'X-Correlation-ID': `test-${__VU}-${__ITER}`
    }
  };
  
  const res = http.post(url, payload, params);
}
```

3. Run the test while watching metrics in Grafana

### 4. Dashboard Validation

Create a validation script for dashboards:

```python
#!/usr/bin/env python3
import requests
import json
import sys

GRAFANA_URL = "http://localhost:3000"
GRAFANA_API_KEY = "your-api-key"
HEADERS = {
    "Authorization": f"Bearer {GRAFANA_API_KEY}",
    "Content-Type": "application/json"
}

def validate_dashboard(dashboard_uid):
    # Get dashboard
    r = requests.get(f"{GRAFANA_URL}/api/dashboards/uid/{dashboard_uid}", headers=HEADERS)
    if r.status_code != 200:
        print(f"Failed to get dashboard {dashboard_uid}: {r.text}")
        return False
    
    dashboard = r.json()
    
    # Check for required panels
    panel_titles = [panel.get('title', '') for panel in dashboard.get('dashboard', {}).get('panels', [])]
    required_panels = ['Service Health', 'Error Rate', 'Request Volume']
    
    missing_panels = [panel for panel in required_panels if panel not in panel_titles]
    if missing_panels:
        print(f"Dashboard {dashboard_uid} missing required panels: {missing_panels}")
        return False
    
    return True

def main():
    dashboards = [
        "system-overview",
        "orchestrator-service", 
        "data-router-service"
    ]
    
    all_valid = True
    for dashboard in dashboards:
        if not validate_dashboard(dashboard):
            all_valid = False
    
    if not all_valid:
        sys.exit(1)
    
    print("All dashboards validated successfully")

if __name__ == "__main__":
    main()
```

## Operational Recommendations

### 1. Monitoring Your Monitoring

Set up monitoring for the monitoring stack itself:

- Health checks for Prometheus, Grafana, and OpenTelemetry Collector
- Alerts for monitoring infrastructure failure
- Backup procedures for Prometheus data

### 2. Log Management

- Implement log rotation to prevent disk space issues
- Create a log retention policy (e.g., ERROR logs: 90 days, INFO logs: 30 days)
- Schedule regular log pruning

### 3. Incident Response Integration

Integrate monitoring with incident response:

1. Create playbooks for common alert scenarios
2. Define on-call rotation and escalation procedures
3. Set up post-mortem templates that pull data from monitoring

### 4. Continuous Improvement

Establish a continuous improvement process:

1. Weekly review of alert noise/value
2. Monthly review of dashboard utility
3. Quarterly review of monitoring coverage
4. Adjust sampling rates and retention periods based on usage

## Documentation

### 1. User Documentation

Create user documentation for the monitoring system:

- Dashboard usage guides
- Alert interpretation guides
- Query examples for Prometheus and Loki
- Troubleshooting procedures

### 2. Developer Documentation

Create developer documentation for adding instrumentation:

- How to add metrics to new services
- Best practices for naming metrics
- How to add custom dashboards
- Testing instrumentation

### 3. Operational Documentation

Create operational documentation:

- Backup and restore procedures
- Scaling guidelines
- Version upgrade procedures
- Security considerations

## Common Pitfalls and Solutions

### 1. Cardinality Explosion

**Problem**: Too many unique combinations of labels, causing performance issues.

**Solution**:
- Limit the number of label values
- Use recording rules for high-cardinality metrics
- Monitor cardinality growth

### 2. Alert Fatigue

**Problem**: Too many alerts causing teams to ignore alerts.

**Solution**:
- Regularly review and tune alert thresholds
- Implement alert grouping and de-duplication
- Use different notification channels based on severity

### 3. Performance Impact

**Problem**: Instrumentation slowing down application performance.

**Solution**:
- Use sampling for high-volume services
- Optimize instrumentation code
- Batch metric updates

### 4. Data Retention Costs

**Problem**: Storing metrics and logs becoming expensive.

**Solution**:
- Implement tiered storage
- Adjust retention periods by importance
- Use aggregation for older data

## Implementation Checklist

Use the following checklist to track implementation progress:

### Phase 1: Foundation

- [ ] Deploy monitoring infrastructure
  - [ ] Prometheus
  - [ ] Loki
  - [ ] Grafana
  - [ ] Jaeger
  - [ ] OpenTelemetry Collector
  
- [ ] Create instrumentation libraries
  - [ ] Tracing library
  - [ ] Metrics library
  - [ ] Logging library

- [ ] Instrument first services
  - [ ] Orchestrator Service
  - [ ] Data Router Service
  
- [ ] Create initial dashboards
  - [ ] System Overview Dashboard

### Phase 2: Core Capabilities

- [ ] Extend instrumentation
  - [ ] LLM Service
  - [ ] Knowledge Base Services
  - [ ] Safety Service
  - [ ] Executor Service
  
- [ ] Create service-specific dashboards
  - [ ] Service Performance Dashboard
  - [ ] LLM Service Dashboard
  - [ ] KB Services Dashboard

- [ ] Set up alerting
  - [ ] Critical service alerts
  - [ ] Error rate alerts
  - [ ] Resource utilization alerts

### Phase 3: Advanced Features

- [ ] Implement SLO monitoring
  - [ ] SLO definitions
  - [ ] Error budget tracking
  - [ ] SLO dashboards

- [ ] Create business dashboards
  - [ ] Business Impact Dashboard
  - [ ] Cost Tracking Dashboard

- [ ] Advanced alerting
  - [ ] PagerDuty integration
  - [ ] Alert routing
  - [ ] Escalation policies

### Phase 4: Optimization

- [ ] Tune sampling strategies
- [ ] Optimize alert thresholds
- [ ] Create runbooks and documentation
- [ ] Train team on monitoring usage

## Conclusion

This implementation guide provides a comprehensive approach to deploying the Phoenix Orchestrator monitoring and observability system. By following the phased approach and integration steps outlined in this document, the engineering team can successfully implement a robust monitoring solution that provides valuable insights into system behavior and performance.

The key to successful implementation is the consistent application of instrumentation patterns across services and the iterative improvement of dashboards and alerting rules based on real-world usage.