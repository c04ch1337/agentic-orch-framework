# PHOENIX ORCH: The Ashen Guard Edition AGI
# Executor-RS Deployment & Operations Guide

## Table of Contents
1. [System Requirements](#system-requirements)
2. [Pre-Installation Checklist](#pre-installation-checklist)
3. [Installation Steps](#installation-steps)
4. [Configuration](#configuration)
5. [Security Considerations](#security-considerations)
6. [Monitoring & Logging](#monitoring--logging)
7. [Operations](#operations)
8. [Troubleshooting](#troubleshooting)
9. [Maintenance](#maintenance)
10. [Emergency Procedures](#emergency-procedures)

---

## System Requirements

### Minimum Requirements

| Component | Specification |
|-----------|--------------|
| **Operating System** | Windows 10 (1809+) / Windows Server 2016+ |
| **Architecture** | x64 (64-bit) |
| **Memory** | 2 GB RAM minimum (1 GB available) |
| **Storage** | 500 MB free disk space |
| **CPU** | 2+ cores recommended |
| **Network** | Port 50055 available |
| **.NET Runtime** | Visual C++ Redistributables 2019+ |
| **PowerShell** | Version 5.1+ |

### Recommended Requirements

| Component | Specification |
|-----------|--------------|
| **Operating System** | Windows 11 / Windows Server 2022 |
| **Memory** | 4 GB RAM |
| **Storage** | 2 GB free disk space |
| **CPU** | 4+ cores |
| **Network** | Dedicated network interface |

### Software Dependencies

```powershell
# Check Windows version
winver

# Check PowerShell version
$PSVersionTable.PSVersion

# Check Visual C++ Redistributables
Get-ItemProperty HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\* | 
    Where-Object {$_.DisplayName -like "*Visual C++*"} | 
    Select-Object DisplayName, DisplayVersion
```

---

## Pre-Installation Checklist

### ✅ System Validation

```powershell
# 1. Verify Windows version
[System.Environment]::OSVersion.Version

# 2. Check available memory
Get-CimInstance Win32_OperatingSystem | 
    Select-Object TotalVisibleMemorySize, FreePhysicalMemory

# 3. Check disk space
Get-PSDrive C | Select-Object Used, Free

# 4. Verify port availability
Test-NetConnection -ComputerName localhost -Port 50055

# 5. Check user permissions
whoami /priv
```

### ✅ Required Components

- [ ] Protocol Buffers Compiler (protoc) v25.1+
- [ ] Rust toolchain 1.75+ (for building from source)
- [ ] Git (for source code management)
- [ ] Visual Studio Build Tools 2019+ (for native compilation)

### ✅ Network Requirements

- [ ] Port 50055 open for gRPC communication
- [ ] Firewall rules configured for executor-rs.exe
- [ ] Network service account (if running as service)

---

## Installation Steps

### Option 1: Binary Installation (Recommended)

```powershell
# 1. Create application directory
New-Item -ItemType Directory -Path "C:\Program Files\PhoenixOrch\executor-rs" -Force

# 2. Download the latest release binary
# (Replace URL with actual release URL)
Invoke-WebRequest -Uri "https://github.com/phoenix-orch/releases/executor-rs.exe" `
                  -OutFile "C:\Program Files\PhoenixOrch\executor-rs\executor-rs.exe"

# 3. Create sandbox directory
New-Item -ItemType Directory -Path "C:\phoenix_sandbox" -Force

# 4. Set directory permissions (restrict access)
$acl = Get-Acl "C:\phoenix_sandbox"
$acl.SetAccessRuleProtection($true, $false)
$permission = "BUILTIN\Users", "ReadAndExecute,Write", "ContainerInherit,ObjectInherit", "None", "Allow"
$accessRule = New-Object System.Security.AccessControl.FileSystemAccessRule $permission
$acl.SetAccessRule($accessRule)
Set-Acl "C:\phoenix_sandbox" $acl

# 5. Download Protocol Buffers compiler
Invoke-WebRequest -Uri "https://github.com/protocolbuffers/protobuf/releases/download/v25.1/protoc-25.1-win64.zip" `
                  -OutFile "$env:TEMP\protoc.zip"
Expand-Archive -Path "$env:TEMP\protoc.zip" -DestinationPath "C:\Program Files\PhoenixOrch\protoc"

# 6. Set environment variables
[Environment]::SetEnvironmentVariable("PROTOC", "C:\Program Files\PhoenixOrch\protoc\bin\protoc.exe", "Machine")
[Environment]::SetEnvironmentVariable("EXECUTOR_ADDR", "0.0.0.0:50055", "Machine")
```

### Option 2: Building from Source

```powershell
# 1. Install Rust
Invoke-WebRequest -Uri "https://win.rustup.rs" -OutFile "$env:TEMP\rustup-init.exe"
& "$env:TEMP\rustup-init.exe" -y
refreshenv

# 2. Clone repository
git clone https://github.com/phoenix-orch/system-build-rs.git
cd system-build-rs

# 3. Install protoc
# (Same as step 5 in Option 1)

# 4. Build the executor
cd executor-rs
cargo build --release

# 5. Copy binary to installation directory
Copy-Item "target\release\executor-rs.exe" `
          "C:\Program Files\PhoenixOrch\executor-rs\" -Force
```

### Windows Service Installation

```powershell
# Create Windows Service
New-Service -Name "PhoenixOrchExecutor" `
            -BinaryPathName "C:\Program Files\PhoenixOrch\executor-rs\executor-rs.exe" `
            -DisplayName "Phoenix Orch Executor Service" `
            -Description "Windows native execution service for Phoenix Orch AGI" `
            -StartupType Automatic

# Configure service recovery
sc.exe failure PhoenixOrchExecutor reset= 86400 actions= restart/60000/restart/60000/restart/60000

# Set service account (optional - for enhanced security)
$credential = Get-Credential -Message "Enter service account credentials"
Set-Service -Name "PhoenixOrchExecutor" -Credential $credential

# Start the service
Start-Service -Name "PhoenixOrchExecutor"
```

---

## Configuration

### Environment Variables

```powershell
# Core Configuration
[Environment]::SetEnvironmentVariable("EXECUTOR_ADDR", "0.0.0.0:50055", "Machine")
[Environment]::SetEnvironmentVariable("RUST_LOG", "info", "Machine")
[Environment]::SetEnvironmentVariable("PHOENIX_SANDBOX", "C:\phoenix_sandbox", "Machine")

# Resource Limits (if configurable in future versions)
[Environment]::SetEnvironmentVariable("MAX_PROCESS_MEMORY_MB", "100", "Machine")
[Environment]::SetEnvironmentVariable("MAX_JOB_MEMORY_MB", "500", "Machine")
[Environment]::SetEnvironmentVariable("MAX_PROCESSES", "5", "Machine")
[Environment]::SetEnvironmentVariable("EXECUTION_TIMEOUT_S", "30", "Machine")

# Security Settings
[Environment]::SetEnvironmentVariable("ENABLE_LOW_INTEGRITY", "true", "Machine")
[Environment]::SetEnvironmentVariable("ENABLE_NETWORK_ISOLATION", "false", "Machine")
```

### Configuration File (Future)

Create `C:\Program Files\PhoenixOrch\executor-rs\config.toml`:

```toml
[service]
address = "0.0.0.0:50055"
max_connections = 100
timeout_ms = 30000

[sandbox]
directory = "C:\\phoenix_sandbox"
cleanup_on_exit = true
max_file_size_mb = 50

[resources]
max_process_memory_mb = 100
max_job_memory_mb = 500
max_processes = 5
cpu_rate_limit = 5000  # 50% of one core

[security]
enable_low_integrity = true
enable_network_isolation = false
allowed_commands = [
    "python", "python3", "pip", "pip3",
    "cmd", "powershell",
    "dir", "ls", "cat", "type", "echo",
    "grep", "find", "findstr"
]

[logging]
level = "info"
file = "C:\\Program Files\\PhoenixOrch\\executor-rs\\logs\\executor.log"
max_size_mb = 100
max_backups = 10
```

---

## Security Considerations

### 1. Service Account Configuration

```powershell
# Create dedicated service account
New-LocalUser -Name "PhoenixExecutor" `
              -Description "Service account for Phoenix Executor" `
              -NoPassword

# Add to specific groups
Add-LocalGroupMember -Group "Event Log Readers" -Member "PhoenixExecutor"

# Assign specific privileges
$tempPath = "$env:TEMP\user_rights.inf"
secedit /export /cfg $tempPath
# Edit the file to add SeAssignPrimaryTokenPrivilege
# Then import:
secedit /configure /db secedit.sdb /cfg $tempPath /quiet
```

### 2. Firewall Configuration

```powershell
# Create inbound rule for gRPC port
New-NetFirewallRule -DisplayName "Phoenix Executor gRPC" `
                    -Direction Inbound `
                    -Protocol TCP `
                    -LocalPort 50055 `
                    -Action Allow `
                    -Program "C:\Program Files\PhoenixOrch\executor-rs\executor-rs.exe"

# Restrict to local network only (optional)
Set-NetFirewallRule -DisplayName "Phoenix Executor gRPC" `
                    -RemoteAddress "192.168.0.0/16,10.0.0.0/8,172.16.0.0/12,127.0.0.1"
```

### 3. Sandbox Permissions

```powershell
# Set restrictive permissions on sandbox
$sandbox = "C:\phoenix_sandbox"
$acl = Get-Acl $sandbox

# Remove inheritance
$acl.SetAccessRuleProtection($true, $false)

# Clear existing permissions
$acl.Access | ForEach-Object { $acl.RemoveAccessRule($_) }

# Add specific permissions
$rules = @(
    @("SYSTEM", "FullControl"),
    @("Administrators", "FullControl"),
    @("PhoenixExecutor", "Modify"),
    @("Users", "ReadAndExecute")
)

foreach ($rule in $rules) {
    $permission = $rule[0], $rule[1], "ContainerInherit,ObjectInherit", "None", "Allow"
    $accessRule = New-Object System.Security.AccessControl.FileSystemAccessRule $permission
    $acl.SetAccessRule($accessRule)
}

Set-Acl $sandbox $acl
```

### 4. AppLocker Policy (Optional)

```xml
<!-- Save as executor-applocker.xml -->
<AppLockerPolicy Version="1">
  <RuleCollection Type="Exe" EnforcementMode="Enabled">
    <FilePathRule Id="executor-sandbox" Name="Phoenix Sandbox Executables"
                  Description="Allow execution in sandbox"
                  UserOrGroupSid="S-1-1-0"
                  Action="Allow">
      <Conditions>
        <FilePathCondition Path="C:\phoenix_sandbox\*" />
      </Conditions>
    </FilePathRule>
  </RuleCollection>
</AppLockerPolicy>
```

```powershell
# Import AppLocker policy
Set-AppLockerPolicy -XmlPolicy executor-applocker.xml
```

---

## Monitoring & Logging

### 1. Enable Windows Event Logging

```powershell
# Create custom event log
New-EventLog -LogName "PhoenixOrch" -Source "ExecutorService"

# Write startup event
Write-EventLog -LogName "PhoenixOrch" `
               -Source "ExecutorService" `
               -EntryType Information `
               -EventId 1000 `
               -Message "Executor service started successfully"
```

### 2. Performance Monitoring

```powershell
# Create performance counter set
$counters = @(
    "\Process(executor-rs)\% Processor Time",
    "\Process(executor-rs)\Working Set",
    "\Process(executor-rs)\Handle Count",
    "\Job Object(PhoenixExecutor)\Current Processes",
    "\.NET CLR Memory(executor-rs)\# Bytes in all Heaps"
)

# Start data collector
$datacollector = New-Object System.Diagnostics.PerformanceCounter
logman create counter PhoenixExecutor -c $counters -f csv -o "C:\Logs\executor-perf.csv"
logman start PhoenixExecutor
```

### 3. Log Rotation

```powershell
# PowerShell script for log rotation (schedule daily)
$logPath = "C:\Program Files\PhoenixOrch\executor-rs\logs"
$maxAge = 30  # days

Get-ChildItem -Path $logPath -Filter "*.log" | 
    Where-Object { $_.LastWriteTime -lt (Get-Date).AddDays(-$maxAge) } | 
    Remove-Item -Force

# Compress old logs
Get-ChildItem -Path $logPath -Filter "*.log" | 
    Where-Object { $_.LastWriteTime -lt (Get-Date).AddDays(-7) } | 
    ForEach-Object {
        Compress-Archive -Path $_.FullName -DestinationPath "$($_.FullName).zip"
        Remove-Item $_.FullName
    }
```

### 4. Health Monitoring

```powershell
# Health check script
function Test-ExecutorHealth {
    param (
        [string]$Address = "localhost:50055"
    )
    
    try {
        # Test port connectivity
        $tcp = Test-NetConnection -ComputerName ($Address -split ':')[0] `
                                  -Port ($Address -split ':')[1]
        
        if (-not $tcp.TcpTestSucceeded) {
            throw "Port not accessible"
        }
        
        # Check service status
        $service = Get-Service -Name "PhoenixOrchExecutor" -ErrorAction Stop
        if ($service.Status -ne "Running") {
            throw "Service not running"
        }
        
        # Check sandbox directory
        if (-not (Test-Path "C:\phoenix_sandbox")) {
            throw "Sandbox directory missing"
        }
        
        return @{
            Status = "Healthy"
            Service = $service.Status
            Port = "Open"
            Sandbox = "Available"
        }
    }
    catch {
        return @{
            Status = "Unhealthy"
            Error = $_.Exception.Message
        }
    }
}

# Schedule health check every 5 minutes
$action = New-ScheduledTaskAction -Execute "PowerShell.exe" `
          -Argument "-File C:\Scripts\executor-health.ps1"
$trigger = New-ScheduledTaskTrigger -Once -At (Get-Date) `
           -RepetitionInterval (New-TimeSpan -Minutes 5)
Register-ScheduledTask -TaskName "ExecutorHealthCheck" `
                       -Action $action -Trigger $trigger
```

---

## Operations

### Starting the Service

```powershell
# Manual start
Start-Service -Name "PhoenixOrchExecutor"

# Start with specific configuration
& "C:\Program Files\PhoenixOrch\executor-rs\executor-rs.exe" `
    --address "0.0.0.0:50055" `
    --sandbox "C:\phoenix_sandbox" `
    --log-level "info"
```

### Stopping the Service

```powershell
# Graceful stop
Stop-Service -Name "PhoenixOrchExecutor" -Force

# Emergency stop (kills all child processes)
Get-Process -Name "executor-rs" | Stop-Process -Force
```

### Status Verification

```powershell
# Check service status
Get-Service -Name "PhoenixOrchExecutor" | 
    Select-Object Name, Status, StartType, @{
        Name='Uptime'
        Expression={
            if ($_.Status -eq 'Running') {
                (Get-Date) - (Get-Process -Id $_.ProcessId).StartTime
            }
        }
    }

# Check active Job Objects
Get-CimInstance Win32_Process | 
    Where-Object { $_.ParentProcessId -eq (Get-Process executor-rs).Id } |
    Select-Object ProcessId, Name, WorkingSetSize, CreationDate
```

### Backup & Recovery

```powershell
# Backup configuration
$backupPath = "C:\Backups\PhoenixOrch\$(Get-Date -Format 'yyyyMMdd')"
New-Item -ItemType Directory -Path $backupPath -Force

Copy-Item "C:\Program Files\PhoenixOrch\executor-rs\*" `
          -Destination $backupPath -Recurse

# Backup registry settings
reg export "HKLM\SYSTEM\CurrentControlSet\Services\PhoenixOrchExecutor" `
           "$backupPath\service-config.reg"

# Recovery procedure
function Restore-ExecutorService {
    param([string]$BackupDate)
    
    $backupPath = "C:\Backups\PhoenixOrch\$BackupDate"
    
    Stop-Service -Name "PhoenixOrchExecutor" -Force
    Copy-Item "$backupPath\*" -Destination "C:\Program Files\PhoenixOrch\executor-rs\" -Force
    reg import "$backupPath\service-config.reg"
    Start-Service -Name "PhoenixOrchExecutor"
}
```

---

## Troubleshooting

### Common Issues & Solutions

#### Issue: Service fails to start

```powershell
# Check event logs
Get-EventLog -LogName System -Source "Service Control Manager" -Newest 10 |
    Where-Object { $_.Message -like "*PhoenixOrchExecutor*" }

# Verify dependencies
Test-Path "C:\phoenix_sandbox"
Test-Path "$env:PROTOC"
Test-NetConnection -ComputerName localhost -Port 50055

# Run in console mode for debugging
& "C:\Program Files\PhoenixOrch\executor-rs\executor-rs.exe" --debug
```

#### Issue: "Access Denied" errors

```powershell
# Check sandbox permissions
Get-Acl "C:\phoenix_sandbox" | Format-List

# Verify service account permissions
whoami /priv
icacls "C:\phoenix_sandbox"

# Reset sandbox permissions
takeown /f "C:\phoenix_sandbox" /r /d y
icacls "C:\phoenix_sandbox" /reset /t
```

#### Issue: High memory usage

```powershell
# Check Job Object statistics
$jobStats = Get-WmiObject Win32_Process | 
    Where-Object { $_.ParentProcessId -eq (Get-Process executor-rs).Id }
$jobStats | Measure-Object WorkingSetSize -Sum

# Identify memory-consuming processes
$jobStats | Sort-Object WorkingSetSize -Descending | 
    Select-Object -First 5 ProcessId, Name, 
    @{Name='Memory(MB)';Expression={[math]::Round($_.WorkingSetSize/1MB,2)}}

# Force cleanup
Restart-Service -Name "PhoenixOrchExecutor"
```

#### Issue: Process timeout errors

```powershell
# Check system resources
Get-Counter "\Processor(_Total)\% Processor Time" -SampleInterval 1 -MaxSamples 10
Get-Counter "\Memory\Available MBytes"

# Increase timeout (requires config change)
[Environment]::SetEnvironmentVariable("EXECUTION_TIMEOUT_S", "60", "Machine")
Restart-Service -Name "PhoenixOrchExecutor"
```

### Debug Commands

```powershell
# Enable verbose logging
[Environment]::SetEnvironmentVariable("RUST_LOG", "debug", "Process")
& "C:\Program Files\PhoenixOrch\executor-rs\executor-rs.exe"

# Test gRPC connectivity
grpcurl -plaintext localhost:50055 list

# Monitor file handles
handle.exe -p executor-rs

# Check network connections
netstat -an | findstr :50055
```

### Log Analysis

```powershell
# Search for errors
Select-String -Path "C:\Logs\executor.log" -Pattern "ERROR|CRITICAL" -Context 2,2

# Parse execution statistics
$log = Get-Content "C:\Logs\executor.log" | ConvertFrom-Json
$log | Where-Object { $_.level -eq "ERROR" } | 
    Group-Object message | 
    Sort-Object Count -Descending

# Generate report
$report = @{
    TotalExecutions = ($log | Where-Object { $_.message -like "*Executing*" }).Count
    Failures = ($log | Where-Object { $_.level -eq "ERROR" }).Count
    AverageExecutionTime = ($log | Where-Object { $_.execution_time } | 
                            Measure-Object execution_time -Average).Average
}
$report | ConvertTo-Json | Out-File "C:\Reports\executor-daily.json"
```

---

## Maintenance

### Daily Tasks

```powershell
# Daily maintenance script
$date = Get-Date -Format "yyyy-MM-dd"

# 1. Check service health
$health = Get-Service "PhoenixOrchExecutor"
if ($health.Status -ne "Running") {
    Send-MailMessage -To "admin@phoenix.local" `
                      -Subject "Executor Service Down" `
                      -Body "Service status: $($health.Status)"
}

# 2. Clean sandbox
Get-ChildItem "C:\phoenix_sandbox" -Recurse | 
    Where-Object { $_.LastWriteTime -lt (Get-Date).AddHours(-24) } |
    Remove-Item -Force -Recurse

# 3. Rotate logs
Compress-Archive -Path "C:\Logs\executor.log" `
                 -DestinationPath "C:\Logs\Archive\executor-$date.zip"
Clear-Content "C:\Logs\executor.log"
```

### Weekly Tasks

```powershell
# Weekly maintenance
# 1. Update Windows Defender exclusions
Add-MpPreference -ExclusionPath "C:\phoenix_sandbox"
Add-MpPreference -ExclusionProcess "executor-rs.exe"

# 2. Analyze performance trends
$perfData = Import-Csv "C:\Logs\executor-perf.csv"
$perfData | Group-Object { [datetime]$_.Timestamp.Date } | 
    ForEach-Object {
        [PSCustomObject]@{
            Date = $_.Name
            AvgCPU = ($_.Group | Measure-Object CPU -Average).Average
            AvgMemory = ($_.Group | Measure-Object Memory -Average).Average
        }
    }

# 3. Check for updates
# (Implement version check against repository)
```

### Monthly Tasks

```powershell
# 1. Full system audit
$auditReport = @{
    ServiceUptime = (Get-Service "PhoenixOrchExecutor" | 
                    Get-Process).StartTime
    TotalExecutions = (Get-EventLog -LogName "PhoenixOrch" | 
                      Where-Object { $_.EventID -eq 2000 }).Count
    SecurityEvents = (Get-EventLog -LogName Security | 
                     Where-Object { $_.Source -like "*Executor*" }).Count
}

# 2. Capacity planning
$capacityMetrics = @{
    PeakMemory = (Get-Counter "\Process(executor-rs)\Working Set - Private" -MaxSamples 30000 | 
                 Measure-Object -Maximum).Maximum
    PeakProcesses = (Get-Counter "\Job Object(PhoenixExecutor)\Current Processes" -MaxSamples 30000 | 
                    Measure-Object -Maximum).Maximum
}

# 3. Security review
Get-LocalUser | Where-Object { $_.Name -like "*Phoenix*" } | 
    Select-Object Name, Enabled, PasswordLastSet, LastLogon
```

---

## Emergency Procedures

### Service Unresponsive

```powershell
# Emergency restart procedure
function Restart-ExecutorEmergency {
    Write-Host "EMERGENCY: Forcing executor restart" -ForegroundColor Red
    
    # 1. Kill all processes
    Get-Process -Name "executor-rs" -ErrorAction SilentlyContinue | 
        Stop-Process -Force
    
    # 2. Clean up Job Objects
    Get-CimInstance Win32_Process | 
        Where-Object { $_.Name -like "*phoenix*" } | 
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
    
    # 3. Clear sandbox
    Remove-Item "C:\phoenix_sandbox\*" -Recurse -Force -ErrorAction SilentlyContinue
    
    # 4. Restart service
    Start-Service "PhoenixOrchExecutor"
    
    # 5. Alert administrators
    Send-MailMessage -To "admin@phoenix.local" `
                      -Subject "EMERGENCY: Executor Restarted" `
                      -Priority High
}
```

### Security Breach

```powershell
# Security incident response
function Invoke-SecurityResponse {
    Write-Host "SECURITY ALERT: Initiating lockdown" -ForegroundColor Red
    
    # 1. Stop service immediately
    Stop-Service "PhoenixOrchExecutor" -Force
    
    # 2. Preserve evidence
    $incidentPath = "C:\Incidents\$(Get-Date -Format 'yyyyMMdd-HHmmss')"
    New-Item -ItemType Directory -Path $incidentPath
    
    # Copy sandbox contents
    Copy-Item "C:\phoenix_sandbox" -Destination "$incidentPath\sandbox" -Recurse
    
    # Export process list
    Get-Process | Export-Csv "$incidentPath\processes.csv"
    
    # Export network connections
    netstat -anob > "$incidentPath\network.txt"
    
    # 3. Block network access
    New-NetFirewallRule -DisplayName "EMERGENCY BLOCK - Phoenix" `
                        -Direction Inbound `
                        -Protocol TCP `
                        -LocalPort 50055 `
                        -Action Block
    
    # 4. Alert security team
    Send-MailMessage -To "security@phoenix.local" `
                      -Subject "SECURITY INCIDENT: Executor Compromised" `
                      -Priority High `
                      -Attachments "$incidentPath\processes.csv"
}
```

### Disaster Recovery

```powershell
# Full recovery from backup
function Start-DisasterRecovery {
    param(
        [string]$BackupDate = (Get-Date).AddDays(-1).ToString('yyyyMMdd')
    )
    
    Write-Host "Starting disaster recovery from $BackupDate" -ForegroundColor Yellow
    
    # 1. Stop all services
    Get-Service -Name "*Phoenix*" | Stop-Service -Force
    
    # 2. Restore from backup
    $backupPath = "C:\Backups\PhoenixOrch\$BackupDate"
    if (-not (Test-Path $backupPath)) {
        throw "Backup not found: $backupPath"
    }
    
    # 3. Clean current installation
    Remove-Item "C:\Program Files\PhoenixOrch\executor-rs\*" -Recurse -Force
    Remove-Item "C:\phoenix_sandbox\*" -Recurse -Force
    
    # 4. Restore files
    Copy-Item "$backupPath\*" -Destination "C:\Program Files\PhoenixOrch\executor-rs\" -Recurse
    
    # 5. Restore registry
    reg import "$backupPath\service-config.reg"
    
    # 6. Recreate sandbox
    New-Item -ItemType Directory -Path "C:\phoenix_sandbox" -Force
    
    # 7. Start services
    Start-Service "PhoenixOrchExecutor"
    
    # 8. Verify recovery
    Test-ExecutorHealth
}
```

---

## Support & Resources

### Documentation
- Architecture Document: `executor-rs-windows-architecture.md`
- Testing Report: `executor-rs-testing-report.md`
- API Reference: Generated from proto files

### Contact Information
- **Operations Team**: ops@phoenix-orch.local
- **Security Team**: security@phoenix-orch.local
- **Development Team**: dev@phoenix-orch.local

### Useful Links
- [Windows Job Objects Documentation](https://docs.microsoft.com/en-us/windows/win32/procthread/job-objects)
- [Windows Integrity Levels](https://docs.microsoft.com/en-us/windows/win32/secauthz/mandatory-integrity-control)
- [gRPC Windows Guide](https://grpc.io/docs/platforms/windows/)

### Version History
- **v1.0.0** - Initial Windows native implementation
- **v1.0.1** - Fixed Low Integrity Level implementation (pending)
- **v1.1.0** - Added network isolation (planned)

---

*Deployment Guide Version: 1.0*
*Last Updated: 2025-12-10*
*System: PHOENIX ORCH - The Ashen Guard Edition AGI*