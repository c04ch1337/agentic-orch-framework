#!/bin/bash
set -e

# Phoenix Orchestrator Chaos Testing Orchestrator
# This script orchestrates chaos tests based on predefined scenarios

# Defaults
SCENARIO_FILE="./chaos-scenarios.json"
RESULTS_DIR="../results/chaos"
LOG_FILE="$RESULTS_DIR/chaos-orchestrator-$(date +%Y%m%d-%H%M%S).log"
RUNNER="./chaos-runner.sh"
SCENARIO_GROUP=${1:-"service-failures"}
LOAD_TEST=${2:-"baseline"}

# Ensure directories exist
mkdir -p "$RESULTS_DIR"

# Function to log messages
log() {
  echo "$(date +'%Y-%m-%d %H:%M:%S') - $1" | tee -a "$LOG_FILE"
}

# Check if the chaos runner script exists
if [ ! -f "$RUNNER" ]; then
  log "Error: Chaos runner script not found at $RUNNER"
  exit 1
fi

# Check for scenario file
if [ ! -f "$SCENARIO_FILE" ]; then
  log "Error: Scenario file not found at $SCENARIO_FILE"
  exit 1
fi

# Make sure the runner is executable
chmod +x "$RUNNER"

# Function to run k6 load test in the background
start_load_test() {
  local scenario=$1
  log "Starting load test with scenario: $scenario"
  cd ..
  # Run the k6 test in Docker using our container
  docker-compose run -d --name load-test k6 run /scenarios/$scenario.js
  cd chaos
  log "Load test started in the background"
}

# Function to stop the load test
stop_load_test() {
  log "Stopping load test"
  cd ..
  docker stop load-test > /dev/null 2>&1 || true
  docker rm load-test > /dev/null 2>&1 || true
  cd chaos
  log "Load test stopped"
}

# Function to run a chaos experiment
run_experiment() {
  local name=$1
  local scenario=$2
  local target=$3
  local duration=$4
  local description=$5
  local multi_target=${6:-false}

  log "=============================================="
  log "Starting experiment: $name"
  log "Description: $description"
  log "Scenario: $scenario, Target: $target, Duration: $duration"
  log "=============================================="

  if [ "$multi_target" = "true" ]; then
    # For multi-target experiments, run chaos for each target
    for t in $(echo $target | tr -d '[]' | tr ',' ' '); do
      # Remove quotes from target if present
      t=$(echo $t | tr -d '"' | tr -d "'")
      log "Running chaos for target: $t"
      "$RUNNER" "$scenario" "$duration" "$t"
      sleep 5  # Brief pause between targets
    done
  else
    # For single target experiments
    "$RUNNER" "$scenario" "$duration" "$target"
  fi

  log "Experiment completed: $name"
}

# Get the scenario group from the JSON file
get_scenario_group() {
  local group=$1
  # Using jq to extract the experiments array for the specified group
  if command -v jq &> /dev/null; then
    jq -r ".scenarios[] | select(.name == \"$group\") | .experiments" "$SCENARIO_FILE"
  else
    log "Error: jq is required but not installed."
    exit 1
  fi
}

# Print banner
log "===== PHOENIX ORCHESTRATOR CHAOS TEST ORCHESTRATOR ====="
log "Scenario Group: $SCENARIO_GROUP"
log "Load Test: $LOAD_TEST"
log "Results Directory: $RESULTS_DIR"
log "=================================================="

# Start the load test
start_load_test "$LOAD_TEST"

# Give the load test time to start up
log "Waiting for load test to stabilize..."
sleep 20

# Run experiments from the scenario group
EXPERIMENTS=$(get_scenario_group "$SCENARIO_GROUP")

if [ "$EXPERIMENTS" = "null" ] || [ -z "$EXPERIMENTS" ]; then
  log "Error: No experiments found for scenario group '$SCENARIO_GROUP'"
  stop_load_test
  exit 1
fi

# Process each experiment in the group
echo $EXPERIMENTS | jq -c '.[]' | while read -r experiment; do
  name=$(echo $experiment | jq -r '.name')
  scenario=$(echo $experiment | jq -r '.scenario')
  target=$(echo $experiment | jq -r '.target')
  duration=$(echo $experiment | jq -r '.duration')
  description=$(echo $experiment | jq -r '.description')
  multi_target=$(echo $experiment | jq -r '.multi_target // false')

  run_experiment "$name" "$scenario" "$target" "$duration" "$description" "$multi_target"
  
  # Wait between experiments
  log "Waiting for system to stabilize before next experiment..."
  sleep 30
done

# Stop the load test
stop_load_test

log "===== CHAOS TESTING COMPLETED ====="
log "All experiments in group '$SCENARIO_GROUP' have been executed"
log "Results logged to: $RESULTS_DIR"