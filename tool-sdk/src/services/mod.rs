//! Service-specific client implementations
//!
//! This module contains client implementations for specific external services.

pub mod openai;
pub mod serpapi;
mod common;

pub use common::UserAgent;