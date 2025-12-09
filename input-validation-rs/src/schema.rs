//! Schema-based validation
//!
//! This module provides utilities for schema-based validation of complex objects,
//! inspired by JSON Schema but with additional safety features.

use std::collections::{HashMap, HashSet};
use serde_json::Value;
use crate::errors::{ValidationError, ValidationResult};
use crate::ValidationConfig;

/// Schema for validating JSON-like structures
#[derive(Debug, Clone)]
pub struct Schema {
    /// Field definitions for object validation
    fields: HashMap<String, FieldSchema>,
    /// Whether to allow additional fields not in the schema
    allow_additional_fields: bool,
    /// Common validation configuration
    config: ValidationConfig,
}

/// Schema for a single field
#[derive(Debug, Clone)]
pub struct FieldSchema {
    /// Field type
    field_type: FieldType,
    /// Whether field is required
    required: bool,
    /// Custom validation function
    custom_validator: Option<fn(&Value) -> ValidationResult<()>>,
    /// Child schema for objects
    object_schema: Option<Box<Schema>>,
    /// Item schema for arrays
    array_item_schema: Option<Box<FieldSchema>>,
    /// Allowed values (enum)
    enum_values: Option<Vec<Value>>,
    /// Pattern for string validation
    pattern: Option<String>,
    /// Minimum value for numeric validation
    minimum: Option<f64>,
    /// Maximum value for numeric validation
    maximum: Option<f64>,
    /// Minimum length for string/array validation
    min_length: Option<usize>,
    /// Maximum length for string/array validation
    max_length: Option<usize>,
}

/// Supported field types
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    /// String type
    String,
    /// Integer type (i64)
    Integer,
    /// Number type (f64)
    Number,
    /// Boolean type
    Boolean,
    /// Object type (with schema)
    Object,
    /// Array type
    Array,
    /// Any JSON type
    Any,
}

impl Schema {
    /// Create a new schema
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            allow_additional_fields: false,
            config: ValidationConfig::default(),
        }
    }

    /// Create a builder for this schema
    pub fn builder() -> SchemaBuilder {
        SchemaBuilder::new()
    }

    /// Allow additional fields not in the schema
    pub fn with_additional_fields(mut self, allow: bool) -> Self {
        self.allow_additional_fields = allow;
        self
    }

    /// Set validation configuration
    pub fn with_config(mut self, config: ValidationConfig) -> Self {
        self.config = config;
        self
    }

    /// Add a field to the schema
    pub fn add_field(mut self, name: &str, schema: FieldSchema) -> Self {
        self.fields.insert(name.to_string(), schema);
        self
    }

    /// Validate a value against this schema
    pub fn validate(&self, value: &Value) -> ValidationResult<()> {
        self.validate_with_path(value, None)
    }

    /// Validate a value with a path for error reporting
    fn validate_with_path(&self, value: &Value, path: Option<String>) -> ValidationResult<()> {
        match value {
            Value::Object(obj) => {
                let mut errors = Vec::new();

                // Check required fields
                for (field_name, field_schema) in &self.fields {
                    if field_schema.required && !obj.contains_key(field_name) {
                        errors.push(ValidationError::MissingFields(format!(
                            "Required field '{}' is missing", field_name
                        )));
                    }
                }

                // Check unknown fields
                if !self.allow_additional_fields {
                    let schema_fields: HashSet<String> = self.fields.keys().cloned().collect();
                    for field_name in obj.keys() {
                        if !schema_fields.contains(field_name) {
                            errors.push(ValidationError::ExtraFields(format!(
                                "Unknown field '{}' is not allowed", field_name
                            )));
                        }
                    }
                }

                // Validate each field
                for (field_name, field_value) in obj {
                    if let Some(field_schema) = self.fields.get(field_name) {
                        let field_path = match &path {
                            Some(p) => format!("{}.{}", p, field_name),
                            None => field_name.clone(),
                        };

                        if let Err(err) = field_schema.validate(field_value, Some(field_path.clone())) {
                            errors.push(err);
                        }
                    }
                }

                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(ValidationError::composite_at(errors, path.unwrap_or_default()))
                }
            }
            _ => Err(ValidationError::InvalidType(format!(
                "Expected object, got {:?}", value
            ))),
        }
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldSchema {
    /// Create a new string field schema
    pub fn string() -> FieldSchemaBuilder {
        FieldSchemaBuilder::new(FieldType::String)
    }

    /// Create a new integer field schema
    pub fn integer() -> FieldSchemaBuilder {
        FieldSchemaBuilder::new(FieldType::Integer)
    }

    /// Create a new number field schema
    pub fn number() -> FieldSchemaBuilder {
        FieldSchemaBuilder::new(FieldType::Number)
    }

    /// Create a new boolean field schema
    pub fn boolean() -> FieldSchemaBuilder {
        FieldSchemaBuilder::new(FieldType::Boolean)
    }

    /// Create a new object field schema
    pub fn object(schema: Schema) -> FieldSchemaBuilder {
        let mut builder = FieldSchemaBuilder::new(FieldType::Object);
        builder.schema.object_schema = Some(Box::new(schema));
        builder
    }

    /// Create a new array field schema
    pub fn array(item_schema: FieldSchema) -> FieldSchemaBuilder {
        let mut builder = FieldSchemaBuilder::new(FieldType::Array);
        builder.schema.array_item_schema = Some(Box::new(item_schema));
        builder
    }

    /// Create a new any-type field schema
    pub fn any() -> FieldSchemaBuilder {
        FieldSchemaBuilder::new(FieldType::Any)
    }

    /// Validate a value against this field schema
    pub fn validate(&self, value: &Value, path: Option<String>) -> ValidationResult<()> {
        // Validate type
        match (&self.field_type, value) {
            (FieldType::String, Value::String(s)) => {
                // String validation logic
                if let Some(min_len) = self.min_length {
                    if s.len() < min_len {
                        return Err(ValidationError::TooShort(format!(
                            "String length {} is less than minimum {}", s.len(), min_len
                        )));
                    }
                }

                if let Some(max_len) = self.max_length {
                    if s.len() > max_len {
                        return Err(ValidationError::TooLong(format!(
                            "String length {} exceeds maximum {}", s.len(), max_len
                        )));
                    }
                }

                if let Some(pattern) = &self.pattern {
                    match regex::Regex::new(pattern) {
                        Ok(re) => {
                            if !re.is_match(s) {
                                return Err(ValidationError::PatternMismatch(format!(
                                    "String does not match pattern '{}'", pattern
                                )));
                            }
                        }
                        Err(e) => {
                            return Err(ValidationError::Generic(format!(
                                "Invalid regex pattern '{}': {}", pattern, e
                            )));
                        }
                    }
                }
            }
            (FieldType::Integer, Value::Number(n)) => {
                if !n.is_i64() {
                    return Err(ValidationError::InvalidType(format!(
                        "Expected integer, got floating point number"
                    )));
                }

                // Integer validation logic
                let num = n.as_f64().unwrap();
                if let Some(min) = self.minimum {
                    if num < min {
                        return Err(ValidationError::OutOfRange(format!(
                            "Value {} is less than minimum {}", num, min
                        )));
                    }
                }

                if let Some(max) = self.maximum {
                    if num > max {
                        return Err(ValidationError::OutOfRange(format!(
                            "Value {} exceeds maximum {}", num, max
                        )));
                    }
                }
            }
            (FieldType::Number, Value::Number(n)) => {
                // Number validation logic
                let num = n.as_f64().unwrap();
                if let Some(min) = self.minimum {
                    if num < min {
                        return Err(ValidationError::OutOfRange(format!(
                            "Value {} is less than minimum {}", num, min
                        )));
                    }
                }

                if let Some(max) = self.maximum {
                    if num > max {
                        return Err(ValidationError::OutOfRange(format!(
                            "Value {} exceeds maximum {}", num, max
                        )));
                    }
                }
            }
            (FieldType::Boolean, Value::Bool(_)) => {
                // Boolean validation - no additional checks needed
            }
            (FieldType::Object, Value::Object(_)) => {
                // Object validation logic
                if let Some(schema) = &self.object_schema {
                    return schema.validate_with_path(value, path);
                }
            }
            (FieldType::Array, Value::Array(items)) => {
                // Array validation logic
                if let Some(min_len) = self.min_length {
                    if items.len() < min_len {
                        return Err(ValidationError::TooShort(format!(
                            "Array length {} is less than minimum {}", items.len(), min_len
                        )));
                    }
                }

                if let Some(max_len) = self.max_length {
                    if items.len() > max_len {
                        return Err(ValidationError::TooLong(format!(
                            "Array length {} exceeds maximum {}", items.len(), max_len
                        )));
                    }
                }

                // Validate array items
                if let Some(item_schema) = &self.array_item_schema {
                    let mut errors = Vec::new();
                    
                    for (idx, item) in items.iter().enumerate() {
                        let item_path = match &path {
                            Some(p) => format!("{}[{}]", p, idx),
                            None => format!("[{}]", idx),
                        };
                        
                        if let Err(err) = item_schema.validate(item, Some(item_path.clone())) {
                            errors.push(err);
                        }
                    }

                    if !errors.is_empty() {
                        return Err(ValidationError::composite_at(errors, path.unwrap_or_default()));
                    }
                }
            }
            (FieldType::Any, _) => {
                // Any type validation - accept any type
            }
            _ => {
                return Err(ValidationError::InvalidType(format!(
                    "Expected {:?}, got {:?}", self.field_type, value
                )));
            }
        }

        // Check enum values if specified
        if let Some(enum_values) = &self.enum_values {
            if !enum_values.contains(value) {
                return Err(ValidationError::InvalidFormat(format!(
                    "Value {:?} is not one of the allowed values", value
                )));
            }
        }

        // Apply custom validator if specified
        if let Some(validator) = self.custom_validator {
            validator(value)?;
        }

        Ok(())
    }
}

/// Builder for field schemas
#[derive(Debug)]
pub struct FieldSchemaBuilder {
    schema: FieldSchema,
}

impl FieldSchemaBuilder {
    /// Create a new field schema builder
    fn new(field_type: FieldType) -> Self {
        Self {
            schema: FieldSchema {
                field_type,
                required: false,
                custom_validator: None,
                object_schema: None,
                array_item_schema: None,
                enum_values: None,
                pattern: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
            },
        }
    }

    /// Mark field as required
    pub fn required(mut self) -> Self {
        self.schema.required = true;
        self
    }

    /// Add custom validator function
    pub fn with_validator(mut self, validator: fn(&Value) -> ValidationResult<()>) -> Self {
        self.schema.custom_validator = Some(validator);
        self
    }

    /// Set enum values
    pub fn with_enum(mut self, values: Vec<Value>) -> Self {
        self.schema.enum_values = Some(values);
        self
    }

    /// Set pattern for string fields
    pub fn with_pattern(mut self, pattern: &str) -> Self {
        self.schema.pattern = Some(pattern.to_string());
        self
    }

    /// Set minimum value for numeric fields
    pub fn with_minimum(mut self, min: f64) -> Self {
        self.schema.minimum = Some(min);
        self
    }

    /// Set maximum value for numeric fields
    pub fn with_maximum(mut self, max: f64) -> Self {
        self.schema.maximum = Some(max);
        self
    }

    /// Set minimum length for strings/arrays
    pub fn with_min_length(mut self, min: usize) -> Self {
        self.schema.min_length = Some(min);
        self
    }

    /// Set maximum length for strings/arrays
    pub fn with_max_length(mut self, max: usize) -> Self {
        self.schema.max_length = Some(max);
        self
    }

    /// Build the field schema
    pub fn build(self) -> FieldSchema {
        self.schema
    }
}

/// Schema builder for constructing schemas fluently
#[derive(Debug)]
pub struct SchemaBuilder {
    schema: Schema,
}

impl SchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self {
            schema: Schema::new(),
        }
    }

    /// Add a required field
    pub fn required_field(mut self, name: &str, field_schema: FieldSchema) -> Self {
        let mut field = field_schema;
        field.required = true;
        self.schema.fields.insert(name.to_string(), field);
        self
    }

    /// Add an optional field
    pub fn optional_field(mut self, name: &str, field_schema: FieldSchema) -> Self {
        let mut field = field_schema;
        field.required = false;
        self.schema.fields.insert(name.to_string(), field);
        self
    }

    /// Allow additional fields not in the schema
    pub fn allow_additional_fields(mut self, allow: bool) -> Self {
        self.schema.allow_additional_fields = allow;
        self
    }

    /// Set validation configuration
    pub fn with_config(mut self, config: ValidationConfig) -> Self {
        self.schema.config = config;
        self
    }

    /// Build the schema
    pub fn build(self) -> Schema {
        self.schema
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_schema_validation() {
        let schema = Schema::builder()
            .required_field("name", FieldSchema::string().with_min_length(2).build())
            .required_field("age", FieldSchema::integer().with_minimum(0).with_maximum(120).build())
            .optional_field("email", FieldSchema::string().with_pattern(r"^\S+@\S+\.\S+$").build())
            .build();

        // Valid object
        let valid = json!({
            "name": "John",
            "age": 30,
            "email": "john@example.com"
        });
        assert!(schema.validate(&valid).is_ok());

        // Missing required field
        let missing_required = json!({
            "age": 30
        });
        assert!(schema.validate(&missing_required).is_err());

        // Invalid type
        let invalid_type = json!({
            "name": "John",
            "age": "thirty"
        });
        assert!(schema.validate(&invalid_type).is_err());

        // Invalid pattern
        let invalid_pattern = json!({
            "name": "John",
            "age": 30,
            "email": "not-an-email"
        });
        assert!(schema.validate(&invalid_pattern).is_err());

        // Invalid range
        let invalid_range = json!({
            "name": "John",
            "age": 200
        });
        assert!(schema.validate(&invalid_range).is_err());
    }

    #[test]
    fn test_nested_schema_validation() {
        let address_schema = Schema::builder()
            .required_field("street", FieldSchema::string().build())
            .required_field("city", FieldSchema::string().build())
            .optional_field("zipcode", FieldSchema::string().build())
            .build();

        let person_schema = Schema::builder()
            .required_field("name", FieldSchema::string().build())
            .required_field("address", FieldSchema::object(address_schema).build())
            .optional_field("hobbies", FieldSchema::array(FieldSchema::string().build())
                .with_max_length(5).build())
            .build();

        // Valid nested object
        let valid = json!({
            "name": "John",
            "address": {
                "street": "123 Main St",
                "city": "Anytown"
            },
            "hobbies": ["reading", "coding"]
        });
        assert!(person_schema.validate(&valid).is_ok());

        // Invalid nested object (missing required field in address)
        let invalid = json!({
            "name": "John",
            "address": {
                "street": "123 Main St"
            }
        });
        assert!(person_schema.validate(&invalid).is_err());

        // Too many items in array
        let too_many_hobbies = json!({
            "name": "John",
            "address": {
                "street": "123 Main St",
                "city": "Anytown"
            },
            "hobbies": ["reading", "coding", "gaming", "hiking", "swimming", "cycling"]
        });
        assert!(person_schema.validate(&too_many_hobbies).is_err());
    }
}