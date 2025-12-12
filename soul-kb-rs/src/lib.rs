//! Soul KB Service
//! Core implementation of the Soul Knowledge Base service

// `input-validation-rs` exposes functions/modules; no macro imports needed here.

// Re-export proto modules
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

// Local modules
#[macro_use]
mod validation_macros;
mod validation;

// Re-exports
pub use validation::*;

// Re-export core types
pub use agi_core::{
    CoreValue, EthicsCheckRequest, StoreValueRequest,
};

// Re-export validation types
pub use input_validation_rs::{
    ValidationResult,
    ValidationError,
};
