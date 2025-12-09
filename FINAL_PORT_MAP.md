# Phoenix ORCH Backend Port Map

This document provides a standardized reference for all service ports in the Phoenix ORCH backend system. Use it as a single source of truth for service addressing in development, staging, and production environments.

## Port Configuration Guidelines

1. **Env Variable Pattern**: `SERVICE_NAME_PORT` (e.g., `API_GATEWAY_PORT`)
2. **Default Bind Address**: `0.0.0.0` (configurable via `SERVICE_NAME_BIND_ADDR`)
3. **Client Connection Pattern**: Use `config-rs::ServiceConfig` methods:
   - `get_service_port(default_port)`: Get configured port for a service
   - `get_bind_address(port)`: Get the bind address with port
   - `get_client_address(service_name, default_port)`: Get client connection address

## Service Port Allocations

| Service Name | Crate Path | Protocol | Default Bind Addr | Default Port | Env Override |
|--------------|------------|----------|------------------|-------------|--------------|
| api-gateway | api-gateway-rs | HTTP/REST | 0.0.0.0 | 8000 | API_GATEWAY_PORT |
| orchestrator | orchestrator-service-rs | gRPC | 0.0.0.0 | 50051 | ORCHESTRATOR_PORT |
| llm-service | llm-service-rs | gRPC | 0.0.0.0 | 50052 | LLM_SERVICE_PORT |
| auth-service | auth-service-rs | gRPC | 0.0.0.0 | 50090 | AUTH_SERVICE_PORT |
| secrets-service | secrets-service-rs | gRPC | 0.0.0.0 | 50080 | SECRETS_SERVICE_PORT |
| data-router | data-router-rs | gRPC | 0.0.0.0 | 50060 | DATA_ROUTER_PORT |
| body-kb | body-kb-rs | gRPC | 0.0.0.0 | 50061 | BODY_KB_PORT |
| heart-kb | heart-kb-rs | gRPC | 0.0.0.0 | 50062 | HEART_KB_PORT |
| agent-registry | agent-registry-rs | gRPC | 0.0.0.0 | 50070 | AGENT_REGISTRY_PORT |
| executor | executor-rs | gRPC | 0.0.0.0 | 50055 | EXECUTOR_PORT |
| context-manager | context-manager-rs | gRPC | 0.0.0.0 | 50056 | CONTEXT_MANAGER_PORT |
| blue-team | blue-team-rs | gRPC | 0.0.0.0 | 50095 | BLUE_TEAM_PORT |
| persistence-kb | persistence-kb-rs | gRPC | 0.0.0.0 | 50071 | PERSISTENCE_KB_PORT |
| deceive-kb | deceive-kb-rs | gRPC | 0.0.0.0 | 50073 | DECEIVE_KB_PORT |
| log-analyzer | log-analyzer-rs | gRPC | 0.0.0.0 | 50075 | LOG_ANALYZER_PORT |
| curiosity-engine | curiosity-engine-rs | gRPC | 0.0.0.0 | 50076 | CURIOSITY_ENGINE_PORT |
| scheduler | scheduler-rs | gRPC | 0.0.0.0 | 50066 | SCHEDULER_PORT |
| reflection-service | reflection-service-rs | gRPC | 0.0.0.0 | 50065 | REFLECTION_PORT |

## Client Configuration Examples

Example of configuring a service to connect to the Orchestrator:

```rust
// Get client address using standard config
let config = ServiceConfig::new("my-service");
let orchestrator_addr = config.get_client_address("orchestrator", 50051);

// Use it to connect
let client = OrchestratorServiceClient::connect(orchestrator_addr).await?;
```

## Development Setup

For local development, default values are sufficient. You can set environment variables to override defaults:

```bash
# Override ports
export API_GATEWAY_PORT=9000
export ORCHESTRATOR_PORT=50052

# Run services
cargo run --bin api-gateway-rs
cargo run --bin orchestrator-service-rs
```

## Environment Variables

The following environment variables can be set to override default configurations:

1. **Port Overrides**:
   - `SERVICE_NAME_PORT` - Override the default port (e.g., `API_GATEWAY_PORT=9000`)

2. **Bind Address Overrides**:
   - `SERVICE_NAME_BIND_ADDR` - Override the default bind address (e.g., `API_GATEWAY_BIND_ADDR=127.0.0.1`)

3. **Client Connection Overrides**: 
   - `SERVICE_NAME_ADDR` - Override the full client connection address (e.g., `ORCHESTRATOR_ADDR=http://orchestrator:50051`)

## Deployment Best Practices

1. Always set explicit ports in production environments
2. For containerized deployments, document port mappings in docker-compose files
3. Ensure port settings match between client and server configurations

## Troubleshooting

If you encounter connection issues:

1. Verify the service is running (`ps aux | grep service-name`)
2. Check if the port is being listened on (`netstat -tuln | grep PORT`)
3. Ensure no port conflicts exist
4. Verify environment variables are correctly set