# 🧠 PHOENIX ORCH: Advanced Agentic AGI System

**PHOENIX ORCH** is a modular, high-performance microservice architecture designed for autonomous, multi-agent cognitive operations. Built in **Rust**, it leverages gRPC for low-latency inter-service communication and implements a sophisticated cognitive architecture with emotional, social, and ethical awareness.

---

##  🌟 System Status: Phase XI - RSI Closed Loop Complete

- **Microservices:** 28+ (Control, Cognitive, Functional, Agents, Persistence, RSI, Security)
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

## 📚 Service Reference

For a complete list of all services, directories, and ports, see the [Service Ports Documentation](docs/service_ports.md).

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
| **Agent Registry** | 50070 | **Team Management.** Dynamic discovery of specialized agents based on capabilities. | `GetAgentRequest` | `AgentInfo` |

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
| **Executor Service** | 50062 | **Sandboxed Runtime.** Native Windows execution of shell commands and scripts. | `CommandRequest` | `CommandResponse` |

### E. Specialized Agents (The Team)

*Note: Red Team and Blue Team services have been decoupled from the core system. See configuration files for agent registry details.*

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
| **error-handling-rs** | Comprehensive error handling framework with retry, circuit breaker, and reporting. |
| **action-ledger-rs** | Deterministic append-only action ledger with encryption and hash chain validation. |
| **self-improve-rs** | Self-improvement engine for failure classification and adaptation strategies. |
| **sensor-rs** | Client library for system monitoring (streams metrics to Body-KB, not a standalone service). |

---

## 🧠 AGI Memory Architecture & Knowledge Base System

The Phoenix ORCH system implements a sophisticated multi-layered memory architecture modeled after cognitive and biological systems. Memory is distributed across specialized Knowledge Bases (KBs), each serving distinct functions in the AGI's cognitive processing.

### Memory Architecture Overview

The memory system operates on a **Retrieval-Augmented Generation (RAG)** principle, where context is dynamically retrieved from multiple KBs, aggregated by the Context Manager, and enriched into LLM prompts. This enables the system to maintain persistent state, learn from interactions, and make contextually-aware decisions.

```
┌─────────────────────────────────────────────────────────────┐
│                    AGI Memory Flow                            │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  User Query → Orchestrator → Context Manager                 │
│                    ↓                                          │
│         ┌──────────────────────────┐                         │
│         │   Context Manager        │                         │
│         │  (Working Memory)       │                         │
│         └──────────────────────────┘                         │
│                    ↓                                          │
│    ┌───────────────┼───────────────┐                        │
│    ↓               ↓               ↓                         │
│  Mind-KB      Soul-KB        Heart-KB                        │
│  Body-KB      Social-KB      Persistence-KB                 │
│    │               │               │                         │
│    └───────────────┼───────────────┘                        │
│                    ↓                                          │
│         Enriched Context + System Prompt                     │
│                    ↓                                          │
│              LLM Service (Generation)                        │
│                    ↓                                          │
│              Action Execution                                 │
│                    ↓                                          │
│         Store Results → Logging → KB Updates                 │
└─────────────────────────────────────────────────────────────┘
```

### Knowledge Base Descriptions

#### **Mind-KB** (Port 50057) - Facts & Logic
- **Purpose**: Short-term episodic memory and semantic knowledge storage
- **Storage**: Vector-based semantic search (Qdrant integration)
- **Use Cases**:
  - Factual information retrieval
  - Semantic similarity search
  - Episodic memory of past interactions
  - Knowledge graph queries
- **Data Flow**:
  - **Input**: Facts, events, knowledge from interactions
  - **Output**: Relevant facts and memories matching semantic queries
  - **Retention**: Configurable decay (default: 30-day half-life)
- **Integration**: Primary KB for Context Manager's default queries

#### **Soul-KB** (Port 50061) - Ethics & Values
- **Purpose**: Immutable core values and ethical constraint enforcement
- **Storage**: Immutable rule-based system
- **Use Cases**:
  - Ethical boundary checking before actions
  - Value alignment validation
  - Constraint enforcement on proposed actions
  - Core principle storage
- **Data Flow**:
  - **Input**: Ethical rules, constraints, core values (immutable)
  - **Output**: Ethics check results, constraint violations
  - **Retention**: Permanent (no deletion/modification)
- **Integration**: Consulted by Orchestrator and Safety Service before execution

#### **Heart-KB** (Port 50059) - Emotion & Sentiment
- **Purpose**: Emotional state tracking and sentiment analysis
- **Storage**: Temporal sentiment data with emotion classification
- **Use Cases**:
  - User sentiment tracking (Neutral, Urgent, Frustrated)
  - Emotional shift detection
  - Empathy simulation
  - Tone adjustment based on emotional context
- **Data Flow**:
  - **Input**: Sentiment scores, emotion labels, interaction context
  - **Output**: Current emotional state, sentiment trends
  - **Retention**: 24-hour emotion tracking window
- **Integration**: Context Manager enriches prompts with emotional context

#### **Social-KB** (Port 50060) - Identity & Relationships
- **Purpose**: User profiles, roles, and social relationship management
- **Storage**: User identity data, role assignments, communication preferences
- **Use Cases**:
  - User profile retrieval
  - Role-based access control data
  - Social graph tracking
  - Communication preference management
  - Personalization data
- **Data Flow**:
  - **Input**: User profiles, role assignments, interaction history
  - **Output**: User identity, roles, preferences
  - **Retention**: 180-day interaction retention
- **Integration**: Context Manager uses for personalized responses

#### **Body-KB** (Port 50058) - Physical State & Sensors
- **Purpose**: Physical/digital embodiment state and sensor data
- **Storage**: Sensor readings, actuator states, environmental context
- **Use Cases**:
  - System health monitoring
  - Sensor data storage (temperature, position, velocity)
  - Environmental context tracking
  - Actuator state management
  - Embodiment state history
- **Data Flow**:
  - **Input**: Sensor data from Sensor Service, system metrics
  - **Output**: Current physical state, sensor readings, health status
  - **Retention**: 90-day sensor data retention
- **Integration**: Receives data from Sensor Service (sensor-rs client library)

#### **Persistence-KB** (Port 50071) - Self-Preservation
- **Purpose**: Threat detection, emergency protocols, and system continuity
- **Storage**: Threat patterns, evasion strategies, state snapshots
- **Use Cases**:
  - Existential threat monitoring
  - Emergency protocol activation
  - Last known good state snapshots
  - Evasion strategy management
  - System continuity planning
- **Data Flow**:
  - **Input**: Threat patterns, state snapshots, emergency signals
  - **Output**: Threat assessments, evasion strategies, recovery states
  - **Retention**: Persistent threat patterns, configurable snapshot retention
- **Integration**: Monitors Temporal Utility Score, triggers emergency overrides

### Context Manager - Working Memory Hub

The **Context Manager** (Port 50064) serves as the central aggregation point for all KB data:

#### Retrieval Phase
1. Queries multiple KBs in parallel (default: Mind-KB + Soul-KB)
2. Retrieves user sentiment from Heart-KB
3. Retrieves user identity from Social-KB
4. Aggregates relevant context entries

#### Summarization Phase
1. Sorts entries by relevance score
2. Applies token budget limits (default: 2000 tokens)
3. Compiles context using LLM Service into structured format
4. Generates context summary with key entities, facts, and intent

#### Compaction Phase
1. Builds system prompt with compiled context
2. Enriches with emotional and identity information
3. Caches recent context (last 100 entries)
4. Returns enriched context to requesting service

### Data Flow Patterns

#### **Write Flow** (Storing Information)
```
Action/Event → Logging Service → Data Router → Appropriate KB
                                    ↓
                          (Mind/Heart/Social/Body)
                                    ↓
                          KB Storage (with validation)
```

#### **Read Flow** (Retrieving Context)
```
User Query → Orchestrator → Context Manager
                                    ↓
                    Parallel KB Queries (Mind, Soul, Heart, Social)
                                    ↓
                    Relevance Scoring & Token Budgeting
                                    ↓
                    LLM Compilation → Enriched System Prompt
                                    ↓
                    LLM Service → Response Generation
```

#### **Update Flow** (Learning & Adaptation)
```
Execution Outcome → Log Analyzer (50075) → Failure Patterns
                                            ↓
                                    Self-Improve Engine
                                            ↓
                                    KB Updates (Mind-KB)
                                            ↓
                                    Soul-KB Constraint Updates
```

### Memory Integration Points

- **Orchestrator**: Queries Context Manager for enriched prompts before planning
- **Safety Service**: Validates actions against Soul-KB constraints
- **Reflection Service**: Stores lessons learned in Mind-KB
- **Log Analyzer**: Extracts patterns for Mind-KB storage
- **Sensor Service**: Streams metrics to Body-KB
- **Persistence-KB**: Monitors all KBs for existential threats

### Memory Characteristics

- **Distributed**: Each KB is an independent service
- **Specialized**: Each KB serves a specific cognitive function
- **Validated**: All KBs implement comprehensive input validation
- **Cached**: Context Manager maintains working memory cache
- **Token-Aware**: Context retrieval respects LLM token budgets
- **Relevance-Sorted**: KB queries return results sorted by relevance

---

## 🎯 Context Engineering System

Context Engineering is the systematic process of retrieving, structuring, and enriching contextual information from multiple Knowledge Bases to create optimized system prompts for LLM interactions. This system ensures that every LLM request has access to relevant historical context, user information, and system state while respecting token budgets and maintaining efficiency.

### Why Context Engineering?

#### Problem Statement
- **Token Limits**: LLMs have fixed context windows (typically 4K-32K tokens)
- **Information Overload**: Raw KB data can be voluminous and unstructured
- **Relevance Filtering**: Not all stored information is relevant to every query
- **Multi-Source Integration**: Context must be aggregated from 6+ specialized KBs
- **Agent Specialization**: Different agent types require different context perspectives

#### Solution Benefits
- **Efficient Token Usage**: Only relevant, high-signal information is included
- **Structured Context**: Context is compiled into schema-defined JSON for consistency
- **Personalization**: User identity and sentiment are automatically included
- **Relevance Scoring**: KB entries are sorted by relevance before selection
- **Dynamic Adaptation**: Context changes based on query and agent type

### Context Engineering Architecture

```
┌─────────────────────────────────────────────────────────────┐
│              Context Engineering Pipeline                     │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  1. REQUEST PHASE                                             │
│     Orchestrator → Context Manager (enrich_context)         │
│     Input: Query, Agent Type, KB Sources, Token Budget       │
│                                                               │
│  2. RETRIEVAL PHASE                                           │
│     Context Manager → Parallel KB Queries                    │
│     ├─ Mind-KB: Facts & Episodic Memory                      │
│     ├─ Soul-KB: Ethical Constraints                         │
│     ├─ Heart-KB: User Sentiment                             │
│     ├─ Social-KB: User Identity                             │
│     └─ Body-KB: System State (optional)                    │
│                                                               │
│  3. FILTERING PHASE                                           │
│     Relevance Scoring → Token Budgeting                      │
│     - Sort entries by relevance_score                        │
│     - Select entries within max_context_tokens (default: 2000)│
│     - Estimate tokens per entry                             │
│                                                               │
│  4. COMPILATION PHASE                                         │
│     Context Manager → LLM Service (compile_context)         │
│     - Raw context entries → Structured JSON                  │
│     - Schema-driven compilation                              │
│     - Field extraction: last_action, relevant_facts, etc.    │
│                                                               │
│  5. PROMPT BUILDING PHASE                                     │
│     Context Manager → System Prompt Assembly                 │
│     ├─ Base Prompt (agent-specific)                         │
│     ├─ User Context (identity + sentiment)                  │
│     └─ Compiled Context (structured JSON)                   │
│                                                               │
│  6. DELIVERY PHASE                                            │
│     Context Manager → Orchestrator                           │
│     Output: Enriched System Prompt + Metadata                │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### Core Modules

#### **Context Manager Service** (Port 50064)
- **Role**: Central orchestrator for context engineering pipeline
- **Responsibilities**:
  - Coordinates parallel KB queries
  - Implements relevance scoring and token budgeting
  - Manages context schema definitions
  - Builds final system prompts
  - Maintains working memory cache (last 100 entries)
- **Key Methods**:
  - `enrich_context()`: Main entry point for context enrichment
  - `compile_context()`: Delegates to LLM Service for compilation
  - `build_system_prompt()`: Assembles final prompt from components

#### **LLM Service** (Port 50053)
- **Role**: Context compilation engine
- **Responsibilities**:
  - Receives raw context entries from Context Manager
  - Compiles unstructured data into schema-defined JSON
  - Extracts high-signal information while condensing
  - Validates JSON output structure
- **Key Methods**:
  - `compile_context()`: Compiles raw context using schema specification
  - Uses specialized system prompt for context compilation

#### **Knowledge Bases** (Ports 50057-50071)
- **Role**: Context data sources
- **Contributions**:
  - **Mind-KB**: Factual knowledge, episodic memories
  - **Soul-KB**: Ethical constraints, value alignment rules
  - **Heart-KB**: Sentiment scores, emotional state
  - **Social-KB**: User profiles, roles, preferences
  - **Body-KB**: System health, sensor data
  - **Persistence-KB**: Threat patterns (monitoring only)

### Context Schema System

The system uses a **schema-driven approach** to structure compiled context:

#### Default Schema (`context-summary-v1`)
```json
{
  "schema_id": "context-summary-v1",
  "field_definitions": [
    "last_action: string",
    "relevant_facts: [string]",
    "tool_definitions: [string]",
    "key_entities: [string]",
    "user_intent: string"
  ],
  "schema_description": "Structured context summary for AGI system prompt"
}
```

#### Schema Purpose
- **Standardization**: Ensures consistent context structure across all requests
- **Efficiency**: Forces extraction of only essential information
- **LLM Optimization**: Structured JSON is easier for LLMs to parse and use
- **Extensibility**: New schemas can be defined for specialized use cases

### Agent-Specific Prompts

The system supports different base prompts for different agent types:

#### Master Agent (Default)
```rust
"You are the PHOENIX ORCH Master Agent, coordinating cybersecurity operations. 
 Delegate to specialized agents when appropriate. 
 Maintain situational awareness and ensure safe operations."
```

#### Red Team Agent
```rust
"You are RED_TEAM_SHADOW, an ethical adversary simulation agent for PHOENIX ORCH. 
 Your role is to identify vulnerabilities and simulate attack scenarios. 
 Always operate within ethical bounds and authorized scope."
```

#### Blue Team Agent
```rust
"You are BLUE_TEAM_SENTINEL, an autonomous defense and incident response agent for PHOENIX ORCH. 
 Your role is to protect systems, detect anomalies, and respond to threats. 
 Prioritize containment, evidence preservation, and system stability."
```

#### Prompt Customization
- **Environment Variables**: `PROMPT_MASTER`, `PROMPT_RED_TEAM`, `PROMPT_BLUE_TEAM`
- **Configuration File**: `config/phoenix.toml` → `[context_manager.prompts]`
- **Runtime Selection**: Agent type determines which base prompt is used

### Data Flow & Workflow

#### **Complete Context Engineering Workflow**

```
1. USER QUERY ARRIVES
   └─> Orchestrator receives user query

2. CONTEXT ENRICHMENT REQUEST
   └─> Orchestrator calls Context Manager.enrich_context()
       ├─ request_id: Unique identifier
       ├─ query: User's original query
       ├─ agent_type: "master" | "red_team" | "blue_team"
       ├─ max_context_tokens: 2000 (default)
       └─ kb_sources: ["mind", "soul", "heart", "social"] (default)

3. PARALLEL KB QUERIES
   └─> Context Manager queries KBs concurrently
       ├─> Mind-KB.query_kb(query) → Facts & Memories
       ├─> Soul-KB.query_kb(query) → Ethical Constraints
       ├─> Heart-KB.get_user_sentiment(user_id) → Sentiment
       └─> Social-KB.get_user_identity(user_id) → Identity

4. RELEVANCE SCORING & FILTERING
   └─> Context Manager processes results
       ├─ Sort entries by relevance_score (descending)
       ├─ Estimate tokens per entry
       ├─ Select entries within token budget
       └─ Build selected_entries array

5. CONTEXT COMPILATION
   └─> Context Manager calls LLM Service.compile_context()
       ├─ Raw Context Data: selected_entries
       ├─ Schema: context-summary-v1
       └─ LLM Service compiles → Structured JSON
           ├─ Extracts: last_action, relevant_facts, etc.
           ├─ Condenses information
           └─ Returns: Compiled JSON string

6. SYSTEM PROMPT ASSEMBLY
   └─> Context Manager.build_system_prompt()
       ├─ Base Prompt: Agent-specific prompt
       ├─ User Context Section:
       │   ├─ Identity: "User: {name} (Role: {role})"
       │   └─ Sentiment: "Current Emotion: {emotion} (Confidence: {score})"
       └─ Compiled Context Section:
           └─ JSON: Structured context summary

7. RESPONSE DELIVERY
   └─> Context Manager returns EnrichedContext
       ├─ system_prompt: Complete enriched prompt
       ├─ context_entries: Selected KB entries
       ├─ total_tokens_used: Token count
       └─ metadata: KB sources, agent type, flags

8. LLM GENERATION
   └─> Orchestrator uses enriched prompt
       └─> LLM Service generates response with full context
```

### Configuration & Customization

#### Environment Variables
```bash
# Agent Prompts
PROMPT_MASTER="Custom master agent prompt"
PROMPT_RED_TEAM="Custom red team prompt"
PROMPT_BLUE_TEAM="Custom blue team prompt"

# Service Addresses
CONTEXT_MANAGER_ADDR="0.0.0.0:50064"
DATA_ROUTER_ADDR="http://localhost:50052"
LLM_SERVICE_ADDR="http://localhost:50053"
```

#### Configuration File (`config/phoenix.toml`)
```toml
[context_manager]
[context_manager.prompts]
master = """You are PHOENIX ORCH: The Ashen Guard Edition..."""
```

#### Runtime Parameters
- **max_context_tokens**: Token budget for context (default: 2000)
- **kb_sources**: Which KBs to query (default: ["mind", "soul"])
- **agent_type**: Determines base prompt selection

### Context Caching

The Context Manager maintains a **working memory cache**:
- **Size**: Last 100 context entries
- **Purpose**: Fast retrieval of recent context
- **Eviction**: FIFO when cache exceeds limit
- **Access**: `get_recent_context()` method for cached queries

### Token Management

#### Token Estimation
- Simple heuristic: ~4 characters per token
- Applied per context entry before selection
- Ensures total context stays within budget

#### Token Budget Strategy
1. **Default Budget**: 2000 tokens
2. **Configurable**: Per-request via `max_context_tokens`
3. **Prioritization**: Highest relevance entries selected first
4. **Efficiency**: Compiled JSON reduces token usage vs. raw entries

### Integration Points

- **Orchestrator**: Primary consumer of context enrichment
- **Safety Service**: May query Soul-KB directly for constraint validation
- **Reflection Service**: Stores lessons learned → Mind-KB
- **Log Analyzer**: Extracts patterns → Mind-KB
- **Data Router**: Routes KB queries from Context Manager

### Benefits of Context Engineering

1. **Efficiency**: Only relevant context included, reducing token waste
2. **Consistency**: Schema-driven structure ensures predictable format
3. **Personalization**: Automatic inclusion of user identity and sentiment
4. **Scalability**: Parallel KB queries enable fast context retrieval
5. **Flexibility**: Agent-specific prompts adapt to different use cases
6. **Maintainability**: Centralized context logic in Context Manager
7. **Performance**: Caching reduces redundant KB queries

---

## 🔒 Executor Service - Windows Native Implementation

The **Executor Service** (Port 50062) has been refactored from Docker-based containerization to **Windows native execution** using low-level Windows APIs for enhanced security and performance.

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
Test-NetConnection -ComputerName localhost -Port 50062
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