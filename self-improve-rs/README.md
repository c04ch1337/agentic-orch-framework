# Self-Improvement Engine

## Overview

Self-Improvement Engine for Phoenix ORCH AGI. Processes critical failures, classifies error patterns, and generates adaptation strategies to improve system performance over time.

## Port / Deployment Model

This is a **library crate only** with no standalone server, HTTP API, or port.
It is designed to be embedded into internal services such as
`orchestrator-service-rs` and `reflection-service-rs`.

## Features

- **Failure Classification**: Heuristic-based error pattern recognition
- **Error Record Persistence**: File-backed error record storage
- **Adaptation Engine**: Conservative adaptation with reviewable artifacts
- **Live Apply Control**: Optional live prompt/config mutation (disabled by default)
- **Correlation Tracking**: Request ID tracking for failure analysis

## Dependencies and Requirements

### Core Dependencies
- Rust 2024 edition
- Tokio 1.48.0 (async runtime)
- Tracing 0.1.43 (structured logging)
- Serde 1.0 (serialization)
- UUID 1.11 (request identification)
- Config-rs (path dependency)

## Configuration

### Environment Variables
- `SELF_IMPROVE_ENABLED`: Enable self-improvement engine (truthy: "1", "true", "yes", "on")
- `SELF_IMPROVE_LIVE_APPLY`: Enable live adaptation (default: false, must be explicitly enabled)

### Configuration Flags
- **enabled**: Enable ingestion and record creation
- **live_apply_enabled**: Allow live adaptation (prompt/config mutation)

## Usage

```rust
use self_improve::{SelfImprover, SelfImproveConfig, CriticalFailure};

// Build configuration from environment (recommended for services)
let config = SelfImproveConfig::from_env();
let engine = SelfImprover::new(config)?;

// Process a critical failure originating from the Reflection Service
let failure = CriticalFailure::from_reflection_failure(
    request_id,
    action_description,
    outcome,
    success,
    metadata,
);

engine.process_failure(failure).await?;
```

## Architecture

- **ErrorRecord**: Persistent error record with classification
- **FailureClassifier**: Heuristic-based failure pattern recognition
- **ErrorRecordRepository**: File-backed storage for error records
- **AdaptationEngine**: Generates adaptation strategies (logging-only by default)

## Failure Classification

Failure types include:
- Reflection action failures
- Orchestrator failures
- Service-specific error patterns

## Adaptation Strategy

- **Conservative by default**: Only logs proposed changes
- **Reviewable artifacts**: All adaptations written to reviewable files
- **No automatic mutation**: Requires explicit `live_apply_enabled` flag
- **Audit trail**: Full trace of all adaptation attempts

## Integration

Designed for integration with:
- **orchestrator-service-rs**: Process orchestration failures
- **reflection-service-rs**: Process reflection action failures

## Security

- No automatic code/prompt modification without explicit opt-in
- All adaptations logged and reviewable
- Conservative failure handling to prevent cascading failures

