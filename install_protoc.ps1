# PowerShell script to install Protocol Buffers compiler (protoc) for Windows systems

# Set the version
$PROTOC_VERSION = "26.0"
$ARCH = "x86_64" # Most Windows systems are x64

# Create temp directory
$TEMP_DIR = Join-Path $env:TEMP "protoc_install"
New-Item -ItemType Directory -Path $TEMP_DIR -Force | Out-Null
Set-Location $TEMP_DIR

# Download protoc
Write-Host "Downloading protoc v$PROTOC_VERSION for Windows-$ARCH..."
$PROTOC_ZIP = "protoc-$PROTOC_VERSION-win64.zip"
$PROTOC_URL = "https://github.com/protocolbuffers/protobuf/releases/download/v$PROTOC_VERSION/$PROTOC_ZIP"

$ProgressPreference = 'SilentlyContinue' # Speeds up download
Invoke-WebRequest -Uri $PROTOC_URL -OutFile $PROTOC_ZIP
$ProgressPreference = 'Continue'

# Extract
Write-Host "Extracting..."
Expand-Archive -Path $PROTOC_ZIP -DestinationPath "protoc" -Force

# Determine installation directory - prefer the user's profile
$INSTALL_DIR = Join-Path $env:USERPROFILE "protoc"
New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null

# Install
Write-Host "Installing to $INSTALL_DIR"
Copy-Item "protoc\bin\*" -Destination $INSTALL_DIR -Force
Copy-Item "protoc\include\*" -Destination $INSTALL_DIR -Recurse -Force

# Add to PATH if not already there
if ($env:PATH -notlike "*$INSTALL_DIR*") {
    Write-Host "Adding $INSTALL_DIR to PATH..."
    $USER_PATH = [Environment]::GetEnvironmentVariable("PATH", "User")
    [Environment]::SetEnvironmentVariable("PATH", "$USER_PATH;$INSTALL_DIR", "User")
    
    # Also update current session path
    $env:PATH = "$env:PATH;$INSTALL_DIR"
}

# Set PROTOC environment variable
[Environment]::SetEnvironmentVariable("PROTOC", (Join-Path $INSTALL_DIR "protoc.exe"), "User")
Write-Host "Set PROTOC environment variable to " (Join-Path $INSTALL_DIR "protoc.exe")

# Clean up
Set-Location $env:USERPROFILE
Remove-Item -Recurse -Force $TEMP_DIR

# Check
try {
    $version = & (Join-Path $INSTALL_DIR "protoc.exe") --version
    Write-Host "protoc successfully installed: $version"
    Write-Host ""
    Write-Host "NOTE: You may need to restart your terminal or IDE for the PATH changes to take effect."
}
catch {
    Write-Host "protoc installation failed: $_"
    exit 1
}