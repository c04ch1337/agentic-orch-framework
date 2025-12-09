# HashiCorp Vault Deployment Guide for Phoenix ORCH

This guide provides comprehensive instructions for implementing secure secrets management with HashiCorp Vault in the Phoenix ORCH AGI system.

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [Configuration](#configuration)
5. [Integration with Phoenix ORCH](#integration-with-phoenix-orch)
6. [Key Rotation Procedures](#key-rotation-procedures)
7. [Authentication Methods](#authentication-methods)
8. [Disaster Recovery](#disaster-recovery)
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)

## Overview

Phoenix ORCH implements a comprehensive secrets management solution using HashiCorp Vault, providing:

- Centralized storage for all sensitive credentials and API keys
- Short-lived token generation and automatic rotation
- Role-based access control for services
- Audit logging for all secrets access
- High availability and disaster recovery

The `secrets-service-rs` component acts as an interface between Vault and other system components, managing authentication, authorization, and credentials retrieval.

## Prerequisites

- HashiCorp Vault 1.13.x or newer
- Linux server (preferably Ubuntu 22.04 LTS or RHEL 8.x)
- Docker and Docker Compose (for containerized deployment)
- SSL certificates for TLS encryption
- Dedicated storage volumes for Vault data
- Proper network segmentation to isolate Vault

## Installation

### Production Installation

For production environments, it's recommended to install Vault using the official HashiCorp packages:

```bash
# Add HashiCorp GPG key
curl -fsSL https://apt.releases.hashicorp.com/gpg | sudo apt-key add -

# Add HashiCorp repository
sudo apt-add-repository "deb [arch=amd64] https://apt.releases.hashicorp.com $(lsb_release -cs) main"

# Update and install Vault
sudo apt-get update && sudo apt-get install vault
```

### Development/Testing with Docker

For development or testing, you can use the Docker Compose setup:

```yaml
# docker-compose.vault.yml
version: '3.8'
services:
  vault:
    image: hashicorp/vault:1.13
    container_name: vault
    ports:
      - "8200:8200"
    environment:
      - VAULT_DEV_ROOT_TOKEN_ID=phoenix-dev-token
      - VAULT_DEV_LISTEN_ADDRESS=0.0.0.0:8200
    cap_add:
      - IPC_LOCK
    volumes:
      - ./vault/config:/vault/config
      - ./vault/data:/vault/data
      - ./vault/logs:/vault/logs
    command: server -dev
```

Start with: `docker-compose -f docker-compose.vault.yml up -d`

## Configuration

### Server Configuration

Create a production Vault configuration file at `/etc/vault.d/vault.hcl`:

```hcl
storage "file" {
  path = "/vault/data"
}

listener "tcp" {
  address = "0.0.0.0:8200"
  tls_disable = false
  tls_cert_file = "/path/to/fullchain.pem"
  tls_key_file = "/path/to/privkey.pem"
}

api_addr = "https://vault.yourdomain.com:8200"
ui = true

# Enable telemetry for monitoring
telemetry {
  statsite_address = "127.0.0.1:8125"
  disable_hostname = true
}
```

### Initialize Vault (Production)

1. Start the Vault server:
   ```bash
   sudo systemctl start vault
   ```

2. Initialize Vault:
   ```bash
   export VAULT_ADDR='https://vault.yourdomain.com:8200'
   vault operator init -key-shares=5 -key-threshold=3
   ```

3. **IMPORTANT**: Securely store the unseal keys and root token. These are critical for disaster recovery.

4. Unseal Vault (requires at least 3 keys in this example):
   ```bash
   vault operator unseal <key1>
   vault operator unseal <key2>
   vault operator unseal <key3>
   ```

### Setup Authentication Methods

Configure AppRole authentication (recommended for services):

```bash
# Login with root token
vault login

# Enable AppRole auth
vault auth enable approle

# Create policy for LLM service
vault policy write llm-service-policy -<<EOF
path "secret/data/llm-api-key/*" {
  capabilities = ["read", "list"]
}
EOF

# Create policy for API Gateway
vault policy write api-gateway-policy -<<EOF
path "secret/data/api-key/*" {
  capabilities = ["read", "list", "create", "update", "delete"]
}
EOF

# Create role for LLM service
vault write auth/approle/role/llm-service \
    token_policies="llm-service-policy" \
    token_ttl=1h \
    token_max_ttl=24h

# Create role for API Gateway
vault write auth/approle/role/api-gateway \
    token_policies="api-gateway-policy" \
    token_ttl=1h \
    token_max_ttl=24h

# Get role ID and secret ID for LLM service
vault read auth/approle/role/llm-service/role-id
vault write -force auth/approle/role/llm-service/secret-id

# Get role ID and secret ID for API Gateway
vault read auth/approle/role/api-gateway/role-id
vault write -force auth/approle/role/api-gateway/secret-id
```

### Setup Secrets Engine

```bash
# Enable KV version 2 engine
vault secrets enable -version=2 kv

# Rename to 'secret'
vault secrets move kv/ secret/

# Store LLM API keys
vault kv put secret/llm-api-key/openai api_key=sk-your-openai-key
vault kv put secret/llm-api-key/openrouter api_key=your-openrouter-key
vault kv put secret/llm-api-key/anthropic api_key=your-anthropic-key

# Store API Gateway keys
vault kv put secret/api-key/default api_key=$(openssl rand -hex 24)
```

## Integration with Phoenix ORCH

### Environment Configuration

Update your environment files to use the Vault integration:

```bash
# .env.production
VAULT_ADDR=https://vault.yourdomain.com:8200
VAULT_ROLE_ID=your-role-id-from-vault
VAULT_SECRET_ID=your-secret-id-from-vault
VAULT_AUTH_METHOD=approle

# Service authentication secrets (for inter-service communication)
LLM_SERVICE_SECRET=generated-secret-for-llm-service
API_GATEWAY_SECRET=generated-secret-for-api-gateway
```

### Service Setup

Each service should be configured to authenticate with Vault at startup:

1. **LLM Service**: Will retrieve API keys from Vault path `secret/llm-api-key/{provider}`
2. **API Gateway**: Will retrieve and validate API keys from Vault path `secret/api-key/{name}`

### Secret Path Conventions

Follow these conventions for organizing secrets:

- `secret/llm-api-key/{provider}` - API keys for LLM providers (openai, anthropic, etc.)
- `secret/api-key/{name}` - API keys for external access
- `secret/service/{service-name}/credentials` - Service-specific credentials
- `secret/database/{db-name}` - Database credentials

## Key Rotation Procedures

### Automated Rotation

Phoenix ORCH includes automated credential rotation through the secrets service:

1. **API Keys**: Rotate every 30 days
2. **Service Tokens**: Short-lived (1 hour by default) with automatic renewal
3. **Database Credentials**: Rotate every 90 days

### Manual Rotation Procedure

For manual rotation of sensitive credentials:

1. Generate new credential in Vault:
   ```bash
   vault kv put secret/llm-api-key/openai api_key=sk-your-new-openai-key
   ```

2. Services will automatically detect and use the new credential on their next key refresh cycle (typically under 15 minutes).

3. Verify rotation success in logs:
   ```bash
   docker logs llm-service-rs | grep "API key refreshed"
   ```

### Emergency Credential Revocation

In case of a security incident:

1. Immediately revoke the compromised credential in its source system (e.g., OpenAI dashboard)
2. Update the credential in Vault with a new value
3. Force all services to reload configurations:
   ```bash
   docker-compose restart secrets-service-rs
   ```

## Authentication Methods

Phoenix ORCH supports multiple Vault authentication methods:

### AppRole Authentication (Recommended for Services)

Services authenticate using role IDs and secret IDs, which provide fine-grained access control.

### Token Authentication (Development/Testing Only)

Simple token-based authentication, suitable for development but not recommended for production.

### Kubernetes Authentication (For Kubernetes Deployments)

When deployed on Kubernetes, services can authenticate using their service account.

## Disaster Recovery

### Backup Procedures

1. **Automatic Backups**: Configure scheduled backups of Vault data
   ```bash
   # Example hourly snapshot script
   #!/bin/bash
   VAULT_ADDR=https://vault.yourdomain.com:8200
   BACKUP_PATH=/backup/vault
   DATE=$(date +%Y%m%d-%H%M%S)
   
   # Create snapshot
   vault operator raft snapshot save $BACKUP_PATH/vault-$DATE.snap
   
   # Rotate backups (keep last 48 hours)
   find $BACKUP_PATH -name "vault-*.snap" -mtime +2 -delete
   ```

2. **Offsite Backup Storage**: Encrypt and store backups offsite
   ```bash
   # Encrypt with age encryption
   age -e -r age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p \
     vault-$DATE.snap > vault-$DATE.snap.age
   
   # Upload to secure storage
   aws s3 cp vault-$DATE.snap.age s3://secure-backups/vault/
   ```

### Recovery Process

In case of system failure:

1. Install Vault on a new server
2. Copy the backup file to the new server
3. Restore the snapshot:
   ```bash
   vault operator raft snapshot restore vault-backup.snap
   ```
4. Unseal the Vault using the original unseal keys

## Best Practices

### Security Hardening

1. **Network Segmentation**: Place Vault in a dedicated security zone
2. **TLS Everywhere**: Always use TLS for Vault communication
3. **Principle of Least Privilege**: Grant minimal required permissions
4. **Audit Logging**: Enable comprehensive audit logging
   ```bash
   vault audit enable file file_path=/var/log/vault/audit.log
   ```

### Monitoring and Alerts

1. **Health Checks**:
   ```bash
   # Check Vault health
   curl -s https://vault.yourdomain.com:8200/v1/sys/health | jq
   ```

2. **Prometheus Integration**:
   ```bash
   # Add to Vault config
   telemetry {
     prometheus_retention_time = "24h"
     disable_hostname = true
   }
   ```

3. **Key Alerts to Configure**:
   - Vault server unhealthy
   - Unsuccessful authentication attempts spike
   - Seal status change
   - Rate limiting triggered

## Troubleshooting

### Common Issues

1. **Authentication Failures**
   - Check service role ID and secret ID
   - Verify policy permissions
   - Examine audit logs

2. **Service Unable to Retrieve Secrets**
   - Verify network connectivity
   - Check path naming conventions
   - Ensure service has correct permissions

3. **Vault Sealed After Reboot**
   - Implement auto-unseal or manual unseal process
   - Consider using cloud auto-unseal features

### Logs and Diagnostics

Important log locations:
- Vault server logs: `/var/log/vault/`
- Secrets service logs: Docker logs or `/var/log/phoenix-orch/secrets-service.log`
- Audit logs: `/var/log/vault/audit.log`

Enable debug logging for troubleshooting:
```bash
# Update Vault configuration
log_level = "debug"

# Update secrets service environment
RUST_LOG=debug
```

## References

- [HashiCorp Vault Documentation](https://www.vaultproject.io/docs)
- [AppRole Authentication Guide](https://www.vaultproject.io/docs/auth/approle)
- [Vault Operations Guide](https://learn.hashicorp.com/collections/vault/operations)
- [Phoenix ORCH Secrets Service Documentation](#) (internal link)