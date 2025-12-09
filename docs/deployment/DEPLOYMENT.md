# Phoenix ORCH Deployment Guide

This guide outlines the steps for deploying the Phoenix ORCH system with proper port configuration in different environments.

## Prerequisites

1. Install Rust (stable version 1.70 or higher)
2. Install Protocol Buffers Compiler (protoc) - see [install_protoc.sh](./install_protoc.sh) or [install_protoc.ps1](./install_protoc.ps1)
3. Clone the repository

## Local Development Deployment

### 1. Setup Environment Configuration

Use the provided normalized environment file as a template:

```bash
cp .env.example.normalized .env.dev
```

### 2. Port Configuration

The `.env.dev` file contains standardized port configurations:

```env
# Core Services
ORCHESTRATOR_SERVICE_PORT=50051
DATA_ROUTER_SERVICE_PORT=50052
LLM_SERVICE_PORT=50053
# ... additional service ports
```

To change a port, simply modify the corresponding environment variable.

### 3. Build the System

```bash
# Build all services
cargo build

# Build a specific service
cargo build -p llm-service-rs
```

### 4. Start Services

Start services in order of dependencies:

```bash
# Start Secrets Service
cargo run -p secrets-service-rs &

# Start Data Router
cargo run -p data-router-rs &

# Start Orchestrator
cargo run -p orchestrator-service-rs &

# Start LLM Service
cargo run -p llm-service-rs &

# ... and other services as needed
```

## Docker Deployment

### 1. Create Docker-specific Environment File

```bash
cp .env.example.normalized .env.docker
```

Edit `.env.docker` to use suitable container network addresses:

```env
# Service Discovery for Docker network
SERVICE_HOST=host.docker.internal    # For Mac/Windows
# SERVICE_HOST=172.17.0.1            # For Linux
```

### 2. Build and Start with Docker Compose

```bash
# Build all containers
docker-compose build

# Start the system
docker-compose up -d
```

## Kubernetes Deployment

### 1. Apply ConfigMaps for Environment Variables

```bash
# Create ConfigMap from normalized env file
kubectl create configmap phoenix-config --from-env-file=.env.example.normalized

# Apply to cluster
kubectl apply -f k8s/
```

### 2. Configure Inter-Pod Communication

When deployed to Kubernetes, services locate each other using the standard Kubernetes service discovery:

```
SERVICE_NAME.NAMESPACE.svc.cluster.local
```

Update the `SERVICE_HOST` value in your ConfigMap accordingly.

## Verifying Deployment

1. **Check Service Health**
   
   ```bash
   # Check API Gateway
   curl http://localhost:8282/health
   
   # Check individual services
   grpcurl -plaintext localhost:50051 agi_core.HealthService/GetHealth
   ```

2. **Validate Port Configuration**

   Verify service binding using standard OS tools:
   
   ```bash
   # Linux/macOS
   netstat -tulpn | grep LISTEN
   
   # Windows
   netstat -ano | findstr LISTENING
   ```

3. **Test Service-to-Service Communication**

   Use the verification steps detailed in [verification_plan.md](./verification_plan.md).

## Troubleshooting

1. **Port Conflicts**

   If you encounter port conflicts, simply override the port in your environment file:
   
   ```env
   # Change LLM Service to port 50153 instead of 50053
   LLM_SERVICE_PORT=50153
   ```

2. **Service Discovery Issues**

   If services can't discover each other, verify:
   
   - Environment variables are properly set
   - Network connectivity between services
   - No firewall rules blocking communication

3. **Build Issues**

   If you encounter Protocol Buffers related errors:
   
   ```bash
   # Install protoc using the provided script
   chmod +x install_protoc.sh
   ./install_protoc.sh
   ```

## Modifying Port Configuration

All port configuration follows the standardized pattern:

1. `SERVICE_NAME_PORT`: Configures the port number (e.g., `LLM_SERVICE_PORT=50053`)
2. `SERVICE_NAME_ADDR`: Full address override (e.g., `LLM_SERVICE_ADDR=http://custom-host:50053`)

The system will use these values in the following order of precedence:
1. `SERVICE_NAME_ADDR` if provided
2. `SERVICE_NAME_PORT` if provided
3. Default port from the configuration library