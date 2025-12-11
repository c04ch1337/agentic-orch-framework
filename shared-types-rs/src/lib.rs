pub mod config;
pub mod secrets;

pub use config::{ConfigError, PhoenixConfig};
pub use secrets::{SecretError, SecretManager};

// Re-export types that might be needed by other crates
pub type Result<T> = std::result::Result<T, SecretError>;
