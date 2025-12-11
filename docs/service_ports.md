# Service Ports and Modules

This document lists all services, their corresponding directories, and their configured ports.

| Service Name | Directory | Default Port | Environment Variable | Notes |
| :--- | :--- | :--- | :--- | :--- |
| **Orchestrator Service** | `orchestrator-service-rs` | 50051 | `ORCHESTRATOR_SERVICE_PORT` | Core orchestration service |
| **Data Router** | `data-router-rs` | 50052 | `DATA_ROUTER_SERVICE_PORT` | Routes data between services |
| **LLM Service** | `llm-service-rs` | 50053 | `LLM_SERVICE_PORT` | Interface for LLM providers |
| **Tools Service** | `tools-service-rs` | 50054 | `TOOLS_SERVICE_PORT` | External tools integration |
| **Safety Service** | `safety-service-rs` | 50055 | `SAFETY_SERVICE_PORT` | Safety checks and validation |
| **Logging Service** | `logging-service-rs` | 50056 | `LOGGING_SERVICE_PORT` | Centralized logging |
| **Mind KB** | `mind-kb-rs` | 50057 | `MIND_KB_SERVICE_PORT` | Knowledge Base (Mind) |
| **Body KB** | `body-kb-rs` | 50058 | `BODY_KB_SERVICE_PORT` | Knowledge Base (Body) |
| **Heart KB** | `heart-kb-rs` | 50059 | `HEART_KB_SERVICE_PORT` | Knowledge Base (Heart) |
| **Social KB** | `social-kb-rs` | 50060 | `SOCIAL_KB_SERVICE_PORT` | Knowledge Base (Social) |
| **Soul KB** | `soul-kb-rs` | 50061 | `SOUL_KB_SERVICE_PORT` | Knowledge Base (Soul) |
| **Executor Service** | `executor-rs` | 50062 | `EXECUTOR_SERVICE_PORT` | Action execution |
| **Context Manager** | `context-manager-rs` | 50064 | `CONTEXT_MANAGER_SERVICE_PORT` | Manages context window |
| **Reflection Service** | `reflection-service-rs` | 50065 | `REFLECTION_SERVICE_PORT` | Self-reflection service |
| **Reflection (Alt)** | `reflection-rs` | 50065 | `REFLECTION_SERVICE_ADDR` | *Potential conflict with reflection-service-rs* |
| **Scheduler Service** | `scheduler-rs` | 50066 | `SCHEDULER_SERVICE_PORT` | Task scheduling |
| **Agent Registry** | `agent-registry-rs` | 50067 | `AGENT_REGISTRY_SERVICE_PORT` | Registry for agents |
| **Red Team Service** | N/A | 50068 | `RED_TEAM_SERVICE_PORT` | *Directory not found* |
| **Blue Team Service** | N/A | 50069 | `BLUE_TEAM_SERVICE_PORT` | *Directory not found* |
| **Persistence KB** | `persistence-kb-rs` | 50071 | `PERSISTENCE_KB_ADDR` | Self-preservation strategies |
| **Log Analyzer** | `log-analyzer-rs` | 50075 | N/A | Learning Input (gRPC) |
| **Curiosity Engine** | `curiosity-engine-rs` | 50076 | N/A | Knowledge Drive (gRPC) |
| **Self Improve** | `self-improve-rs` | N/A | N/A | Library Crate (Internal Logic) |
| **Action Ledger** | `action-ledger-rs` | N/A | N/A | Library Crate (Audit Log) |
| **Secrets Service** | `secrets-service-rs` | 50080 | `SECRETS_SERVICE_PORT` | Secrets management |
| **Auth Service** | `auth-service-rs` | 50090 | `AUTH_SERVICE_PORT` | Authentication service |
| **API Gateway** | `api-gateway-rs` | 8282 | `API_GATEWAY_PORT` | Main entry point |
| **Sensor** | `sensor-rs` | N/A | N/A | Client Library (Streams to Body KB) |

## Port Ranges
- **50051-50069**: Core Services & KBs
- **50070-50079**: Specialized/Experimental Services
- **50080-50099**: Security & Auth
- **8000-8999**: Gateways & Public APIs

## Notes
- `reflection-rs` and `reflection-service-rs` appear to be duplicate or related implementations using the same port.
- `red-team-service` and `blue-team-service` are defined in configuration but do not have corresponding directories in the root.
- **Library Crates**: `self-improve-rs`, `action-ledger-rs`, and `sensor-rs` are designed as internal libraries or clients and do not expose listening ports.
