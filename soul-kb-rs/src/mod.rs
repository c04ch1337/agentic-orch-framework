//! Soul KB Service modules

pub mod validation;
mod validation_macros;

// Re-export validation functions and macros
pub use validation::*;
// These macros are `#[macro_export]`-ed and live at the crate root.
pub use crate::{validate, validate_length, validate_range};

// Re-export core types
pub use crate::agi_core::{
    CoreValue, EthicsCheckRequest, StoreValueRequest,
    KnowledgeFragment, KnowledgeQuery, StoreResponse,
};

// Re-export validation types
pub use input_validation_rs::{ValidationError, ValidationResult};

// Convenience re-exports for call sites
pub use input_validation_rs::{sanitizers, validators};
