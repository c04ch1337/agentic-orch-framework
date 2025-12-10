# Curiosity Engine Service

## Overview

The Curiosity Engine is a core service within the Phoenix ORCH backend system that drives autonomous learning and knowledge exploration. It implements a gRPC-based service that analyzes knowledge gaps and generates research tasks to fill those gaps, enabling continuous system learning and improvement.

## Port Configuration

- **Default Port**: 50076
- **Protocol**: gRPC
- **Default Bind Address**: 0.0.0.0
- **Environment Variable**: `CURIOSITY_ENGINE_PORT`

## Key Features

### Knowledge Gap Analysis
- Processes knowledge gap identification requests
- Evaluates gaps in system knowledge
- Prioritizes learning opportunities based on importance

### Research Task Generation
- Converts knowledge gaps into actionable research tasks
- Implements priority-based task scheduling (default priority: 8/10)
- Generates structured task descriptions for system exploration

### Health Monitoring
- Implements standard gRPC health checking protocol
- Provides real-time service status monitoring
- Supports service mesh integration with health reporting

## Dependencies

### Core Dependencies
- `tonic`: gRPC framework for Rust
- `tokio`: Asynchronous runtime
- `tonic-health`: Health checking implementation
- `prost`: Protocol buffer support
- `serde`: Serialization framework
- `anyhow`: Error handling

### System Requirements
- Rust 1.70 or later
- SSL support (libssl-dev)
- CA certificates for secure communication

## Configuration

### Environment Variables
- `CURIOSITY_ENGINE_PORT`: Override default port (50076)
- `CURIOSITY_ENGINE_BIND_ADDR`: Override default bind address (0.0.0.0)

### Docker Configuration
The service includes a development Dockerfile (`Dockerfile.dev`) with:
- Multi-stage build process
- Minimal runtime image based on debian:bullseye-slim
- Automatic port exposure (50076)
- Required runtime dependencies

## Integration

### Client Connection
```rust
let config = ServiceConfig::new("my-service");
let curiosity_engine_addr = config.get_client_address("curiosity-engine", 50076);
let client = CuriosityEngineClient::connect(curiosity_engine_addr).await?;
```

### Health Checks
The service implements the standard gRPC health checking protocol, allowing for:
- Service mesh integration
- Load balancer health monitoring
- System status reporting

## Development

### Building
```bash
cargo build --package curiosity-engine-rs
```

### Running
```bash
# With default configuration
cargo run --package curiosity-engine-rs

# With custom port
export CURIOSITY_ENGINE_PORT=50077
cargo run --package curiosity-engine-rs
```

### Docker Development
```bash
docker build -f Dockerfile.dev -t curiosity-engine-rs .
docker run -p 50076:50076 curiosity-engine-rs