# Scripts

## Overview

Utility scripts for deployment, installation, and system management of Phoenix ORCH AGI system.

## Files

- **deploy.sh**: Main deployment script for production environments
- **install_protoc.sh**: Protocol Buffer compiler installation (Linux/macOS)
- **install_protoc.ps1**: Protocol Buffer compiler installation (Windows)

## Usage

### Deployment Script
```bash
./scripts/deploy.sh
```

The deployment script handles:
- Service build and compilation
- Docker image creation
- Service startup and health checks
- Configuration validation

### Protocol Buffer Compiler Installation

#### Linux/macOS
```bash
./scripts/install_protoc.sh
```

#### Windows (PowerShell)
```powershell
.\scripts\install_protoc.ps1
```

## Script Details

### deploy.sh
- Builds all Rust services
- Validates configuration files
- Starts services in dependency order
- Performs health checks
- Reports deployment status

### install_protoc.sh / install_protoc.ps1
- Downloads Protocol Buffer compiler
- Extracts to appropriate location
- Adds to PATH
- Verifies installation

## Prerequisites

### deploy.sh
- Rust toolchain installed
- Docker and Docker Compose installed
- Configuration files present in `config/`
- Required environment variables set

### install_protoc.*
- Internet connection for download
- Write permissions for installation directory
- PATH modification permissions

## Platform Support

- **Linux**: Full support via bash scripts
- **macOS**: Full support via bash scripts
- **Windows**: PowerShell scripts for Windows-specific operations

## Error Handling

All scripts include:
- Error checking at each step
- Rollback capabilities where applicable
- Detailed error messages
- Exit code reporting

