pub mod secrets;
pub mod config;

pub use secrets::{SecretManager, SecretError};
pub use config::{PhoenixConfig, ConfigError};

// Re-export types that might be needed by other crates
pub type Result<T> = std::result::Result<T, SecretError>;