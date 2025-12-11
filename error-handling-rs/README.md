# Error Handling Framework

## Overview

Comprehensive error handling framework for the Phoenix ORCH AGI system with standardized error types, context-preserving error propagation, structured logging, sanitization, and centralized reporting.

## Features

- **Standardized error types**: Service-specific error extensions with consistent structure
- **Context preservation**: Error context maintained through propagation chains
- **Structured logging**: Correlation IDs for request tracing
- **Error sanitization**: Prevents information leakage in error messages
- **Centralized reporting**: Error aggregation and monitoring
- **Retry mechanisms**: Exponential backoff with configurable policies
- **Circuit breaker**: Automatic failure detection and recovery
- **Fallback strategies**: Graceful degradation for critical operations

## Port Information

This is a library crate with no standalone service. It is used by all services in the Phoenix ORCH system.

## Key Modules

- **types**: Core error types and result wrappers
- **context**: Error context management and propagation
- **logging**: Structured logging with correlation IDs
- **sanitization**: Error message sanitization
- **reporting**: Centralized error reporting
- **retry**: Retry policies and mechanisms
- **circuit_breaker**: Circuit breaker implementation
- **fallback**: Fallback strategy execution
- **supervisor**: Error supervision and recovery

## Dependencies and Requirements

### Core Dependencies
- Rust 2021 edition
- Tokio 1.0 (async runtime)
- Tracing 0.1 (structured logging)
- Thiserror 1.0 (error types)
- Anyhow 1.0 (error context)
- Metrics 0.20 (error metrics)
- Reqwest 0.11 (HTTP reporting)

## Configuration

### Environment Variables
- `ERROR_REPORTING_ENABLED`: Enable centralized error reporting
- `ERROR_REPORTING_ENDPOINT`: HTTP endpoint for error reports
- `CORRELATION_ID_HEADER`: HTTP header name for correlation IDs

## Usage

```rust
use error_handling_rs::{Error, Result, init, report_error};

// Initialize framework
init()?;

// Use Result type
fn my_function() -> Result<String> {
    // Operations that may fail
    Ok("success".to_string())
}

// Report errors
report_error(&error, &context).await?;
```

## Error Types

- **ErrorKind**: Categorized error types (Service, Network, Validation, etc.)
- **ServiceError**: Service-specific error extensions
- **ErrorContext**: Additional context for error analysis

## Integration

All Phoenix ORCH services should use this framework for:
- Consistent error handling
- Request correlation tracking
- Error metrics collection
- Centralized error reporting

