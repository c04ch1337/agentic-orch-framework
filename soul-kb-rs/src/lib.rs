//! Soul KB Service
//! Core implementation of the Soul Knowledge Base service

#[macro_use]
extern crate input_validation_rs;

// Re-export proto modules
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

// Local modules
mod validation;
mod validation_macros;

// Re-exports
pub use validation::*;
pub use validation_macros::*;

// Re-export core types
pub use agi_core::{
    CoreValue, EthicsCheckRequest, StoreValueRequest,
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

// Re-export macros
#[macro_export]
macro_rules! validate {
    ($input:expr, $($validator:expr),+) => {{
        let mut result = Ok(());
        $(
            if let Err(e) = $validator.validate(&$input) {
                result = Err(e);
                break;
            }
        )+
        result
    }};
}

#[macro_export]
macro_rules! validate_range {
    ($value:expr, $min:expr, $max:expr) => {{
        if $value < $min || $value > $max {
            Err(ValidationError::OutOfRange(format!(
                "Value {} must be between {} and {}",
                $value, $min, $max
            )))
        } else {
            Ok(())
        }
    }};
}

#[macro_export]
macro_rules! validate_length {
    ($value:expr, $max:expr) => {{
        if $value.len() > $max {
            Err(ValidationError::TooLong(format!(
                "Length {} exceeds maximum {}",
                $value.len(),
                $max
            )))
        } else {
            Ok(())
        }
    }};
}