# PHOENIX ORCH Environment Configuration Guide

## Overview

This guide explains the new unified approach to environment configuration in the PHOENIX ORCH system.

## Table of Contents

- [Introduction](#introduction)
- [Why We Consolidated](#why-we-consolidated)
- [File Structure](#file-structure)
- [Environment Switching](#environment-switching)
- [Variable Precedence](#variable-precedence)
- [Adding New Variables](#adding-new-variables)
- [Secrets Management](#secrets-management)
- [Best Practices](#best-practices)

## Introduction

The PHOENIX ORCH system now uses a consolidated environment configuration approach that replaces the previous multiple `.env` file system (`.env.dev`, `.env.staging`, `.env.production`, etc.).

The new approach:
1. Uses a single template file (`.env.example.consolidated`) with comprehensive documentation
2. Employs environment switching scripts for easy environment changes
3. Provides clearer organization and documentation of all variables
4. Reduces duplication and potential inconsistencies

## Why We Consolidated

The previous approach using multiple `.env` files had several drawbacks:
- Duplication of variables across files
- Inconsistent variable naming and organization
- Difficulty tracking which variables were environment-specific
- Risk of missing variables in certain environments
- No clear documentation of variable purpose and valid values

The new approach addresses these issues while maintaining the ability to use different settings in different environments.

## File Structure

The new environment configuration system consists of:

- **`.env.example.consolidated`**: The master template containing all supported variables with documentation and default values.
- **`.env`**: The active environment file containing the actual values used by the system.
- **`env_switcher.sh`**: Bash script for Unix/macOS to switch between environments.
- **`env_switcher.ps1`**: PowerShell script for Windows to switch between environments.

The `.env.example.consolidated` file is organized into logical sections:
- Core System Configuration
- Secrets Management
- LLM Service Configuration
- Vector Database Configuration
- Service Port Configuration
- Container Resources
- Agent Personality
- Safety Configuration
- Memory Configuration
- And several other specialized sections

Each variable has:
- Clear documentation of its purpose
- Default or example values
- Notes about environment-specific customizations where relevant

## Environment Switching

You can easily switch between environments (development, staging, production) using the provided scripts.

### For Linux/macOS:

```bash
# Switch to development environment
./env_switcher.sh -e development

# Switch to staging environment
./env_switcher.sh -e staging

# Switch to production environment
./env_switcher.sh -e production

# Help
./env_switcher.sh -h
```

### For Windows:

```powershell
# Switch to development environment
.\env_switcher.ps1 -Environment development

# Switch to staging environment
.\env_switcher.ps1 -Environment staging

# Switch to production environment
.\env_switcher.ps1 -Environment production

# Help
.\env_switcher.ps1 -Help
```

When you switch environments:
1. A backup of your current `.env` file is created
2. A new `.env` file is generated from the template
3. The `ENVIRONMENT` variable is set to your selected environment
4. Environment-specific overrides are uncommented and activated

After switching environments, restart your containers to apply the changes:
```bash
docker-compose down && docker-compose up -d
```

## Variable Precedence

Variables are evaluated in this order:

1. **Base variables**: Default values defined in the main sections
2. **Environment-specific overrides**: Values defined in the environment-specific sections
3. **Vault secrets**: Sensitive values retrieved from HashiCorp Vault at runtime

This means environment-specific values take precedence over the base values.

## Adding New Variables

To add a new configuration variable:

1. Add it to the appropriate section in `.env.example.consolidated`
2. Include comprehensive documentation about its purpose
3. Provide a sensible default value
4. If the variable should be different across environments, add overrides in the environment-specific sections

Example:
```
# Maximum number of concurrent connections
MAX_CONNECTIONS=100

# Environment-specific overrides section
# DEVELOPMENT_MAX_CONNECTIONS=50
# STAGING_MAX_CONNECTIONS=200
# PRODUCTION_MAX_CONNECTIONS=500
```

## Secrets Management

Sensitive values like API keys, passwords, and tokens should NEVER be committed to version control, even in examples.

The recommended approach is:
1. Store sensitive values in HashiCorp Vault
2. Use the `secrets-service-rs` to retrieve them at runtime
3. Only use placeholder values in `.env` files

For development:
- You may use actual values in your local `.env` file that is not committed
- Ensure `.env` is listed in `.gitignore`
- Consider using a local Vault development server

## Best Practices

1. **Never commit actual credentials** to version control
2. **Keep the template up to date** - whenever you add a variable to `.env`, add it to `.env.example.consolidated`
3. **Use clear, descriptive variable names** - prefer `LLM_MODEL_TEMPERATURE` over `TEMP`
4. **Add comments** for non-obvious settings
5. **Use the environment switcher** rather than manually editing `.env` files
6. **Keep environment-specific settings minimal** - most values should be the same across environments
7. **Group related variables** in the same section
8. **Use consistent naming conventions** for related variables
9. **Periodically review** all environment variables to remove obsolete ones

By following these guidelines, the PHOENIX ORCH system will maintain a clean, well-documented, and consistent environment configuration approach.