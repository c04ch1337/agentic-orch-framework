# Docker Configuration

## Overview

Docker Compose configurations and templates for Phoenix ORCH AGI system deployment. Includes production, development, monitoring, and security metrics configurations.

## Files

- **docker-compose.yml**: Production configuration with security and resource limits
- **docker-compose.dev.yml**: Development configuration with hot-reload support
- **docker-compose.monitoring.yml**: Monitoring stack (Prometheus, Grafana, Alertmanager)
- **docker-compose.security-metrics.yml**: Security metrics collection
- **Dockerfile.template**: Base template for service Dockerfiles

## Services

### Core Services
- Orchestrator Service
- Data Router Service
- API Gateway
- Agent Registry
- Auth Service
- LLM Service
- Tools Service

### Knowledge Base Services
- Mind KB
- Soul KB
- Heart KB
- Body KB
- Social KB
- Persistence KB

### Supporting Services
- Logging Service
- Reflection Service
- Safety Service
- Secrets Service
- Scheduler Service
- Sensor Service

## Usage

### Production Deployment
```bash
docker-compose -f docker-compose.yml up -d
```

### Development Environment
```bash
docker-compose -f docker-compose.dev.yml up
```

### Monitoring Stack
```bash
docker-compose -f docker-compose.monitoring.yml up -d
```

### Security Metrics
```bash
docker-compose -f docker-compose.security-metrics.yml up -d
```

## Configuration

### Environment Variables
- `TAG`: Docker image tag (default: "latest")
- Service-specific port mappings via environment variables
- Resource limits configured per service

## Security Features

- Resource limits (memory, CPU)
- Health checks for all services
- Network isolation
- Volume mounts for persistent data
- Restart policies

## Network Architecture

- **agi_network**: Main application network
- **orchestrator_network**: Orchestrator-specific network
- Service-to-service communication via service names

## Volume Management

Persistent volumes for:
- Service data directories
- Log storage
- Configuration files
- Certificate storage

