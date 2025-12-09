#!/bin/bash
# AGI Microservices Deployment Script (system-build-rs)

# Ensure we are in the project root
cd "$(dirname "$0")/.."

# --- Environment Setup ---
# Check if an environment parameter was provided
ENV=${1:-production}  # Default to production if not specified

# Validate environment parameter
if [[ ! "$ENV" =~ ^(development|dev|staging|production|prod)$ ]]; then
  echo "Error: Invalid environment. Must be one of: development, staging, production"
  echo "Usage: ./deploy.sh [environment]"
  echo "Examples:"
  echo "  ./deploy.sh development"
  echo "  ./deploy.sh staging"
  echo "  ./deploy.sh production"
  exit 1
fi

echo "Deploying for $ENV environment..."

# Switch to the appropriate environment configuration
if [ -f "./env_switcher.sh" ]; then
  echo "Setting up environment configuration..."
  chmod +x ./env_switcher.sh
  ./env_switcher.sh -e "$ENV"
else
  echo "Warning: env_switcher.sh not found. Environment may not be properly configured."
  echo "Please run: cp .env.example.consolidated .env"
fi

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

# --- Deploy with Docker Compose ---
echo "Deploying containers with Docker Compose..."

# Select the appropriate Docker Compose file based on environment
if [ "$ENV" == "development" ] || [ "$ENV" == "dev" ]; then
  COMPOSE_FILE="docker-compose.dev.yml"
elif [ "$ENV" == "staging" ]; then
  COMPOSE_FILE="docker-compose.yml"  # Using the main compose file for staging
else
  COMPOSE_FILE="docker-compose.yml"  # Using the main compose file for production
fi

# Stop any running containers and start fresh
echo "Starting services using $COMPOSE_FILE..."
docker-compose -f "$COMPOSE_FILE" down
docker-compose -f "$COMPOSE_FILE" up -d

echo "Deployment complete for $ENV environment."
echo "Check container status with: docker-compose ps"

# --- GitHub Backup Reminder ---
echo "-------------------------------------"
echo "REMINDER: Back up your work to GitHub."
echo "git add . ; git commit -m 'DEPLOY: $ENV environment deployment' ; git push"
echo "-------------------------------------"
