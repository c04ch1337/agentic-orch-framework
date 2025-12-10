# Agent Registry Service

## Overview

The Agent Registry Service is a core component of the Phoenix Orchestration system, responsible for managing and coordinating specialized agents within the system. It provides a centralized registry for agent discovery, health monitoring, and capability management.

## Port Information

- Default Port: 50067
- Listen Address: 0.0.0.0:50067 (configurable via AGENT_REGISTRY_ADDR environment variable)
- Protocol: gRPC

## Key Functionalities

- **Agent Registration**: Allows agents to register themselves with capabilities and metadata
- **Agent Discovery**: Provides lookup functionality to find agents by name or capability
- **Health Monitoring**: Performs periodic health checks on registered agents
- **Capability Management**: Maintains a catalog of available capabilities across all agents
- **Dynamic Status Tracking**: Monitors agent status (ONLINE/OFFLINE) in real-time

## Dependencies and Requirements

### Core Dependencies
- Rust 2021 edition
- Tokio 1.48.0 (async runtime)
- Tonic 0.14.2 (gRPC framework)
- Prost 0.14.1 (Protocol Buffers)

### Additional Dependencies
- UUID v4 for agent identification
- TOML 0.8 for configuration parsing
- env_logger 0.11 for logging
- tonic-health 0.14.2 for health checking

## Configuration

### Environment Variables
- `AGENT_REGISTRY_ADDR`: Service address (default: "0.0.0.0:50067")
- `AGENT_REGISTRY_CONFIG`: Path to config file (default: "../config/agent_registry.toml")

### Configuration File (agent_registry.toml)
```toml
[[agent]]
name = "AGENT_NAME"
port = PORT_NUMBER
role = "Agent Role Description"
capabilities = [
    "capability_1",
    "capability_2"
]
```

### Configuration Fields
- `name`: Unique identifier for the agent
- `port`: Port number where the agent is accessible
- `role`: Description of agent's primary function
- `capabilities`: List of supported capabilities

## Health Checks

The service implements the gRPC Health Checking Protocol and provides:
- Periodic health verification of registered agents (60-second intervals)
- Uptime tracking
- Dependency status monitoring
- Agent count metrics (total and online)