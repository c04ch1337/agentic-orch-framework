# Test script for the new environment configuration approach in PowerShell
# This script validates that the environment switcher works as expected

# Colors for output
$Green = @{ForegroundColor = "Green" }
$Yellow = @{ForegroundColor = "Yellow" }
$Red = @{ForegroundColor = "Red" }

# Helper function for output formatting
function Write-Header($message) {
    Write-Host "`n==== $message ====" @Yellow
}

function Write-Success($message) {
    Write-Host "✓ $message" @Green
}

function Write-TestError($message) {
    Write-Host "✗ $message" @Red
    Script:FailedTests++
}

function Test-FileExists($path) {
    if (Test-Path $path) {
        Write-Success "File $path exists"
        return $true
    }
    else {
        Write-TestError "File $path does not exist"
        return $false
    }
}

function Test-VariableInEnv($file, $varName, $expectedValue) {
    $content = Get-Content $file
    $line = $content | Where-Object { $_ -match "^$varName=" }
    
    if ($line) {
        $actualValue = $line -replace "^$varName=", ""
        if ($actualValue -eq $expectedValue) {
            Write-Success "Variable $varName is correctly set to '$expectedValue'"
            return $true
        }
        else {
            Write-TestError "Variable $varName should be '$expectedValue' but is '$actualValue'"
            return $false
        }
    }
    else {
        Write-TestError "Variable $varName not found in $file"
        return $false
    }
}

$script:FailedTests = 0

# --- Validate Files Exist ---
Write-Header "Validating files exist"

$requiredFiles = @(
    ".env.example.consolidated",
    "env_switcher.sh",
    "env_switcher.ps1",
    "docs/environment-configuration-guide.md"
)

foreach ($file in $requiredFiles) {
    Test-FileExists $file
}

# --- Test Development Environment ---
Write-Header "Testing Development Environment Setup"

# Switch to development environment
& .\env_switcher.ps1 -Environment development

# Verify .env file was created
if (-not (Test-FileExists ".env")) {
    Write-Host "Skipping environment variable tests since .env wasn't created"
}
else {
    # Verify ENVIRONMENT variable
    Test-VariableInEnv ".env" "ENVIRONMENT" "development"
    
    # Check development-specific values were uncommented
    $envContent = Get-Content ".env"
    $developmentVars = $envContent | Where-Object { $_ -match "^DEVELOPMENT_" }
    
    if ($developmentVars) {
        Write-Success "Development-specific variables were uncommented"
    }
    else {
        Write-TestError "No development-specific variables were uncommented"
    }
}

# --- Test Staging Environment ---
Write-Header "Testing Staging Environment Setup"

# Switch to staging environment
& .\env_switcher.ps1 -Environment staging

# Verify .env file was created
if (-not (Test-FileExists ".env")) {
    Write-Host "Skipping environment variable tests since .env wasn't created"
}
else {
    # Verify ENVIRONMENT variable
    Test-VariableInEnv ".env" "ENVIRONMENT" "staging"
    
    # Check staging-specific values were uncommented
    $envContent = Get-Content ".env"
    $stagingVars = $envContent | Where-Object { $_ -match "^STAGING_" }
    
    if ($stagingVars) {
        Write-Success "Staging-specific variables were uncommented"
    }
    else {
        Write-TestError "No staging-specific variables were uncommented"
    }
}

# --- Test Production Environment ---
Write-Header "Testing Production Environment Setup"

# Switch to production environment
& .\env_switcher.ps1 -Environment production

# Verify .env file was created
if (-not (Test-FileExists ".env")) {
    Write-Host "Skipping environment variable tests since .env wasn't created"
}
else {
    # Verify ENVIRONMENT variable
    Test-VariableInEnv ".env" "ENVIRONMENT" "production"
    
    # Check production-specific values were uncommented
    $envContent = Get-Content ".env"
    $productionVars = $envContent | Where-Object { $_ -match "^PRODUCTION_" }
    
    if ($productionVars) {
        Write-Success "Production-specific variables were uncommented"
    }
    else {
        Write-TestError "No production-specific variables were uncommented"
    }
}

# --- Test Docker Compose Integration ---
Write-Header "Testing Docker Compose Integration"

# Test if docker-compose can validate the file with the new .env
if (Get-Command "docker-compose" -ErrorAction SilentlyContinue) {
    $result = docker-compose config 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Docker Compose configuration is valid with the new .env"
    }
    else {
        Write-TestError "Docker Compose configuration has errors with the new .env file"
        Write-Host $result
    }
}
else {
    Write-Host "Docker Compose not installed, skipping Docker Compose validation test"
}

# --- Test Summary ---
Write-Header "Test Summary"

if ($script:FailedTests -eq 0) {
    Write-Host "All tests passed successfully!" @Green
    Write-Host "The new environment configuration approach is working as expected."
}
else {
    Write-Host "$script:FailedTests test(s) failed." @Red
    Write-Host "Please review the test output to fix any issues."
}

# Return to development environment for continued development
Write-Header "Resetting to development environment for continued development"
& .\env_switcher.ps1 -Environment development

exit $script:FailedTests
