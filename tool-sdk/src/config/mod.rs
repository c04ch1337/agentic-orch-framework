//! Configuration management for service clients
//!
//! This module provides utilities for loading and validating configuration
//! for external service clients, with support for environment variables.

use std::env;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::str::FromStr;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::error::{Result, ServiceError};
use once_cell::sync::Lazy;

/// Base trait for configuration providers
pub trait ConfigProvider: Send + Sync {
    /// Get a string configuration value
    fn get_string(&self, key: &str) -> Result<String>;
}

/// Extension methods for configuration providers
pub trait ConfigProviderExt: ConfigProvider {
    /// Get an integer configuration value
    fn get_int(&self, key: &str) -> Result<i64> {
        let value = self.get_string(key)?;
        value.parse::<i64>()
            .map_err(|e| ServiceError::configuration(format!("Invalid integer for key {}: {}", key, e)))
    }
    
    /// Get a float configuration value
    fn get_float(&self, key: &str) -> Result<f64> {
        let value = self.get_string(key)?;
        value.parse::<f64>()
            .map_err(|e| ServiceError::configuration(format!("Invalid float for key {}: {}", key, e)))
    }
    
    /// Get a boolean configuration value
    fn get_bool(&self, key: &str) -> Result<bool> {
        let value = self.get_string(key)?;
        match value.to_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => Ok(true),
            "false" | "no" | "0" | "off" => Ok(false),
            _ => Err(ServiceError::configuration(format!("Invalid boolean value for key {}: {}", key, value))),
        }
    }
    
    /// Get a string configuration value with a default
    fn get_string_or(&self, key: &str, default: &str) -> String {
        self.get_string(key).unwrap_or_else(|_| default.to_string())
    }
    
    /// Get an integer configuration value with a default
    fn get_int_or(&self, key: &str, default: i64) -> i64 {
        self.get_int(key).unwrap_or(default)
    }
    
    /// Get a float configuration value with a default
    fn get_float_or(&self, key: &str, default: f64) -> f64 {
        self.get_float(key).unwrap_or(default)
    }
    
    /// Get a boolean configuration value with a default
    fn get_bool_or(&self, key: &str, default: bool) -> bool {
        self.get_bool(key).unwrap_or(default)
    }
}

impl<T: ConfigProvider> ConfigProviderExt for T {}

/// Generic configuration provider trait
pub trait GenericConfigProvider: ConfigProvider {
    /// Get a typed configuration value by parsing from string
    fn get<T>(&self, key: &str) -> Result<T>
    where
        T: FromStr,
        <T as FromStr>::Err: std::fmt::Display,
    {
        let value = self.get_string(key)?;
        value.parse::<T>()
            .map_err(|e| ServiceError::configuration(format!("Invalid value for key {}: {}", key, e)))
    }
    
    /// Get a typed configuration with a default value
    fn get_or<T>(&self, key: &str, default: T) -> T
    where
        T: FromStr + Clone,
        <T as FromStr>::Err: std::fmt::Display,
    {
        self.get::<T>(key).unwrap_or_else(|_| default)
    }
}

/// Environment variable based configuration provider
#[derive(Debug, Clone)]
pub struct EnvConfigProvider {
    /// Optional prefix for environment variables
    prefix: Option<String>,
    
    /// Optional namespace for variables (e.g., "OPENAI", "SERPAPI")
    namespace: Option<String>,
}

impl Default for EnvConfigProvider {
    fn default() -> Self {
        Self {
            prefix: None,
            namespace: None,
        }
    }
}

impl EnvConfigProvider {
    /// Create a new environment variable config provider
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set a prefix for environment variables
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }
    
    /// Set a namespace for environment variables
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }
    
    /// Format a configuration key as an environment variable
    fn format_key(&self, key: &str) -> String {
        let mut env_key = String::new();
        
        // Add prefix if specified
        if let Some(ref prefix) = self.prefix {
            env_key.push_str(prefix);
            env_key.push('_');
        }
        
        // Add namespace if specified
        if let Some(ref namespace) = self.namespace {
            env_key.push_str(namespace);
            env_key.push('_');
        }
        
        // Add the key itself (uppercase and replace non-alphanumeric with underscores)
        env_key.push_str(&key.to_uppercase().replace(|c: char| !c.is_ascii_alphanumeric(), "_"));
        
        env_key
    }
}

impl ConfigProvider for EnvConfigProvider {
    fn get_string(&self, key: &str) -> Result<String> {
        let env_key = self.format_key(key);
        
        env::var(&env_key)
            .map_err(|e| {
                match e {
                    env::VarError::NotPresent => {
                        ServiceError::configuration(format!("Environment variable not set: {}", env_key))
                    }
                    env::VarError::NotUnicode(_) => {
                        ServiceError::configuration(format!("Environment variable is not valid unicode: {}", env_key))
                    }
                }
            })
    }
}

/// In-memory config provider for testing or static configuration
#[derive(Debug, Clone)]
pub struct MemoryConfigProvider {
    /// Configuration values
    values: HashMap<String, String>,
}

impl MemoryConfigProvider {
    /// Create a new empty memory config provider
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }
    
    /// Create a memory config provider with initial values
    pub fn with_values(values: HashMap<String, String>) -> Self {
        Self { values }
    }
    
    /// Set a configuration value
    pub fn set<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: ToString,
    {
        self.values.insert(key.into(), value.to_string());
    }
}

impl ConfigProvider for MemoryConfigProvider {
    fn get_string(&self, key: &str) -> Result<String> {
        self.values
            .get(key)
            .cloned()
            .ok_or_else(|| ServiceError::configuration(format!("Configuration key not found: {}", key)))
    }
}

/// A composite config provider that tries multiple providers in order
#[derive(Debug, Clone)]
pub struct CompositeConfigProvider<P: ConfigProvider> {
    /// Ordered list of config providers to try
    providers: Vec<P>,
}

impl<P: ConfigProvider> CompositeConfigProvider<P> {
    /// Create a new composite config provider
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }
    
    /// Add a provider to the chain
    pub fn add_provider(&mut self, provider: P) {
        self.providers.push(provider);
    }
    
    /// Create a new provider with an initial list
    pub fn with_providers(providers: Vec<P>) -> Self {
        Self { providers }
    }
}

impl<P: ConfigProvider> ConfigProvider for CompositeConfigProvider<P> {
    fn get_string(&self, key: &str) -> Result<String> {
        for provider in &self.providers {
            match provider.get_string(key) {
                Ok(value) => return Ok(value),
                Err(_) => continue,
            }
        }
        
        Err(ServiceError::configuration(format!("Configuration key not found in any provider: {}", key)))
    }
}

/// Global default configuration provider
pub static DEFAULT_PROVIDER: Lazy<Arc<EnvConfigProvider>> = Lazy::new(|| {
    Arc::new(EnvConfigProvider::new().with_prefix("PHOENIX"))
});

/// Trait for service-specific configuration
pub trait ServiceConfig: Debug + Send + Sync {
    /// Validate this configuration
    fn validate(&self) -> Result<()>;
    
    /// Service name
    fn service_name(&self) -> &str;
}

/// Base configuration for OpenAI API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    /// API key
    pub api_key: String,
    
    /// Organization ID (optional)
    pub org_id: Option<String>,
    
    /// Base URL (can be changed for proxies)
    pub base_url: String,
    
    /// Timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            org_id: None,
            base_url: "https://api.openai.com/v1".to_string(),
            timeout_seconds: 30,
        }
    }
}

impl OpenAIConfig {
    /// Load configuration from a config provider
    pub fn from_provider<P: ConfigProvider + ConfigProviderExt>(provider: &P) -> Result<Self> {
        let api_key = provider.get_string("openai_api_key")?;
        let org_id = provider.get_string("openai_org_id").ok();
        let base_url = provider.get_string_or("openai_base_url", "https://api.openai.com/v1");
        let timeout_seconds = provider.get_int_or("openai_timeout_seconds", 30) as u64;
        
        let config = Self {
            api_key,
            org_id,
            base_url,
            timeout_seconds,
        };
        
        config.validate()?;
        Ok(config)
    }
}

impl ServiceConfig for OpenAIConfig {
    fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(ServiceError::configuration("OpenAI API key is required"));
        }
        
        if self.base_url.is_empty() {
            return Err(ServiceError::configuration("OpenAI base URL is required"));
        }
        
        Ok(())
    }
    
    fn service_name(&self) -> &str {
        "openai"
    }
}

/// Base configuration for SerpAPI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerpAPIConfig {
    /// API key
    pub api_key: String,
    
    /// Base URL
    pub base_url: String,
    
    /// Default search engine to use
    pub default_engine: String,
    
    /// Timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for SerpAPIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://serpapi.com".to_string(),
            default_engine: "google".to_string(),
            timeout_seconds: 30,
        }
    }
}

impl SerpAPIConfig {
    /// Load configuration from a config provider
    pub fn from_provider<P: ConfigProvider + ConfigProviderExt>(provider: &P) -> Result<Self> {
        let api_key = provider.get_string("serpapi_api_key")?;
        let base_url = provider.get_string_or("serpapi_base_url", "https://serpapi.com");
        let default_engine = provider.get_string_or("serpapi_default_engine", "google");
        let timeout_seconds = provider.get_int_or("serpapi_timeout_seconds", 30) as u64;
        
        let config = Self {
            api_key,
            base_url,
            default_engine,
            timeout_seconds,
        };
        
        config.validate()?;
        Ok(config)
    }
}

impl ServiceConfig for SerpAPIConfig {
    fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(ServiceError::configuration("SerpAPI key is required"));
        }
        
        if self.base_url.is_empty() {
            return Err(ServiceError::configuration("SerpAPI base URL is required"));
        }
        
        if self.default_engine.is_empty() {
            return Err(ServiceError::configuration("SerpAPI default engine is required"));
        }
        
        Ok(())
    }
    
    fn service_name(&self) -> &str {
        "serpapi"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_config_provider() {
        let mut provider = MemoryConfigProvider::new();
        provider.set("key1", "value1");
        provider.set("key2", "123");
        
        assert_eq!(provider.get_string("key1").unwrap(), "value1");
        assert_eq!(provider.get_int("key2").unwrap(), 123);
        assert!(provider.get_string("key3").is_err());
    }
    
    #[test]
    fn test_env_config_provider() {
        let provider = EnvConfigProvider::new()
            .with_prefix("TEST")
            .with_namespace("CONFIG");
        
        // Format key test
        assert_eq!(provider.format_key("api_key"), "TEST_CONFIG_API_KEY");
        assert_eq!(provider.format_key("base-url"), "TEST_CONFIG_BASE_URL");
    }
    
    #[test]
    fn test_composite_config_provider() {
        let mut mem1 = MemoryConfigProvider::new();
        mem1.set("key1", "value1");
        
        let mut mem2 = MemoryConfigProvider::new();
        mem2.set("key2", "value2");
        
        let mut provider = CompositeConfigProvider::new();
        provider.add_provider(mem1);
        provider.add_provider(mem2);
        
        assert_eq!(provider.get_string("key1").unwrap(), "value1");
        assert_eq!(provider.get_string("key2").unwrap(), "value2");
        assert!(provider.get_string("key3").is_err());
    }
    
    #[test]
    fn test_openai_config() {
        let mut provider = MemoryConfigProvider::new();
        provider.set("openai_api_key", "test_api_key");
        provider.set("openai_base_url", "https://test.openai.com");
        
        let config = OpenAIConfig::from_provider(&provider).unwrap();
        assert_eq!(config.api_key, "test_api_key");
        assert_eq!(config.base_url, "https://test.openai.com");
        assert_eq!(config.timeout_seconds, 30); // Default value
        
        // Test validation
        let config = OpenAIConfig {
            api_key: "".to_string(),
            ..OpenAIConfig::default()
        };
        assert!(config.validate().is_err());
    }
}