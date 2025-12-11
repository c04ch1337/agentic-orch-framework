// body-kb-rs/src/validation.rs
// Input validation for Body KB Service

use input_validation_rs::{
    ValidationResult,
    sanitizers::StringSanitizer,
    validate,
    validators::{
        numeric::NumericValidation, security::SecurityValidation, string::StringValidation,
    },
};
use std::collections::HashMap;

// Maximum allowed sizes
const MAX_QUERY_LENGTH: usize = 2048; // 2KB for queries
const MAX_KEY_LENGTH: usize = 256;
const MAX_VALUE_LENGTH: usize = 262_144; // 256KB
const MAX_FILTER_COUNT: usize = 10;
const MAX_FILTER_KEY_LENGTH: usize = 64;
const MAX_FILTER_VALUE_LENGTH: usize = 256;
const MIN_LIMIT: u64 = 1;
const MAX_LIMIT: u64 = 50;

/// Validates a query request
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

/// Validates a key for retrieval or storage
pub fn validate_key(key: &str) -> ValidationResult<String> {
    // Validate and sanitize the key
    validate!(
        key,
        StringValidation::not_empty(),
        StringValidation::max_length(MAX_KEY_LENGTH),
        StringValidation::alphanumeric_with_underscore_and_dots(),
        SecurityValidation::no_path_traversal(),
        SecurityValidation::no_code_injection()
    )?;

    // Sanitize for safe usage
    let sanitized = StringSanitizer::sanitize_identifier(key);
    Ok(sanitized)
}

/// Validates and sanitizes value data for storage
pub fn validate_value(value: &[u8]) -> ValidationResult<Vec<u8>> {
    // Check length constraints
    if value.is_empty() {
        return Err("Value cannot be empty".to_string());
    }

    if value.len() > MAX_VALUE_LENGTH {
        return Err(format!(
            "Value too large: {} bytes, max allowed: {} bytes",
            value.len(),
            MAX_VALUE_LENGTH
        ));
    }

    // If it's a text value, attempt to validate it as UTF-8
    if let Ok(text) = std::str::from_utf8(value) {
        // Validate for security issues if it's text
        validate!(
            text,
            SecurityValidation::no_code_injection(),
            SecurityValidation::no_command_injection(),
            SecurityValidation::no_script_tags()
        )?;

        // Return sanitized text as bytes
        return Ok(StringSanitizer::sanitize(text).into_bytes());
    }

    // For binary data, just verify it's within size limits
    Ok(value.to_vec())
}

/// Validates request filters (key-value pairs)
pub fn validate_filters(
    filters: &HashMap<String, String>,
) -> ValidationResult<HashMap<String, String>> {
    let mut sanitized_filters = HashMap::new();

    // Check number of filters
    if filters.len() > MAX_FILTER_COUNT {
        return Err(format!(
            "Too many filters: {}, max allowed: {}",
            filters.len(),
            MAX_FILTER_COUNT
        ));
    }

    // Validate each filter key-value pair
    for (key, value) in filters {
        // Validate key
        validate!(
            key,
            StringValidation::not_empty(),
            StringValidation::max_length(MAX_FILTER_KEY_LENGTH),
            StringValidation::alphanumeric_with_underscore(),
            SecurityValidation::no_code_injection()
        )?;

        // Validate value
        validate!(
            value,
            StringValidation::max_length(MAX_FILTER_VALUE_LENGTH),
            SecurityValidation::no_code_injection(),
            SecurityValidation::no_sql_injection(),
            SecurityValidation::no_script_tags()
        )?;

        // Sanitize and store
        let sanitized_key = StringSanitizer::sanitize_identifier(key);
        let sanitized_value = StringSanitizer::sanitize(value);

        sanitized_filters.insert(sanitized_key, sanitized_value);
    }

    Ok(sanitized_filters)
}

/// Validates a store request checking key, value, and metadata
pub fn validate_store_request(
    key: &str,
    value: &[u8],
    metadata: &HashMap<String, String>,
) -> ValidationResult<(String, Vec<u8>, HashMap<String, String>)> {
    let sanitized_key = validate_key(key)?;
    let sanitized_value = validate_value(value)?;
    let sanitized_metadata = validate_filters(metadata)?;

    Ok((sanitized_key, sanitized_value, sanitized_metadata))
}

/// Validates a retrieve request (key and optional filters)
pub fn validate_retrieve_request(
    key: &str,
    filters: &HashMap<String, String>,
) -> ValidationResult<(String, HashMap<String, String>)> {
    let sanitized_key = validate_key(key)?;
    let sanitized_filters = validate_filters(filters)?;

    Ok((sanitized_key, sanitized_filters))
}
