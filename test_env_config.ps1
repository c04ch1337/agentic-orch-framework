# Test script for the new environment configuration approach in PowerShell
# This script validates that the environment switcher works as expected

# Colors for output
$Green = @{ForegroundColor = "Green" }
$Yellow = @{ForegroundColor = "Yellow" }
$Red = @{ForegroundColor = "Red" }

# Helper function for output formatting
function Print-Header($message) {
    Write-Host "`n==== $message ====" @Yellow
}

function Print-Success($message) {
    Write-Host "✓ $message" @Green
}

function Print-Error($message) {
    Write-Host "✗ $message" @Red
    Script:FailedTests++
}

function Check-FileExists($path) {
    if (Test-Path $path) {
        Print-Success "File $path exists"
        return $true
    }
    else {
        Print-Error "File $path does not exist"
        return $false
    }
}

function Check-VariableInEnv($file, $varName, $expectedValue) {
    $content = Get-Content $file
    $line = $content | Where-Object { $_ -match "^$varName=" }
    
    if ($line) {
        $actualValue = $line -replace "^$varName=", ""
        if ($actualValue -eq $expectedValue) {
            Print-Success "Variable $varName is correctly set to '$expectedValue'"
            return $true
        }
        else {
            Print-Error "Variable $varName should be '$expectedValue' but is '$actualValue'"
            return $false
        }
    }
    else {
        Print-Error "Variable $varName not found in $file"
        return $false
    }
}

$script:FailedTests = 0

# --- Validate Files Exist ---
Print-Header "Validating files exist"

$requiredFiles = @(
    ".env.example.consolidated",
    "env_switcher.sh",
    "env_switcher.ps1",
    "docs/environment-configuration-guide.md"
)

foreach ($file in $requiredFiles) {
    Check-FileExists $file
}

# --- Test Development Environment ---
Print-Header "Testing Development Environment Setup"

# Switch to development environment
& .\env_switcher.ps1 -Environment development

# Verify .env file was created
if (-not (Check-FileExists ".env")) {
    Write-Host "Skipping environment variable tests since .env wasn't created"
}
else {
    # Verify ENVIRONMENT variable
    Check-VariableInEnv ".env" "ENVIRONMENT" "development"
    
    # Check development-specific values were uncommented
    $envContent = Get-Content ".env"
    $developmentVars = $envContent | Where-Object { $_ -match "^DEVELOPMENT_" }
    
    if ($developmentVars) {
        Print-Success "Development-specific variables were uncommented"
    }
    else {
        Print-Error "No development-specific variables were uncommented"
    }
}

# --- Test Staging Environment ---
Print-Header "Testing Staging Environment Setup"

# Switch to staging environment
& .\env_switcher.ps1 -Environment staging

# Verify .env file was created
if (-not (Check-FileExists ".env")) {
    Write-Host "Skipping environment variable tests since .env wasn't created"
}
else {
    # Verify ENVIRONMENT variable
    Check-VariableInEnv ".env" "ENVIRONMENT" "staging"
    
    # Check staging-specific values were uncommented
    $envContent = Get-Content ".env"
    $stagingVars = $envContent | Where-Object { $_ -match "^STAGING_" }
    
    if ($stagingVars) {
        Print-Success "Staging-specific variables were uncommented"
    }
    else {
        Print-Error "No staging-specific variables were uncommented"
    }
}

# --- Test Production Environment ---
Print-Header "Testing Production Environment Setup"

# Switch to production environment
& .\env_switcher.ps1 -Environment production

# Verify .env file was created
if (-not (Check-FileExists ".env")) {
    Write-Host "Skipping environment variable tests since .env wasn't created"
}
else {
    # Verify ENVIRONMENT variable
    Check-VariableInEnv ".env" "ENVIRONMENT" "production"
    
    # Check production-specific values were uncommented
    $envContent = Get-Content ".env"
    $productionVars = $envContent | Where-Object { $_ -match "^PRODUCTION_" }
    
    if ($productionVars) {
        Print-Success "Production-specific variables were uncommented"
    }
    else {
        Print-Error "No production-specific variables were uncommented"
    }
}

# --- Test Docker Compose Integration ---
Print-Header "Testing Docker Compose Integration"

# Test if docker-compose can validate the file with the new .env
if (Get-Command "docker-compose" -ErrorAction SilentlyContinue) {
    $result = docker-compose config 2>&1
    if ($LASTEXITCODE -eq 0) {
        Print-Success "Docker Compose configuration is valid with the new .env"
    }
    else {
        Print-Error "Docker Compose configuration has errors with the new .env file"
        Write-Host $result
    }
}
else {
    Write-Host "Docker Compose not installed, skipping Docker Compose validation test"
}

# --- Test Summary ---
Print-Header "Test Summary"

if ($script:FailedTests -eq 0) {
    Write-Host "All tests passed successfully!" @Green
    Write-Host "The new environment configuration approach is working as expected."
}
else {
    Write-Host "$script:FailedTests test(s) failed." @Red
    Write-Host "Please review the test output to fix any issues."
}

# Return to development environment for continued development
Print-Header "Resetting to development environment for continued development"
& .\env_switcher.ps1 -Environment development

exit $script:FailedTests