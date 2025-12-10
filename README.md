# 🧠 PHOENIX ORCH: Advanced Agentic AGI System

**PHOENIX ORCH** is a modular, high-performance microservice architecture designed for autonomous, multi-agent cognitive operations. Built in **Rust**, it leverages gRPC for low-latency inter-service communication and implements a sophisticated cognitive architecture with emotional, social, and ethical awareness.

---

##  🌟 System Status: Phase XI - RSI Closed Loop Complete

- **Microservices:** 25+ (Control, Cognitive, Functional, Agents, Persistence, RSI, Security)
- **Communication:** gRPC (Internal), REST (External via API Gateway)
- **Language:** Rust (2021/2024 edition)
- **Architecture:** Event-driven, capability-based delegation with emergency protocols
- **Safety Features:** Human-In-The-Loop (HITL) emergency protocols, transparent failure reporting

---

## 🌍 Environment Configuration

The system uses a **Template + Override** strategy to manage configurations across Development, Staging, and Production environments safely.

### Configuration Files
- **`.env`**: The **active configuration file** read by the application. **Do not edit manually** if using the switcher.
- **`.env.example.consolidated`**: The **Master Template** containing all possible variables.
- **`.env.dev`**: Development-specific overrides (Debug logging, local services).
- **`.env.staging`**: Staging-specific overrides (Info logging, staging endpoints).
- **`.env.production`**: Production-specific overrides (Warn logging, secure endpoints).

### Switching Environments
Use the provided scripts to switch environments. This will backup your current `.env`, copy the template, and apply the correct overrides.

**PowerShell (Windows):**
```powershell
.\env_switcher.ps1 -Environment development
.\env_switcher.ps1 -Environment staging
.\env_switcher.ps1 -Environment production
```

**Bash (Linux/Mac):**
```bash
./env_switcher.sh development
./env_switcher.sh staging
./env_switcher.sh production
```

---

## 🚀 Architecture Overview

The system is organized as a single **Rust Workspace** containing multiple crates. Below is a detailed reference of all modules.

### A. Control Plane (The Brain)

| Service | Port | Description | Inputs | Outputs |
| :--- | :--- | :--- | :--- | :--- |
| **Orchestrator** | 50051 | **Central Nervous System.** Coordinates planning, execution, and delegation. Implements the "Plan-Validate-Execute-Reflect" loop. | `Request` (User Query) | `AgiResponse` (Final Answer, Plan) |
| **Data Router** | 50052 | **Neural Bus.** Dynamic routing mesh. Handles service discovery and load balancing. | `RouteRequest` | `RouteResponse` |
| **Context Manager** | 50064 | **Working Memory.** Aggregates context from KBs and enriches LLM prompts with sentiment/identity. | `ContextRequest` | `EnrichedContext` |
| **Reflection Service** | 50065 | **Meta-Cognition.** Analyzes past actions to improve future performance (self-learning). | `ReflectionRequest` | `ReflectionResult` |
| **Scheduler** | 50066 | **Time Management.** CRON-based task scheduling and execution. | `ScheduleTaskRequest` | `ScheduleTaskResponse` |
| **Agent Registry** | 50067 | **Team Management.** Dynamic discovery of specialized agents based on capabilities. | `GetAgentRequest` | `AgentInfo` |

### B. RSI Layer (Recursive Self-Improvement)

| Service | Port | Description | Inputs | Outputs |
| :--- | :--- | :--- | :--- | :--- |
| **Log Analyzer** | 50075 | **Learning Input.** Analyzes execution logs for patterns, generates structured failure reports for learning. | `LogEntry` (Stream) | `AnalysisReport` |
| **Curiosity Engine** | 50076 | **Knowledge Drive.** Identifies knowledge gaps and proactively generates high-priority research tasks. | `KnowledgeGap` | `ResearchTask` |

### C. Cognitive Layer (The Soul)

| Service | Port | Specialization | Inputs | Outputs |
| :--- | :--- | :--- | :--- | :--- |
| **Mind-KB** | 50057 | **Facts & Logic.** Short-term episodic memory and vector-based semantic search. | `QueryRequest` | `QueryResponse` |
| **Body-KB** | 50058 | **Physical State.** Sensor data, system health, and environment context. | `StoreRequest` | `StoreResponse` |
| **Heart-KB** | 50059 | **Emotion.** Tracks sentiment (Neutral, Urgent, Frustrated) and emotional shifts. | `StoreSentimentRequest` | `StoreSentimentResponse` |
| **Social-KB** | 50060 | **Identity.** Manages user profiles, roles, and communication preferences. | `GetUserRequest` | `UserIdentity` |
| **Soul-KB** | 50061 | **Ethics.** Immutable core values and ethical constraint enforcement. | `CheckEthicsRequest` | `EthicsCheckResponse` |
| **Persistence-KB** | 50071 | **Self-Preservation.** Threat detection, emergency protocols, and system continuity strategies. | `StoreRequest` | `StoreResponse` |

### D. Functional Layer (The Body)

| Service | Port | Role | Inputs | Outputs |
| :--- | :--- | :--- | :--- | :--- |
| **LLM Service** | 50053 | Interface to LLM providers (OpenAI, Anthropic, Local). Handles generation and embedding. | `GenerateRequest` | `GenerateResponse` |
| **Tools Service** | 50054 | Safe execution of external tools (Web Search, Calculator, Code Execution). | `ToolRequest` | `ToolResponse` |
| **Safety Service** | 50055 | Input/Output filtering, PII redaction, and threat detection. | `ValidationRequest` | `ValidationResponse` |
| **Logging Service** | 50056 | Centralized telemetry and structured logging. | `LogEntry` | `LogResponse` |
| **Sensor Service** | 50062 | Hardware/System monitoring (CPU, Memory, Network). | `GetMetricsRequest` | `MetricsResponse` |
| **Executor Service** | 50055 | **Sandboxed Runtime.** Native Windows execution of shell commands and scripts. | `CommandRequest` | `CommandResponse` |

### E. Specialized Agents (The Team)

| Service | Port | Role | Inputs | Outputs |
| :--- | :--- | :--- | :--- | :--- |
| **Red Team** | 50068 | **Adversary.** Vulnerability scanning, attack simulation, security auditing. | `ScanRequest` | `ScanResult` |
| **Blue Team** | 50069 | **Defender.** Threat containment, system hardening, incident response. | `AnomalyTriageRequest` | `TriageResult` |

### F. Security & Infrastructure

| Service | Port | Role | Inputs | Outputs |
| :--- | :--- | :--- | :--- | :--- |
| **Auth Service** | 50090 | **Identity Provider.** JWT issuance, RBAC enforcement, and mTLS certificate management. | `LoginRequest` | `AuthToken` |
| **Secrets Service** | 50080 | **Vault.** Secure storage for API keys, certificates, and sensitive configuration. | `GetSecretRequest` | `SecretValue` |

### G. Gateway (The Interface)

| Service | Port | Role | Inputs | Outputs |
| :--- | :--- | :--- | :--- | :--- |
| **API Gateway** | 8000 | **REST Interface.** Exposes `POST /api/v1/execute` to external clients. Translates JSON to gRPC. | `JSON Payload` | `JSON Response` |

### H. Shared Libraries

| Crate | Description |
| :--- | :--- |
| **tool-sdk** | SDK for building and registering new tools. |
| **config-rs** | Centralized configuration management and service discovery utilities. |
| **input-validation-rs** | Common validation logic for sanitizing inputs across services. |
| **shared-types-rs** | Common Rust types and traits shared across the workspace. |

---

## 🔒 Executor Service - Windows Native Implementation

The **Executor Service** (Port 50055) has been refactored from Docker-based containerization to **Windows native execution** using low-level Windows APIs for enhanced security and performance.

### Key Features

#### Security Architecture
- **Windows Job Objects**: Process isolation with resource limits (100MB/process, 500MB total)
- **Low Integrity Level**: Restricted process privileges (pending full implementation)
- **Sandboxed Execution**: All code runs in `C:\phoenix_sandbox` with path validation
- **Process Watchdog**: 30-second timeout enforcement with automatic termination

#### Resource Management
- **Memory Limits**: 100MB per process, 500MB for entire job
- **Process Limits**: Maximum 5 concurrent processes
- **CPU Controls**: Configurable CPU rate limiting
- **Automatic Cleanup**: Job Object ensures child process termination

#### Command Validation
- **Allowlist Enforcement**: Only permitted commands can execute
- **Path Validation**: File operations restricted to sandbox directory
- **Error Sanitization**: Sensitive information removed from error messages

### Deployment

See [`executor-rs-deployment-guide.md`](executor-rs-deployment-guide.md) for detailed installation and configuration instructions.

**Quick Start:**
```powershell
# Windows (PowerShell as Administrator)
New-Item -ItemType Directory -Path "C:\phoenix_sandbox" -Force
.\target\release\executor-rs.exe

# Verify service is running
Test-NetConnection -ComputerName localhost -Port 50055
```

### Architecture Documentation

For complete technical details, see:
- [`executor-rs-windows-architecture.md`](executor-rs-windows-architecture.md) - Full architecture and component documentation
- [`executor-rs-testing-report.md`](executor-rs-testing-report.md) - Testing results and identified issues

---

## 🛠️ Build & Development

### Prerequisites
- **Rust**: Latest stable version (2021/2024 edition)
- **Protoc**: Protocol Buffers compiler (handled via `protoc-bin-vendored` in build scripts)
- **OpenSSL**: Required for some dependencies (on Windows, use `vcpkg` or pre-built binaries)

### Building the Workspace
```powershell
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace
```

### Running Services
Services can be run individually or orchestrated via scripts (coming soon).
```powershell
# Example: Run the Orchestrator
cargo run -p orchestrator-service-rs
```

---

## 📋 Development Guidelines

- Coordinate with Soul-KB for ethical boundary checks
- Use the Logging Service for immutable audit trails
- Test emergency shutdown procedures thoroughly
- Feed execution outcomes to Log Analyzer for RSI learning

### RSI Loop Development
When building self-improvement features:
- Route all execution outcomes to Log Analyzer
- Store lessons and constraints in Soul-KB with immediate use flag
- Monitor Temporal Utility Score in Planning KB
- Integrate with Curiosity Engine for knowledge acquisition
- Prioritize self-improvement tasks in the Scheduler