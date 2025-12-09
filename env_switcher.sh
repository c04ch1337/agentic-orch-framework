#!/bin/bash
# ==============================================================
# PHOENIX ORCH Environment Switcher
# ==============================================================
# This script helps switch between different environments by:
# 1. Creating a proper .env file from the .env.example.consolidated template
# 2. Setting the ENVIRONMENT variable
# 3. Applying environment-specific overrides automatically
# ==============================================================

set -e # Exit on any error

# Default values
ENV_EXAMPLE="$(pwd)/.env.example.consolidated"
ENV_FILE="$(pwd)/.env"
SELECTED_ENV=""

print_usage() {
  echo "Usage: ./env_switcher.sh [OPTIONS]"
  echo "Options:"
  echo "  -e, --environment ENV   Set environment (development|staging|production)"
  echo "                          Shorthand: dev, prod"
  echo "  -h, --help              Show this help message"
  echo ""
  echo "Examples:"
  echo "  ./env_switcher.sh -e development"
  echo "  ./env_switcher.sh --environment staging"
  echo "  ./env_switcher.sh -e prod"
}

copy_template() {
  # Check if .env.example.consolidated exists
  if [ ! -f "$ENV_EXAMPLE" ]; then
    echo "Error: Template file not found at $ENV_EXAMPLE"
    echo "Make sure you're running this script from the project root directory."
    exit 1
  fi
  
  # Create backup of existing .env if it exists
  if [ -f "$ENV_FILE" ]; then
    timestamp=$(date +"%Y%m%d%H%M%S")
    echo "Creating backup of existing .env file to .env.backup.$timestamp"
    cp "$ENV_FILE" "$ENV_FILE.backup.$timestamp"
  fi
  
  # Copy the template
  cp "$ENV_EXAMPLE" "$ENV_FILE"
  echo "Created new .env file from template."
}

apply_environment_settings() {
  local env=$1
  
  # First, ensure environment variable itself is set properly
  sed -i "s/^ENVIRONMENT=.*/ENVIRONMENT=$env/" "$ENV_FILE"
  
  echo "Set ENVIRONMENT=$env in .env file"
  
  # Uncomment the environment-specific overrides section for the selected environment
  if [ "$env" = "development" ] || [ "$env" = "dev" ]; then
    # Uncomment development overrides
    sed -i '/^# --- DEVELOPMENT-SPECIFIC OVERRIDES ---/,/^# --- STAGING-SPECIFIC OVERRIDES ---/s/^# DEVELOPMENT_/DEVELOPMENT_/' "$ENV_FILE"
    echo "Applied development environment overrides"
  elif [ "$env" = "staging" ]; then
    # Uncomment staging overrides
    sed -i '/^# --- STAGING-SPECIFIC OVERRIDES ---/,/^# --- PRODUCTION-SPECIFIC OVERRIDES ---/s/^# STAGING_/STAGING_/' "$ENV_FILE"
    echo "Applied staging environment overrides"
  elif [ "$env" = "production" ] || [ "$env" = "prod" ]; then
    # Uncomment production overrides
    sed -i '/^# --- PRODUCTION-SPECIFIC OVERRIDES ---/,/$/s/^# PRODUCTION_/PRODUCTION_/' "$ENV_FILE"
    echo "Applied production environment overrides"
  else
    echo "Unknown environment: $env"
    exit 1
  fi
}

# Parse command-line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    -e|--environment)
      SELECTED_ENV="$2"
      shift 2
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      print_usage
      exit 1
      ;;
  esac
done

# Validate input
if [ -z "$SELECTED_ENV" ]; then
  echo "Error: No environment specified."
  print_usage
  exit 1
fi

# Normalize environment names
if [ "$SELECTED_ENV" = "dev" ]; then
  SELECTED_ENV="development"
elif [ "$SELECTED_ENV" = "prod" ]; then
  SELECTED_ENV="production"
fi

# Check for valid environment
if [[ ! "$SELECTED_ENV" =~ ^(development|staging|production)$ ]]; then
  echo "Error: Invalid environment. Must be one of: development, staging, production"
  exit 1
fi

# Execute the environment switching
echo "Switching to $SELECTED_ENV environment..."
copy_template
apply_environment_settings "$SELECTED_ENV"

echo "Environment successfully switched to $SELECTED_ENV"
echo "To apply these changes, restart your containers with:"
echo "  docker-compose down && docker-compose up -d"
echo ""
echo "For local development without Docker, source the environment variables:"
echo "  source .env"