# API Gateway Service

## Overview

The API Gateway service acts as a REST to gRPC translation layer, providing a secure entry point for external clients to interact with the Phoenix Orchestrator system. It implements comprehensive request validation, authentication, and rate limiting to ensure secure and reliable API access.

### Key Features

- REST to gRPC translation for seamless client integration
- Secure API key management and token-based authentication
- Request validation and sanitization
- Rate limiting with per-API-key quotas
- TLS/mTLS support for secure communication
- Phoenix auth service integration
- Comprehensive input validation framework

## Port Information

The service runs on port 8000 by default and serves as the HTTP/REST entry point for external clients. The port can be configured through the service configuration.

## Authentication

The API Gateway implements a multi-layered authentication system:

1. **Token-based Authentication**
   - Bearer token support via Authorization header
   - Integration with auth service for token validation
   - Role-based access control (RBAC)
   - Permission-based endpoint protection

2. **API Key Authentication**
   - Support for API keys via X-PHOENIX-API-KEY header
   - Fallback support for legacy API key validation
   - Secure key management through secrets service

## Rate Limiting

Implements a sophisticated rate limiting system:

- Per-API-key rate limiting using sliding window algorithm
- Default limit: 100 requests per minute per API key
- Configurable rate limits
- Automatic rate limit headers in responses
- Built using tower-governor for reliable rate limiting

## Validation and Security

### Request Validation
- Content-type validation and enforcement
- JSON schema validation for request payloads
- Request size limits (default: 10MB)
- Input sanitization and security checks

### Security Features
- Request payload sanitization
- Schema-based validation
- Content-type enforcement
- Request size limiting
- Secure header validation

## Phoenix Auth Integration

Integrates with the Phoenix authentication service for:

- Token validation and generation
- Permission checking
- Role-based access control
- API key validation
- Service-to-service authentication

## Dependencies

Main dependencies include:

- `axum`: Web framework for REST API
- `tonic`: gRPC support
- `tower-http`: Middleware components
- `tower_governor`: Rate limiting
- `serde`: JSON serialization/deserialization
- `jsonschema`: Request validation
- `rustls`: TLS support

## Configuration

The service can be configured through environment variables:

```env
# Server Configuration
PORT=8000                           # Default port
TLS_ENABLED=false                   # Enable/disable TLS
TLS_CERT_PATH=certs/api-gateway.pem # TLS certificate path
TLS_KEY_PATH=certs/api-gateway.key  # TLS key path

# Authentication
SERVICE_ID=api-gateway              # Service identifier
CLIENT_ID=api-gateway-client        # Client identifier
CLIENT_SECRET=default-client-secret # Client secret
USE_MTLS=false                     # Enable/disable mTLS

# API Keys
PHOENIX_API_KEYS_FILE=config/phoenix_api_keys.txt # API keys file path
```

The service also supports configuration through the standard service configuration system provided by `config-rs`.