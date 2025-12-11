# Monitoring Configuration

## Overview

Monitoring stack configuration for Phoenix ORCH AGI system. Includes Prometheus metrics collection, Alertmanager alert routing, and Grafana dashboards.

## Directory Structure

- **prometheus/**: Prometheus server configuration
  - `prometheus.yml`: Main Prometheus configuration
  - `rules/`: Alerting rules
    - `resource-alerts.yml`: Resource-based alerts
- **alertmanager/**: Alertmanager configuration
  - `alertmanager.yml`: Alert routing and notification rules
- **dashboards/**: Grafana dashboard definitions
  - `circuit-breaker-dashboard.json`: Circuit breaker metrics dashboard

## Prometheus Configuration

### Scrape Targets
- Service endpoints for metrics collection
- Health check endpoints
- Custom metrics exporters

### Alert Rules
- Resource utilization alerts
- Service health alerts
- Error rate thresholds
- Latency alerts

## Alertmanager Configuration

### Alert Routing
- Route groups by severity
- Notification channels (email, Slack, PagerDuty)
- Alert suppression rules
- Repeat interval configuration

### Notification Templates
- Alert message formatting
- Severity-based routing
- Grouping and aggregation

## Dashboards

### Circuit Breaker Dashboard
- Circuit state metrics
- Failure rate tracking
- Recovery time monitoring
- Request volume visualization

## Usage

### Deploy with Docker Compose
```bash
docker-compose -f docker/docker-compose.monitoring.yml up -d
```

### Manual Prometheus Configuration
1. Update `prometheus/prometheus.yml` with service targets
2. Add alert rules to `prometheus/rules/`
3. Configure Alertmanager in `alertmanager/alertmanager.yml`
4. Import dashboards to Grafana

## Metrics Collection

### Service Metrics
- Request rates
- Error rates
- Latency percentiles
- Resource utilization

### System Metrics
- CPU usage
- Memory usage
- Disk I/O
- Network traffic

## Alerting

### Alert Severities
- **Critical**: Immediate attention required
- **Warning**: Requires monitoring
- **Info**: Informational alerts

### Alert Conditions
- High error rates
- Resource exhaustion
- Service unavailability
- Circuit breaker state changes

