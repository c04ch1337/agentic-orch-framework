#!/bin/bash
# Script to generate self-signed certificates for API Gateway TLS

# Create certs directory if it doesn't exist
mkdir -p certs

# Generate private key
openssl genrsa -out api-gateway.key 2048

# Generate certificate signing request
openssl req -new -key api-gateway.key -out api-gateway.csr -subj "/C=US/ST=WA/L=Seattle/O=Phoenix AGI/OU=API Gateway/CN=localhost"

# Generate self-signed certificate (valid for 365 days)
openssl x509 -req -days 365 -in api-gateway.csr -signkey api-gateway.key -out api-gateway.pem

# Clean up CSR
rm api-gateway.csr

echo "Certificates generated successfully:"
echo "  - Private Key: api-gateway.key"
echo "  - Certificate: api-gateway.pem"
echo ""
echo "To enable TLS, set the following environment variables:"
echo "  TLS_ENABLED=true"
echo "  TLS_CERT_PATH=certs/api-gateway.pem"
echo "  TLS_KEY_PATH=certs/api-gateway.key"