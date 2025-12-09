# ==============================================================
# PHOENIX ORCH Environment Switcher (PowerShell Version)
# ==============================================================
# This script helps switch between different environments by:
# 1. Creating a proper .env file from the .env.example.consolidated template
# 2. Setting the ENVIRONMENT variable
# 3. Applying environment-specific overrides automatically
# ==============================================================

# Default values
$EnvExample = Join-Path $PSScriptRoot ".env.example.consolidated"
$EnvFile = Join-Path $PSScriptRoot ".env"
$SelectedEnv = ""

function Print-Usage {
    Write-Host "Usage: .\env_switcher.ps1 [OPTIONS]"
    Write-Host "Options:"
    Write-Host "  -Environment <ENV>   Set environment (development|staging|production)"
    Write-Host "                       Shorthand: dev, prod"
    Write-Host "  -Help                Show this help message"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  .\env_switcher.ps1 -Environment development"
    Write-Host "  .\env_switcher.ps1 -Environment staging"
    Write-Host "  .\env_switcher.ps1 -Environment prod"
}

function Copy-Template {
    # Check if .env.example.consolidated exists
    if (-not (Test-Path $EnvExample)) {
        Write-Host "Error: Template file not found at $EnvExample" -ForegroundColor Red
        Write-Host "Make sure you're running this script from the project root directory."
        exit 1
    }
    
    # Create backup of existing .env if it exists
    if (Test-Path $EnvFile) {
        $timestamp = Get-Date -Format "yyyyMMddHHmmss"
        $backupFile = "$EnvFile.backup.$timestamp"
        Write-Host "Creating backup of existing .env file to $backupFile"
        Copy-Item $EnvFile $backupFile
    }
    
    # Copy the template
    Copy-Item $EnvExample $EnvFile
    Write-Host "Created new .env file from template."
}

function Apply-EnvironmentSettings {
    param (
        [string]$env
    )
    
    # First, ensure environment variable itself is set properly
    (Get-Content $EnvFile) -replace "^ENVIRONMENT=.*", "ENVIRONMENT=$env" | Set-Content $EnvFile
    
    Write-Host "Set ENVIRONMENT=$env in .env file"
    
    # Uncomment the environment-specific overrides section for the selected environment
    $content = Get-Content $EnvFile
    
    if ($env -eq "development" -or $env -eq "dev") {
        # Uncomment development overrides
        $inDevSection = $false
        $newContent = @()
        
        foreach ($line in $content) {
            if ($line -match "^# --- DEVELOPMENT-SPECIFIC OVERRIDES ---") {
                $inDevSection = $true
                $newContent += $line
            }
            elseif ($line -match "^# --- STAGING-SPECIFIC OVERRIDES ---") {
                $inDevSection = $false
                $newContent += $line
            }
            elseif ($inDevSection -and $line -match "^# DEVELOPMENT_") {
                $newContent += $line -replace "^# DEVELOPMENT_", "DEVELOPMENT_"
            }
            else {
                $newContent += $line
            }
        }
        
        $newContent | Set-Content $EnvFile
        Write-Host "Applied development environment overrides"
    }
    elseif ($env -eq "staging") {
        # Uncomment staging overrides
        $inStagingSection = $false
        $newContent = @()
        
        foreach ($line in $content) {
            if ($line -match "^# --- STAGING-SPECIFIC OVERRIDES ---") {
                $inStagingSection = $true
                $newContent += $line
            }
            elseif ($line -match "^# --- PRODUCTION-SPECIFIC OVERRIDES ---") {
                $inStagingSection = $false
                $newContent += $line
            }
            elseif ($inStagingSection -and $line -match "^# STAGING_") {
                $newContent += $line -replace "^# STAGING_", "STAGING_"
            }
            else {
                $newContent += $line
            }
        }
        
        $newContent | Set-Content $EnvFile
        Write-Host "Applied staging environment overrides"
    }
    elseif ($env -eq "production" -or $env -eq "prod") {
        # Uncomment production overrides
        $inProdSection = $false
        $newContent = @()
        
        foreach ($line in $content) {
            if ($line -match "^# --- PRODUCTION-SPECIFIC OVERRIDES ---") {
                $inProdSection = $true
                $newContent += $line
            }
            elseif ($inProdSection -and $line -match "^# PRODUCTION_") {
                $newContent += $line -replace "^# PRODUCTION_", "PRODUCTION_"
            }
            else {
                $newContent += $line
            }
        }
        
        $newContent | Set-Content $EnvFile
        Write-Host "Applied production environment overrides"
    }
    else {
        Write-Host "Unknown environment: $env" -ForegroundColor Red
        exit 1
    }
}

# Parse command-line arguments
param (
    [string]$Environment,
    [switch]$Help
)

# Show help if requested
if ($Help) {
    Print-Usage
    exit 0
}

# Set selected environment
$SelectedEnv = $Environment

# Validate input
if ([string]::IsNullOrEmpty($SelectedEnv)) {
    Write-Host "Error: No environment specified." -ForegroundColor Red
    Print-Usage
    exit 1
}

# Normalize environment names
if ($SelectedEnv -eq "dev") {
    $SelectedEnv = "development"
}
elseif ($SelectedEnv -eq "prod") {
    $SelectedEnv = "production"
}

# Check for valid environment
if ($SelectedEnv -notin @("development", "staging", "production")) {
    Write-Host "Error: Invalid environment. Must be one of: development, staging, production" -ForegroundColor Red
    exit 1
}

# Execute the environment switching
Write-Host "Switching to $SelectedEnv environment..."
Copy-Template
Apply-EnvironmentSettings $SelectedEnv

Write-Host "Environment successfully switched to $SelectedEnv" -ForegroundColor Green
Write-Host "To apply these changes, restart your containers with:" -ForegroundColor Yellow
Write-Host "  docker-compose down; docker-compose up -d" -ForegroundColor Yellow
Write-Host ""
Write-Host "For local development without Docker, load the environment variables in your shell." -ForegroundColor Yellow