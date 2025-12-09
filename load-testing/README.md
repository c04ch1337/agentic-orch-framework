# Phoenix Orchestrator Load Testing Framework

A comprehensive load testing framework for validating the performance and resilience of the Phoenix Orchestrator platform under various conditions.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Test Scenarios](#test-scenarios)
- [Metrics and Reporting](#metrics-and-reporting)
- [CI/CD Integration](#cicd-integration)
- [Chaos Testing](#chaos-testing)
- [Extending the Framework](#extending-the-framework)
- [Troubleshooting](#troubleshooting)
- [FAQs](#faqs)

## Overview

The Phoenix Orchestrator Load Testing Framework provides a complete solution for performance testing, load testing, and chaos testing of the Phoenix Orchestrator platform. It is designed to:

- Validate system performance under various load conditions
- Identify bottlenecks and performance issues
- Test system resilience and recovery capabilities
- Establish performance benchmarks
- Detect performance regressions
- Simulate real-world usage patterns

The framework leverages [k6](https://k6.io/) for load generation, along with a comprehensive metrics collection, visualization, and analysis pipeline built on Prometheus, Grafana, and custom tooling.

## Architecture

The load testing framework consists of the following components:

![Load Testing Architecture](./docs/images/load-testing-architecture.png)

### Components

1. **Test Infrastructure**
   - `k6`: Main load testing tool for distributed load testing
   - Docker-based containerized test runner
   - Custom test scenarios and scripts

2. **Metrics Collection**
   - Prometheus: Metrics storage and querying
   - InfluxDB: Time-series data for k6 metrics
   - StatsD: Metrics aggregation

3. **Visualization and Analysis**
   - Grafana: Real-time dashboards
   - Custom metrics aggregation scripts
   - Benchmark comparison tools

4. **Chaos Testing**
   - Chaos runner scripts
   - Predefined chaos scenarios
   - Integration with load testing

5. **CI/CD Integration**
   - GitHub Actions workflows
   - Daily baseline tests
   - Pull request verification
   - Regression detection

## Installation

### Prerequisites

- Docker and Docker Compose
- Node.js 16+ (for metrics aggregation scripts)
- Git

### Setup

1. Clone this repository:
   ```bash
   git clone https://github.com/phoenix-orchestrator/load-testing
   cd load-testing
   ```

2. Create required directories:
   ```bash
   mkdir -p results/aggregated
   mkdir -p benchmarks
   ```

3. Start the monitoring infrastructure:
   ```bash
   docker-compose up -d prometheus influxdb grafana
   ```

4. Wait a few seconds for the services to start, then access Grafana at http://localhost:3001 (credentials: admin/admin)

## Quick Start

To run a basic load test:

```bash
# Build the test runner
docker build -t phoenix-load-test .

# Run a baseline test
docker run --rm --network=load-testing_monitoring_network \
  -e VUS=10 \
  -e DURATION=30s \
  -e TARGET_URL=http://localhost:50051 \
  -e K6_OUT=influxdb=http://influxdb:8086/k6 \
  -v "$(pwd)/results:/results" \
  phoenix-load-test baseline
```

View results in Grafana at [http://localhost:3001/d/phoenix-load-testing](http://localhost:3001/d/phoenix-load-testing)

## Test Scenarios

The framework includes several predefined test scenarios:

### 1. Baseline Tests

Basic performance tests targeting key services to establish performance baselines.

```bash
# Run baseline test
./run-test.sh baseline
```

Key metrics:
- Request throughput
- Response time (p50, p95, p99)
- Error rate
- Resource utilization

### 2. User Journey Tests

Realistic user journey simulations that test end-to-end flows.

```bash
# Run user journey test
./run-test.sh user-journey
```

These tests:
- Simulate real user behavior
- Test multi-step workflows
- Validate end-to-end functionality under load

### 3. Stress Tests

Stress tests designed to identify breaking points and bottlenecks.

```bash
# Run stress test
./run-test.sh stress
```

Features:
- Progressive load increase
- Breaking point detection
- Recovery testing

### 4. Custom Test Scenarios

You can create custom test scenarios by adding new JavaScript files to the `scenarios` directory. See [Extending the Framework](#extending-the-framework) for details.

## Metrics and Reporting

### Real-time Monitoring

During test execution, you can monitor performance in real-time using Grafana dashboards:

- **Load Testing Dashboard**: [http://localhost:3001/d/phoenix-load-testing](http://localhost:3001/d/phoenix-load-testing)
  - Overview of current test performance
  - Real-time request rates and latencies
  - Error tracking

- **Phoenix Services Dashboard**: [http://localhost:3001/d/phoenix-services](http://localhost:3001/d/phoenix-services)
  - Service-level metrics
  - Resource utilization
  - Detailed service performance

### Test Reports

After test completion, reports are generated in the `results` directory:

```bash
# Generate aggregated metrics
node scripts/aggregate-metrics.js ./results ./results/aggregated

# Analyze results
./scripts/analyze-results.sh ./results/aggregated/summary-report.json
```

The following reports are available:

- `summary-report.json`: Overall test results and statistics
- `*.csv`: Detailed metrics in CSV format for further analysis
- `*.prom`: Prometheus-compatible metrics for long-term storage

### Benchmark Comparison

Compare current test results against established benchmarks:

```bash
./scripts/compare-benchmarks.sh ./benchmarks/baseline.json ./results/aggregated/summary-report.json
```

This will generate a detailed comparison report highlighting performance improvements or regressions.

## CI/CD Integration

The framework includes GitHub Actions workflows for automated testing:

### Workflows

- **Scheduled Daily Tests**: Runs baseline performance tests daily against the staging environment
- **Pull Request Tests**: Runs smoke tests for PRs to main/release branches
- **Manual Triggers**: Allows running any test on demand against any environment

### Configuration

The workflow is configured in `.github/workflows/load-testing.yml` and supports:

- Multiple test types (smoke, baseline, journey, stress)
- Multiple environments (dev, staging, production)
- Customizable test duration and VU count
- Performance regression detection
- Slack notifications

### Usage

To trigger a manual test:

1. Go to GitHub Actions
2. Select "Phoenix Orchestrator Load Testing"
3. Click "Run workflow"
4. Configure the test parameters
5. Click "Run workflow"

## Chaos Testing

The framework includes chaos testing capabilities to validate system resilience:

### Available Chaos Scenarios

- **Service Failures**: Test system behavior when services crash
- **Network Issues**: Introduce latency, packet loss, and network partitions
- **Resource Exhaustion**: Simulate CPU, memory, and disk stress
- **Timing Issues**: Introduce clock skew to test synchronization

### Running Chaos Tests

```bash
# Run a specific chaos scenario
./chaos/chaos-runner.sh service-kill orchestrator-service 30s

# Run a predefined chaos test suite
./chaos/orchestrate-chaos-tests.sh service-failures baseline
```

### Chaos Testing in CI

You can include chaos testing in CI pipelines by adding the `chaos` parameter:

```yaml
- name: Run Load Test with Chaos
  uses: ./.github/actions/load-test
  with:
    scenario: baseline
    chaos: network-issues
    duration: 120s
```

## Extending the Framework

### Adding New Test Scenarios

1. Create a new JavaScript file in the `scenarios` directory:

```javascript
// scenarios/my-custom-test.js
import { sleep } from 'k6';
import { createServiceRequest } from '../scripts/common.js';

export const options = {
  stages: [
    { duration: '30s', target: 10 },
    { duration: '1m', target: 10 },
    { duration: '30s', target: 0 }
  ]
};

export default function() {
  // Your test logic here
  const result = createServiceRequest(
    'http://my-service/endpoint',
    'POST',
    JSON.stringify({ key: 'value' }),
    { service: 'my-service', endpoint: 'endpoint' }
  );
  
  sleep(1);
}
```

2. Run your custom test:

```bash
./run-test.sh my-custom-test
```

### Adding Custom Metrics

Define custom metrics in your test scenarios:

```javascript
import { Trend, Rate } from 'k6/metrics';

// Define custom metrics
const myCustomMetric = new Trend('my_custom_metric');
const myErrorRate = new Rate('my_error_rate');

export default function() {
  // Record metric values
  myCustomMetric.add(someValue);
  myErrorRate.add(didSucceed ? 0 : 1);
}
```

### Creating Custom Dashboards

1. Create a new dashboard JSON file in `configs/grafana/dashboards/`
2. Add panels for your custom metrics
3. Import the dashboard into Grafana

## Troubleshooting

### Common Issues

#### Connection Refused to Services

**Problem:** Tests fail with connection refused errors.

**Solution:** Ensure the services are running and accessible from the test runner:
```bash
docker exec -it load-testing_k6_1 ping orchestrator-service
```

#### No Metrics in Grafana

**Problem:** Test runs but no metrics appear in Grafana.

**Solution:** Check data source configuration and connectivity:
```bash
# Check InfluxDB status
docker exec -it load-testing_influxdb_1 influx -execute 'SHOW DATABASES'

# Check Prometheus targets
curl http://localhost:9090/api/v1/targets
```

#### Out of Memory Errors

**Problem:** Test runner crashes with out of memory errors.

**Solution:** Adjust the test parameters or increase container memory limits:
```bash
docker run --rm --memory=2g ... phoenix-load-test baseline
```

### Logging

Logs are available in:

- `results/*.log`: Test run logs
- Docker container logs: `docker logs load-testing_k6_1`
- Application logs: Check the Phoenix Orchestrator service logs

## FAQs

### How many virtual users should I use?

Start with 10-20 VUs for baseline testing, and scale up for stress tests. The appropriate number depends on your expected production load and available resources.

### What thresholds should I set for my tests?

Suggested starting points:
- Response time (p95): < 500ms for API endpoints
- Error rate: < 1%
- Throughput: Depends on your specific requirements

### How do I test with authentication?

Add authentication headers to your requests:

```javascript
const result = createServiceRequest(
  url,
  'POST',
  payload,
  {
    headers: {
      'Authorization': 'Bearer your-token-here'
    }
  }
);
```

### Can I run tests from my local machine against remote environments?

Yes, you can point the tests at any environment:

```bash
./run-test.sh baseline https://staging-api.phoenix-orchestrator.com
```

### How should I interpret test results?

Focus on:
1. Response time trends over time
2. Error rates under increasing load
3. Breaking points in stress tests
4. Services that become bottlenecks first

## License

MIT License