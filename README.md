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