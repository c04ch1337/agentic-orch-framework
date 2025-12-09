# Phoenix Orchestrator Load Testing - Quick Reference

This document provides a quick reference for running various types of load tests using the Phoenix Orchestrator load testing framework.

## Basic Commands

### Running a Test

```bash
# Basic syntax
./run-test.sh <scenario> [target_url] [vus] [duration]

# Examples:
./run-test.sh baseline                            # Run baseline test with defaults
./run-test.sh user-journey http://localhost:50051 # Run user journey test against local instance
./run-test.sh stress https://staging-api.example.com 50 5m # Run stress test with 50 VUs for 5 minutes
```

### Available Test Scenarios

- `baseline.js` - Basic performance test for key services
- `llm-service-test.js` - Test specifically targeting the LLM service
- `kb-services-test.js` - Test specifically targeting knowledge base services
- `user-journey-test.js` - Test simulating realistic user journeys
- `stress-test.js` - Stress test to identify breaking points

## Common Testing Patterns

### 1. Smoke Test

Quick test to verify system is operational:

```bash
./run-test.sh baseline http://localhost:50051 2 15s
```

### 2. Baseline Performance Test

Establish performance benchmarks:

```bash
./run-test.sh baseline http://localhost:50051 10 60s
```

### 3. User Journey Test

Test realistic user flows:

```bash
./run-test.sh user-journey http://localhost:50051 10 120s
```

### 4. Load Test

Test system under moderate load:

```bash
./run-test.sh baseline http://localhost:50051 50 300s
```

### 5. Stress Test

Find breaking points:

```bash
./run-test.sh stress http://localhost:50051 100 600s
```

### 6. Soak Test

Test system stability over time:

```bash
./run-test.sh baseline http://localhost:50051 20 3600s
```

## Chaos Testing

### Running a Single Chaos Experiment

```bash
# Syntax
./chaos/chaos-runner.sh <scenario> <duration> <target>

# Examples
./chaos/chaos-runner.sh service-kill 30s orchestrator-service     # Kill a service
./chaos/chaos-runner.sh network-latency 60s data-router-service   # Add network latency
./chaos/chaos-runner.sh cpu-stress 120s llm-service               # Add CPU stress
```

### Running a Chaos Scenario Group

```bash
# Syntax
./chaos/orchestrate-chaos-tests.sh <scenario-group> [load-test-type]

# Examples
./chaos/orchestrate-chaos-tests.sh service-failures baseline      # Run service failure scenarios with baseline load
./chaos/orchestrate-chaos-tests.sh network-issues user-journey    # Run network issues scenarios with user journey test
```

## Metrics and Reporting

### Viewing Results in Real-time

Access Grafana dashboards:
- Main Dashboard: http://localhost:3001/d/phoenix-load-testing
- Services Dashboard: http://localhost:3001/d/phoenix-services

### Generating Reports

```bash
# Generate aggregated metrics
node scripts/aggregate-metrics.js ./results ./results/aggregated

# Analyze results
./scripts/analyze-results.sh ./results/aggregated/summary-report.json

# Compare with benchmark
./scripts/compare-benchmarks.sh ./benchmarks/baseline.json ./results/aggregated/summary-report.json
```

### Creating Benchmarks

To save current results as a benchmark:

```bash
cp ./results/aggregated/summary-report.json ./benchmarks/baseline.json
```

## CI/CD Integration

### Triggering Tests from GitHub Actions

Go to GitHub Actions → "Phoenix Orchestrator Load Testing" → "Run workflow" and configure:
- Test type: smoke, baseline, journey, or stress
- Environment: dev, staging, or production
- Duration: test duration in seconds

### Local CI Simulation

```bash
# Simulate CI test run
export CI=true
export GITHUB_WORKFLOW="load-testing"
export GITHUB_EVENT_NAME="workflow_dispatch"

# Then run the test
./run-test.sh baseline http://localhost:50051 5 30s
```

## Troubleshooting

### Checking Container Status

```bash
docker ps
docker logs load-testing_k6_1
docker exec -it load-testing_k6_1 sh
```

### Resetting the Environment

```bash
# Stop and remove containers
docker-compose down

# Remove volumes (caution: this will delete all data)
docker-compose down -v

# Restart the environment
docker-compose up -d
```

### Debugging Test Scenarios

```bash
# Run with K6 debug output
K6_DEBUG=true ./run-test.sh baseline
```

## Common Parameters

### Virtual Users (VUs)

- Smoke test: 2-5 VUs
- Baseline test: 10-20 VUs
- Load test: 50-100 VUs
- Stress test: 100+ VUs (increase gradually)

### Test Duration

- Smoke test: 15-30 seconds
- Baseline test: 1-5 minutes
- Load test: 5-15 minutes
- Stress test: 10-30 minutes
- Soak test: 1+ hours