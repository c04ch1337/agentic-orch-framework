// social-kb-rs/src/validation.rs
// Input validation for Social KB Service (social dynamics and relationships)

use crate::agi_core::{RegisterUserRequest, UserIdentity};
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
const MAX_USER_NAME_LENGTH: usize = 128;
const MAX_USER_ID_LENGTH: usize = 64;
const MAX_PERMISSION_LENGTH: usize = 32;
const MAX_PERMISSIONS_COUNT: usize = 20;
const MAX_ATTRIBUTES_COUNT: usize = 50;
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
        return Err(input_validation_rs::ValidationError::TooMany(
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

/// Validates a user ID
pub fn validate_user_id(user_id: &str) -> ValidationResult<String> {
    // Empty user_id is acceptable for new registrations (will be generated)
    if user_id.is_empty() {
        return Ok(user_id.to_string());
    }

    validate!(
        user_id,
        StringValidation::max_length(MAX_USER_ID_LENGTH),
        StringValidation::alphanumeric_with_underscore_and_dots(),
        SecurityValidation::no_path_traversal(),
        SecurityValidation::no_code_injection()
    )?;

    Ok(StringSanitizer::sanitize_identifier(user_id))
}

/// Validates a user name
pub fn validate_user_name(name: &str) -> ValidationResult<String> {
    validate!(
        name,
        StringValidation::not_empty(),
        StringValidation::max_length(MAX_USER_NAME_LENGTH),
        SecurityValidation::no_script_tags(),
        SecurityValidation::no_code_injection()
    )?;

    Ok(StringSanitizer::sanitize(name))
}

/// Validates a role value
pub fn validate_role(role: i32) -> ValidationResult<i32> {
    // Defined roles: 0 = ROLE_SYSTEM, 1 = ROLE_USER, 2 = ROLE_ADMIN, 3 = ROLE_AGENT
    if role < 0 || role > 3 {
        return Err(input_validation_rs::ValidationError::OutOfRange(
            format!("Invalid role: {}. Must be between 0 and 3", role)
        ));
    }

    Ok(role)
}

/// Validates user permissions
pub fn validate_permissions(permissions: &[String]) -> ValidationResult<Vec<String>> {
    if permissions.len() > MAX_PERMISSIONS_COUNT {
        return Err(input_validation_rs::ValidationError::TooMany(
            format!(
                "Too many permissions: {}, max allowed: {}",
                permissions.len(),
                MAX_PERMISSIONS_COUNT
            )
        ));
    }

    let mut sanitized_permissions = Vec::new();

    for permission in permissions {
        validate!(
            permission,
            StringValidation::not_empty(),
            StringValidation::max_length(MAX_PERMISSION_LENGTH),
            StringValidation::alphanumeric_with_underscore(),
            SecurityValidation::no_code_injection()
        )?;

        sanitized_permissions.push(StringSanitizer::sanitize_identifier(permission));
    }

    Ok(sanitized_permissions)
}

/// Validates user attributes
pub fn validate_attributes(
    attributes: &HashMap<String, String>,
) -> ValidationResult<HashMap<String, String>> {
    if attributes.len() > MAX_ATTRIBUTES_COUNT {
        return Err(input_validation_rs::ValidationError::TooMany(
            format!(
                "Too many attributes: {}, max allowed: {}",
                attributes.len(),
                MAX_ATTRIBUTES_COUNT
            )
        ));
    }

    let mut sanitized_attributes = HashMap::new();

    for (key, value) in attributes {
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
            SecurityValidation::no_script_tags()
        )?;

        // Sanitize and store
        let sanitized_key = StringSanitizer::sanitize_identifier(key);
        let sanitized_value = StringSanitizer::sanitize(value);

        sanitized_attributes.insert(sanitized_key, sanitized_value);
    }

    Ok(sanitized_attributes)
}

/// Comprehensive validation for a user identity
pub fn validate_user_identity(identity: &UserIdentity) -> ValidationResult<UserIdentity> {
    // Validate user_id
    let sanitized_user_id = validate_user_id(&identity.user_id)?;

    // Validate name
    let sanitized_name = validate_user_name(&identity.name)?;

    // Validate role
    let validated_role = validate_role(identity.role)?;

    // Validate permissions
    let sanitized_permissions = validate_permissions(&identity.permissions)?;

    // Validate attributes
    let sanitized_attributes = validate_attributes(&identity.attributes)?;

    // Create validated and sanitized identity
    let validated_identity = UserIdentity {
        user_id: sanitized_user_id,
        name: sanitized_name,
        role: validated_role,
        permissions: sanitized_permissions,
        created_at: identity.created_at,
        last_active: identity.last_active,
        attributes: sanitized_attributes,
    };

    Ok(validated_identity)
}

/// Validates a RegisterUserRequest
pub fn validate_register_user_request(
    request: &RegisterUserRequest,
) -> ValidationResult<RegisterUserRequest> {
    // Check if identity is present
    let identity = match &request.identity {
        Some(identity) => identity,
        None => return Err(input_validation_rs::ValidationError::InvalidFormat(
            "Missing user identity".to_string()
        )),
    };

    // Validate the identity
    let validated_identity = validate_user_identity(identity)?;

    // Create validated request
    let validated_request = RegisterUserRequest {
        identity: Some(validated_identity),
    };

    Ok(validated_request)
}

/// Validates a role filter
pub fn validate_role_filter(role_filter: i32) -> ValidationResult<i32> {
    // -1 means no filter, 0-3 are valid roles
    if role_filter < -1 || role_filter > 3 {
        return Err(input_validation_rs::ValidationError::OutOfRange(
            format!(
                "Invalid role filter: {}. Must be between -1 and 3",
                role_filter
            )
        ));
    }

    Ok(role_filter)
}
