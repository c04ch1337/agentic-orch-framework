// mind-kb-rs/src/validation.rs
// Input validation for Mind KB Service

use input_validation_rs::{
    sanitizers::{JsonSanitizer, StringSanitizer},
    validate,
    validators::{
        numeric::NumericValidation, path::PathValidation, security::SecurityValidation,
        string::StringValidation,
    },
    ValidationResult,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;

// Maximum allowed sizes
const MAX_QUERY_LENGTH: usize = 4096; // 4KB query text
const MAX_CONTENT_LENGTH: usize = 1_048_576; // 1MB content
const MAX_METADATA_ENTRIES: usize = 50;
const MAX_METADATA_KEY_LENGTH: usize = 128;
const MAX_METADATA_VALUE_LENGTH: usize = 1024;
const VALID_EMBEDDING_DIMENSION: usize = 1536;
const MIN_LIMIT: u64 = 1;
const MAX_LIMIT: u64 = 100;

/// Validates a query request string
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

/// Validates embedding vector dimensions and values
pub fn validate_embedding(embedding: &[f32]) -> ValidationResult<()> {
    // Check embedding dimension
    if embedding.len() != VALID_EMBEDDING_DIMENSION {
        return Err(format!(
            "Invalid embedding dimension: expected {}, got {}",
            VALID_EMBEDDING_DIMENSION,
            embedding.len()
        ));
    }

    // Check for NaN values
    if embedding.iter().any(|val| val.is_nan()) {
        return Err("Embedding contains NaN values".to_string());
    }

    // Check if embedding is normalized (approximately 1.0 length)
    let squared_sum: f32 = embedding.iter().map(|val| val * val).sum();
    let magnitude = squared_sum.sqrt();

    if (magnitude - 1.0).abs() > 0.01 {
        return Err(format!(
            "Embedding is not properly normalized, magnitude: {}",
            magnitude
        ));
    }

    Ok(())
}

/// Validates and sanitizes metadata
pub fn validate_metadata(
    metadata: &HashMap<String, String>,
) -> ValidationResult<HashMap<String, String>> {
    let mut sanitized_metadata = HashMap::new();

    // Check number of entries
    if metadata.len() > MAX_METADATA_ENTRIES {
        return Err(format!(
            "Too many metadata entries: {}, max allowed: {}",
            metadata.len(),
            MAX_METADATA_ENTRIES
        ));
    }

    // Validate each key-value pair
    for (key, value) in metadata {
        // Validate key
        validate!(
            key,
            StringValidation::not_empty(),
            StringValidation::max_length(MAX_METADATA_KEY_LENGTH),
            StringValidation::alphanumeric_with_underscore(),
            SecurityValidation::no_path_traversal()
        )?;

        // Validate value
        validate!(
            value,
            StringValidation::max_length(MAX_METADATA_VALUE_LENGTH),
            SecurityValidation::no_code_injection(),
            SecurityValidation::no_command_injection()
        )?;

        // Sanitize and store
        let sanitized_key = StringSanitizer::sanitize_identifier(key);
        let sanitized_value = StringSanitizer::sanitize(value);

        sanitized_metadata.insert(sanitized_key, sanitized_value);
    }

    Ok(sanitized_metadata)
}

/// Validates and sanitizes content for storage
pub fn validate_content(content: &str) -> ValidationResult<String> {
    // Validate content length
    validate!(
        content,
        StringValidation::not_empty(),
        StringValidation::max_length(MAX_CONTENT_LENGTH),
        SecurityValidation::no_code_injection(),
        SecurityValidation::no_command_injection()
    )?;

    // Sanitize the content
    let sanitized = StringSanitizer::sanitize(content);

    Ok(sanitized)
}

/// Schema for validating a store request
#[derive(Serialize, Deserialize)]
struct StoreRequestSchema {
    key: String,
    value: String,
    metadata: HashMap<String, String>,
}

/// Validates a store request against schema
pub fn validate_store_request(
    key: &str,
    value: &[u8],
    metadata: &HashMap<String, String>,
) -> ValidationResult<()> {
    // Validate key
    validate!(
        key,
        StringValidation::not_empty(),
        StringValidation::max_length(256),
        StringValidation::alphanumeric_with_underscore_and_dots(),
        SecurityValidation::no_path_traversal()
    )?;

    // Validate value
    if value.is_empty() {
        return Err("Value cannot be empty".to_string());
    }

    if value.len() > MAX_CONTENT_LENGTH {
        return Err(format!(
            "Value too large: {} bytes, max allowed: {} bytes",
            value.len(),
            MAX_CONTENT_LENGTH
        ));
    }

    // Ensure value is valid UTF-8
    match std::str::from_utf8(value) {
        Ok(_) => {}
        Err(e) => return Err(format!("Value is not valid UTF-8: {}", e)),
    }

    // Validate metadata
    validate_metadata(metadata)?;

    Ok(())
}

/// Validates a retrieve request
pub fn validate_retrieve_request(key: &str) -> ValidationResult<()> {
    validate!(
        key,
        StringValidation::not_empty(),
        StringValidation::max_length(256),
        StringValidation::alphanumeric_with_underscore_and_dots(),
        SecurityValidation::no_path_traversal(),
        SecurityValidation::no_code_injection()
    )
}
