#!/bin/bash
set -e

# Phoenix Orchestrator Chaos Testing Runner
# This script introduces controlled chaos into the Phoenix Orchestrator environment
# to test system resilience and recovery capabilities.

# Usage: ./chaos-runner.sh [scenario] [duration] [target]
# Example: ./chaos-runner.sh network-latency 30s data-router

# Default values
SCENARIO=${1:-random}
DURATION=${2:-60s}
TARGET=${3:-random}
LOG_FILE="./results/chaos-test-$(date +%Y%m%d-%H%M%S).log"

# Available chaos scenarios
SCENARIOS=(
  "service-kill"
  "service-restart"
  "network-latency"
  "network-packet-loss"
  "network-partition"
  "cpu-stress"
  "memory-stress"
  "disk-stress"
  "clock-skew"
)

# Available targets
SERVICES=(
  "orchestrator-service"
  "data-router-service"
  "llm-service"
  "tools-service"
  "safety-service"
  "logging-service"
  "executor-service"
  "mind-kb"
  "body-kb"
  "heart-kb"
  "social-kb"
  "soul-kb"
  "context-manager-service"
  "reflection-service"
)

# Function to log messages
log() {
  echo "$(date +'%Y-%m-%d %H:%M:%S') - $1" | tee -a "$LOG_FILE"
}

# Select a random scenario if requested
if [ "$SCENARIO" = "random" ]; then
  SCENARIO=${SCENARIOS[$RANDOM % ${#SCENARIOS[@]}]}
  log "Randomly selected scenario: $SCENARIO"
fi

# Select a random target if requested
if [ "$TARGET" = "random" ]; then
  TARGET=${SERVICES[$RANDOM % ${#SERVICES[@]}]}
  log "Randomly selected target: $TARGET"
fi

# Ensure the results directory exists
mkdir -p ./results

# Log the test parameters
log "===== CHAOS TEST STARTED ====="
log "Scenario: $SCENARIO"
log "Target: $TARGET"
log "Duration: $DURATION"
log "============================"

# Helper function to check if the target exists
check_target() {
  docker ps --format '{{.Names}}' | grep -q "^$TARGET$"
  return $?
}

# Execute the chaos scenario
case "$SCENARIO" in
  service-kill)
    log "Executing service kill on $TARGET"
    if check_target; then
      log "Stopping service $TARGET"
      docker stop "$TARGET" || log "Failed to stop $TARGET"
      log "Waiting for specified duration: $DURATION"
      sleep $(echo "$DURATION" | sed 's/s$//')
      log "Restarting service $TARGET"
      docker start "$TARGET" || log "Failed to restart $TARGET"
      log "Service kill test completed"
    else
      log "Error: Target service $TARGET not found"
      exit 1
    fi
    ;;

  service-restart)
    log "Executing service restart on $TARGET"
    if check_target; then
      # Restart the service repeatedly
      count=$(echo "$DURATION" | sed 's/s$//' | awk '{print int($1/10)}')
      count=$((count > 0 ? count : 1))
      log "Will restart $TARGET $count times"
      
      for ((i=1; i<=count; i++)); do
        log "Restart cycle $i/$count"
        docker restart "$TARGET" || log "Failed to restart $TARGET"
        sleep 10
      done
      log "Service restart test completed"
    else
      log "Error: Target service $TARGET not found"
      exit 1
    fi
    ;;

  network-latency)
    log "Adding network latency to $TARGET"
    if check_target; then
      # Get container ID
      CONTAINER_ID=$(docker ps --filter "name=$TARGET" --format "{{.ID}}")
      
      # Add latency (100ms ± 30ms)
      log "Adding 100ms ± 30ms latency to $TARGET"
      docker exec "$CONTAINER_ID" tc qdisc add dev eth0 root netem delay 100ms 30ms distribution normal || log "Failed to add latency"
      
      log "Waiting for specified duration: $DURATION"
      sleep $(echo "$DURATION" | sed 's/s$//')
      
      # Remove the network constraint
      log "Removing network latency from $TARGET"
      docker exec "$CONTAINER_ID" tc qdisc del dev eth0 root || log "Failed to remove latency"

      log "Network latency test completed"
    else
      log "Error: Target service $TARGET not found"
      exit 1
    fi
    ;;

  network-packet-loss)
    log "Adding packet loss to $TARGET"
    if check_target; then
      # Get container ID
      CONTAINER_ID=$(docker ps --filter "name=$TARGET" --format "{{.ID}}")
      
      # Add packet loss (7% ± 1%)
      log "Adding 7% ± 1% packet loss to $TARGET"
      docker exec "$CONTAINER_ID" tc qdisc add dev eth0 root netem loss 7% 1% || log "Failed to add packet loss"
      
      log "Waiting for specified duration: $DURATION"
      sleep $(echo "$DURATION" | sed 's/s$//')
      
      # Remove the network constraint
      log "Removing packet loss from $TARGET"
      docker exec "$CONTAINER_ID" tc qdisc del dev eth0 root || log "Failed to remove packet loss"

      log "Packet loss test completed"
    else
      log "Error: Target service $TARGET not found"
      exit 1
    fi
    ;;

  network-partition)
    log "Creating network partition for $TARGET"
    if check_target; then
      # Get container ID
      CONTAINER_ID=$(docker ps --filter "name=$TARGET" --format "{{.ID}}")
      
      # Create network partition by blocking all traffic
      log "Blocking all network traffic for $TARGET"
      docker exec "$CONTAINER_ID" iptables -A INPUT -j DROP || log "Failed to block incoming traffic"
      docker exec "$CONTAINER_ID" iptables -A OUTPUT -j DROP || log "Failed to block outgoing traffic"
      
      log "Waiting for specified duration: $DURATION"
      sleep $(echo "$DURATION" | sed 's/s$//')
      
      # Remove the network constraint
      log "Removing network partition from $TARGET"
      docker exec "$CONTAINER_ID" iptables -F || log "Failed to restore network"

      log "Network partition test completed"
    else
      log "Error: Target service $TARGET not found"
      exit 1
    fi
    ;;

  cpu-stress)
    log "Adding CPU stress to $TARGET"
    if check_target; then
      # Get container ID
      CONTAINER_ID=$(docker ps --filter "name=$TARGET" --format "{{.ID}}")
      
      # Install stress-ng if not available
      docker exec "$CONTAINER_ID" which stress-ng || docker exec "$CONTAINER_ID" apt-get update -qq && docker exec "$CONTAINER_ID" apt-get install -y stress-ng
      
      # Create CPU stress with 2 workers at 80% load
      log "Starting CPU stress with 2 workers at 80% load on $TARGET"
      docker exec -d "$CONTAINER_ID" stress-ng --cpu 2 --cpu-load 80 --timeout $(echo "$DURATION" | sed 's/s$//') || log "Failed to start CPU stress"
      
      log "CPU stress will run for: $DURATION"
      sleep $(echo "$DURATION" | sed 's/s$//')s
      
      log "CPU stress test completed"
    else
      log "Error: Target service $TARGET not found"
      exit 1
    fi
    ;;

  memory-stress)
    log "Adding memory stress to $TARGET"
    if check_target; then
      # Get container ID
      CONTAINER_ID=$(docker ps --filter "name=$TARGET" --format "{{.ID}}")
      
      # Install stress-ng if not available
      docker exec "$CONTAINER_ID" which stress-ng || docker exec "$CONTAINER_ID" apt-get update -qq && docker exec "$CONTAINER_ID" apt-get install -y stress-ng
      
      # Calculate 80% of container memory limit
      MEM_LIMIT=$(docker inspect "$CONTAINER_ID" --format '{{.HostConfig.Memory}}')
      if [ "$MEM_LIMIT" = "0" ]; then
        # If no limit set, use 512MB as default
        MEM_STRESS=512M
      else
        MEM_STRESS=$(echo "$MEM_LIMIT * 0.8 / 1024 / 1024" | bc)M
      fi
      
      # Create memory stress
      log "Starting memory stress with $MEM_STRESS on $TARGET"
      docker exec -d "$CONTAINER_ID" stress-ng --vm 1 --vm-bytes "$MEM_STRESS" --timeout $(echo "$DURATION" | sed 's/s//') || log "Failed to start memory stress"
      
      log "Memory stress will run for: $DURATION"
      sleep $(echo "$DURATION" | sed 's/s$//')s
      
      log "Memory stress test completed"
    else
      log "Error: Target service $TARGET not found"
      exit 1
    fi
    ;;

  disk-stress)
    log "Adding disk I/O stress to $TARGET"
    if check_target; then
      # Get container ID
      CONTAINER_ID=$(docker ps --filter "name=$TARGET" --format "{{.ID}}")
      
      # Install stress-ng if not available
      docker exec "$CONTAINER_ID" which stress-ng || docker exec "$CONTAINER_ID" apt-get update -qq && docker exec "$CONTAINER_ID" apt-get install -y stress-ng
      
      # Create disk I/O stress
      log "Starting disk I/O stress on $TARGET"
      docker exec -d "$CONTAINER_ID" stress-ng --io 2 --timeout $(echo "$DURATION" | sed 's/s//') || log "Failed to start disk stress"
      
      log "Disk I/O stress will run for: $DURATION"
      sleep $(echo "$DURATION" | sed 's/s$//')s
      
      log "Disk I/O stress test completed"
    else
      log "Error: Target service $TARGET not found"
      exit 1
    fi
    ;;

  clock-skew)
    log "Introducing clock skew to $TARGET"
    if check_target; then
      # Get container ID
      CONTAINER_ID=$(docker ps --filter "name=$TARGET" --format "{{.ID}}")
      
      # Calculate a random time offset (±100 minutes)
      OFFSET=$((RANDOM % 200 - 100))
      NEW_TIME=$(date -d "$OFFSET minutes" "+%Y-%m-%d %H:%M:%S")
      
      log "Setting clock to $NEW_TIME on $TARGET (offset: $OFFSET minutes)"
      docker exec "$CONTAINER_ID" date -s "$NEW_TIME" || log "Failed to set clock"
      
      log "Waiting for specified duration: $DURATION"
      sleep $(echo "$DURATION" | sed 's/s$//')
      
      # Reset the clock
      CURRENT_TIME=$(date "+%Y-%m-%d %H:%M:%S")
      log "Resetting clock to $CURRENT_TIME on $TARGET"
      docker exec "$CONTAINER_ID" date -s "$CURRENT_TIME" || log "Failed to reset clock"
      
      log "Clock skew test completed"
    else
      log "Error: Target service $TARGET not found"
      exit 1
    fi
    ;;

  *)
    log "Error: Unknown scenario '$SCENARIO'"
    log "Available scenarios: ${SCENARIOS[*]}"
    exit 1
    ;;
esac

log "===== CHAOS TEST COMPLETED ====="
log "Results logged to: $LOG_FILE"