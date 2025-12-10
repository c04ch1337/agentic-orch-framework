# Auth Service (auth-service-rs)

A comprehensive authentication and authorization service for Phoenix ORCH AGI, providing centralized auth services for all system components.

## Overview

The Auth Service is a high-performance, secure authentication and authorization service written in Rust. It provides essential security infrastructure including JWT token management, Role-Based Access Control (RBAC), audit logging, and service mesh integration.

## Key Features

### Authentication & JWT Management
- JWT token generation and validation
- Flexible token types (access, refresh, service)
- Automated key rotation and management
- Token revocation and blacklisting
- Secure token metadata storage

### Role-Based Access Control (RBAC)
- Fine-grained permission management
- Hierarchical role support
- Resource-based authorization
- Dynamic permission evaluation
- Attribute-based access control

### Audit Logging
- Comprehensive security event logging
- Structured event categorization
- Tamper-evident log storage
- Compliance-ready logging
- Integration with external logging systems

### Token Management
- Centralized token blacklist
- Automated key rotation
- Token metadata tracking
- Token revocation propagation
- Refresh token handling

### Service Mesh Integration
- Service discovery
- Load balancing
- Circuit breaking
- Secure mTLS connections
- Connection pooling and backoff

## Port Information

The service runs on port 50090 by default and exposes a gRPC interface.

## Dependencies

### Core Dependencies
- Tokio (async runtime)
- Tonic (gRPC framework)
- JWT handling (jsonwebtoken)
- Redis (token storage)
- SQLx (database operations)
- Tracing (logging)

### Security Dependencies
- Ring (cryptography)
- Ed25519-dalek (digital signatures)
- X509-parser (certificate handling)
- RCGen (certificate generation)

## Configuration

The service can be configured through environment variables:

### Required Environment Variables
- `AUTH_SERVICE_PORT`: Service port (default: 50090)
- `JWT_SECRET`: Secret key for JWT signing
- `REDIS_URL`: Redis connection URL
- `SECRETS_SERVICE_ADDR`: Address of the secrets service

### Optional Environment Variables
- `SERVICE_ID`: Service identifier (default: "auth-service")
- `TOKEN_ISSUER`: JWT issuer name (default: "phoenix-orch-agi")
- `LOG_LEVEL`: Logging level (default: "info")

## Development Setup

1. Install dependencies:
```bash
apt-get install pkg-config libssl-dev protobuf-compiler
```

2. Build the service:
```bash
cargo build --release
```

3. Run with Docker:
```bash
docker build -f Dockerfile.dev -t auth-service-rs .
docker run -p 50090:50090 auth-service-rs
```

## Security Features

- Secure JWT token handling with automated key rotation
- Fine-grained RBAC with hierarchical roles
- Comprehensive audit logging
- Secure service-to-service communication with mTLS
- Token revocation and blacklisting
- Circuit breaker pattern for resilience

## API Documentation

The service exposes a gRPC API with the following main endpoints:

### Authentication
- `ValidateToken`: Validate JWT tokens
- `GenerateToken`: Generate new tokens
- `RenewToken`: Refresh expired tokens
- `RevokeToken`: Revoke active tokens

### Authorization
- `CheckPermission`: Check access permissions
- `GetUserPermissions`: List user permissions
- `CreateRole`: Create new roles
- `AssignRole`: Assign roles to users

### Administration
- `CreateUser`: Create new users
- `RegisterService`: Register new services
- `GenerateServiceCertificate`: Generate service certificates
- `GetAuditLogs`: Retrieve audit logs

## Health Checks

The service implements a standard health check endpoint that returns:
- Service status (SERVING/NOT_SERVING)
- Component health status (storage, token manager, secrets)
- Detailed health metrics

## Monitoring

The service exports metrics for:
- Token operations (generated, validated, revoked)
- Authorization decisions
- Circuit breaker status
- Service mesh status
- Audit log events

## Contributing

1. Fork the repository
2. Create a feature branch
3. Submit a pull request

## License

[License information here]