# Phoenix Orchestrator CI/CD Documentation

This document provides a comprehensive overview of the CI/CD pipeline for the Phoenix Orchestrator platform.

## Table of Contents
- [Overview](#overview)
- [Workflows](#workflows)
  - [PR Validation](#pr-validation)
  - [Deployment Pipelines](#deployment-pipelines)
    - [Development Environment](#development-environment)
    - [Staging Environment](#staging-environment)
    - [Production Environment](#production-environment)
  - [Security Scanning](#security-scanning)
  - [Performance Benchmarking](#performance-benchmarking)
- [Quality Gates](#quality-gates)
  - [Code Quality Checks](#code-quality-checks)
  - [Test Coverage Requirements](#test-coverage-requirements)
  - [Security Thresholds](#security-thresholds)
- [Environment Configuration](#environment-configuration)
- [Secrets Management](#secrets-management)
- [Canary Deployments](#canary-deployments)
- [Rollback Procedures](#rollback-procedures)
- [Load Testing Integration](#load-testing-integration)
- [Alerts and Notifications](#alerts-and-notifications)
- [Troubleshooting](#troubleshooting)

## Overview

The Phoenix Orchestrator CI/CD pipeline implements a comprehensive workflow for building, testing, and deploying the platform across multiple environments. The pipeline is built using GitHub Actions and follows modern DevOps practices including:

- **Continuous Integration**: Automatic building and testing of code changes
- **Continuous Delivery**: Automated deployment to development and staging environments
- **Continuous Deployment**: Semi-automated deployment to production with approval gates
- **Quality Gates**: Automated checks for code quality, test coverage, and security issues
- **Canary Deployments**: Gradual rollout of changes to production
- **Automated Rollbacks**: Intelligent rollback capabilities if issues are detected

## Workflows

### PR Validation

The PR validation workflow runs automatically on all pull requests to the main branch. It performs the following checks:

- Build verification for all Rust services
- Code formatting checks with `rustfmt`
- Linting with `clippy`
- Unit tests with coverage reporting
- Frontend build and test verification
- Dependency vulnerability scanning
- Secret detection

**How to use it:**
- Create a pull request against the main branch
- The workflow will run automatically
- Fix any issues reported by the workflow
- Once all checks pass, the PR can be merged

### Deployment Pipelines

#### Development Environment

The development deployment pipeline runs automatically when changes are merged to the `develop` branch.

**Features:**
- Builds and pushes Docker images for all services
- Tags images with `dev` and the commit SHA
- Signs images with Cosign for security
- Performs vulnerability scanning
- Deploys to the dev Kubernetes cluster
- Runs post-deployment health checks

**How to use it:**
```bash
# To manually trigger a deployment to dev
gh workflow run deploy-dev.yml

# To deploy a specific service only
gh workflow run deploy-dev.yml --field specific_service=orchestrator-service-rs
```

#### Staging Environment

The staging deployment pipeline runs automatically when changes are merged to the `staging` branch.

**Features:**
- Builds and pushes Docker images for all services
- Tags images with `staging` and the commit SHA
- Signs images with Cosign for security
- Performs vulnerability scanning
- Runs pre-deployment integration tests
- Deploys to the staging Kubernetes cluster
- Runs post-deployment health checks
- Conducts load testing to verify performance

**How to use it:**
```bash
# To manually trigger a deployment to staging
gh workflow run deploy-staging.yml

# To deploy a specific service only
gh workflow run deploy-staging.yml --field specific_service=data-router-rs
```

#### Production Environment

The production deployment pipeline runs automatically when a new version tag (e.g., `v1.2.3`) is pushed.

**Features:**
- Validates release signature
- Builds and pushes Docker images for all services
- Tags images with `production`, the version tag, and `latest`
- Signs images with Cosign for security
- Performs vulnerability scanning
- Runs extensive pre-production tests
- Requires manual approval before deployment
- Implements canary deployment strategy
- Monitors canary deployment for issues
- Supports automated rollbacks if problems are detected

**How to use it:**
```bash
# Tag a new release
git tag -a v1.2.3 -m "Release v1.2.3"
git push origin v1.2.3

# To manually trigger a production deployment
gh workflow run deploy-production.yml

# To deploy a specific service only with custom canary percentage
gh workflow run deploy-production.yml --field specific_service=llm-service-rs --field deploy_percentage=20
```

### Security Scanning

The security scanning workflow runs daily to check for vulnerabilities and dependency issues.

**Features:**
- Scans dependencies for security vulnerabilities
- Checks Docker images for security issues
- Runs code scanning using CodeQL
- Looks for secrets accidentally committed to the repository
- Generates a comprehensive security report
- Creates issues for any critical findings

**How to use it:**
```bash
# To manually trigger a security scan
gh workflow run security-scan.yml
```

### Performance Benchmarking

The performance benchmarking workflow runs weekly to establish performance baselines and validate against them.

**Features:**
- Runs load tests against specified environment
- Supports multiple test scenarios (baseline, user journey, stress)
- Establishes performance baselines
- Validates current performance against historical benchmarks
- Creates issues for any performance regressions
- Generates detailed performance reports

**How to use it:**
```bash
# To manually run performance benchmarks against staging
gh workflow run performance-benchmark.yml --field environment=staging --field scenario=baseline

# To run all test scenarios
gh workflow run performance-benchmark.yml --field environment=staging --field scenario=all
```

## Quality Gates

### Code Quality Checks

The CI/CD pipeline implements various code quality checks:

- **Rust Formatting**: All Rust code must pass `rustfmt` checks
- **Rust Linting**: All Rust code must pass `clippy` checks with no warnings
- **TypeScript Linting**: Frontend code must pass ESLint checks
- **TypeScript Type Checking**: Frontend code must pass type checking
- **Commit Message Standards**: Commits must follow conventional commit format

### Test Coverage Requirements

Code coverage requirements are enforced using Codecov:

- **Overall Project**: 80% coverage required
- **New Code**: 80% coverage required for new code
- **Service-Specific Targets**:
  - `orchestrator-service-rs`: 85%
  - `data-router-rs`: 80%
  - `llm-service-rs`: 80%
  - `tools-service-rs`: 80%
  - `frontend`: 85%

Coverage reports are generated for both Rust services (using `cargo-tarpaulin`) and the frontend (using Jest).

### Security Thresholds

The CI/CD pipeline enforces the following security thresholds:

- No critical vulnerabilities allowed in dependencies
- No critical vulnerabilities allowed in container images
- No medium or higher severity CodeQL alerts
- No committed secrets
- Fail builds if dependency scanning cannot complete

## Environment Configuration

The CI/CD pipeline supports different environment configurations:

- **Development**: Used for ongoing development work
- **Staging**: Used for pre-production testing
- **Production**: Live production environment

Environment-specific configuration is stored in environment variables and Kubernetes ConfigMaps. The deployment workflows automatically apply the appropriate configuration for each environment.

### Directory Structure:

```
k8s/
├── 00-namespace.yml
├── dev/
│   ├── configmaps/
│   ├── deployments/
│   └── services/
├── staging/
│   ├── configmaps/
│   ├── deployments/
│   └── services/
└── production/
    ├── configmaps/
    ├── deployments/
    └── services/
```

## Secrets Management

Sensitive information is managed securely using AWS Secrets Manager and Kubernetes Secrets:

- No secrets are stored in the repository
- Secrets are injected at deployment time
- Deployment workflows have the necessary permissions to access secrets
- Secrets are rotated regularly using automated processes

## Canary Deployments

The production deployment workflow supports canary deployments:

1. A small percentage of traffic is routed to the new version
2. The canary deployment is monitored for errors and performance issues
3. If issues are detected, the deployment is automatically rolled back
4. If no issues are detected, the deployment proceeds with full rollout

**Configurable parameters:**
- Initial canary percentage (default: 10%)
- Monitoring duration (default: 5 minutes)
- Error threshold (default: 5 errors in 100 log entries)

## Rollback Procedures

The CI/CD pipeline includes automatic rollback procedures:

### Automatic Rollbacks

The production deployment workflow will automatically roll back in the following cases:
- Canary deployment shows excessive errors
- Post-deployment health checks fail
- Key metrics show significant degradation

### Manual Rollbacks

To manually rollback a deployment:

```bash
# Using GitHub Actions
gh workflow run deploy-production.yml --field rollback=true

# Using kubectl directly
kubectl rollout undo deployment <deployment-name> -n phoenix-orch
```

## Load Testing Integration

The CI/CD pipeline integrates with the load testing framework:

- **Pre-deployment Testing**: Run basic load tests before deployment to staging and production
- **Post-deployment Validation**: Run load tests after deployment to verify performance
- **Regular Performance Benchmarking**: Establish and validate against performance baselines
- **Regression Detection**: Automatically detect and alert on performance regressions

Load test configurations are stored in the `load-testing/` directory.

## Alerts and Notifications

The CI/CD pipeline includes alerts and notifications for important events:

- Failed workflow runs (via GitHub notifications)
- Production deployment approvals (via GitHub Issues)
- Security vulnerabilities (via GitHub Issues)
- Performance regressions (via GitHub Issues)
- Critical deployment failures (via Slack)

## Troubleshooting

### Common Issues

1. **Workflow Failures**

   If a workflow fails, check the GitHub Actions logs for details. Common causes include:
   - Test failures
   - Linting/formatting issues
   - Build errors
   - Security scan failures

2. **Deployment Failures**

   If a deployment fails, check:
   - Kubernetes logs: `kubectl logs -n phoenix-orch <pod-name>`
   - Service health: `kubectl get pods -n phoenix-orch`
   - Deployment status: `kubectl describe deployment <deployment-name> -n phoenix-orch`

3. **Performance Validation Failures**

   If performance validation fails:
   - Check the validation reports in the workflow artifacts
   - Compare the results with the baseline
   - Investigate potential code changes that might have affected performance
   - Verify if the test environment itself has issues

### Getting Help

For assistance with CI/CD issues:
- Open an issue in the GitHub repository
- Contact the DevOps team on Slack
- Consult the detailed logs in GitHub Actions