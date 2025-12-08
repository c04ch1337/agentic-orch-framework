#!/bin/bash
# AGI Microservices Deployment Script (system-build-rs)

# --- Prerequisite Check: protoc ---
# Note: In a production Docker/Linux environment, protoc must be installed.
# Assuming a standard Debian/Ubuntu-based container for demonstration:
if ! command -v protoc &> /dev/null
then
    echo "protoc not found. Installing protobuf-compiler..."
    sudo apt-get update && sudo apt-get install -y protobuf-compiler
fi

# --- Main Build Process ---
echo "Building AGI microservices (system-build-rs workspace)..."
# The 'cargo build' command runs the build.rs scripts, generating the gRPC code.
cargo build --release

# --- Run/Cleanup (Optional, for production readiness) ---
# Add deployment logic here (e.g., stopping old containers, starting new ones)

echo "Build complete. Ready for Dockerization via docker-compose."

# --- GitHub Backup Reminder ---
echo "-------------------------------------"
echo "REMINDER: Back up your work to GitHub."
echo "git add . ; git commit -m 'FEAT: Finalized gRPC config with tonic-prost-build' ; git push"
echo "-------------------------------------"
