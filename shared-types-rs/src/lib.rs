pub mod secrets;

pub use secrets::{SecretManager, SecretError};

// Re-export types that might be needed by other crates
pub type Result<T> = std::result::Result<T, SecretError>;