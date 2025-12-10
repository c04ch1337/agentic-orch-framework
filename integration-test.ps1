# Phoenix AGI System - API Gateway Security Integration Test Script
# ==================================================================
# PowerShell script to run comprehensive security tests on Windows
# 
# This script:
# - Starts necessary services (API Gateway, Orchestrator)
# - Runs the Python security test suite
# - Generates a test report
# - Performs clean shutdown of services
#
# Usage: .\integration-test.ps1 [-SkipBuild] [-KeepRunning] [-TestOnly]
#

param(
    [switch]$SkipBuild = $false,      # Skip building services
    [switch]$KeepRunning = $false,    # Keep services running after tests
    [switch]$TestOnly = $false,        # Only run tests (assume services are already running)
    [string]$ConfigFile = ".env.dev"  # Environment configuration file
)

# Configuration
$ErrorActionPreference = "Stop"
$script:StartTime = Get-Date
$script:TestResults = @()
$script:ProcessesToCleanup = @()

# Color output functions
function Write-Header {
    param([string]$Message)
    Write-Host "`n$("="*60)" -ForegroundColor Cyan
    Write-Host $Message -ForegroundColor Cyan
    Write-Host "$("="*60)" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host "[OK] $Message" -ForegroundColor Green
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Yellow
}

# Check prerequisites
function Test-Prerequisites {
    Write-Header "Checking Prerequisites"
    
    $prereqsPassed = $true
    
    # Check Python
    Write-Info "Checking Python installation..."
    try {
        $pythonVersion = python --version 2>&1
        if ($pythonVersion -match "Python (\d+\.\d+)") {
            $version = [version]$Matches[1]
            if ($version -ge [version]"3.8") {
                Write-Success "Python $version found"
            }
            else {
                Write-Error "Python 3.8+ required, found $version"
                $prereqsPassed = $false
            }
        }
    }
    catch {
        Write-Error "Python not found. Please install Python 3.8+"
        $prereqsPassed = $false
    }
    
    # Check for Python requests module
    Write-Info "Checking Python 'requests' module..."
    try {
        python -c "import requests" 2>&1 | Out-Null
        Write-Success "Python 'requests' module found"
    }
    catch {
        Write-Error "Python 'requests' module not found. Installing..."
        pip install requests
    }
    
    # Check Rust/Cargo (if not skipping build)
    if (-not $SkipBuild) {
        Write-Info "Checking Rust/Cargo installation..."
        try {
            $cargoVersion = cargo --version 2>&1
            if ($cargoVersion) {
                Write-Success "Cargo found: $cargoVersion"
            }
        }
        catch {
            Write-Error "Cargo not found. Please install Rust"
            $prereqsPassed = $false
        }
    }
    
    # Check test files
    Write-Info "Checking test files..."
    $testFiles = @(
        "api-gateway-security-test.py",
        "config/test_api_keys.txt"
    )
    
    foreach ($file in $testFiles) {
        if (Test-Path $file) {
            Write-Success "Found $file"
        }
        else {
            Write-Error "Missing $file"
            $prereqsPassed = $false
        }
    }
    
    # Check environment configuration
    if (Test-Path $ConfigFile) {
        Write-Success "Environment configuration found: $ConfigFile"
        
        # Load environment variables
        Get-Content $ConfigFile | ForEach-Object {
            if ($_ -match '^([^#][^=]+)=(.*)$') {
                $key = $Matches[1].Trim()
                $value = $Matches[2].Trim()
                [Environment]::SetEnvironmentVariable($key, $value, "Process")
            }
        }
    }
    else {
        Write-Info "Environment file not found: $ConfigFile (using defaults)"
    }
    
    return $prereqsPassed
}

# Build services
function Build-Services {
    if ($SkipBuild) {
        Write-Info "Skipping build (using existing binaries)"
        return $true
    }
    
    Write-Header "Building Services"
    
    $services = @(
        @{Name = "API Gateway"; Path = "api-gateway-rs" },
        @{Name = "Orchestrator"; Path = "orchestrator-service-rs" }
    )
    
    foreach ($service in $services) {
        Write-Info "Building $($service.Name)..."
        Push-Location $service.Path
        try {
            $buildOutput = cargo build --release 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Success "$($service.Name) built successfully"
            }
            else {
                Write-Error "Failed to build $($service.Name): $buildOutput"
                Pop-Location
                return $false
            }
        }
        catch {
            Write-Error "Build error for $($service.Name): $_"
            Pop-Location
            return $false
        }
        Pop-Location
    }
    
    return $true
}

# Start a service
function Start-Service {
    param(
        [string]$Name,
        [string]$Path,
        [int]$Port,
        [hashtable]$EnvVars = @{}
    )
    
    Write-Info "Starting $Name on port $Port..."
    
    # Set environment variables for the process
    $processEnv = [System.Diagnostics.ProcessStartInfo]::new()
    $processEnv.FileName = $Path
    $processEnv.WorkingDirectory = Get-Location
    $processEnv.UseShellExecute = $false
    $processEnv.RedirectStandardOutput = $true
    $processEnv.RedirectStandardError = $true
    $processEnv.CreateNoWindow = $true
    
    # Add environment variables
    foreach ($key in $EnvVars.Keys) {
        $processEnv.EnvironmentVariables[$key] = $EnvVars[$key]
    }
    
    # Copy current environment variables
    foreach ($key in [Environment]::GetEnvironmentVariables("Process").Keys) {
        if (-not $processEnv.EnvironmentVariables.ContainsKey($key)) {
            $processEnv.EnvironmentVariables[$key] = [Environment]::GetEnvironmentVariable($key, "Process")
        }
    }
    
    try {
        $process = [System.Diagnostics.Process]::Start($processEnv)
        $script:ProcessesToCleanup += $process
        
        # Give service time to start
        Start-Sleep -Seconds 3
        
        # Check if process is still running
        if ($process.HasExited) {
            Write-Error "$Name failed to start (exited with code $($process.ExitCode))"
            return $false
        }
        
        # Verify service is responding
        $maxAttempts = 10
        $attempt = 0
        $serviceReady = $false
        
        while (($attempt -lt $maxAttempts) -and (-not $serviceReady)) {
            $attempt++
            try {
                $response = Invoke-WebRequest -Uri "http://localhost:$Port/health" -Method GET -TimeoutSec 2 -ErrorAction SilentlyContinue
                if ($response.StatusCode -eq 200) {
                    $serviceReady = $true
                    Write-Success "$Name is running (PID: $($process.Id))"
                }
            }
            catch {
                Write-Info "Waiting for $Name to be ready... (attempt $attempt/$maxAttempts)"
                Start-Sleep -Seconds 2
            }
        }
        
        if (-not $serviceReady) {
            Write-Error "$Name failed to respond on port $Port"
            return $false
        }
        
        return $true
        
    }
    catch {
        Write-Error "Failed to start $Name : $_"
        return $false
    }
}

# Start all required services
function Start-Services {
    Write-Header "Starting Services"
    
    # Update the API keys file path for testing
    $testKeysPath = Join-Path (Get-Location) "config/test_api_keys.txt"
    
    # Start Orchestrator Service first
    $orchestratorStarted = Start-Service `
        -Name "Orchestrator Service" `
        -Path "target\release\orchestrator-service-rs.exe" `
        -Port 50051
    
    if (-not $orchestratorStarted) {
        Write-Error "Failed to start Orchestrator Service"
        return $false
    }
    
    # Start API Gateway
    $apiGatewayStarted = Start-Service `
        -Name "API Gateway" `
        -Path "target\release\api-gateway-rs.exe" `
        -Port 8000 `
        -EnvVars @{
        "PHOENIX_API_KEYS_FILE" = $testKeysPath
        "TLS_ENABLED"           = "false"  # Disable TLS for testing
    }
    
    if (-not $apiGatewayStarted) {
        Write-Error "Failed to start API Gateway"
        return $false
    }
    
    Write-Success "All services started successfully"
    return $true
}

# Run security tests
function Run-SecurityTests {
    Write-Header "Running Security Tests"
    
    # Set environment variables for the test
    [Environment]::SetEnvironmentVariable("API_GATEWAY_HOST", "localhost", "Process")
    [Environment]::SetEnvironmentVariable("API_GATEWAY_PORT", "8000", "Process")
    [Environment]::SetEnvironmentVariable("TLS_ENABLED", "false", "Process")
    
    Write-Info "Starting security test suite..."
    
    try {
        $testOutput = python api-gateway-security-test.py 2>&1
        $testExitCode = $LASTEXITCODE
        
        # Display test output
        $testOutput | ForEach-Object {
            if ($_ -match "✓ PASS") {
                Write-Host $_ -ForegroundColor Green
            }
            elseif ($_ -match "✗ FAIL") {
                Write-Host $_ -ForegroundColor Red
            }
            else {
                Write-Host $_
            }
        }
        
        if ($testExitCode -eq 0) {
            Write-Success "Security tests completed successfully"
            return $true
        }
        else {
            Write-Error "Security tests failed with exit code $testExitCode"
            return $false
        }
        
    }
    catch {
        Write-Error "Failed to run security tests: $_"
        return $false
    }
}

# Generate test report
function Generate-TestReport {
    param([bool]$TestsPassed)
    
    Write-Header "Generating Test Report"
    
    $endTime = Get-Date
    $duration = $endTime - $script:StartTime
    
    $report = @{
        "timestamp"        = $endTime.ToString("yyyy-MM-dd HH:mm:ss")
        "duration_seconds" = [math]::Round($duration.TotalSeconds, 2)
        "tests_passed"     = $TestsPassed
        "environment"      = @{
            "os"                 = [System.Environment]::OSVersion.ToString()
            "powershell_version" = $PSVersionTable.PSVersion.ToString()
            "api_gateway_host"   = "localhost"
            "api_gateway_port"   = 8000
            "tls_enabled"        = $false
        }
    }
    
    $reportFile = "integration-test-report.json"
    $report | ConvertTo-Json -Depth 10 | Out-File $reportFile -Encoding UTF8
    
    Write-Success "Test report saved to $reportFile"
    
    # Also check for the detailed Python test report
    if (Test-Path "api-gateway-security-test-report.json") {
        Write-Success "Detailed security test report available: api-gateway-security-test-report.json"
    }
}

# Cleanup services
function Stop-Services {
    if ($KeepRunning) {
        Write-Info "Services kept running (use -KeepRunning:$false to stop)"
        Write-Info "Running services:"
        foreach ($process in $script:ProcessesToCleanup) {
            if (-not $process.HasExited) {
                Write-Info "  - PID $($process.Id): $($process.ProcessName)"
            }
        }
        return
    }
    
    Write-Header "Stopping Services"
    
    foreach ($process in $script:ProcessesToCleanup) {
        if (-not $process.HasExited) {
            Write-Info "Stopping process $($process.Id)..."
            try {
                $process.Kill()
                $process.WaitForExit(5000)
                Write-Success "Process $($process.Id) stopped"
            }
            catch {
                Write-Error "Failed to stop process $($process.Id): $_"
            }
        }
    }
    
    Write-Success "All services stopped"
}

# Main execution
function Main {
    Write-Header "Phoenix AGI System - Security Integration Test"
    Write-Host "Start Time: $($script:StartTime.ToString('yyyy-MM-dd HH:mm:ss'))"
    
    # Check prerequisites
    if (-not (Test-Prerequisites)) {
        Write-Error "Prerequisites check failed"
        exit 1
    }
    
    $testsPassed = $false
    
    try {
        if (-not $TestOnly) {
            # Build services
            if (-not (Build-Services)) {
                Write-Error "Build failed"
                exit 1
            }
            
            # Start services
            if (-not (Start-Services)) {
                Write-Error "Failed to start services"
                exit 1
            }
        }
        else {
            Write-Info "Running tests only (assuming services are already running)"
        }
        
        # Run security tests
        $testsPassed = Run-SecurityTests
        
        # Generate report
        Generate-TestReport -TestsPassed $testsPassed
        
    }
    finally {
        if (-not $TestOnly) {
            # Cleanup
            Stop-Services
        }
    }
    
    # Final summary
    Write-Header "Test Summary"
    
    $endTime = Get-Date
    $duration = $endTime - $script:StartTime
    
    Write-Host "End Time: $($endTime.ToString('yyyy-MM-dd HH:mm:ss'))"
    Write-Host "Duration: $([math]::Round($duration.TotalSeconds, 2)) seconds"
    
    if ($testsPassed) {
        Write-Success "All security tests passed!"
        exit 0
    }
    else {
        Write-Error "Some tests failed. Check reports for details."
        exit 1
    }
}

# Run main function
Main