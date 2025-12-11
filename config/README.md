# Configuration Files

## Overview

Configuration files for Phoenix ORCH AGI system services. Includes service-specific configurations, API keys, and system-wide settings.

## Files

- **phoenix.toml**: Main system configuration
- **agent_registry.toml**: Agent registry service configuration
- **phoenix_api_keys.txt**: Production API keys (secure)
- **test_api_keys.txt**: Test/development API keys

## Configuration Structure

### phoenix.toml
Main system configuration including:
- Service addresses and ports
- Database connections
- External service endpoints
- Feature flags
- Logging configuration

### agent_registry.toml
Agent registry configuration:
- Agent definitions
- Capability mappings
- Health check intervals
- Registration policies

## API Keys

### Production Keys (phoenix_api_keys.txt)
- Format: `SERVICE_NAME=API_KEY`
- One key per line
- Must be kept secure
- Never committed to version control

### Test Keys (test_api_keys.txt)
- Format: `SERVICE_NAME=TEST_KEY`
- Used for development and testing
- May be committed for team access

## Security

### Key Management
- Production keys stored separately from code
- Test keys clearly marked
- Key rotation procedures documented
- Access control enforced

### Configuration Security
- Sensitive values should use environment variables
- Configuration files excluded from public repos
- Encryption at rest for production configs

## Usage

### Loading Configuration
Services load configuration from:
1. Environment variables (highest priority)
2. Configuration files in `config/` directory
3. Default values (lowest priority)

### Environment Override
All configuration values can be overridden via environment variables:
- Format: `SERVICE_NAME_CONFIG_KEY=value`
- Example: `ORCHESTRATOR_PORT=50051`

## Service-Specific Configs

Each service may have:
- Default configuration in service directory
- Override configuration in `config/` directory
- Runtime configuration via environment variables

## Validation

Configuration validation:
- Type checking
- Required field validation
- Range validation for numeric values
- Format validation for strings
- Dependency validation

