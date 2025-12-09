# Phoenix ORCH Port Management Verification Plan

## Overview

This document outlines the steps to verify the port management and configuration changes made to standardize and normalize how ports are configured across all services in the Phoenix ORCH system.

## Prerequisites

1. All code changes have been applied:
   - New `config-rs` crate added to the workspace
   - Updated service configuration patterns in key services
   - Normalized `.env.example.normalized` file

2. Install Protocol Buffers compiler (protoc) - required for building gRPC services:
   - Unix/Linux/macOS: `./install_protoc.sh` (may need `chmod +x install_protoc.sh` first)
   - Windows: Run PowerShell as Administrator, then `.\install_protoc.ps1`
   
3. Verify protoc installation: `protoc --version`

## Verification Steps

### 1. Build Verification

```bash
# Ensure our config-rs builds correctly
cargo check -p config-rs

# Verify that services with modified configuration build
cargo check --bin api-gateway-rs
cargo check --bin orchestrator-service-rs
cargo check --bin llm-service-rs
cargo check --bin secrets-service-rs
cargo check --bin executor-rs
```

### 2. Port Configuration Verification

For each service that was modified, verify that:

1. **Default Port Binding**: Service binds to the correct default port when no environment variables are set.
   ```bash
   # Run the service with default configuration
   cargo run --bin orchestrator-service-rs
   ```
   Expected: Service should log that it's starting on the correct default port (50051 for orchestrator)

2. **Environment Variable Override**: Service respects port and address overrides via environment variables.
   ```bash
   # On Unix/Linux/macOS
   export ORCHESTRATOR_PORT=50551
   cargo run --bin orchestrator-service-rs
   
   # On Windows PowerShell
   $env:ORCHESTRATOR_PORT=50551; cargo run --bin orchestrator-service-rs
   ```
   Expected: Service should log that it's starting on the overridden port (50551)

3. **Client Connectivity**: Verify that clients can connect to the service using the standardized addressing pattern.
   ```bash
   # On Unix/Linux/macOS
   export ORCHESTRATOR_PORT=50551
   cargo run --bin data-router-rs

   # On Windows PowerShell
   $env:ORCHESTRATOR_PORT=50551; cargo run --bin data-router-rs
   ```
   Expected: Data router should successfully connect to the orchestrator at the overridden port

### 3. Service Connectivity Verification

Test the full chain of service-to-service communication to ensure port resolution works:

1. **Start All Core Services**:
   ```bash
   # Set the environment variables from .env.example.normalized
   source .env.example.normalized
   
   # In separate terminals:
   cargo run --bin orchestrator-service-rs
   cargo run --bin data-router-rs
   cargo run --bin llm-service-rs
   cargo run --bin secrets-service-rs
   ```

2. **Test API Gateway calls**: Verify that the API Gateway can route requests through the service chain.
   ```bash
   cargo run --bin api-gateway-rs
   curl -X POST http://localhost:8000/api/v1/execute -H "Content-Type: application/json" -d '{"method": "generate_text", "payload": "{\"prompt\": \"Test the connectivity\"}"}'
   ```
   Expected: Request should successfully propagate through the service chain and return a response

3. **Port Conflict Test**: Verify that services won't conflict when environment variables are properly configured.
   ```bash
   # On Unix/Linux/macOS
   export ORCHESTRATOR_PORT=50091
   export DATA_ROUTER_PORT=50092

   # On Windows PowerShell
   $env:ORCHESTRATOR_PORT=50091
   $env:DATA_ROUTER_PORT=50092

   # In separate terminals:
   cargo run --bin orchestrator-service-rs
   cargo run --bin data-router-rs
   ```
   Expected: Both services should start successfully on their respective ports with no conflicts

### 4. Documentation Verification

1. Confirm the final Port Map document contains:
   - All services and their ports
   - The standardized environment variable names for overrides
   - No port conflicts between services
   - Proper routing and connectivity information

## Rollback Plan

In case issues are detected:

1. Revert the changes to `Cargo.toml` files removing the dependency on `config-rs`
2. Revert the changes to all modified service files
3. Restore original addressing patterns

## Success Criteria

The verification is considered successful when:

1. All services build successfully with the new configuration pattern
2. Services can be started with both default and overridden port configurations
3. Service-to-service communication functions correctly
4. No port conflicts are observed when environment variables are properly set
5. The FINAL_PORT_MAP.md document accurately reflects the configuration of all services
6. The protoc installation scripts work on both Unix/Linux/macOS and Windows systems

## Sign-off

After successful verification, update the Port Map with any additional findings and mark the task as complete.