# Body Knowledge Base Service (body-kb-rs)

A gRPC service that manages physical/digital embodiment state information, including sensor readings and actuator states. This service is part of the AGI Core system and handles the storage, retrieval, and querying of body-related state data.

## Service Overview

The Body KB Service provides a centralized knowledge base for:
- Current sensor readings (temperature, position, velocity, etc.)
- Actuator states and capabilities
- Environmental context (location, orientation)
- Embodiment state history

## Port Information

- Default Port: 50058
- Environment Variable: `BODY_KB_ADDR`
- Format: Supports both "0.0.0.0:50058" and "http://127.0.0.1:50058" formats

## Key Functionalities

### Document Store Operations

The service provides the following core operations:
- `store_fact`/`store`: Store new state data with metadata
- `retrieve`: Retrieve state data by key with optional filters
- `query`/`query_kb`: Query current embodiment state

### Vector Search Capabilities

*Note: Vector search functionality is planned for future implementation.*

### Validation Mechanisms

The service implements comprehensive input validation:

#### Query Validation
- Maximum query length: 2KB (2048 bytes)
- Query limit range: 1-50 results
- Protection against code and SQL injection

#### Key Validation
- Maximum key length: 256 bytes
- Alphanumeric with underscores and dots
- Protection against path traversal and code injection

#### Value Validation
- Maximum value size: 256KB (262,144 bytes)
- UTF-8 validation for text values
- Protection against:
  - Code injection
  - Command injection
  - Script tag injection

#### Filter Validation
- Maximum filter count: 10
- Maximum filter key length: 64 bytes
- Maximum filter value length: 256 bytes
- Alphanumeric keys with underscores

## Dependencies and Requirements

### Core Dependencies
- Tokio: Async runtime
- Tonic: gRPC framework
- Prost: Protocol buffers
- Serde: Serialization
- Input Validation RS: Custom validation library

### System Requirements
- Rust 2024 edition
- gRPC support
- Environment variable configuration (optional)

## Configuration Details

### Environment Variables
- `BODY_KB_ADDR`: Service address and port (default: "0.0.0.0:50058")

### Health Checking
- Implements standard gRPC health checking protocol
- Reports service status (SERVING/NOT_SERVING)
- Provides uptime and dependency status monitoring

### Logging
- Uses env_logger
- Default log level: INFO
- Configurable through environment variables

## Service Status

The service provides health status information through:
- Standard gRPC health checks
- Custom health endpoint with:
  - Service uptime
  - Dependency status
  - Service name and status