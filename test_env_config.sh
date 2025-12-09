#!/bin/bash
# Test script for the new environment configuration approach
# This script validates that the environment switcher works as expected

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Helper function for output formatting
print_header() {
  echo -e "\n${YELLOW}==== $1 ====${NC}"
}

print_success() {
  echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
  echo -e "${RED}✗ $1${NC}"
  FAILED_TESTS=$((FAILED_TESTS+1))
}

check_file_exists() {
  if [ -f "$1" ]; then
    print_success "File $1 exists"
    return 0
  else
    print_error "File $1 does not exist"
    return 1
  fi
}

check_variable_in_env() {
  local file=$1
  local var_name=$2
  local expected_value=$3
  
  # Use grep to check if the variable is set correctly
  if grep -q "^$var_name=$expected_value" "$file"; then
    print_success "Variable $var_name is correctly set to '$expected_value'"
    return 0
  else
    actual_value=$(grep "^$var_name=" "$file" | sed "s/^$var_name=//")
    print_error "Variable $var_name should be '$expected_value' but is '$actual_value'"
    return 1
  fi
}

FAILED_TESTS=0

# --- Validate Files Exist ---
print_header "Validating files exist"

required_files=(
  ".env.example.consolidated"
  "env_switcher.sh"
  "env_switcher.ps1"
  "docs/environment-configuration-guide.md"
)

for file in "${required_files[@]}"; do
  check_file_exists "$file"
done

# --- Test Development Environment ---
print_header "Testing Development Environment Setup"

# Make the env_switcher.sh script executable if needed
chmod +x ./env_switcher.sh

# Switch to development environment
./env_switcher.sh -e development

# Verify .env file was created
if ! check_file_exists ".env"; then
  echo "Skipping environment variable tests since .env wasn't created"
else
  # Verify ENVIRONMENT variable
  check_variable_in_env ".env" "ENVIRONMENT" "development"
  
  # Check a few development-specific values were uncommented
  grep -q "^DEVELOPMENT_" ".env" || print_error "No development-specific variables were uncommented"
  
  if grep -q "^DEVELOPMENT_" ".env"; then
    print_success "Development-specific variables were uncommented"
  fi
fi

# --- Test Staging Environment ---
print_header "Testing Staging Environment Setup"

# Switch to staging environment
./env_switcher.sh -e staging

# Verify .env file was created
if ! check_file_exists ".env"; then
  echo "Skipping environment variable tests since .env wasn't created"
else
  # Verify ENVIRONMENT variable
  check_variable_in_env ".env" "ENVIRONMENT" "staging"
  
  # Check a few staging-specific values were uncommented
  grep -q "^STAGING_" ".env" || print_error "No staging-specific variables were uncommented"
  
  if grep -q "^STAGING_" ".env"; then
    print_success "Staging-specific variables were uncommented"
  fi
fi

# --- Test Production Environment ---
print_header "Testing Production Environment Setup"

# Switch to production environment
./env_switcher.sh -e production

# Verify .env file was created
if ! check_file_exists ".env"; then
  echo "Skipping environment variable tests since .env wasn't created"
else
  # Verify ENVIRONMENT variable
  check_variable_in_env ".env" "ENVIRONMENT" "production"
  
  # Check a few production-specific values were uncommented
  grep -q "^PRODUCTION_" ".env" || print_error "No production-specific variables were uncommented"
  
  if grep -q "^PRODUCTION_" ".env"; then
    print_success "Production-specific variables were uncommented"
  fi
fi

# --- Test Docker Compose Integration ---
print_header "Testing Docker Compose Integration"

# Test if docker-compose can validate the file with the new .env
if command -v docker-compose &> /dev/null; then
  if docker-compose config -q; then
    print_success "Docker Compose configuration is valid with the new .env"
  else
    print_error "Docker Compose configuration has errors with the new .env file"
  fi
else
  echo "Docker Compose not installed, skipping Docker Compose validation test"
fi

# --- Test Summary ---
print_header "Test Summary"

if [ $FAILED_TESTS -eq 0 ]; then
  echo -e "${GREEN}All tests passed successfully!${NC}"
  echo "The new environment configuration approach is working as expected."
else
  echo -e "${RED}$FAILED_TESTS test(s) failed.${NC}"
  echo "Please review the test output to fix any issues."
fi

# Return to development environment for continued development
print_header "Resetting to development environment for continued development"
./env_switcher.sh -e development

exit $FAILED_TESTS