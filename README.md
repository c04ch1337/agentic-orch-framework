# 🧠 PHOENIX ORCH: Advanced Agentic AGI System

**PHOENIX ORCH** is a modular, high-performance microservice architecture designed for autonomous, multi-agent cognitive operations. Built in **Rust**, it leverages gRPC for low-latency inter-service communication and implements a sophisticated cognitive architecture with emotional, social, and ethical awareness.

---

## 🌟 System Status: Phase 7 Complete

- **Microservices:** 20 (Control, Cognitive, Functional, Agents)
- **Communication:** gRPC (Internal), REST (External via API Gateway)
- **Language:** Rust (2021/2024 edition)
- **Architecture:** Event-driven, capability-based delegation

---

## 🚀 Architecture Overview

The system is organized as a single **Rust Workspace** containing 20 crates.

### A. Control Plane (The Brain)

| Service | Port | Description |
| :--- | :--- | :--- |
| **Orchestrator** | 50051 | **Central Nervous System.** Coordinates planning, execution, and delegation. Implements the "Plan-Validate-Execute-Reflect" loop. |
| **Data Router** | 50052 | **Neural Bus.** Dynamic routing mesh. Handles service discovery and load balancing. |
| **Context Manager** | 50064 | **Working Memory.** Aggregates context from KBs and enriches LLM prompts with sentiment/identity. |
| **Reflection Service** | 50065 | **Meta-Cognition.** Analyzes past actions to improve future performance (self-learning). |
| **Scheduler** | 50066 | **Time Management.** CRON-based task scheduling and execution. |
| **Agent Registry** | 50067 | **Team Management.** Dynamic discovery of specialized agents based on capabilities. |

### B. Cognitive Layer (The Soul)

| Service | Port | Specialization |
| :--- | :--- | :--- |
| **Mind-KB** | 50057 | **Facts & Logic.** Short-term episodic memory and vector-based semantic search. |
| **Body-KB** | 50058 | **Physical State.** Sensor data, system health, and environment context. |
| **Heart-KB** | 50059 | **Emotion.** Tracks sentiment (Neutral, Urgent, Frustrated) and emotional shifts. |
| **Social-KB** | 50060 | **Identity.** Manages user profiles, roles, and communication preferences. |
| **Soul-KB** | 50061 | **Ethics.** Immutable core values and ethical constraint enforcement. |

### C. Functional Layer (The Body)

| Service | Port | Role |
| :--- | :--- | :--- |
| **LLM Service** | 50053 | Interface to LLM providers (OpenAI, Anthropic, Local). Handles generation and embedding. |
| **Tools Service** | 50054 | Safe execution of external tools (Web Search, Calculator, Code Execution). |
| **Safety Service** | 50055 | Input/Output filtering, PII redaction, and threat detection. |
| **Logging Service** | 50056 | Centralized telemetry and structured logging. |
| **Sensor Service** | 50062 | Hardware/System monitoring (CPU, Memory, Network). |
| **Executor Service** | 50063 | Sandboxed command execution runtime. |

### D. Specialized Agents (The Team)

| Service | Port | Role |
| :--- | :--- | :--- |
| **Red Team** | 50068 | **Adversary.** Vulnerability scanning, attack simulation, security auditing. |
| **Blue Team** | 50069 | **Defender.** Threat containment, system hardening, incident response. |

### E. Gateway (The Interface)

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

---

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