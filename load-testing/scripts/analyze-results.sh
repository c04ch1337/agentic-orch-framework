#!/bin/bash
set -e

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required but not installed."
    exit 1
fi

# Check if a file is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <result_file.json>"
    exit 1
fi

RESULT_FILE=$1

# Check if file exists
if [ ! -f "$RESULT_FILE" ]; then
    echo "Error: Result file not found: $RESULT_FILE"
    exit 1
fi

# Extract key metrics
echo "=========================================================="
echo "Phoenix Orchestrator Load Test Results Analysis"
echo "=========================================================="
echo "File: $RESULT_FILE"
echo "=========================================================="

# Extract metrics
echo "Summary:"
echo -n "  - Total requests: "
jq '.metrics.http_reqs.values.count' "$RESULT_FILE"

echo -n "  - Failed requests: "
jq '.metrics.http_req_failed.values.passes' "$RESULT_FILE"

echo -n "  - HTTP request rate (reqs/s): "
jq '.metrics.http_reqs.values.rate' "$RESULT_FILE"

echo "Response Time:"
echo -n "  - Min: "
jq '.metrics.http_req_duration.values.min | . / 1000' "$RESULT_FILE"
echo -n "  - Avg: "
jq '.metrics.http_req_duration.values.avg | . / 1000' "$RESULT_FILE"
echo -n "  - Median (p50): "
jq '.metrics.http_req_duration.values.med | . / 1000' "$RESULT_FILE"
echo -n "  - p90: "
jq '.metrics.http_req_duration.values.p90 | . / 1000' "$RESULT_FILE"
echo -n "  - p95: "
jq '.metrics.http_req_duration.values.p95 | . / 1000' "$RESULT_FILE"
echo -n "  - p99: "
jq '.metrics.http_req_duration.values.p99 | . / 1000' "$RESULT_FILE"
echo -n "  - Max: "
jq '.metrics.http_req_duration.values.max | . / 1000' "$RESULT_FILE"

echo "Thresholds:"
if jq -e '.metrics | has("thresholds")' "$RESULT_FILE" > /dev/null; then
    if jq -e '.metrics.thresholds.values.passed' "$RESULT_FILE" > /dev/null; then
        echo "  - All thresholds passed"
    else
        echo "  - Some thresholds failed:"
        jq -r '.metrics.thresholds.values | to_entries[] | select(.value==false) | "    - " + .key' "$RESULT_FILE"
    fi
else
    echo "  - No thresholds defined"
fi

# Check for specific services if available
if jq -e '.metrics | keys[] | select(. | test("^http_req_duration{.*service:"))' "$RESULT_FILE" > /dev/null; then
    echo "Service Performance:"
    
    # Extract services and their metrics
    for service in $(jq -r '.metrics | keys[] | select(. | test("^http_req_duration{.*service:")) | .' "$RESULT_FILE"); do
        service_name=$(echo "$service" | grep -oP 'service:\K[^,}]*')
        echo "  - $service_name:"
        
        echo -n "    - Avg response time: "
        avg=$(jq ".metrics[\"$service\"].values.avg | . / 1000" "$RESULT_FILE")
        echo "$avg seconds"
        
        echo -n "    - p95 response time: "
        p95=$(jq ".metrics[\"$service\"].values.p95 | . / 1000" "$RESULT_FILE")
        echo "$p95 seconds"
    done
fi

echo "=========================================================="
echo "Analyzed at: $(date)"
echo "=========================================================="