#!/bin/bash
set -e

# Default values
SCENARIO=${SCENARIO:-$DEFAULT_SCENARIO}
JSON_CONFIG=${JSON_CONFIG:-""}
OUTPUT_DIR="/results"
TEST_TIME=$(date +%Y%m%d_%H%M%S)

# Print banner
echo "=========================================================="
echo "Phoenix Orchestrator Load Testing Framework"
echo "=========================================================="
echo "Running scenario: $SCENARIO"
echo "VUs: $VUS, Duration: $DURATION"
echo "Starting test at: $(date)"
echo "=========================================================="

# If JSON_CONFIG is provided, use it
if [ -n "$JSON_CONFIG" ]; then
    echo "Using JSON configuration: $JSON_CONFIG"
    k6 run --out json=$OUTPUT_DIR/result_${SCENARIO}_${TEST_TIME}.json $JSON_CONFIG
else
    # Check if scenario file exists
    if [ ! -f "/app/scenarios/${SCENARIO}.js" ] && [ ! -f "/app/scenarios/${SCENARIO}.ts" ]; then
        echo "Error: Scenario file not found!"
        echo "Available scenarios:"
        ls -la /app/scenarios/
        exit 1
    fi

    # Determine file extension
    if [ -f "/app/scenarios/${SCENARIO}.js" ]; then
        SCENARIO_FILE="/app/scenarios/${SCENARIO}.js"
    else
        SCENARIO_FILE="/app/scenarios/${SCENARIO}.ts"
    fi

    # Run the test with environment variables
    echo "Executing: $SCENARIO_FILE"
    k6 run \
        --out json=$OUTPUT_DIR/result_${SCENARIO}_${TEST_TIME}.json \
        --env VUS=$VUS \
        --env DURATION=$DURATION \
        --env TARGET_URL=${TARGET_URL:-http://orchestrator-service:50051} \
        --env RAMP_TIME=${RAMP_TIME:-5s} \
        --env THRESHOLD_HTTP_FAIL=${THRESHOLD_HTTP_FAIL:-1} \
        --env THRESHOLD_HTTP_RESPONSE=${THRESHOLD_HTTP_RESPONSE:-2000} \
        $SCENARIO_FILE
fi

echo "=========================================================="
echo "Test completed at: $(date)"
echo "Results saved to: $OUTPUT_DIR/result_${SCENARIO}_${TEST_TIME}.json"
echo "=========================================================="

# If ANALYZE_RESULTS is set, run analysis script
if [ "${ANALYZE_RESULTS}" = "true" ]; then
    echo "Analyzing test results..."
    if [ -f "/app/scripts/analyze-results.sh" ]; then
        /app/scripts/analyze-results.sh "$OUTPUT_DIR/result_${SCENARIO}_${TEST_TIME}.json"
    else
        echo "Warning: Analysis script not found!"
    fi
fi

# Check if this is a CI run
if [ "${CI_RUN}" = "true" ]; then
    echo "CI run detected, checking thresholds..."
    
    # Exit with non-zero code if thresholds are breached
    if grep -q '"thresholds":{"passed":false}' "$OUTPUT_DIR/result_${SCENARIO}_${TEST_TIME}.json"; then
        echo "Test thresholds breached! Check the results for details."
        exit 1
    fi
    
    echo "All thresholds passed."
fi