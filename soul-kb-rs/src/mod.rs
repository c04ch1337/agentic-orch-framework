//! Soul KB Service modules

pub mod validation;
mod validation_macros;

// Re-export validation functions and macros
pub use validation::*;
pub use validation_macros::{validate, validate_length, validate_range};

// Re-export core types
pub use crate::agi_core::{
    CoreValue, EthicsCheckRequest, StoreValueRequest,
    KnowledgeFragment, KnowledgeQuery, StoreResponse,
};

// Re-export validation types
pub use input_validation_rs::{
    ValidationResult,
    ValidationError,
    sanitizers::StringSanitizer,
    validators::{
        numeric::NumericValidation,
        security::SecurityValidation,
        string::StringValidation,
    },
};