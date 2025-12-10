# API Gateway Security Testing Guide

## Overview

This guide provides comprehensive documentation for the Phoenix AGI System API Gateway security testing suite. The test suite validates authentication, rate limiting, schema validation, TLS configuration, and security headers to ensure the API Gateway meets security requirements.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start](#quick-start)
3. [Test Scenarios](#test-scenarios)
4. [Expected Results](#expected-results)
5. [Running Tests](#running-tests)
6. [Configuration](#configuration)
7. [Troubleshooting](#troubleshooting)
8. [Test Reports](#test-reports)
9. [Security Checklist](#security-checklist)

---

## Prerequisites

### Required Software

- **Python 3.8+** - Required for running the test suite
- **Python requests module** - Install with `pip install requests`
- **Rust/Cargo** - Required if building services from source
- **PowerShell 5.0+** (Windows) - For running the integration test script

### Required Files

| File | Description | Location |
|------|-------------|----------|
| `api-gateway-security-test.py` | Main security test suite | Root directory |
| `config/test_api_keys.txt` | Test API keys configuration | `config/` directory |
| `integration-test.ps1` | Windows integration script | Root directory |
| `.env.dev` | Environment configuration (optional) | Root directory |

### Service Requirements

The following services must be available for testing:

1. **API Gateway** - Port 8000 (HTTP) or 8443 (HTTPS with TLS)
2. **Orchestrator Service** - Port 50051 (gRPC)

---

## Quick Start

### Windows Quick Start

```powershell
# Run complete integration test (build, start services, test, cleanup)
.\integration-test.ps1

# Skip building (use existing binaries)
.\integration-test.ps1 -SkipBuild

# Keep services running after tests
.\integration-test.ps1 -KeepRunning

# Run tests only (services already running)
.\integration-test.ps1 -TestOnly
```

### Manual Testing

```bash
# 1. Start the Orchestrator Service
cd orchestrator-service-rs
cargo run --release

# 2. Start the API Gateway (in new terminal)
cd api-gateway-rs
PHOENIX_API_KEYS_FILE=../config/test_api_keys.txt cargo run --release

# 3. Run security tests (in new terminal)
python api-gateway-security-test.py
```

---

## Test Scenarios

### 1. API Key Authentication Tests

#### Test 1.1: Valid API Key
- **Description**: Verify that valid API keys are accepted
- **Method**: Send request with valid `X-PHOENIX-API-KEY` header
- **Expected**: 200 OK response

#### Test 1.2: Invalid API Key
- **Description**: Verify that invalid API keys are rejected
- **Method**: Send request with invalid key
- **Expected**: 401 Unauthorized with error message

#### Test 1.3: Missing API Key
- **Description**: Verify that requests without API key are rejected
- **Method**: Send request without `X-PHOENIX-API-KEY` header
- **Expected**: 401 Unauthorized with "Missing X-PHOENIX-API-KEY header" error

### 2. Rate Limiting Tests

#### Test 2.1: Within Rate Limit
- **Description**: Verify first 100 requests succeed within rate limit window
- **Method**: Send 100 requests with same API key within 1 minute
- **Expected**: All 100 requests return 200 OK

#### Test 2.2: Exceeding Rate Limit
- **Description**: Verify 101st request is rate limited
- **Method**: Send 101st request with same API key
- **Expected**: 429 Too Many Requests with rate limit error

#### Test 2.3: Per-Key Isolation
- **Description**: Verify each API key has separate rate limit
- **Method**: Send requests with different API key after first key is limited
- **Expected**: New key requests succeed (separate rate limit)

### 3. Response Schema Validation

#### Test 3.1: AgiResponse Structure
- **Description**: Verify response contains all required AgiResponse fields
- **Method**: Send valid execute request
- **Expected**: Response contains:
  - ExecuteResponse fields: `id`, `status_code`, `payload`, `error`, `metadata`
  - AgiResponse fields (when applicable): `final_answer`, `execution_plan`, `routed_service`, `phoenix_session_id`, `output_artifact_urls`

### 4. TLS/HTTPS Connection Tests

#### Test 4.1: HTTPS Connection
- **Description**: Verify secure HTTPS connection when TLS is enabled
- **Method**: Connect via HTTPS protocol
- **Expected**: Successful HTTPS connection

#### Test 4.2: Certificate Validation
- **Description**: Verify TLS certificate is valid
- **Method**: Retrieve and validate certificate
- **Expected**: Valid certificate (self-signed accepted for testing)

### 5. Security Headers Tests

#### Test 5.1: CORS Configuration
- **Description**: Verify CORS headers are properly configured
- **Method**: Send request with Origin header
- **Expected**: Proper CORS headers in response:
  - `Access-Control-Allow-Origin`
  - `Access-Control-Allow-Methods`
  - `Access-Control-Allow-Headers`

### 6. Error Handling Tests

#### Test 6.1: Malformed JSON
- **Description**: Verify malformed JSON is rejected safely
- **Method**: Send invalid JSON payload
- **Expected**: 400/422 error without sensitive information leak

#### Test 6.2: Invalid Endpoint
- **Description**: Verify invalid endpoints return proper error
- **Method**: Request non-existent endpoint
- **Expected**: 404 Not Found

#### Test 6.3: Payload Size Limit
- **Description**: Verify oversized payloads are rejected
- **Method**: Send payload > 10MB
- **Expected**: 413/400 Payload Too Large

---

## Expected Results

### Success Criteria

| Test Category | Pass Criteria |
|--------------|---------------|
| Authentication | All valid keys accepted, invalid/missing keys return 401 |
| Rate Limiting | 100 requests allowed, 101st returns 429, per-key isolation works |
| Schema | Response contains all required fields with correct types |
| TLS | HTTPS works when enabled, certificates are valid |
| Headers | CORS headers present and configured |
| Error Handling | No sensitive info leaked, proper HTTP status codes |

### Sample Successful Output

```
============================================================
API Gateway Security Test Suite
Target: http://localhost:8000
TLS Enabled: false
============================================================

Testing API Key Authentication...
  ✓ PASS: Auth: Valid API Key
  ✓ PASS: Auth: Invalid API Key  
  ✓ PASS: Auth: Missing API Key

Testing Rate Limiting...
  Progress: 10/100 requests
  Progress: 20/100 requests
  ...
  ✓ PASS: Rate Limit: Within Limit (100 requests)
  ✓ PASS: Rate Limit: Exceeding Limit (101st request)
  ✓ PASS: Rate Limit: Per-Key Isolation

Testing AgiResponse Schema...
  ✓ PASS: Schema: AgiResponse Structure

Testing TLS Connection...
  ✓ PASS: TLS: Connection Security

Testing Security Headers...
  ✓ PASS: Headers: CORS Configuration

Testing Error Handling...
  ✓ PASS: Error Handling: Malformed JSON
  ✓ PASS: Error Handling: Invalid Endpoint
  ✓ PASS: Error Handling: Payload Size Limit

============================================================
TEST SUMMARY
============================================================
Total Tests: 13
Passed: 13
Failed: 0
Success Rate: 100.0%

Detailed report saved to: api-gateway-security-test-report.json
```

---

## Running Tests

### Environment Variables

Set these environment variables to customize test behavior:

| Variable | Description | Default |
|----------|-------------|---------|
| `API_GATEWAY_HOST` | API Gateway hostname | `localhost` |
| `API_GATEWAY_PORT` | API Gateway port | `8000` |
| `TLS_ENABLED` | Enable HTTPS/TLS | `false` |
| `PHOENIX_API_KEYS_FILE` | Path to API keys file | `config/test_api_keys.txt` |

### Command Line Options

#### Python Test Script
```bash
# Basic run
python api-gateway-security-test.py

# With custom host/port
API_GATEWAY_HOST=192.168.1.100 API_GATEWAY_PORT=8080 python api-gateway-security-test.py

# With TLS enabled
TLS_ENABLED=true python api-gateway-security-test.py
```

#### PowerShell Integration Script
```powershell
# Full test with build
.\integration-test.ps1

# Options:
#   -SkipBuild      Skip building services
#   -KeepRunning    Keep services running after tests
#   -TestOnly       Only run tests (services already running)
#   -ConfigFile     Specify environment file (default: .env.dev)

# Examples:
.\integration-test.ps1 -SkipBuild -KeepRunning
.\integration-test.ps1 -TestOnly
.\integration-test.ps1 -ConfigFile .env.production
```

---

## Configuration

### API Keys Configuration

Edit `config/test_api_keys.txt` to manage test API keys:

```
# Authentication test keys
test-valid-key-001
test-valid-key-002

# Rate limiting test keys
test-rate-limit-key-001
test-rate-limit-key-002

# Add custom test keys here
custom-test-key-123
```

### TLS Configuration

For TLS testing:

1. Enable TLS in environment:
   ```
   TLS_ENABLED=true
   TLS_CERT_PATH=certs/api-gateway.pem
   TLS_KEY_PATH=certs/api-gateway.key
   ```

2. Generate test certificates:
   ```bash
   cd certs
   ./generate_certs.sh
   ```

---

## Troubleshooting

### Common Issues and Solutions

#### Issue: "Python not found"
**Solution**: Install Python 3.8+ from https://www.python.org/downloads/

#### Issue: "requests module not found"
**Solution**: Install with `pip install requests`

#### Issue: "Connection refused" errors
**Solution**: 
1. Ensure services are running: `netstat -an | findstr :8000`
2. Check firewall settings
3. Verify correct ports in configuration

#### Issue: "Invalid API key" when using valid keys
**Solution**:
1. Verify `config/test_api_keys.txt` exists
2. Check `PHOENIX_API_KEYS_FILE` environment variable
3. Ensure API Gateway loaded keys correctly (check logs)

#### Issue: Rate limiting not working
**Solution**:
1. Ensure rate limiter is enabled in API Gateway
2. Check that requests include same API key
3. Verify sliding window duration (60 seconds)

#### Issue: TLS/Certificate errors
**Solution**:
1. For self-signed certs, the test script disables verification
2. Ensure certificates exist in specified paths
3. Check certificate validity dates

#### Issue: Tests hang or timeout
**Solution**:
1. Increase timeout in test configuration
2. Check service logs for errors
3. Verify network connectivity

### Debug Mode

Enable verbose logging:

```bash
# Set log level for services
RUST_LOG=debug cargo run --release

# Run tests with verbose output
python -v api-gateway-security-test.py
```

---

## Test Reports

### Report Files

After running tests, the following reports are generated:

1. **api-gateway-security-test-report.json** - Detailed test results
2. **integration-test-report.json** - Integration test summary

### Report Structure

```json
{
  "timestamp": "2024-01-15T10:30:45",
  "summary": {
    "total": 13,
    "passed": 12,
    "failed": 1,
    "success_rate": "92.3%"
  },
  "tests": [
    {
      "name": "Auth: Valid API Key",
      "passed": true,
      "message": "Valid API key accepted",
      "duration": 0.234,
      "details": {
        "status_code": 200
      }
    }
  ]
}
```

### Interpreting Results

- **Green/✓ PASS**: Test passed successfully
- **Red/✗ FAIL**: Test failed - review details
- **Duration**: Time taken for test (helps identify performance issues)
- **Details**: Additional context for debugging failures

---

## Security Checklist

Before deploying to production, ensure:

### Authentication
- [ ] Remove all test API keys from production
- [ ] Implement secure API key rotation
- [ ] Use strong, randomly generated API keys
- [ ] Never commit production keys to version control

### Rate Limiting
- [ ] Adjust rate limits based on expected traffic
- [ ] Implement different limits for different key types
- [ ] Add monitoring for rate limit violations
- [ ] Consider distributed rate limiting for scaling

### TLS/HTTPS
- [ ] Use valid SSL certificates (not self-signed)
- [ ] Enable TLS 1.2 or higher only
- [ ] Implement certificate rotation
- [ ] Use strong cipher suites

### Error Handling
- [ ] Ensure no stack traces in production
- [ ] Log security events for monitoring
- [ ] Implement request sanitization
- [ ] Add input validation for all endpoints

### Monitoring
- [ ] Set up alerts for authentication failures
- [ ] Monitor rate limit violations
- [ ] Track error rates and response times
- [ ] Implement audit logging

### Additional Security
- [ ] Implement request signing for critical operations
- [ ] Add IP allowlisting where appropriate
- [ ] Use secure defaults for all configurations
- [ ] Regular security audits and penetration testing

---

## Support

For issues or questions:

1. Check service logs: `target/release/*.log`
2. Review test reports: `*-report.json`
3. Enable debug logging: `RUST_LOG=debug`
4. Consult the main project documentation

---

## Version History

- **1.0.0** - Initial security test suite
  - API key authentication
  - Rate limiting (100 req/min)
  - AgiResponse schema validation
  - TLS/HTTPS support
  - Security headers
  - Error handling

---

*Last Updated: 2024-01-15*