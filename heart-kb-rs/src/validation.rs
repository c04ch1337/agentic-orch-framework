// heart-kb-rs/src/validation.rs
// Input validation for Heart KB Service (emotion/sentiment)

use crate::agi_core::SentimentFact;
use input_validation_rs::{
    ValidationResult,
    sanitizers::StringSanitizer,
    validation::validate,
    validators::{
        numeric::NumericValidation, security::SecurityValidation, string::StringValidation,
    },
};
use std::collections::HashMap;

// Maximum allowed sizes
const MAX_QUERY_LENGTH: usize = 2048; // 2KB for queries
const MAX_KEY_LENGTH: usize = 256;
const MAX_VALUE_LENGTH: usize = 262_144; // 256KB
const MAX_SOURCE_ID_LENGTH: usize = 256;
const MAX_TEXT_LENGTH: usize = 4096; // 4KB for sentiment text
const MIN_CONFIDENCE: f32 = 0.0;
const MAX_CONFIDENCE: f32 = 1.0;
const MAX_FILTER_COUNT: usize = 10;
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
        return Err(input_validation_rs::ValidationError::InvalidFormat(
            "Value cannot be empty".to_string()
        ));
    }

    if value.len() > MAX_VALUE_LENGTH {
        return Err(input_validation_rs::ValidationError::TooLong(
            format!(
                "Value too large: {} bytes, max allowed: {} bytes",
                value.len(),
                MAX_VALUE_LENGTH
            )
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
        return Err(input_validation_rs::ValidationError::new(
            format!(
                "Too many filters: {}, max allowed: {}",
                filters.len(),
                MAX_FILTER_COUNT
            )
        ));
    }

    // Validate each filter key-value pair
    for (key, value) in filters {
        // Validate key
        validate!(
            key,
            StringValidation::not_empty(),
            StringValidation::max_length(64),
            StringValidation::alphanumeric_with_underscore(),
            SecurityValidation::no_code_injection()
        )?;

        // Validate value
        validate!(
            value,
            StringValidation::max_length(256),
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

/// Validates a source ID (user or entity ID)
pub fn validate_source_id(source_id: &str) -> ValidationResult<String> {
    validate!(
        source_id,
        StringValidation::not_empty(),
        StringValidation::max_length(MAX_SOURCE_ID_LENGTH),
        StringValidation::alphanumeric_with_underscore_and_dots(),
        SecurityValidation::no_path_traversal(),
        SecurityValidation::no_code_injection()
    )?;

    Ok(StringSanitizer::sanitize_identifier(source_id))
}

/// Validates sentiment enum value
pub fn validate_sentiment(sentiment: i32) -> ValidationResult<i32> {
    // Check sentiment range (0 to 6 in the proto definition)
    if sentiment < 0 || sentiment > 6 {
        return Err(input_validation_rs::ValidationError::OutOfRange(
            format!(
                "Invalid sentiment value: {}. Must be between 0 and 6",
                sentiment
            )
        ));
    }

    Ok(sentiment)
}

/// Validates confidence score
pub fn validate_confidence(confidence: f32) -> ValidationResult<f32> {
    if confidence < MIN_CONFIDENCE || confidence > MAX_CONFIDENCE {
        return Err(input_validation_rs::ValidationError::OutOfRange(
            format!(
                "Invalid confidence score: {}. Must be between {} and {}",
                confidence, MIN_CONFIDENCE, MAX_CONFIDENCE
            )
        ));
    }

    Ok(confidence)
}

/// Validates text input for sentiment sources
pub fn validate_text(text: &str) -> ValidationResult<String> {
    validate!(
        text,
        StringValidation::max_length(MAX_TEXT_LENGTH),
        SecurityValidation::no_code_injection(),
        SecurityValidation::no_command_injection(),
        SecurityValidation::no_script_tags()
    )?;

    // Sanitize text input
    Ok(StringSanitizer::sanitize(text))
}

/// Comprehensive validation for a sentiment fact
pub fn validate_sentiment_fact(fact: &SentimentFact) -> ValidationResult<SentimentFact> {
    // Validate source ID
    let sanitized_source_id = validate_source_id(&fact.source_id)?;

    // Validate sentiment
    let validated_sentiment = validate_sentiment(fact.sentiment)?;

    // Validate confidence score
    let validated_confidence = validate_confidence(fact.confidence_score)?;

    // Validate text if present and not empty
    let sanitized_text = if !fact.raw_text.is_empty() {
        validate_text(&fact.raw_text)?
    } else {
        fact.raw_text.clone()
    };

    // Create validated and sanitized fact
    let validated_fact = SentimentFact {
        source_id: sanitized_source_id,
        sentiment: validated_sentiment,
        confidence_score: validated_confidence,
        raw_text: sanitized_text,
        timestamp: fact.timestamp, // Timestamp is just a number, no validation needed
        metadata: fact.metadata.clone(), // Clone metadata as-is
    };

    Ok(validated_fact)
}
