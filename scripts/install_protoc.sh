#!/bin/bash
# Script to install Protocol Buffers compiler (protoc) for Unix-like systems

# Set the version
PROTOC_VERSION=26.0
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')

# Determine architecture
case "$ARCH" in
    x86_64)
        ARCH_NAME="x86_64"
        ;;
    aarch64|arm64)
        ARCH_NAME="aarch_64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Create temp directory
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

# Download protoc
echo "Downloading protoc v${PROTOC_VERSION} for ${OS}-${ARCH_NAME}..."
if [ "$OS" = "darwin" ]; then
    # macOS
    PROTOC_ZIP="protoc-${PROTOC_VERSION}-osx-${ARCH_NAME}.zip"
else
    # Linux
    PROTOC_ZIP="protoc-${PROTOC_VERSION}-linux-${ARCH_NAME}.zip"
fi

PROTOC_URL="https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/${PROTOC_ZIP}"
curl -LO "$PROTOC_URL"

# Extract
echo "Extracting..."
unzip -o "$PROTOC_ZIP" -d protoc

# Install
echo "Installing..."
sudo mv protoc/bin/protoc /usr/local/bin/
sudo mv protoc/include/* /usr/local/include/

# Clean up
cd
rm -rf "$TEMP_DIR"

# Check
if command -v protoc >/dev/null 2>&1; then
    echo "protoc successfully installed:"
    protoc --version
else
    echo "protoc installation failed."
    exit 1
fi