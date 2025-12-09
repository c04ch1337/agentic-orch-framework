# Phoenix ORCH System Port Map

## Summary of Findings
After auditing the codebase, I've identified several issues with port configuration:

1. **Inconsistent environment variable naming patterns**:
   - Some services use `SERVICE_PORT` variables
   - Others use `SERVICE_ADDR` variables
   - A few have hardcoded addresses with no env var overrides

2. **Inconsistent fallback patterns**:
   - Most services properly read from environment variables with fallbacks
   - Orchestrator service has a hardcoded address without env var override
   - Some client configurations have hardcoded addresses

3. **Non-sequential port allocation for some services**:
   - Secrets Service uses port 50080 instead of following the sequential numbering
   - Auth Service uses port 50090

## Current Port Allocation Map

| Service | Crate Path | Protocol | Default Bind Address | Port | Env Variable Override |
|---------|------------|----------|---------------------|------|----------------------|
| OrchestratorService | orchestrator-service-rs | gRPC | 0.0.0.0:50051 | 50051 | Hardcoded (no env var) |
| DataRouterService | data-router-rs | gRPC | 0.0.0.0:50052 | 50052 | DATA_ROUTER_ADDR |
| LLMService | llm-service-rs | gRPC | 0.0.0.0:50053 | 50053 | LLM_SERVICE_ADDR |
| ToolsService | tools-service-rs | gRPC | 0.0.0.0:50054 | 50054 | TOOLS_SERVICE_ADDR |
| SafetyService | safety-service-rs | gRPC | 0.0.0.0:50055 | 50055 | SAFETY_SERVICE_ADDR |
| LoggingService | logging-service-rs | gRPC | 0.0.0.0:50056 | 50056 | LOGGING_SERVICE_ADDR |
| MindKBService | mind-kb-rs | gRPC | 0.0.0.0:50057 | 50057 | MIND_KB_ADDR |
| BodyKBService | body-kb-rs | gRPC | 0.0.0.0:50058 | 50058 | BODY_KB_ADDR |
| HeartKBService | heart-kb-rs | gRPC | 0.0.0.0:50059 | 50059 | HEART_KB_ADDR |
| SocialKBService | social-kb-rs | gRPC | 0.0.0.0:50060 | 50060 | SOCIAL_KB_ADDR |
| SoulKBService | soul-kb-rs | gRPC | 0.0.0.0:50061 | 50061 | SOUL_KB_ADDR |
| ExecutorService | executor-rs | gRPC | 0.0.0.0:50062 | 50062 | EXECUTOR_ADDR |
| ContextManagerService | context-manager-rs | gRPC | 0.0.0.0:50064 | 50064 | CONTEXT_MANAGER_ADDR |
| ReflectionService | reflection-rs | gRPC | 0.0.0.0:50065 | 50065 | REFLECTION_SERVICE_ADDR |
| SchedulerService | scheduler-rs | gRPC | 0.0.0.0:50066 | 50066 | SCHEDULER_SERVICE_ADDR |
| AgentRegistryService | agent-registry-rs | gRPC | 0.0.0.0:50067 | 50067 | AGENT_REGISTRY_ADDR |
| RedTeamService | red-team-rs | gRPC | 0.0.0.0:50068 | 50068 | RED_TEAM_ADDR |
| BlueTeamService | blue-team-rs | gRPC | 0.0.0.0:50069 | 50069 | BLUE_TEAM_ADDR |
| SecretsService | secrets-service-rs | gRPC | 0.0.0.0:50080 | 50080 | SECRETS_SERVICE_PORT |
| AuthService | auth-service-rs | gRPC | 0.0.0.0:50090 | 50090 | AUTH_SERVICE_ADDR (implied) |
| API Gateway | api-gateway-rs | HTTP | 0.0.0.0:8282 | 8282 | API_GATEWAY_PORT |

## Client Configuration Issues
1. API Gateway uses `ORCHESTRATOR_ADDR` with fallback to hardcoded `http://127.0.0.1:50051`
2. SecretsClient uses `SECRETS_SERVICE_ADDR` with fallback to hardcoded `http://localhost:50080`
3. API Gateway uses `AUTH_SERVICE_ADDR` with fallback to hardcoded `http://127.0.0.1:50090` 

## Recommended Configuration Pattern
For consistency, all services should follow this pattern:
```rust
// Environment variables
let port = env::var("SERVICE_NAME_PORT").unwrap_or_else(|_| "50xxx".to_string());
let addr = format!("0.0.0.0:{}", port).parse()?;

// Client connections
let addr = env::var("SERVICE_NAME_ADDR").unwrap_or_else(|_| {
    let default_port = env::var("SERVICE_NAME_PORT").unwrap_or_else(|_| "50xxx".to_string());
    format!("http://localhost:{}", default_port)
});
```

This pattern ensures:
- Consistent naming convention for all services
- Port numbers can be overridden in one place
- Client connections respect port overrides
- Sensible defaults are always available