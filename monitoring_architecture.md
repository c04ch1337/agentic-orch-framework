# Phoenix Orchestrator Monitoring & Observability Architecture

## Overall Architecture

The Phoenix Orchestrator monitoring architecture will be based on the three pillars of observability:

1. **Distributed Tracing** - For tracking request flows through the system
2. **Metrics Collection** - For monitoring system health and performance
3. **Logging** - For detailed event information and diagnostics

The architecture will use industry-standard tools and follow a consistent implementation pattern across all services:

![Architecture Diagram](https://mermaid.ink/img/pako:eNqNVMtu2zAQ_BViaAOkgF1bTdMePSRF0KBBe2mBXlo0B1paS2INPkhSjR3435tl5CQFCrQ-WdzlzHB2SJ9IahhSRirLfAOzLIZQ5JA1TF_PdVnXoBZuFuAF2NoKGrQnQ31uPLPgyJQWR1Lz1pNwJYjVG77NE1rJVUBNmpBbHuUfUjQlNaFnE1JwJhDqRIcU3gSLnI6T8l7jMjSdYZ7MhIqoWbIX1TqrUZw96obrKqrXXsHbqDfWjdIHVoKIYj-KLajadqiESojCmcgfwZnWgbWRHRcuKCiYxrwYw-fxeHDLhYzD-mEcb94a4_EIhp-XRULuroBaK-MXcCSEkI4_bI1fZOFQXQ8uqwFvVh2evvAGFlXUhYC-Sw88a4wE1R7TcTGGUdExwTuQ0vI06yTxsLENz3GOH-Q9k12c726Pw3Vfec5xqcAzrXmlV4Fto_SMoufWX9Vsa_ieCRjBt6vjPfz-9Wu3O-wiHEcQXUQ4PoeDPpwNyaXv7R7LqU_ldHrdIl6lG59_2ofLMZyTi2HKpOQRdtDqGw9YVOekoJWS_CmbdlmcX2RTdDQezlDFcclsDvJDm0i9Nki9zJDlUXiMjp6YPXN0jFaWTVmbN0z7LczoQEZD6hgzxpXBHUeqDqxBi9RZ7t-yjhQVLyXSfkuYY0XtrQRj26hTUuPKGJ2S1sHcuYx3aBwrUVJ57-i1tNSywcO75kZ_8MfEXDNt6LSVuvwrLJihfw3-A4vNjUo)

### Components

#### 1. Instrumentation Layer
- OpenTelemetry SDK integrated with all services
- Common instrumentation middleware for correlation ID propagation
- Standardized metric collectors with Prometheus integration
- Structured logging patterns with consistent formatting

#### 2. Collection Layer
- OpenTelemetry Collector for trace aggregation and processing
- Prometheus for metrics scraping and storage
- Loki for log aggregation and indexing

#### 3. Visualization Layer
- Grafana for dashboards and visualization
- Alertmanager for notifications
- Custom service health dashboards

#### 4. Analysis Layer
- Log-based anomaly detection
- Synthetic monitoring
- Alerting rules based on SLOs/SLIs

## Distributed Tracing Implementation

### Key Components

1. **OpenTelemetry Integration**
   - All services will use OpenTelemetry SDK for Rust (opentelemetry-rs)
   - Automatic instrumentation for HTTP, gRPC, and database connections
   - Manual instrumentation for critical business operations

2. **Correlation ID Propagation**
   - Use W3C Trace Context format for standard propagation
   - Ensure all service boundaries propagate trace context
   - Extend existing correlation ID infrastructure to support OpenTelemetry

3. **Sampling Strategy**
   - Production: Sample 10% of normal traffic, 100% of error cases
   - Development: Sample 100% of all traffic
   - Critical services: Sample 100% of traffic (Orchestrator, Data Router, LLM Service)

4. **Trace Processing**
   - OpenTelemetry Collector for processing and enrichment
   - Export to Jaeger for visualization and analysis
   - Retention policy: 7 days for normal traces, 30 days for error traces

## Metrics Collection Framework

### Metric Types & Categories

1. **Service-Level Metrics**
   - Request rates, error rates, duration (RED)
   - Success/failure counts by service and endpoint
   - Resource utilization (CPU, memory, connections)
   - Circuit breaker states and transitions

2. **Business-Level Metrics**
   - LLM service request counts and latencies
   - Knowledge base query performance
   - Tool execution success rates
   - Cache hit/miss ratios

3. **Infrastructure Metrics**
   - Node-level metrics (CPU, memory, disk, network)
   - Container metrics (resource usage, restart counts)
   - Network metrics (bandwidth, latency, error rates)

### Implementation

- Leverage existing metrics crate instrumentation
- Standardize naming conventions across services
- Define clear metric types (counters, gauges, histograms)
- Implement Prometheus exporters in all services
- Custom metrics for specific service behaviors

## Logging Framework

### Log Levels and Usage

1. **ERROR** - System errors requiring immediate attention
2. **WARN** - Potential issues that might need investigation
3. **INFO** - Normal operational events, service lifecycle events
4. **DEBUG** - Detailed information for troubleshooting (development)
5. **TRACE** - Very detailed debugging information (development only)

### Structured Logging Format

```json
{
  "timestamp": "2025-12-09T18:33:25Z",
  "level": "INFO",
  "service": "orchestrator-service",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "correlation_id": "abcdef-123456",
  "message": "Request processed successfully",
  "request_id": "req-123-456",
  "duration_ms": 45,
  "metadata": {
    "user_id": "anonymous",
    "http.method": "POST",
    "http.path": "/api/process"
  }
}
```

### Implementation

- Use tracing-opentelemetry for integration
- Centralized log collection with Loki
- Correlation between logs and traces via trace_id/span_id
- Log rotation and retention policies

## Health Check and Dashboard Design

### Health Check Endpoints

Each service will implement standardized health check endpoints:

1. **/health/liveness** - Basic aliveness check
2. **/health/readiness** - Ready to accept traffic
3. **/health/metrics** - Prometheus metrics endpoint
4. **/health/dependencies** - Status of dependent services

### Dashboard Templates

1. **System Overview Dashboard**
   - High-level view of all services
   - Key performance indicators
   - Alert status

2. **Service-Specific Dashboards**
   - Detailed metrics for each service
   - Request flows and performance
   - Error rates and types

3. **Business Metrics Dashboard**
   - User-centric metrics
   - LLM usage and performance
   - Knowledge base operations

4. **Infrastructure Dashboard**
   - Host and container metrics
   - Network performance
   - Resource utilization

## Alerting Framework

### Alert Levels

1. **Critical** - Immediate action required, services down
2. **Warning** - Degraded performance, potential issues
3. **Info** - Noteworthy events, no immediate action needed

### Alert Channels

- PagerDuty for critical alerts
- Slack for warnings and operational alerts
- Email for daily/weekly summaries

### Common Alert Thresholds

| Metric | Warning | Critical |
|--------|---------|----------|
| Service Error Rate | >1% for 5m | >5% for 3m |
| API Latency p95 | >500ms for 10m | >1s for 5m |
| LLM Request Failures | >2% for 5m | >10% for 3m |
| CPU Usage | >75% for 15m | >90% for 5m |
| Memory Usage | >80% for 15m | >90% for 5m |
| Disk Space | <20% free | <10% free |
| Circuit Breaker | Any open for >5m | Multiple open for >5m |

## Implementation Plan

1. **Initial Setup**
   - Deploy OpenTelemetry Collector and Jaeger
   - Configure Prometheus and Grafana
   - Set up Loki for log aggregation

2. **Instrumentation**
   - Add OpenTelemetry SDK to core services
   - Implement correlation ID propagation
   - Standardize metrics collection

3. **Dashboard Creation**
   - Develop template dashboards
   - Create service-specific views
   - Establish alert rules

4. **Testing and Validation**
   - Load testing to validate metrics
   - Chaos testing to verify alerting
   - End-to-end trace validation

## References

- [OpenTelemetry Rust Documentation](https://opentelemetry.io/docs/instrumentation/rust/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/naming/)
- [Grafana Dashboard Best Practices](https://grafana.com/docs/grafana/latest/best-practices/best-practices-for-creating-dashboards/)