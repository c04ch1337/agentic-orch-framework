#!/bin/bash
set -e

# Phoenix Orchestrator Load Test Benchmark Comparison Script
# Compares current test results with previous benchmarks to detect regressions

# Usage: ./compare-benchmarks.sh <benchmark-file> <current-results-file> [threshold]
# Example: ./compare-benchmarks.sh ./benchmarks/baseline.json ./results/aggregated/summary-report.json 10

# Default threshold percentage (how much worse metrics can be before flagging as regression)
DEFAULT_THRESHOLD=10
THRESHOLD=${3:-$DEFAULT_THRESHOLD}

# Check if jq is installed
if ! command -v jq &> /dev/null; then
  echo "Error: jq is required but not installed."
  exit 1
fi

# Verify input files
if [ $# -lt 2 ] || [ ! -f "$1" ] || [ ! -f "$2" ]; then
  echo "Usage: $0 <benchmark-file> <current-results-file> [threshold]"
  echo "  <benchmark-file>: Path to the benchmark JSON file to compare against"
  echo "  <current-results-file>: Path to the current test results JSON file"
  echo "  [threshold]: Optional percentage threshold for regression detection (default: $DEFAULT_THRESHOLD)"
  exit 1
fi

BENCHMARK_FILE=$1
RESULTS_FILE=$2
OUTPUT_FILE="./results/performance-regression.txt"

# Create the output directory if it doesn't exist
mkdir -p "$(dirname "$OUTPUT_FILE")"

# Function to calculate percentage change
calc_percentage_change() {
  local baseline=$1
  local current=$2
  
  # Avoid division by zero
  if [ $(echo "$baseline == 0" | bc -l) -eq 1 ]; then
    if [ $(echo "$current > 0" | bc -l) -eq 1 ]; then
      echo "∞" # Infinity
      return
    else
      echo "0"
      return
    fi
  fi
  
  local change=$(echo "scale=2; ($current - $baseline) / $baseline * 100" | bc -l)
  echo $change
}

# Function to determine if change is a regression
is_regression() {
  local metric_name=$1
  local change=$2
  
  # Skip if change is "∞"
  if [ "$change" == "∞" ]; then
    return 0
  fi
  
  # For certain metrics (like response time, error rates), higher is worse
  case $metric_name in
    *timings*|*duration*|*latency*|*response_time*|*error*|*failure*)
      if [ $(echo "$change > $THRESHOLD" | bc -l) -eq 1 ]; then
        return 0  # true, it's a regression
      fi
      ;;
    *)
      # For other metrics (like throughput), lower is worse
      if [ $(echo "$change < -$THRESHOLD" | bc -l) -eq 1 ]; then
        return 0  # true, it's a regression
      fi
      ;;
  esac
  
  return 1  # false, not a regression
}

# Load the test metadata
benchmark_date=$(jq -r '.generatedAt // "Unknown"' "$BENCHMARK_FILE")
current_date=$(jq -r '.generatedAt // "Unknown"' "$RESULTS_FILE")
test_name=$(jq -r '.tests[0].name // "Unknown"' "$RESULTS_FILE")

echo "===== PHOENIX ORCHESTRATOR PERFORMANCE COMPARISON =====" > "$OUTPUT_FILE"
echo "Benchmark: $benchmark_date" >> "$OUTPUT_FILE"
echo "Current: $current_date" >> "$OUTPUT_FILE"
echo "Test: $test_name" >> "$OUTPUT_FILE"
echo "Threshold: ±$THRESHOLD%" >> "$OUTPUT_FILE"
echo "===================================================" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Initialize regression flag
has_regression=false

# Compare common top-level metrics
echo "Overall Metrics:" >> "$OUTPUT_FILE"
echo "-----------------" >> "$OUTPUT_FILE"

# Parse and compare aggregated metrics
benchmark_avg_rt=$(jq -r '.aggregatedMetrics.avgResponseTime' "$BENCHMARK_FILE")
current_avg_rt=$(jq -r '.aggregatedMetrics.avgResponseTime' "$RESULTS_FILE")
rt_change=$(calc_percentage_change $benchmark_avg_rt $current_avg_rt)

benchmark_p95_rt=$(jq -r '.aggregatedMetrics.p95ResponseTime' "$BENCHMARK_FILE")
current_p95_rt=$(jq -r '.aggregatedMetrics.p95ResponseTime' "$RESULTS_FILE")
p95_change=$(calc_percentage_change $benchmark_p95_rt $current_p95_rt)

benchmark_error_rate=$(jq -r '.aggregatedMetrics.avgErrorRate' "$BENCHMARK_FILE")
current_error_rate=$(jq -r '.aggregatedMetrics.avgErrorRate' "$RESULTS_FILE")
error_change=$(calc_percentage_change $benchmark_error_rate $current_error_rate)

benchmark_reqs=$(jq -r '.aggregatedMetrics.totalRequests' "$BENCHMARK_FILE")
current_reqs=$(jq -r '.aggregatedMetrics.totalRequests' "$RESULTS_FILE")
reqs_change=$(calc_percentage_change $benchmark_reqs $current_reqs)

# Format and output the comparisons
printf "%-25s %-10s %-10s %-10s %-10s\n" "Metric" "Benchmark" "Current" "Change" "Status" >> "$OUTPUT_FILE"
printf "%-25s %-10.2f %-10.2f %-10s " "Avg Response Time (ms)" $benchmark_avg_rt $current_avg_rt "${rt_change}%" >> "$OUTPUT_FILE"
if is_regression "response_time" $rt_change; then
  printf "%-10s\n" "⚠️ WORSE" >> "$OUTPUT_FILE"
  has_regression=true
else
  printf "%-10s\n" "✅ OK" >> "$OUTPUT_FILE"
fi

printf "%-25s %-10.2f %-10.2f %-10s " "P95 Response Time (ms)" $benchmark_p95_rt $current_p95_rt "${p95_change}%" >> "$OUTPUT_FILE"
if is_regression "response_time" $p95_change; then
  printf "%-10s\n" "⚠️ WORSE" >> "$OUTPUT_FILE"
  has_regression=true
else
  printf "%-10s\n" "✅ OK" >> "$OUTPUT_FILE"
fi

printf "%-25s %-10.2f %-10.2f %-10s " "Error Rate" $benchmark_error_rate $current_error_rate "${error_change}%" >> "$OUTPUT_FILE"
if is_regression "error_rate" $error_change; then
  printf "%-10s\n" "⚠️ WORSE" >> "$OUTPUT_FILE"
  has_regression=true
else
  printf "%-10s\n" "✅ OK" >> "$OUTPUT_FILE"
fi

printf "%-25s %-10.2f %-10.2f %-10s " "Total Requests" $benchmark_reqs $current_reqs "${reqs_change}%" >> "$OUTPUT_FILE"
if is_regression "requests" $reqs_change; then
  printf "%-10s\n" "⚠️ WORSE" >> "$OUTPUT_FILE"
  has_regression=true
else
  printf "%-10s\n" "✅ OK" >> "$OUTPUT_FILE"
fi

echo "" >> "$OUTPUT_FILE"

# Compare test-level metrics
echo "Detailed Test Comparisons:" >> "$OUTPUT_FILE"
echo "--------------------------" >> "$OUTPUT_FILE"

# Iterate through tests in current results
jq -c '.tests[]' "$RESULTS_FILE" | while read -r test; do
  test_name=$(echo "$test" | jq -r '.name')
  
  echo "Test: $test_name" >> "$OUTPUT_FILE"
  
  # Find matching test in benchmark
  benchmark_test=$(jq -c ".tests[] | select(.name == \"$test_name\")" "$BENCHMARK_FILE")
  
  if [ -z "$benchmark_test" ]; then
    echo "  No matching benchmark data found" >> "$OUTPUT_FILE"
    continue
  fi
  
  # Extract metrics
  benchmark_duration=$(echo "$benchmark_test" | jq -r '.duration')
  current_duration=$(echo "$test" | jq -r '.duration')
  
  benchmark_avg_rt=$(echo "$benchmark_test" | jq -r '.avgResponseTime')
  current_avg_rt=$(echo "$test" | jq -r '.avgResponseTime')
  rt_change=$(calc_percentage_change $benchmark_avg_rt $current_avg_rt)
  
  benchmark_p95_rt=$(echo "$benchmark_test" | jq -r '.p95ResponseTime')
  current_p95_rt=$(echo "$test" | jq -r '.p95ResponseTime')
  p95_change=$(calc_percentage_change $benchmark_p95_rt $current_p95_rt)
  
  benchmark_error=$(echo "$benchmark_test" | jq -r '.errorRate')
  current_error=$(echo "$test" | jq -r '.errorRate')
  error_change=$(calc_percentage_change $benchmark_error $current_error)
  
  # Output comparison
  printf "  %-25s %-10.2f %-10.2f %-10s " "Avg Response Time (ms)" $benchmark_avg_rt $current_avg_rt "${rt_change}%" >> "$OUTPUT_FILE"
  if is_regression "response_time" $rt_change; then
    printf "%-10s\n" "⚠️ WORSE" >> "$OUTPUT_FILE"
    has_regression=true
  else
    printf "%-10s\n" "✅ OK" >> "$OUTPUT_FILE"
  fi
  
  printf "  %-25s %-10.2f %-10.2f %-10s " "P95 Response Time (ms)" $benchmark_p95_rt $current_p95_rt "${p95_change}%" >> "$OUTPUT_FILE"
  if is_regression "response_time" $p95_change; then
    printf "%-10s\n" "⚠️ WORSE" >> "$OUTPUT_FILE"
    has_regression=true
  else
    printf "%-10s\n" "✅ OK" >> "$OUTPUT_FILE"
  fi
  
  printf "  %-25s %-10.4f %-10.4f %-10s " "Error Rate" $benchmark_error $current_error "${error_change}%" >> "$OUTPUT_FILE"
  if is_regression "error_rate" $error_change; then
    printf "%-10s\n" "⚠️ WORSE" >> "$OUTPUT_FILE"
    has_regression=true
  else
    printf "%-10s\n" "✅ OK" >> "$OUTPUT_FILE"
  fi
  
  echo "" >> "$OUTPUT_FILE"
done

# Summarize the comparison
echo "===================================================" >> "$OUTPUT_FILE"
if [ "$has_regression" = true ]; then
  echo "⛔ REGRESSION DETECTED: Performance is worse than benchmark!" >> "$OUTPUT_FILE"
else
  echo "✅ SUCCESS: Performance is within acceptable thresholds." >> "$OUTPUT_FILE"
fi
echo "===================================================" >> "$OUTPUT_FILE"

# Output the results to stdout as well
cat "$OUTPUT_FILE"

# Return exit code based on regression status
if [ "$has_regression" = true ]; then
  exit 1
else
  exit 0
fi