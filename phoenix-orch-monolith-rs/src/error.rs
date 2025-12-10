use thiserror::Error;
use windows_service::Error as WindowsServiceError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Windows service error: {0}")]
    WindowsService(#[from] WindowsServiceError),

    #[error("Configuration error: {0}")]
    Config(#[from] config_rs::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Emergency system error: {0}")]
    Emergency(String),

    #[error("Service initialization error: {0}")]
    ServiceInit(String),

    #[error("Data integrity error: {0}")]
    DataIntegrity(String),

    #[error("Secret management error: {0}")]
    SecretManagement(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("Service communication error: {0}")]
    ServiceCommunication(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Error::Emergency(_) |
            Error::DataIntegrity(_) |
            Error::SecurityViolation(_) |
            Error::ResourceLimit(_)
        )
    }

    pub fn requires_rollback(&self) -> bool {
        matches!(
            self,
            Error::DataIntegrity(_) |
            Error::Emergency(_)
        )
    }

    pub fn log_level(&self) -> tracing::Level {
        match self {
            Error::Emergency(_) |
            Error::SecurityViolation(_) |
            Error::ResourceLimit(_) |
            Error::DataIntegrity(_) => tracing::Level::ERROR,
            
            Error::ServiceInit(_) |
            Error::ServiceCommunication(_) |
            Error::SecretManagement(_) => tracing::Level::WARN,
            
            _ => tracing::Level::INFO,
        }
    }
}