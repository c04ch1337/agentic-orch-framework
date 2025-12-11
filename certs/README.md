# Certificates

## Overview

Certificate generation and management for TLS/SSL encryption in Phoenix ORCH AGI system. Provides scripts for generating self-signed certificates for development and testing.

## Files

- **generate_certs.sh**: Bash script to generate self-signed certificates

## Usage

### Generate Certificates
```bash
./certs/generate_certs.sh
```

This generates:
- `api-gateway.key`: Private key (2048-bit RSA)
- `api-gateway.pem`: Self-signed certificate (valid 365 days)

## Certificate Details

### Self-Signed Certificate
- **Subject**: `/C=US/ST=WA/L=Seattle/O=Phoenix AGI/OU=API Gateway/CN=localhost`
- **Validity**: 365 days
- **Key Size**: 2048 bits
- **Format**: X.509 PEM

## TLS Configuration

### Enable TLS in API Gateway
Set environment variables:
```bash
TLS_ENABLED=true
TLS_CERT_PATH=certs/api-gateway.pem
TLS_KEY_PATH=certs/api-gateway.key
```

## Security Notes

### Development Use
- Self-signed certificates are for **development only**
- Browsers will show security warnings
- Not suitable for production deployments

### Production Requirements
- Use certificates from trusted Certificate Authority (CA)
- Implement certificate rotation procedures
- Monitor certificate expiration
- Use proper key management (HSM, key vault)

## Certificate Management

### Key Storage
- Private keys must be kept secure
- Never commit private keys to version control
- Use appropriate file permissions (600)

### Certificate Rotation
- Monitor expiration dates
- Generate new certificates before expiration
- Update service configurations
- Restart services to load new certificates

## File Permissions

Recommended permissions:
```bash
chmod 600 api-gateway.key
chmod 644 api-gateway.pem
```

## Integration

Certificates are used by:
- API Gateway for HTTPS termination
- Service-to-service TLS (if configured)
- Client certificate authentication (if configured)

