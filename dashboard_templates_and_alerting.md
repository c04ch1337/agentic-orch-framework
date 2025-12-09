# Dashboard Templates and Alerting Thresholds for Phoenix Orchestrator

## Overview

This document provides dashboard templates and alerting threshold configurations for the Phoenix Orchestrator platform, designed to provide comprehensive visibility and proactive incident response capabilities.

## Dashboard Philosophy

The Phoenix Orchestrator dashboard system follows these core principles:

1. **Hierarchy of Information** - Dashboards are organized from high-level overview to detailed service-specific views
2. **Contextual Correlation** - Metrics, logs, and traces are linked for seamless investigation
3. **Business Impact Focus** - Technical metrics are tied to business outcomes
4. **Actionable Insights** - Dashboards provide clear visualization of what's wrong and potential remediation steps

## Standard Dashboard Layout

All dashboards follow a consistent layout pattern:

```
┌─────────────────────────────────────┐
│ Service/System Status at a Glance   │
├──────────────┬──────────────────────┤
│              │                      │
│ Key Metrics  │ Time Series Charts   │
│              │                      │
├──────────────┴──────────────────────┤
│ Detail Panels (Traffic/Errors/etc.) │
├─────────────────────────────────────┤
│ Logs & Events Timeline              │
└─────────────────────────────────────┘
```

## Dashboard Templates

### 1. System Overview Dashboard

**Purpose**: High-level view of the entire Phoenix Orchestrator platform

**Key Panels**:

* **Service Health Matrix**
  - All services with health status (green/yellow/red)
  - Link to service-specific dashboards
  - Immediate visualization of problem areas

* **Key Performance Indicators**
  - Request Rate: `sum(rate(phoenix_requests_total[5m])) by (service)`
  - Error Rate: `sum(rate(phoenix_errors_total[5m])) by (service) / sum(rate(phoenix_requests_total[5m])) by (service)`
  - 95th Percentile Latency: `histogram_quantile(0.95, sum(rate(phoenix_request_duration_seconds_bucket[5m])) by (service, le))`
  - Active Circuits: `count(phoenix_circuit_breaker_state{} > 0)`

* **Resource Utilization**
  - CPU Usage: `sum(rate(process_cpu_seconds_total[5m])) by (service)`
  - Memory Usage: `process_resident_memory_bytes{} by (service)`
  - Goroutine Count: `go_goroutines{} by (service)`
  - Open File Descriptors: `process_open_fds{} by (service)`

* **Traffic Overview**
  - Request Volume by Service: `sum(rate(phoenix_requests_total[5m])) by (service)`
  - Success/Failure Ratio: `sum(rate(phoenix_requests_total{status=~"2.."}[5m])) by (service) / sum(rate(phoenix_requests_total[5m])) by (service)`

* **Alert Status**
  - Current Active Alerts
  - Recent Alert History (24h)

**Example JSON for Grafana**:

```json
{
  "title": "Phoenix Orchestrator - System Overview",
  "panels": [
    {
      "title": "Service Health",
      "type": "stat",
      "datasource": "Prometheus",
      "targets": [
        {
          "expr": "sum(up{job=~\"phoenix-.*\"}) by (job)",
          "legendFormat": "{{job}}"
        }
      ],
      "options": {
        "colorMode": "value",
        "graphMode": "area",
        "justifyMode": "auto",
        "orientation": "horizontal",
        "reduceOptions": {
          "calcs": ["lastNotNull"],
          "values": false
        },
        "textMode": "auto",
        "links": [
          {
            "title": "Service Detail",
            "url": "/d/service-detail?var-service=${__field.label}"
          }
        ]
      },
      "fieldConfig": {
        "defaults": {
          "mappings": [
            {
              "type": "value",
              "options": {
                "0": { "text": "Down", "color": "red" },
                "1": { "text": "Up", "color": "green" }
              }
            }
          ],
          "thresholds": {
            "mode": "absolute",
            "steps": [
              { "color": "red", "value": null },
              { "color": "green", "value": 1 }
            ]
          },
          "color": { "mode": "thresholds" }
        }
      }
    }
    // Additional panels would follow the same pattern
  ],
  "refresh": "10s",
  "time": {
    "from": "now-6h",
    "to": "now"
  },
  "templating": {
    "list": [
      {
        "name": "environment",
        "type": "query",
        "datasource": "Prometheus",
        "query": "label_values(environment)"
      }
    ]
  }
}
```

### 2. Service-Specific Dashboard

**Purpose**: Detailed view of a specific service's health and performance

**Template Variables**:
- `service`: Service name selection
- `instance`: Instance selection (for multi-instance services)
- `interval`: Time range interval (5m, 1h, 6h, 24h)

**Key Panels**:

* **Service Status**
  - Uptime: `time() - process_start_time_seconds{service="$service"}`
  - Version: `phoenix_build_info{service="$service"}`
  - Health Status: `up{service="$service"}`

* **Traffic & Performance**
  - Request Rate: `sum(rate(phoenix_service_requests_total{service="$service"}[$interval])) by (method)`
  - Error Rate: `sum(rate(phoenix_service_requests_total{service="$service", status=~"[45].."}[$interval])) by (method) / sum(rate(phoenix_service_requests_total{service="$service"}[$interval])) by (method)`
  - Latency Heatmap: `sum(rate(phoenix_service_request_duration_seconds_bucket{service="$service"}[$interval])) by (le, method)`
  - Latency Percentiles: `histogram_quantile(0.5, sum(rate(phoenix_service_request_duration_seconds_bucket{service="$service"}[$interval])) by (le))` (repeat for 0.90, 0.95, 0.99)

* **Resources**
  - Memory Usage: `process_resident_memory_bytes{service="$service"}`
  - CPU Usage: `rate(process_cpu_seconds_total{service="$service"}[$interval])`
  - Goroutines: `go_goroutines{service="$service"}`

* **Circuit Breakers**
  - Circuit Status: `phoenix_circuit_breaker_state{service="$service"}`
  - Circuit Transitions: `rate(phoenix_circuit_breaker_transitions_total{service="$service"}[$interval])`

* **Top Requests**
  - Most Frequent Endpoints: `topk(10, sum(rate(phoenix_service_requests_total{service="$service"}[$interval])) by (path))`
  - Slowest Endpoints: `topk(10, histogram_quantile(0.95, sum(rate(phoenix_service_request_duration_seconds_bucket{service="$service"}[$interval])) by (path, le)))`
  - Highest Error Rate: `topk(10, sum(rate(phoenix_service_requests_total{service="$service", status=~"[45].."}[$interval])) by (path) / sum(rate(phoenix_service_requests_total{service="$service"}[$interval])) by (path))`

* **Logs Panel**
  - Error Logs: Loki query `{service="$service", level="ERROR"}`
  - Warning Logs: Loki query `{service="$service", level="WARN"}`

* **Traces Panel**
  - Recent Slow Traces: Jaeger query for service with duration > threshold
  - Recent Error Traces: Jaeger query for service with error=true

### 3. Knowledge Base Performance Dashboard

**Purpose**: Monitor the performance and health of the various knowledge base services

**Key Panels**:

* **Vector Database Performance**
  - Query Rate: `sum(rate(phoenix_kb_queries_total[$interval])) by (kb)`
  - Query Latency: `histogram_quantile(0.95, sum(rate(phoenix_kb_query_duration_seconds_bucket[$interval])) by (kb, le))`
  - Cache Hit Rate: `sum(rate(phoenix_kb_cache_hit_total[$interval])) by (kb) / sum(rate(phoenix_kb_queries_total[$interval])) by (kb)`

* **Vector Store Metrics**
  - Vector Count: `phoenix_kb_vector_count{}`
  - Index Size: `phoenix_kb_index_size_bytes{}`
  - Memory Usage: `process_resident_memory_bytes{service=~".*-kb"}`

* **Operation Breakdown**
  - Operations by Type: `sum(rate(phoenix_kb_operations_total[$interval])) by (operation, kb)`
  - Errors by Type: `sum(rate(phoenix_kb_errors_total[$interval])) by (error_type, kb)`
  - Pruning Operations: `rate(phoenix_kb_pruning_operations_total[$interval])`

* **Similarity Metrics**
  - Similarity Score Distribution: Histogram of similarity scores
  - Query Vector Dimension Analysis

### 4. LLM Service Dashboard

**Purpose**: Monitor LLM service performance, cost, and usage patterns

**Key Panels**:

* **Request Metrics**
  - Requests by Model: `sum(rate(phoenix_llm_requests_total[$interval])) by (model)`
  - Request Duration: `histogram_quantile(0.95, sum(rate(phoenix_llm_generation_duration_seconds_bucket[$interval])) by (model, le))`
  - Concurrent Requests: `phoenix_llm_concurrent_requests{}`

* **Token Usage**
  - Token Rate by Type: `sum(rate(phoenix_llm_tokens_total[$interval])) by (model, token_type)`
  - Token Distribution: `sum(phoenix_llm_tokens_total) by (model)`
  - Token Efficiency (Output/Input Ratio): `sum(phoenix_llm_tokens_total{token_type="completion"}) by (model) / sum(phoenix_llm_tokens_total{token_type="prompt"}) by (model)`

* **Cost Metrics**
  - Cost by Model: `sum(increase(phoenix_llm_cost_dollars_total[$interval])) by (model)`
  - Cost Trend: `rate(phoenix_llm_cost_dollars_total[$interval])`
  - Projected Monthly Cost: `sum(rate(phoenix_llm_cost_dollars_total[7d])) * 86400 * 30`

* **Error Analysis**
  - Error Rate by Type: `sum(rate(phoenix_llm_errors_total[$interval])) by (error_type, model)`
  - Error Duration Impact: `sum(rate(phoenix_llm_error_duration_seconds_sum[$interval])) by (error_type)`

* **Cache Performance**
  - Cache Hit Rate: `sum(rate(phoenix_llm_cache_hit_total[$interval])) / sum(rate(phoenix_llm_requests_total[$interval]))`
  - Cache Size: `phoenix_llm_cache_size_bytes{}`
  - Cache Evictions: `rate(phoenix_llm_cache_evictions_total[$interval])`

### 5. Safety & Security Dashboard

**Purpose**: Monitor security-related metrics and potential threats

**Key Panels**:

* **Safety Service Metrics**
  - Request Rate: `rate(phoenix_safety_requests_total[$interval])`
  - Blocked Requests: `sum(rate(phoenix_safety_blocked_requests_total[$interval])) by (reason)`
  - Risk Level Distribution: `sum(phoenix_safety_risk_level_total) by (level)`

* **Authentication Metrics**
  - Auth Attempts: `rate(phoenix_auth_attempts_total[$interval])`
  - Failed Auths: `sum(rate(phoenix_auth_failures_total[$interval])) by (reason)`
  - Token Validations: `rate(phoenix_token_validations_total[$interval])`

* **Executor Service Security**
  - Sandbox Violations: `sum(rate(phoenix_executor_sandbox_violations_total[$interval])) by (type)`
  - Resource Limit Hits: `sum(rate(phoenix_executor_resource_limit_hits_total[$interval])) by (resource)`

* **Security Log Events**
  - Security Event Timeline: Loki query for security-related events
  - Auth Failures Map: GeoIP visualization of auth failures

## Specialized Dashboards

### 1. SLO & Error Budget Dashboard

**Purpose**: Track service level objectives and error budgets

**Key Panels**:

* **SLO Performance**
  - Availability SLO: `sum(rate(phoenix_requests_total{status!~"5.."}[1h])) / sum(rate(phoenix_requests_total[1h]))`
  - Latency SLO: `histogram_quantile(0.95, sum(rate(phoenix_request_duration_seconds_bucket[1h])) by (le)) < 0.3`
  - Overall SLO Compliance: Combined metric of all SLOs

* **Error Budget**
  - Budget Consumption: `sum(rate(phoenix_errors_total[30d])) / (sum(rate(phoenix_requests_total[30d])) * 0.001)`
  - Burn Rate: `sum(rate(phoenix_errors_total[1h])) / (sum(rate(phoenix_requests_total[1h])) * 0.001)`
  - Projected Depletion Date: Time series forecast

* **SLO by Service**
  - Table of all services with SLO performance

### 2. Business Impact Dashboard

**Purpose**: Connect technical metrics to business outcomes

**Key Panels**:

* **User Experience Metrics**
  - Session Success Rate
  - User-Perceived Latency
  - Feature Availability

* **Business Operations**
  - Cost per Transaction
  - Resource Utilization Efficiency
  - Capacity Planning

## Alert Thresholds

### Service Health Alerts

| Alert Name | Expression | Threshold | Duration | Severity |
|------------|------------|-----------|----------|----------|
| ServiceDown | `up{} == 0` | N/A | 1m | Critical |
| HighErrorRate | `sum(rate(phoenix_errors_total[5m])) by (service) / sum(rate(phoenix_requests_total[5m])) by (service)` | > 0.05 (5%) | 5m | Warning |
| HighErrorRate | `sum(rate(phoenix_errors_total[5m])) by (service) / sum(rate(phoenix_requests_total[5m])) by (service)` | > 0.10 (10%) | 5m | Critical |
| SlowRequests | `histogram_quantile(0.95, sum(rate(phoenix_request_duration_seconds_bucket[5m])) by (service, le))` | > 2s | 10m | Warning |
| CircuitBreakerOpen | `phoenix_circuit_breaker_state{} == 1` | N/A | 5m | Warning |
| MultipleCircuitsOpen | `count(phoenix_circuit_breaker_state{} == 1) by (service)` | >= 3 | 5m | Critical |

### Resource Utilization Alerts

| Alert Name | Expression | Threshold | Duration | Severity |
|------------|------------|-----------|----------|----------|
| HighCPUUsage | `sum(rate(process_cpu_seconds_total[5m])) by (service)` | > 0.8 (80%) | 15m | Warning |
| HighMemoryUsage | `process_resident_memory_bytes / process_resident_memory_bytes_limit` | > 0.85 (85%) | 15m | Warning |
| HighMemoryUsage | `process_resident_memory_bytes / process_resident_memory_bytes_limit` | > 0.95 (95%) | 5m | Critical |
| FileDCountHigh | `process_open_fds / process_max_fds` | > 0.8 (80%) | 10m | Warning |
| DiskSpaceLow | `node_filesystem_avail_bytes{mountpoint="/"} / node_filesystem_size_bytes{mountpoint="/"}` | < 0.1 (10%) | 5m | Critical |

### LLM Service Alerts

| Alert Name | Expression | Threshold | Duration | Severity |
|------------|------------|-----------|----------|----------|
| LLMHighErrorRate | `sum(rate(phoenix_llm_errors_total[5m])) by (model) / sum(rate(phoenix_llm_requests_total[5m])) by (model)` | > 0.05 (5%) | 5m | Warning |
| LLMCostSpike | `sum(rate(phoenix_llm_cost_dollars_total[10m])) / sum(rate(phoenix_llm_cost_dollars_total[1h] offset 1h))` | > 2.0 | 10m | Warning |
| LLMTokenUsageHigh | `sum(increase(phoenix_llm_tokens_total[24h]))` | > defined budget | 30m | Warning |
| LLMLatencyHigh | `histogram_quantile(0.95, sum(rate(phoenix_llm_generation_duration_seconds_bucket[5m])) by (model, le))` | > 5s | 10m | Warning |

### Knowledge Base Alerts

| Alert Name | Expression | Threshold | Duration | Severity |
|------------|------------|-----------|----------|----------|
| KBLatencyHigh | `histogram_quantile(0.95, sum(rate(phoenix_kb_query_duration_seconds_bucket[5m])) by (kb, le))` | > 1s | 5m | Warning |
| VectorStorageHigh | `phoenix_kb_vector_count * 1500` | > 80% of capacity | 15m | Warning |
| LowSimilarityMatching | `avg_over_time(phoenix_kb_top_match_similarity[15m])` | < 0.7 | 15m | Warning |
| PruningFailures | `increase(phoenix_kb_pruning_failures_total[1h])` | > 0 | 1m | Warning |

### Security Alerts

| Alert Name | Expression | Threshold | Duration | Severity |
|------------|------------|-----------|----------|----------|
| HighAuthFailures | `sum(rate(phoenix_auth_failures_total[15m])) by (service)` | > 10 | 15m | Warning |
| SuspiciousAuthAttempts | `sum(rate(phoenix_auth_failures_total{reason="invalid_credentials"}[5m])) by (service, ip)` | > 5 | 5m | Critical |
| SandboxViolationDetected | `increase(phoenix_executor_sandbox_violations_total[10m])` | > 0 | 1m | Critical |
| HighRiskRequestsBlocked | `sum(rate(phoenix_safety_blocked_requests_total{risk_level="high"}[15m]))` | > 5 | 15m | Warning |

### SLO Alerts

| Alert Name | Expression | Threshold | Duration | Severity |
|------------|------------|-----------|----------|----------|
| SLOAvailabilityBreach | `sum(rate(phoenix_requests_total{status!~"5.."}[1h])) / sum(rate(phoenix_requests_total[1h]))` | < 0.995 | 1h | Warning |
| SLOLatencyBreach | `histogram_quantile(0.95, sum(rate(phoenix_request_duration_seconds_bucket[1h])) by (le))` | > 0.3s | 1h | Warning |
| ErrorBudgetBurnRate | `sum(rate(phoenix_errors_total[1h])) / (sum(rate(phoenix_requests_total[1h])) * 0.001)` | > 14.4 | 1h | Critical |

## Alert Notification Channels

The alerting framework integrates with multiple notification channels based on severity:

### Critical Alerts
- Primary: PagerDuty
- Secondary: Slack #alerts-critical channel
- Additional: Email to oncall@example.com

### Warning Alerts
- Primary: Slack #alerts-warnings channel
- Secondary: Email digest (hourly)

### Info Alerts
- Primary: Slack #alerts-info channel
- Secondary: Daily email digest

## Custom Alert Routing Rules

Different services can have specific routing rules:

```yaml
# Alertmanager Configuration
route:
  receiver: default
  group_by: ['alertname', 'service']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h
  
  routes:
  - match:
      severity: critical
    receiver: pagerduty
    continue: true
  
  - match_re:
      service: orchestrator|data-router
    receiver: core-services-team
    
  - match_re:
      service: llm-service
    receiver: llm-team
    
  - match_re:
      service: .*-kb
    receiver: knowledge-base-team
    
  # This is a time-based routing that reduces weekend noise
  - match:
      severity: warning
    receiver: slack
    routes:
    - match:
        weekday: 'Saturday|Sunday'
      receiver: weekend-slack-digest
      group_wait: 30m
      repeat_interval: 12h
```

## Dashboard Implementation Guidelines

### Grafana Provisioning

Dashboard templates are provisioned through the Grafana API or using configuration files:

```yaml
# /etc/grafana/provisioning/dashboards/phoenix.yaml
apiVersion: 1

providers:
  - name: 'Phoenix Orchestrator'
    folder: 'Phoenix Orchestrator'
    type: file
    disableDeletion: false
    updateIntervalSeconds: 30
    options:
      path: /var/lib/grafana/dashboards/phoenix
      foldersFromFilesStructure: true
```

### Reusable Dashboard Components

Key dashboard components are defined as reusable panels or libraries:

1. **Service Status Indicator**
   - Standard health status visualization
   - Includes uptime, version, instance count

2. **Error Rate Widget**
   - Standard error rate visualization with thresholds
   - Consistent color scheme and thresholds

3. **Request Timeline**
   - Time-series visualization for request rates
   - Consistent bucketing and aggregations

### Visual Design Guidelines

1. **Color Scheme**
   - Traffic Light for Status (Green, Yellow, Red)
   - Blue scale for normal metrics
   - Orange/Red for error conditions
   - Consistent color mapping across dashboards

2. **Panel Density**
   - Maximum of 12-15 panels per dashboard
   - Group related metrics
   - Use drill-down links for details

3. **Time Range Consistency**
   - Default to 6h time range
   - Include relative time range selector
   - Sync time ranges across panels

## Implementation Plan

1. **Phase 1: Core Monitoring Dashboards**
   - System Overview Dashboard
   - Service-Specific Dashboard template
   - Key alert definitions

2. **Phase 2: Service-Specific Dashboards**
   - LLM Service Dashboard
   - Knowledge Base Dashboard
   - Safety & Security Dashboard

3. **Phase 3: Advanced Dashboards**
   - SLO & Error Budget Tracking
   - Business Impact Dashboard
   - Advanced Correlation Views

4. **Phase 4: Dashboard Automation**
   - Automated dashboard generation
   - Dynamic thresholds based on historical patterns
   - AI-assisted anomaly detection