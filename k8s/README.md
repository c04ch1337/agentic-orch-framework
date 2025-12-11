# Kubernetes Manifests

## Overview

Kubernetes deployment manifests for Phoenix ORCH AGI system. Includes namespace configuration, resource quotas, network policies, pod security, autoscaling, and volume security.

## Files

- **00-namespace.yml**: Namespace definition and labels
- **01-resource-quotas.yml**: Resource quotas and limits per namespace
- **02-network-policies.yml**: Network isolation and traffic policies
- **03-pod-security.yml**: Pod security standards and policies
- **04-autoscaling.yml**: Horizontal Pod Autoscaler configurations
- **05-volume-security.yml**: Persistent volume claims and security

## Deployment Order

Manifests are numbered to indicate deployment sequence:
1. Namespace creation
2. Resource quotas
3. Network policies
4. Pod security
5. Autoscaling
6. Volume security

## Usage

### Apply All Manifests
```bash
kubectl apply -f k8s/
```

### Apply Individual Components
```bash
kubectl apply -f k8s/00-namespace.yml
kubectl apply -f k8s/01-resource-quotas.yml
# ... etc
```

## Security Features

- **Network Policies**: Restrict pod-to-pod communication
- **Pod Security Standards**: Enforce security contexts
- **Resource Quotas**: Prevent resource exhaustion
- **Volume Security**: Encrypted persistent volumes

## Resource Management

- CPU and memory limits per namespace
- Storage quotas
- Pod count limits
- Service account restrictions

## Network Policies

Policies define:
- Allowed ingress traffic
- Allowed egress traffic
- Service-to-service communication rules
- External access restrictions

## Autoscaling

- Horizontal Pod Autoscaling based on CPU/memory
- Target utilization thresholds
- Min/max replica counts
- Scale-down stabilization periods

## Volume Security

- Encrypted persistent volumes
- Access mode restrictions
- Storage class selection
- Backup and restore policies

