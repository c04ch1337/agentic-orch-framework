# Phoenix ORCH AGI Input Validation Framework

This document outlines the input validation and sanitization framework implemented across the Phoenix ORCH AGI system to ensure consistent and secure handling of all inputs.

## Overview

A comprehensive validation system has been implemented with the following components:

1. **Shared Validation Library**: The `input-validation-rs` crate provides reusable validation utilities for all services.
2. **Enhanced Gateway Validation**: API Gateway now includes request validation, size limits, and sanitization.
3. **Improved Safety Validation**: Safety Service has stronger regex patterns with ReDoS protection.
4. **Tools Service Validation**: All command parameters are validated and sanitized.
5. **Knowledge Base Service Validation**: All KB services validate their specific data types.

## Core Validation Library (input-validation-rs)

The shared validation library provides:

### Validators
- `StringValidation`: Length checks, pattern matching, character set validation
- `NumericValidation`: Range checks, type validation, overflow protection
- `PathValidation`: Path traversal protection, safe path checks
- `SecurityValidation`: Protection against code injection, SQL injection, command injection
- `UrlValidation`: URL format and safety checks

### Sanitizers
- `StringSanitizer`: Safe character replacement, HTML entity encoding
- `JsonSanitizer`: JSON structure validation and protection
- `PathSanitizer`: Normalize and secure file paths
- `CommandSanitizer`: Prevent command injection in tool executions

### Pattern Matching and ReDoS Protection
- Token-based pattern matching as a safer alternative to complex regexes
- Regex timeout mechanism to prevent Regex Denial of Service attacks
- Safe regex patterns with limited complexity

## Integration Guidelines

### 1. Import the Validation Library
```rust
// Add to your Cargo.toml
input-validation-rs = { path = "../input-validation-rs" }

// Import in your source files
use input_validation_rs::{
    validate,
    validators::{
        string::StringValidation,
        numeric::NumericValidation,
        security::SecurityValidation,
    },
    sanitizers::StringSanitizer,
    ValidationResult,
};
```

### 2. Create Service-Specific Validation Module
Each service should have a `validation.rs` module that:
- Defines constants for maximum sizes
- Implements service-specific validation functions
- Provides comprehensive validation for complex data types
- Implements defense-in-depth by validating at multiple layers

### 3. Apply Validation at Multiple Points
- **Service Entry Points**: Validate all incoming requests
- **Data Storage Layer**: Validate before storing data
- **Data Retrieval Layer**: Validate after retrieving data
- **Business Logic Layer**: Validate domain-specific constraints

### 4. Consistent Error Handling
- Return clear validation errors using `Status::invalid_argument`
- Log validation failures with appropriate warning level
- Include error details to help diagnose issues

### 5. Defense in Depth
- Implement redundant validation at different layers
- Apply different validation techniques to the same data
- Always sanitize output, even if input has been validated

## Example Validation Implementation

```rust
// Validate a query request
pub fn validate_query(query: &str, limit: u64) -> ValidationResult<()> {
    // Validate the query string
    validate!(
        query,
        StringValidation::not_empty(),
        StringValidation::max_length(MAX_QUERY_LENGTH),
        SecurityValidation::no_code_injection(),
        SecurityValidation::no_sql_injection()
    )?;

    // Validate the limit
    validate!(
        limit,
        NumericValidation::min_value_u64(MIN_LIMIT),
        NumericValidation::max_value_u64(MAX_LIMIT)
    )?;

    Ok(())
}

// Example use in service
if let Err(err) = validate_query(&req_data.query, req_data.limit) {
    log::warn!("Query validation failed: {}", err);
    return Err(Status::invalid_argument(format!("Invalid query: {}", err)));
}
```

## Security Considerations

1. **Size Limits**: All input sizes are strictly constrained
2. **Content Validation**: Input content is validated for allowed patterns and characters
3. **Character Encoding**: Proper UTF-8 validation and handling
4. **Defense Against Attacks**:
   - SQL Injection protection
   - Command Injection protection
   - Path Traversal protection
   - ReDoS protection
   - Code Injection protection
5. **Sanitization**: All inputs are sanitized before use

## Service-Specific Validations

### API Gateway
- Request payload size limits
- Schema validation against authorized endpoints
- Parameter sanitization
- Content-type validation

### Safety Service
- Enhanced regex patterns with timeout protection
- Token-based pattern matching
- Malformed UTF-8 sequence detection

### Tools Service
- Command parameter whitelisting
- Strict type checking
- Command sanitization

### Knowledge Base Services
- Schema validation for stored objects
- Query input sanitization
- Embedding vector validation
- Metadata validation

## Testing Validation

Validation should be tested with:
1. Valid inputs (positive tests)
2. Invalid inputs (negative tests)
3. Edge cases (boundary tests)
4. Malicious inputs (security tests)

Remember: Validation is a critical security mechanism. Always validate all inputs, even from trusted sources.