# Context Manager Service

## Overview
The Context Manager Service is a critical component of the PHOENIX ORCH system responsible for context aggregation and prompt enrichment. It manages and enriches context for various agent types by integrating information from multiple knowledge bases (KBs) and maintaining contextual state.

## Port Information
- Default Port: 50064
- Default Address: 0.0.0.0:50064
- Configuration: Can be overridden using `CONTEXT_MANAGER_ADDR` environment variable

## Key Functionalities

### Context Management
- Retrieves and aggregates context from multiple knowledge bases (KBs)
- Supports dynamic KB source selection with defaults to "mind" and "soul" KBs
- Maintains a cache of recent context (up to 100 entries)
- Implements token-based context limiting for efficient processing

### State Tracking
- Integrates with Heart-KB for user sentiment tracking
- Connects with Social-KB for user identity information
- Maintains context relevance scoring
- Tracks uptime and service health metrics

### Integration Points
- **Data Router Service** (default: http://localhost:50052)
  - Routes requests to appropriate knowledge bases
  - Handles KB query operations
  
- **LLM Service** (default: http://localhost:50053)
  - Compiles and structures context data
  - Processes context using defined schemas

## Dependencies
```toml
tokio = "1.48.0"
tonic = "0.14.2"
prost = "0.14.1"
log = "0.4.29"
env_logger = "0.11"
chrono = "0.4"
uuid = "1.11"
dotenv = "0.15"
once_cell = "1.20"
```

## Configuration

### Environment Variables
- `CONTEXT_MANAGER_ADDR`: Service address (default: "0.0.0.0:50064")
- `DATA_ROUTER_ADDR`: Data Router Service address (default: "http://localhost:50052")
- `LLM_SERVICE_ADDR`: LLM Service address (default: "http://localhost:50053")
- `PROMPT_RED_TEAM`: Custom prompt for red team agents
- `PROMPT_BLUE_TEAM`: Custom prompt for blue team agents
- `PROMPT_MASTER`: Custom prompt for master agents

### Context Schema
The service uses a default context schema for structuring compiled context:
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