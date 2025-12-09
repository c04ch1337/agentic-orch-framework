# Phoenix ORCH Container Security & Resource Management Guide

This document provides comprehensive information about the security hardening, resource limitations, and best practices implemented in the Phoenix ORCH AGI system's containerization.

## Table of Contents

1. [Overview](#overview)
2. [Docker Compose Configurations](#docker-compose-configurations)
3. [Dockerfile Security Hardening](#dockerfile-security-hardening)
4. [Container Security Scanning](#container-security-scanning)
5. [Resource Monitoring & Alerts](#resource-monitoring-alerts)
6. [Kubernetes Orchestration](#kubernetes-orchestration)
7. [Security Best Practices](#security-best-practices)
8. [Maintenance & Updates](#maintenance-updates)

## Overview

The Phoenix ORCH AGI system has been secured through a defense-in-depth approach that addresses security at multiple layers:

- Container resource constraints
- Container isolation and privilege reduction
- Network segmentation
- Continuous security scanning
- Real-time monitoring and alerts
- Secure orchestration with Kubernetes
- Volume encryption and access controls

These measures work together to create a robust security posture while ensuring application performance, resilience, and scalability.

## Docker Compose Configurations

### Development Environment (`docker-compose.dev.yml`)

The development environment includes essential resource constraints and security features while remaining flexible for development work:

- **Memory Limits**: Each service has explicit memory limits to prevent resource exhaustion
- **CPU Limits**: Services have CPU limits to ensure fair resource sharing
- **Restart Policies**: Services use `on-failure:5` to prevent infinite restart loops
- **Healthchecks**: All services implement comprehensive health checks
- **Read-Only Volumes**: Application code mounted as read-only
- **Isolated Volumes**: Each service has its own isolated data volume
- **Network Segmentation**: Internal networks isolate service-to-service communication

### Production Environment (`docker-compose.yml`)

The production environment builds on the development configuration with stricter resource controls:

- **Higher Resource Reservations**: Production services reserve more resources for reliability
- **Stricter Restart Policies**: Faster failure detection and fewer retry attempts
- **Enhanced Healthchecks**: More aggressive healthcheck intervals (15s vs 30s)
- **Volume Driver Specification**: All volumes use the local driver with explicit configuration
- **No Development Code Mounts**: Production never mounts source code directories

## Dockerfile Security Hardening

All service Dockerfiles implement these security hardening techniques:

### Multi-Stage Builds

All Dockerfiles use multi-stage builds to:
- Minimize final image size
- Reduce attack surface
- Eliminate build tools and artifacts from the runtime image

Example from `secrets-service-rs/Dockerfile`:
```dockerfile
FROM rust:1.75-slim AS builder
# Build steps...

FROM debian:bookworm-slim AS runtime
# Only copy the built application and necessary runtime files
```

### Non-Root User Execution

All containers run as non-root users:
- Custom user accounts created with minimal privileges
- Container processes have no ability to modify system files
- Prevents privilege escalation attacks

Example implementation:
```dockerfile
RUN groupadd -r appuser && useradd -r -g appuser -s /bin/false -d /app appuser
USER appuser
```

### Least Privilege Principle

Containers follow least privilege principles:
- All processes execute with minimal required access
- Capability dropping: `--cap-drop=ALL` with only necessary capabilities added back
- Read-only root filesystem where possible
- Volume mounts are minimal and isolated

### Minimal Base Images

All images use minimal base images:
- Debian slim variants to reduce attack surface
- Only necessary runtime dependencies installed
- Package caches removed after installation

### Security Auditing

Each build includes a security audit step:
- `cargo audit` scans Rust dependencies for vulnerabilities
- Container configurations are verified against security benchmarks

## Container Security Scanning

The CI/CD pipeline implements comprehensive container security scanning:

### Vulnerability Scanning with Trivy

Trivy scans all container images for:
- Operating system vulnerabilities
- Language-specific vulnerabilities
- Dependency vulnerabilities
- Misconfigurations

Scan results are reported as SARIF files and integrated into GitHub Security tab.

### Docker Best Practices with Dockle

Dockle enforces Docker best practices:
- CIS Benchmark compliance
- Dockerfile optimization
- Security configuration verification

### Container Signing

Production images are signed using Cosign for integrity verification:
- OIDC-based signing with GitHub identity
- Signature verification on deployment
- Prevents tampering and supply chain attacks

### Policy Enforcement

Security policies are enforced across the pipeline:
- Custom seccomp profiles limit system calls
- Admission controllers enforce security standards
- Image scanning gates deployment on security findings

## Resource Monitoring & Alerts

Comprehensive monitoring is implemented using:

### Prometheus

- Collects metrics from all services
- Monitors resource usage (CPU, memory, disk)
- Tracks application-specific metrics
- Implements alerting rules for resource exhaustion

### Grafana

- Visualizes metrics and performance data
- Provides comprehensive dashboards
- Manages notification channels
- Implements alert escalation

### AlertManager

- Routes alerts to appropriate teams
- Implements notification policies
- Handles alert aggregation and deduplication
- Integrates with external systems (email, Slack, PagerDuty)

### Key Alert Thresholds

- Container CPU usage > 85% for 5 minutes (warning)
- Container CPU usage > 95% for 2 minutes (critical)
- Container memory usage > 85% for 5 minutes (warning)
- Container memory usage > 95% for 2 minutes (critical)
- Service restart frequency > 2 in 15 minutes (warning)
- Host resource constraints (CPU, memory, disk) > 85% (warning)

## Kubernetes Orchestration

Kubernetes configurations implement secure orchestration:

### Resource Quotas

- Namespace-level resource quotas prevent resource exhaustion
- Deployment-level resource requests/limits ensure equitable distribution
- LimitRange objects establish defaults and constraints

### Network Policies

- Default deny-all policy for ingress/egress
- Explicit service-to-service communication paths
- Internal networks for service isolation
- Allow rules only for necessary communications

### Pod Security Standards

- Restricted pod security standard is enforced
- SecurityContext objects implement:
  - Non-root users
  - Read-only root filesystem
  - Dropped capabilities
  - Seccomp profiles

### RBAC Controls

- Service accounts with minimal permissions
- Role-based access control using least privilege
- No use of cluster-wide permissions

### Volume Security

- Encrypted persistent storage with secure storage classes
- Volume permissions configured via init containers
- Sensitive data segregation in separate volumes
- Volume auditing for security compliance

### Autoscaling

Horizontal Pod Autoscaler configurations:
- Scale based on CPU and memory utilization
- Service-specific scaling parameters
- Controlled scale-up/down behavior to prevent thrashing

## Security Best Practices

### Container Runtime Security

1. **Keep Images Updated**: Regularly update base images to incorporate security patches
2. **Scan Images Regularly**: Run vulnerability scans on both new and existing images
3. **Monitor Container Activity**: Implement runtime behavior monitoring
4. **Use Immutable Containers**: Never modify running containers - rebuild and redeploy instead

### Secrets Management

1. **Never Store Secrets in Images**: Use external secrets management
2. **Rotate Secrets Regularly**: Implement automated secret rotation
3. **Least-Privilege Access**: Only provide secrets to containers that need them
4. **Encrypt Secrets at Rest**: Use encrypted volumes for secret storage

### Network Security

1. **Segment Networks**: Use internal networks for service-to-service communication
2. **Encrypt Traffic**: Use TLS for all external and sensitive internal communications
3. **Implement Ingress Controls**: Protect public endpoints with WAF and rate limiting
4. **Monitor Network Activity**: Alert on unexpected connection patterns

### Resource Management

1. **Set Appropriate Limits**: Configure resource limits based on observed usage
2. **Include Headroom**: Allow for traffic spikes and growth
3. **Test Under Load**: Verify behavior under resource constraints
4. **Implement Graceful Degradation**: Services should handle resource pressure appropriately

## Maintenance & Updates

### Regular Security Maintenance

1. **Weekly Image Updates**: Rebuild container images weekly to incorporate security patches
2. **Monthly Security Review**: Review security scan results and address findings
3. **Quarterly Configuration Audit**: Review and update security configurations

### Update Procedures

1. **Test Updates in Development**: Verify all updates in dev environment first
2. **Incremental Production Updates**: Roll out changes gradually with monitoring
3. **Maintain Rollback Capability**: Keep previous images available for quick rollback
4. **Document Changes**: Maintain changelog of security-related updates

### Emergency Response

1. **Vulnerability Response Plan**: Document procedures for handling critical vulnerabilities
2. **Container Isolation**: Ability to isolate compromised containers
3. **Forensic Readiness**: Configure logging for security investigation

---

## References

- [Docker Security Documentation](https://docs.docker.com/engine/security/)
- [Kubernetes Security Best Practices](https://kubernetes.io/docs/concepts/security/overview/)
- [OWASP Docker Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Docker_Security_Cheat_Sheet.html)
- [CIS Docker Benchmark](https://www.cisecurity.org/benchmark/docker)
- [NIST Container Security Guide](https://nvlpubs.nist.gov/nistpubs/specialpublications/nist.sp.800-190.pdf)