# Reflection Service

## Overview

Reflection Service provides self-reflection and action evaluation capabilities for the Phoenix ORCH AGI system. Enables meta-cognitive analysis of actions, outcomes, and system capabilities.

## Port Information

- Default Port: 50065
- Listen Address: 0.0.0.0:50065 (configurable via `REFLECTION_SERVICE_ADDR` environment variable)
- Protocol: gRPC

## Key Functionalities

- **Action Reflection**: Analyze completed actions and their outcomes
- **Action Evaluation**: Pre-execution evaluation of proposed actions
- **Meta-Cognition**: Self-assessment of system capabilities and limitations
- **Lessons Learned**: Extract insights from success and failure patterns
- **Improvement Suggestions**: Generate actionable improvement recommendations

## Dependencies and Requirements

### Core Dependencies
- Rust 2021 edition
- Tokio 1.48.0 (async runtime)
- Tonic 0.14.2 (gRPC framework)
- Prost 0.14.1 (Protocol Buffers)
- Chrono 0.4 (timestamp handling)

## Configuration

### Environment Variables
- `REFLECTION_SERVICE_ADDR`: Service address (default: "0.0.0.0:50065")
- `RUST_LOG`: Logging level (default: "info")

## gRPC Services

- **ReflectionService**: Core reflection operations
  - `ReflectOnAction`: Analyze completed actions
  - `EvaluateAction`: Pre-execution action evaluation
  - `MetaCognition`: Self-assessment and capability analysis

- **HealthService**: Health checking
  - `GetHealth`: Service health status with reflection count metrics

## Reflection Capabilities

### Action Reflection
- Success/failure analysis
- Outcome interpretation
- Lessons learned extraction
- Improvement suggestions

### Action Evaluation
- Pre-execution risk assessment
- Constraint validation
- Alternative suggestion
- Recommendation generation

### Meta-Cognition
- Self-assessment by topic
- Strength identification
- Weakness analysis
- Growth area recommendations

## Metrics

- Reflection count: Total number of reflections processed
- Uptime tracking: Service uptime in seconds
- Dependency status: Component health monitoring

## Integration

Used by orchestrator-service-rs and other services for:
- Post-action analysis
- Pre-execution validation
- Continuous improvement feedback loops

