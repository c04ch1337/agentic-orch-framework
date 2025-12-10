# Tool SDK Documentation

Welcome to the Tool SDK documentation. This guide provides comprehensive information about the Tool SDK for the PHOENIX ORCH project.

## Documentation Index

| Document | Description |
|----------|-------------|
| [README.md](../README.md) | Overview, features, installation, and basic usage examples |
| [Architecture](./ARCHITECTURE.md) | Detailed explanation of core abstractions and component relationships |
| [Error Handling](./ERROR_HANDLING.md) | Comprehensive guide to error handling and resilience patterns |
| [Integration Guide](./INTEGRATION_GUIDE.md) | Step-by-step guide for integrating the SDK into services |
| [Extending the SDK](./EXTENDING.md) | Guide for adding new service integrations and enhancing the SDK |
| [Troubleshooting](./TROUBLESHOOTING.md) | Common issues and their solutions |

## Quick Navigation

### For New Users

If you're new to the Tool SDK, start with:

1. [README.md](../README.md) for an overview and basic usage examples
2. [Integration Guide](./INTEGRATION_GUIDE.md) for step-by-step instructions on getting started

### For Developers

If you're developing with the Tool SDK:

1. [Architecture](./ARCHITECTURE.md) to understand the core concepts
2. [Error Handling](./ERROR_HANDLING.md) to learn about handling failures
3. [Troubleshooting](./TROUBLESHOOTING.md) when you encounter issues

### For Contributors

If you're extending or enhancing the Tool SDK:

1. [Extending the SDK](./EXTENDING.md) for detailed instructions on adding new features
2. [Architecture](./ARCHITECTURE.md) to understand the current design patterns
3. [Error Handling](./ERROR_HANDLING.md) to maintain consistent error handling

## Core Concepts

The Tool SDK is built around several key concepts:

### 1. Service Clients

Standardized interfaces for interacting with external services:

```
ServiceClient (Base trait)
├── RequestExecutor (HTTP handling)
├── AuthenticatedClient (Auth capabilities)
├── RateLimited (Rate limiting)
└── Telemetry (Metrics & observability)
```

### 2. Resilience Patterns

Built-in patterns for handling transient failures:

- **Retry**: Automatically retry failed operations with exponential backoff
- **Circuit Breaker**: Prevent cascading failures by failing fast when a service is unhealthy

### 3. Error Handling

Rich error system with contextual information:

- **Error Categories**: Network, Authentication, RateLimit, etc.
- **Context**: Additional information for debugging (status codes, request IDs, etc.)
- **Retryability**: Clear distinction between retryable and permanent errors

### 4. Configuration Management

Flexible configuration through:

- **Environment Variables**: Standardized variables with prefixes
- **Builder Pattern**: Fluent interface for programmatic configuration
- **Configuration Providers**: Abstract interface for loading configuration from different sources

## Examples

Full examples are available in the `examples/` directory:

- `openai_completion.rs`: Shows how to use the OpenAI client for chat completions
- `serpapi_search.rs`: Demonstrates using the SerpAPI client for search
- `resilience_demo.rs`: Demonstrates the resilience patterns in action

Run an example with:

```
PHOENIX_OPENAI_API_KEY=your_api_key cargo run --example openai_completion
```

## API Documentation

For detailed API documentation, run:

```
cargo doc --open
```

## Getting Help

If you encounter issues or need further assistance:

1. Check the [Troubleshooting Guide](./TROUBLESHOOTING.md) for common issues
2. Read the [Error Handling](./ERROR_HANDLING.md) documentation to understand error patterns
3. See the examples in the `examples/` directory for reference implementations