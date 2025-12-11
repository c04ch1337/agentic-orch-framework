// soul-kb-rs/src/validation.rs
// Input validation for Soul KB Service (ethics and values)

use crate::agi_core::{CoreValue, EthicsCheckRequest, StoreValueRequest};
use crate::validation_macros::{validate, validate_length, validate_range};
use input_validation_rs::{
    ValidationResult, ValidationError,
    sanitizers::StringSanitizer,
    validators::{
        numeric::NumericValidation, security::SecurityValidation, string::StringValidation,
    },
};
use std::collections::HashMap;

// Re-export macros
pub use crate::validation_macros::*;

// Maximum allowed sizes
const MAX_QUERY_LENGTH: usize = 2048; // 2KB for queries
const MAX_KEY_LENGTH: usize = 256;
const MAX_VALUE_LENGTH: usize = 262_144; // 256KB
const MAX_FILTER_COUNT: usize = 10;
const MAX_VALUE_NAME_LENGTH: usize = 64;
const MAX_VALUE_DESC_LENGTH: usize = 1024;
const MAX_CONSTRAINT_LENGTH: usize = 2048;
const MAX_ACTION_LENGTH: usize = 4096;
const MIN_PRIORITY: i32 = 1;
const MAX_PRIORITY: i32 = 4;
const MIN_LIMIT: u64 = 1;
const MAX_LIMIT: u64 = 50;
const MAX_METADATA_ENTRIES: usize = 50;

/// Validates a query request
pub fn validate_query(query: &str, limit: u64) -> ValidationResult<()> {
    // Validate query length
    validate_length!(query, MAX_QUERY_LENGTH)?;

    // Validate query content
    validate!(query,
        StringValidation::not_empty(),
        SecurityValidation::no_code_injection(),
        SecurityValidation::no_sql_injection()
    )?;

    // Validate limit range
    validate_range!(limit, MIN_LIMIT, MAX_LIMIT)?;

    Ok(())
}

/// Validates a key for retrieval or storage
pub fn validate_key(key: &str) -> ValidationResult<String> {
    // Validate key length
    validate_length!(key, MAX_KEY_LENGTH)?;

    // Validate key content
    validate!(key,
        StringValidation::not_empty(),
        StringValidation::alphanumeric_with_underscore_and_dots(),
        SecurityValidation::no_path_traversal(),
        SecurityValidation::no_code_injection()
    )?;

    // Sanitize for safe usage
    Ok(StringSanitizer::sanitize_identifier(key))
}

/// Validates and sanitizes value data for storage
pub fn validate_value(value: &[u8]) -> ValidationResult<Vec<u8>> {
    // Check length constraints
    if value.is_empty() {
        return Err(ValidationError::InvalidFormat(
            "Value cannot be empty".to_string()
        ));
    }

    if value.len() > MAX_VALUE_LENGTH {
        return Err(ValidationError::TooLong(
            format!(
                "Value too large: {} bytes, max allowed: {} bytes",
                value.len(),
                MAX_VALUE_LENGTH
            )
        ));
    }

    // If it's a text value, attempt to validate it as UTF-8
    if let Ok(text) = std::str::from_utf8(value) {
        // Validate text content
        validate!(text,
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
        return Err(ValidationError::new(
            format!(
                "Too many filters: {}, max allowed: {}",
                filters.len(),
                MAX_FILTER_COUNT
            )
        ));
    }

    // Validate each filter key-value pair
    for (key, value) in filters {
        // Validate key and value
        validate_length!(key, 64)?;
        validate_length!(value, 256)?;

        validate!(key,
            StringValidation::not_empty(),
            StringValidation::alphanumeric_with_underscore(),
            SecurityValidation::no_code_injection()
        )?;

        validate!(value,
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

/// Validates a value name
pub fn validate_value_name(name: &str) -> ValidationResult<String> {
    validate!(
        name,
        StringValidation::not_empty(),
        StringValidation::max_length(MAX_VALUE_NAME_LENGTH),
        StringValidation::alphanumeric_with_underscore(),
        SecurityValidation::no_code_injection()
    )?;

    Ok(StringSanitizer::sanitize_identifier(name))
}

/// Validates a value description
pub fn validate_description(description: &str) -> ValidationResult<String> {
    validate!(
        description,
        StringValidation::not_empty(),
        StringValidation::max_length(MAX_VALUE_DESC_LENGTH),
        SecurityValidation::no_code_injection(),
        SecurityValidation::no_script_tags()
    )?;

    Ok(StringSanitizer::sanitize(description))
}

/// Validates a constraint string
pub fn validate_constraint(constraint: &str) -> ValidationResult<String> {
    validate!(
        constraint,
        StringValidation::not_empty(),
        StringValidation::max_length(MAX_CONSTRAINT_LENGTH),
        SecurityValidation::no_code_injection(),
        SecurityValidation::no_script_tags()
    )?;

    Ok(StringSanitizer::sanitize(constraint))
}

/// Validates a priority value
pub fn validate_priority(priority: i32) -> ValidationResult<i32> {
    validate_range!(priority, MIN_PRIORITY, MAX_PRIORITY)?;
    Ok(priority)
}

/// Validates value metadata
pub fn validate_value_metadata(
    metadata: &HashMap<String, String>,
) -> ValidationResult<HashMap<String, String>> {
    let mut sanitized_metadata = HashMap::new();

    // Check number of entries
    if metadata.len() > MAX_METADATA_ENTRIES {
        return Err(ValidationError::new(
            format!(
                "Too many metadata entries: {}, max allowed: {}",
                metadata.len(),
                MAX_METADATA_ENTRIES
            )
        ));
    }

    // Validate each key-value pair
    for (key, value) in metadata {
        // Validate lengths
        validate_length!(key, 64)?;
        validate_length!(value, 256)?;

        // Validate content
        validate!(key,
            StringValidation::not_empty(),
            StringValidation::alphanumeric_with_underscore(),
            SecurityValidation::no_code_injection()
        )?;

        validate!(value,
            SecurityValidation::no_code_injection(),
            SecurityValidation::no_script_tags()
        )?;

        // Sanitize and store
        let sanitized_key = StringSanitizer::sanitize_identifier(key);
        let sanitized_value = StringSanitizer::sanitize(value);

        sanitized_metadata.insert(sanitized_key, sanitized_value);
    }

    Ok(sanitized_metadata)
}

/// Validates a core value ID
pub fn validate_value_id(id: &str) -> ValidationResult<String> {
    // Empty ID is acceptable for new values (will be generated)
    if id.is_empty() {
        return Ok(id.to_string());
    }

    validate_length!(id, 64)?;
    validate!(id,
        StringValidation::alphanumeric_with_underscore_and_dots(),
        SecurityValidation::no_path_traversal(),
        SecurityValidation::no_code_injection()
    )?;

    Ok(StringSanitizer::sanitize_identifier(id))
}

/// Comprehensive validation for a core value
pub fn validate_core_value(value: &CoreValue) -> ValidationResult<CoreValue> {
    // Validate ID
    let sanitized_id = validate_value_id(&value.value_id)?;

    // Validate name
    let sanitized_name = validate_value_name(&value.name)?;

    // Validate description
    let sanitized_description = validate_description(&value.description)?;

    // Validate priority
    let validated_priority = validate_priority(value.priority)?;

    // Validate constraint
    let sanitized_constraint = validate_constraint(&value.constraint)?;

    // Validate metadata
    let sanitized_metadata = validate_value_metadata(&value.metadata)?;

    // Create validated value
    let validated_value = CoreValue {
        value_id: sanitized_id,
        name: sanitized_name,
        description: sanitized_description,
        priority: validated_priority,
        constraint: sanitized_constraint,
        is_active: value.is_active,
        metadata: sanitized_metadata,
    };

    Ok(validated_value)
}

/// Validates a StoreValueRequest
pub fn validate_store_value_request(
    request: &StoreValueRequest,
) -> ValidationResult<StoreValueRequest> {
    // Check if value is present
    let value = match &request.value {
        Some(value) => value,
        None => return Err(ValidationError::InvalidFormat(
            "Missing core value".to_string()
        )),
    };

    // Validate the core value
    let validated_value = validate_core_value(value)?;

    // Create validated request
    let validated_request = StoreValueRequest {
        value: Some(validated_value),
    };

    Ok(validated_request)
}

/// Validates an ethics check action
pub fn validate_ethics_action(action: &str) -> ValidationResult<String> {
    validate_length!(action, MAX_ACTION_LENGTH)?;
    validate!(action,
        StringValidation::not_empty(),
        SecurityValidation::no_code_injection(),
        SecurityValidation::no_command_injection()
    )?;

    Ok(StringSanitizer::sanitize(action))
}

/// Validates an EthicsCheckRequest
pub fn validate_ethics_check_request(
    request: &EthicsCheckRequest,
) -> ValidationResult<EthicsCheckRequest> {
    // Validate action
    let sanitized_action = validate_ethics_action(&request.action)?;

    // Create validated request
    let validated_request = EthicsCheckRequest {
        action: sanitized_action,
        context: request.context.clone(), // Context is optional, just clone it
    };

    Ok(validated_request)
}

/// Validates a min priority filter
pub fn validate_min_priority(min_priority: i32) -> ValidationResult<i32> {
    // 0 means no filter, 1-4 are valid priorities
    validate_range!(min_priority, 0, MAX_PRIORITY)?;
    Ok(min_priority)
}
