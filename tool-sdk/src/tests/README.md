# Tool SDK Test Suite Documentation

This document provides detailed information about the comprehensive test suite for the Tool SDK crate. The tests are designed to ensure that the SDK functions correctly, handles errors appropriately, and maintains expected behavior across various scenarios.

## Testing Structure

The test suite is organized into several categories:

1. **Core Component Tests**: Tests for the fundamental traits and abstractions.
2. **Error Handling Tests**: Tests for the error system, including error propagation and context enrichment.
3. **Resilience Tests**: Tests for retry logic, circuit breaker, and other resilience patterns.
4. **Configuration Tests**: Tests for loading and validating service configurations.
5. **Service-Specific Tests**: Mock tests for the supported external services (OpenAI, SerpAPI).
6. **Integration Tests**: End-to-end tests that verify component interactions.

## Running the Tests

To run the entire test suite:

```bash
cargo test --package tool-sdk
```

To run a specific category of tests:

```bash
cargo test --package tool-sdk --test config_tests
cargo test --package tool-sdk --test core_tests
# etc.
```

To run a specific test:

```bash
cargo test --package tool-sdk test_service_client_interface
```

## Test Categories

### Core Component Tests (`core_tests.rs`, `core_extension_tests.rs`)

Tests for the fundamental traits and interfaces:
- ServiceClient
- RequestExecutor
- AuthenticatedClient
- RateLimited
- Telemetry

These tests verify that the core abstractions work as expected, using mock implementations to simulate real behavior.

### Error Handling Tests (`error_tests.rs`, `error_extension_tests.rs`)

Tests for the error system:
- Error creation and classification
- Error context enrichment
- Error propagation through function chains
- Service-specific error mapping (OpenAI, SerpAPI)

These tests ensure that errors are correctly created, propagated, and enriched with context.

### Resilience Tests (`resilience_tests.rs`, `resilience_extension_tests.rs`)

Tests for resilience patterns:
- Retry logic with exponential backoff
- Circuit breaker state transitions
- Integration of multiple resilience patterns
- Error categorization for retry decisions

These tests verify that the SDK can recover from transient failures and prevent cascading failures.

### Configuration Tests (`config_tests.rs`, `config_extension_tests.rs`)

Tests for configuration management:
- Loading configurations from different sources
- Validating configurations
- Applying configurations to clients
- Handling environment variable-based configuration

### Service Mock Tests (`openai_mock_tests.rs`, `serpapi_mock_tests.rs`)

Mock tests for external services:
- OpenAI API (chat completions, embeddings, models)
- SerpAPI (Google search, Bing search, account info)
- Error scenarios (rate limiting, authentication errors, server errors)

These tests use WireMock to simulate the external APIs and verify client behavior.

### Integration Tests (`integration_tests.rs`)

End-to-end tests that verify component interactions:
- Client lifecycle (creation, configuration, execution)
- Data flow between components
- Error handling across components
- Resilience patterns in real-world scenarios

## Best Practices for Testing

When extending or modifying the test suite, follow these best practices:

1. **Mock External Dependencies**: Use wiremock or custom mocks to avoid real API calls.
2. **Cover Success and Error Cases**: Test both successful paths and error scenarios.
3. **Test Edge Cases**: Include tests for edge cases like empty inputs, rate limiting, and timeouts.
4. **Document Tests**: Each test function should have a clear purpose documented in its name and comments.
5. **Avoid Test Interdependence**: Tests should be independent and not rely on the results of other tests.
6. **Use Helper Functions**: Create utility functions for common test operations.
7. **Follow AAA Pattern**: Arrange, Act, Assert - structure tests clearly with setup, action, and verification.

## Adding New Tests

When adding new functionality to the SDK, follow these steps to ensure adequate test coverage:

1. **Unit Tests**: Add tests for new components, traits, or functions.
2. **Integration Tests**: Update integration tests to include new components.
3. **Mock Tests**: Add mock tests for any new service integrations.
4. **Error Tests**: Ensure error handling is tested for new functionality.
5. **Document Tests**: Update this documentation to include information about new test categories.

## Continuous Integration

The test suite is designed to run as part of CI/CD pipelines. Tests are isolated and do not require external services, making them suitable for automated testing environments.

## Test Coverage

The test suite aims for high code coverage, including:
- All public traits and interfaces
- All error handling code paths
- All resilience pattern logic
- All service client implementations
- All configuration management code