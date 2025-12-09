# 🧠 PHOENIX ORCH: Advanced Agentic AGI System

**PHOENIX ORCH** is a modular, high-performance microservice architecture designed for autonomous, multi-agent cognitive operations. Built in **Rust**, it leverages gRPC for low-latency inter-service communication and implements a sophisticated cognitive architecture with emotional, social, and ethical awareness.

---

##  🌟 System Status: Phase XI - RSI Closed Loop Complete

- **Microservices:** 23 (Control, Cognitive, Functional, Agents, Persistence, RSI)
- **Communication:** gRPC (Internal), REST (External via API Gateway)
- **Language:** Rust (2021/2024 edition)
- **Architecture:** Event-driven, capability-based delegation with emergency protocols
- **Safety Features:** Human-In-The-Loop (HITL) emergency protocols, transparent failure reporting

---

## 🚀 Architecture Overview

The system is organized as a single **Rust Workspace** containing 23 crates.

### A. Control Plane (The Brain)

| Service | Port | Description |
| :--- | :--- | :--- |
| **Orchestrator** | 50051 | **Central Nervous System.** Coordinates planning, execution, and delegation. Implements the "Plan-Validate-Execute-Reflect" loop. |
| **Data Router** | 50052 | **Neural Bus.** Dynamic routing mesh. Handles service discovery and load balancing. |
| **Context Manager** | 50064 | **Working Memory.** Aggregates context from KBs and enriches LLM prompts with sentiment/identity. |
| **Reflection Service** | 50065 | **Meta-Cognition.** Analyzes past actions to improve future performance (self-learning). |
| **Scheduler** | 50066 | **Time Management.** CRON-based task scheduling and execution. |
| **Agent Registry** | 50067 | **Team Management.** Dynamic discovery of specialized agents based on capabilities. |

### B. RSI Layer (Recursive Self-Improvement)

| Service | Port | Description |
| :--- | :--- | :--- |
| **Log Analyzer** | 50075 | **Learning Input.** Analyzes execution logs for patterns, generates structured failure reports for learning. |
| **Curiosity Engine** | 50076 | **Knowledge Drive.** Identifies knowledge gaps and proactively generates high-priority research tasks. |

### C. Cognitive Layer (The Soul)

| Service | Port | Specialization |
| :--- | :--- | :--- |
| **Mind-KB** | 50057 | **Facts & Logic.** Short-term episodic memory and vector-based semantic search. |
| **Body-KB** | 50058 | **Physical State.** Sensor data, system health, and environment context. |
| **Heart-KB** | 50059 | **Emotion.** Tracks sentiment (Neutral, Urgent, Frustrated) and emotional shifts. |
| **Social-KB** | 50060 | **Identity.** Manages user profiles, roles, and communication preferences. |
| **Soul-KB** | 50061 | **Ethics.** Immutable core values and ethical constraint enforcement. |
| **Persistence-KB** | 50071 | **Self-Preservation.** Threat detection, emergency protocols, and system continuity strategies. |

### D. Functional Layer (The Body)

| Service | Port | Role |
| :--- | :--- | :--- |
| **LLM Service** | 50053 | Interface to LLM providers (OpenAI, Anthropic, Local). Handles generation and embedding. |
| **Tools Service** | 50054 | Safe execution of external tools (Web Search, Calculator, Code Execution). |
| **Safety Service** | 50055 | Input/Output filtering, PII redaction, and threat detection. |
| **Logging Service** | 50056 | Centralized telemetry and structured logging. |
| **Sensor Service** | 50062 | Hardware/System monitoring (CPU, Memory, Network). |
| **Executor Service** | 50063 | Sandboxed command execution runtime. |

### E. Specialized Agents (The Team)

| Service | Port | Role |
| :--- | :--- | :--- |
| **Red Team** | 50068 | **Adversary.** Vulnerability scanning, attack simulation, security auditing. |
| **Blue Team** | 50069 | **Defender.** Threat containment, system hardening, incident response. |

### F. Gateway (The Interface)

| Service | Port | Role |
| :--- | :--- | :--- |
| **API Gateway** | 8000 | **REST Interface.** Exposes `POST /api/v1/execute` to external clients. Translates JSON to gRPC. |

---

## 🛠️ Build and Run

### Prerequisites
- **Rust:** Latest stable toolchain (`rustup update`).
- **Protobuf Compiler:** `protoc` (required for code generation).

### 1. Build Workspace
Compile all 20 services in parallel:
```bash
cargo build --workspace
```

### 2. Run Services
You can run services individually or via a process manager.
```bash
# Example: Run Orchestrator
cargo run -p orchestrator-service-rs

# Example: Run API Gateway
cargo run -p api-gateway-rs
```

### 3. Verification
Check if all services compile without errors:
```bash
cargo check --workspace
```

---

## 🔌 API Usage

### Execute a Request
Send a natural language request to the system via the API Gateway.

**Endpoint:** `POST http://localhost:8000/api/v1/execute`
**Auth:** header `Authorization: Bearer phoenix-default-key`

**Payload:**
```json
{
  "method": "PlanAndExecute",
  "payload": "Analyze the system logs for suspicious activity and generate a report."
}
```

**Response:**
```json
{
  "id": "uuid-...",
  "status_code": 200,
  "payload": "Orchestrator completed PlanAndExecute...",
  "metadata": {
    "plan": "1. Scan logs... 2. Identify threats...",
    "routed_to": "blue-team-agent"
  }
}

---

## 🔐 Emergency Protocols & Safety Features

### HITL (Human-In-The-Loop) Emergency Protocols

The system implements robust safety mechanisms for emergency scenarios:

- **Emergency Directive Approval**: All emergency actions triggered by the Persistence KB require human confirmation via the Safety Service
- **Audit Trail**: All emergency protocol activations are logged immutably for human review
- **Graceful Degradation**: Instead of evasion, the system prioritizes transparent failure reporting
- **Circuit Breaker Patterns**: Human operators can disable emergency features if needed
- **Recursive Self-Improvement**: The RSI Closed Loop continuously learns from failures and improves system performance

### RSI Closed Loop Architecture

The **Recursive Self-Improvement (RSI) Closed Loop** is a critical system feature that enables continuous learning and self-enhancement:

1. **Execution → Analysis**:
   - All plan executions flow through the Log Analyzer for pattern detection
   - Failures are automatically classified by severity and root cause

2. **Analysis → Learning**:
   - Critical failures trigger automatic constraint generation
   - Constraints are immediately stored in Soul KB for future planning

3. **Learning → Research**:
   - Knowledge gaps are proactively identified by Temporal Utility scoring
   - Curiosity Engine continuously schedules high-priority research tasks

4. **Research → Improvement**:
   - Self-improvement tasks are given execution priority
   - System self-preserves by maintaining strategic utility above acceptable thresholds

### Transparent Failure Reporting

- **Real-time Alerts**: Emergency protocol activations generate immediate alerts to operators
- **Failure Documentation**: All system failures are documented with root cause analysis
- **Ethical Boundary Verification**: Persistence KB actions are validated against Soul-KB ethical constraints
- **Recovery Protocols**: Automated recovery attempts with human oversight
```

---

## 🔧 Troubleshooting

### Common Issues

1.  **Port Conflicts:**
    *   Ensure ports 50051-50069 and 8000 are free.
    *   Check `netstat -ano | findstr <port>` on Windows.

2.  **Protobuf Errors:**
    *   Ensure `protoc` is in your system PATH.
    *   If `agi_core.proto` changes, run `cargo clean` and rebuild.

3.  **Service Discovery Failures:**
    *   Ensure **Data Router** (50052) is running. It is required for all inter-service communication.
    *   Check logs for "Failed to connect to Data Router".

4.  **LLM Connection:**
    *   Set `LLM_PROVIDER` and `LLM_API_KEY` environment variables if using external providers.
    *   Default is `mock` provider for testing.

5.  **Safety System Status:**
    *   Check Safety Service logs for HITL protocol activations
    *   Monitor Persistence KB for emergency protocol readiness
    *   Verify emergency contact channels are configured for human oversight

---

## 📊 Complete Service Matrix

| Layer | Service | Port | Description | Emergency Protocol |
| :--- | :--- | :--- | :--- | :--- |
| **Control** | **Orchestrator** | 50051 | Central planning and execution coordination | Manual override |
| **Control** | **Data Router** | 50052 | Service routing and discovery | Emergency routing bypass |
| **Control** | **Context Manager** | 50064 | Working memory and context enrichment | Context preservation |
| **Control** | **Reflection Service** | 50065 | Meta-cognition and self-improvement | Ethical reflection pause |
| **Control** | **Scheduler** | 50066 | CRON-based task scheduling | Task suspension |
| **Control** | **Agent Registry** | 50067 | Agent discovery and management | Agent isolation |
| **RSI** | **Log Analyzer** | 50075 | Execution log analysis and learning | Failure pattern detection |
| **RSI** | **Curiosity Engine** | 50076 | Knowledge gap identification | Knowledge prioritization |
| **Cognitive** | **Mind-KB** | 50057 | Facts and logical reasoning | Data backup priority |
| **Cognitive** | **Body-KB** | 50058 | Physical and system state | Health monitoring |
| **Cognitive** | **Heart-KB** | 50059 | Emotional state tracking | Emotional stability check |
| **Cognitive** | **Social-KB** | 50060 | User identity management | Access control verification |
| **Cognitive** | **Soul-KB** | 50061 | Ethical constraint enforcement | Ethical override protection |
| **Cognitive** | **Persistence-KB** | 50071 | Self-preservation strategies | HITL emergency protocols |
| **Functional** | **LLM Service** | 50053 | LLM interface and processing | Response filtering |
| **Functional** | **Tools Service** | 50054 | External tool execution | Execution sandboxing |
| **Functional** | **Safety Service** | 50055 | Policy enforcement and threat detection | Emergency escalation |
| **Functional** | **Logging Service** | 50056 | Centralized telemetry | Immutable audit logs |
| **Functional** | **Sensor Service** | 50062 | Hardware/system monitoring | Alert escalation |
| **Functional** | **Executor Service** | 50063 | Command execution runtime | Execution limits |
| **Agents** | **Red Team** | 50068 | Security testing and vulnerability scanning | Controlled testing |
| **Agents** | **Blue Team** | 50069 | Threat detection and incident response | Emergency containment |
| **Gateway** | **API Gateway** | 8000 | REST interface for external clients | Request throttling |

## 📚 Development Guide

### Adding a New Service
1.  Create new crate: `cargo new my-service-rs`
2.  Add to `Cargo.toml` workspace members.
3.  Add dependencies (`tonic`, `prost`, `tokio`).
4.  Define service in `.proto/agi_core.proto`.
5.  Implement `main.rs` with gRPC server.
6.  Register with **Data Router** or **Agent Registry**.

### Modifying Protobufs
1.  Edit `.proto/agi_core.proto`.
2.  Run `cargo build` to trigger `tonic-build` recompilation.
3.  Update service implementations to match new trait signatures.

### Emergency Protocol Development
When developing emergency or persistence-related features:
- Always integrate with Safety Service for HITL approval
- Implement transparent failure reporting mechanisms
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