#!/bin/bash
set -e

# Phoenix Orchestrator Load Testing Runner Script
# This script provides a simple command-line interface for running load tests

# Default values
SCENARIO=${1:-"baseline"}
TARGET_URL=${2:-"http://orchestrator-service:50051"}
VUS=${3:-10}
DURATION=${4:-"30s"}
TAG=$(date +%Y%m%d-%H%M%S)
RESULTS_DIR="./results"
GRAFANA_URL="http://localhost:3001"

# Show usage if requested
if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
  echo "Usage: $0 <scenario> [target_url] [vus] [duration]"
  echo ""
  echo "Arguments:"
  echo "  scenario    Test scenario to run (default: baseline)"
  echo "               Available scenarios: $(ls ./scenarios/ | grep -E '\.js$' | sed 's/\.js$//' | tr '\n' ', ' | sed 's/,$//g')"
  echo "  target_url  Target URL for testing (default: http://orchestrator-service:50051)"
  echo "  vus         Number of virtual users (default: 10)"
  echo "  duration    Test duration (default: 30s)"
  echo ""
  echo "Example:"
  echo "  $0 user-journey http://localhost:50051 20 60s"
  exit 0
fi

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
  echo "Error: Docker is not running or not accessible"
  exit 1
fi

# Ensure results directory exists
mkdir -p $RESULTS_DIR

# Print banner
echo "===== Phoenix Orchestrator Load Test Runner ====="
echo "Scenario:   $SCENARIO"
echo "Target URL: $TARGET_URL"
echo "VUs:        $VUS"
echo "Duration:   $DURATION"
echo "=============================================="

# Check if scenario exists
if [ ! -f "./scenarios/${SCENARIO}.js" ]; then
  echo "Error: Scenario '${SCENARIO}' not found!"
  echo "Available scenarios:"
  ls -1 ./scenarios/ | grep -E '\.js$' | sed 's/\.js$//'
  exit 1
fi

# Check if monitoring stack is running
if ! docker ps | grep -q 'grafana'; then
  echo "Warning: Monitoring stack not detected. Starting required services..."
  docker-compose up -d prometheus influxdb grafana
  
  # Wait for services to start
  echo "Waiting for monitoring services to start..."
  sleep 10
fi

# Build the test runner if needed
if ! docker images | grep -q 'phoenix-load-test'; then
  echo "Building test runner image..."
  docker build -t phoenix-load-test .
fi

# Execute the test
echo "Starting load test with scenario: $SCENARIO"
echo "Test will run for $DURATION with $VUS virtual users"
echo "Press Ctrl+C to abort"
echo ""

CONTAINER_NAME="load-test-${TAG}"

# Run the test in Docker
docker run --name $CONTAINER_NAME \
  --network=load-testing_monitoring_network \
  -e VUS="$VUS" \
  -e DURATION="$DURATION" \
  -e TARGET_URL="$TARGET_URL" \
  -e K6_OUT="influxdb=http://influxdb:8086/k6" \
  -v "$(pwd)/results:/results" \
  phoenix-load-test "$SCENARIO"

echo "Test completed. Results stored in: $RESULTS_DIR"

# Generate the report
echo "Generating test report..."
if command -v node &> /dev/null; then
  node scripts/aggregate-metrics.js "$RESULTS_DIR" "$RESULTS_DIR/aggregated"
  
  # Compare with benchmarks if available
  if [ -f "./benchmarks/${SCENARIO}.json" ]; then
    echo "Comparing with benchmark..."
    ./scripts/compare-benchmarks.sh "./benchmarks/${SCENARIO}.json" "$RESULTS_DIR/aggregated/summary-report.json"
  else
    echo "No benchmark found for comparison. To create one:"
    echo "  cp $RESULTS_DIR/aggregated/summary-report.json ./benchmarks/${SCENARIO}.json"
  fi
else
  echo "Node.js not found. Skipping report generation."
  echo "To generate reports, install Node.js and run:"
  echo "  node scripts/aggregate-metrics.js '$RESULTS_DIR' '$RESULTS_DIR/aggregated'"
fi

# Provide Grafana URL
echo ""
echo "View results in Grafana at $GRAFANA_URL"
echo "Dashboard: Phoenix Orchestrator Load Testing"
echo ""

echo "Test run complete!"