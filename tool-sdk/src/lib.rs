//! # Tool SDK
//! 
//! A unified SDK for external service integrations in the Phoenix ORCH project.
//! 
//! This crate provides:
//! 
//! - Core abstractions for service clients with a unified interface
//! - Service-specific typed clients for external APIs
//! - Comprehensive error handling system
//! - Resilience patterns (retries, circuit breakers)
//! - Configuration management utilities
//!
//! ## Architecture
//!
//! The Tool SDK is designed around the following key abstractions:
//!
//! - `ServiceClient`: The base trait for all external service clients
//! - `RequestExecutor`: Handles the actual HTTP or other transport mechanism requests
//! - `AuthenticatedClient`: Adds authentication capabilities to clients
//! - `RateLimited`: Adds rate limiting capabilities to clients
//! - `Resilience`: Facade for resilience patterns (retry, circuit breaker, etc.)
//! - `ServiceError`: Comprehensive error handling system

// Re-export core modules
pub mod core;
pub use core::{
    ServiceClient, RequestExecutor, AuthenticatedClient, 
    RateLimited, Telemetry, ClientBuilder
};

// Re-export service-specific modules
pub mod services;
pub use services::{serpapi, openai};

// Re-export error handling
pub mod error;
pub use error::{ServiceError, ErrorContext, Result};

// Re-export resilience patterns
pub mod resilience;
pub use resilience::{Resilience, RetryExecutor, CircuitBreaker};

// Re-export configuration management
pub mod config;
pub use config::{ConfigProvider, ServiceConfig};

// Utility module for common functionality
mod util;

/// Create a new default client builder
pub fn client() -> core::ClientBuilder {
    core::ClientBuilder::new()
}

/// Create a pre-configured OpenAI client
pub fn openai_client() -> services::openai::OpenAIClient {
    services::openai::OpenAIClient::new()
}

/// Create a pre-configured SerpAPI client
pub fn serpapi_client() -> services::serpapi::SerpAPIClient {
    services::serpapi::SerpAPIClient::new()
}