# Blue Team Service

## Overview
The Blue Team Service is a critical security component that provides autonomous defense, incident triage, and system hardening capabilities. It acts as a sentinel within the system, continuously monitoring and responding to security events, anomalies, and potential threats.

## Port Information
- Default Port: 50069
- Service Address: `0.0.0.0:50069` (configurable via environment variables)
- Protocol: gRPC

## Key Functionalities

### 1. Anomaly Triage
- Automated analysis of security anomalies
- Risk classification and threat assessment
- Priority-based response recommendations
- Supports multiple anomaly types:
  - Network anomalies
  - Behavioral anomalies
  - Access-related anomalies

### 2. Threat Containment
- Automated threat isolation and containment
- Multiple containment strategies:
  - Network isolation
  - Process quarantine
  - Firewall rule management
- Optional automatic remediation

### 3. System Hardening
- Security compliance assessment
- Support for multiple hardening profiles:
  - CIS (Center for Internet Security)
  - NIST (National Institute of Standards and Technology)
- Automated security controls implementation
- Compliance scoring and reporting

## Dependencies and Requirements

### Core Dependencies
```toml
tokio = "1.48.0"
tonic = "0.14.2"
tonic-health = "0.8.0"
prost = "0.14.1"
log = "0.4.29"
env_logger = "0.11"
uuid = "1.11"
```

### System Requirements
- Rust 2021 edition
- gRPC support
- Network access for inter-service communication

## Configuration

### Environment Variables
- `BLUE_TEAM_ADDR`: Service address (default: "0.0.0.0:50069")
- `AGENT_REGISTRY_ADDR`: Agent registry service address (default: "http://127.0.0.1:50067")

### Service Registration
The service automatically registers with the Agent Registry service on startup with the following capabilities:
- Anomaly triage
- Threat containment
- Patch management
- Security hardening
- Log analysis

### Health Monitoring
- Implements standard gRPC health checking protocol
- Exposes health metrics:
  - Service uptime
  - Number of anomalies triaged
  - Number of threats contained
  - Number of systems hardened
  - Dependency status

## Integration
The service integrates with the broader system through gRPC interfaces and can be monitored through standard observability tools. It maintains its state and metrics, providing real-time insights into security operations and system health.