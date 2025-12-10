//! Extended tests for configuration management
//!
//! These tests verify proper configuration behavior for service-specific
//! configurations, validation, and loading from different sources.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::env;
    use std::time::Duration;
    
    use crate::config::{
        ConfigProvider, EnvConfigProvider, MemoryConfigProvider,
        CompositeConfigProvider, ServiceConfig, OpenAIConfig, SerpAPIConfig
    };
    use crate::error::ServiceError;
    
    /// Tests for service-specific configurations
    mod service_config_tests {
        use super::*;
        
        #[test]
        fn test_openai_config_defaults() {
            // Test default configuration
            let config = OpenAIConfig::default();
            
            // Check default values
            assert_eq!(config.base_url, "https://api.openai.com/v1");
            assert_eq!(config.api_key, "");
            assert_eq!(config.timeout_seconds, 30);
            assert!(config.org_id.is_none());
        }
        
        #[test]
        fn test_openai_config_from_provider() {
            // Set up environment variables
            env::set_var("OPENAI_API_KEY", "test_key_123");
            env::set_var("OPENAI_TIMEOUT", "60");
            env::set_var("OPENAI_ORG_ID", "org-12345");
            
            // Create provider
            let env_provider = EnvConfigProvider::new()
                .with_prefix("OPENAI");
            
            // Create config from provider
            let config = OpenAIConfig::from_provider(&env_provider).unwrap();
            
            // Verify values were loaded correctly
            assert_eq!(config.api_key, "test_key_123");
            assert_eq!(config.timeout_seconds, 60);
            assert_eq!(config.org_id, Some("org-12345".to_string()));
            
            // Clean up
            env::remove_var("OPENAI_API_KEY");
            env::remove_var("OPENAI_TIMEOUT");
            env::remove_var("OPENAI_ORG_ID");
        }
        
        #[test]
        fn test_serpapi_config_defaults() {
            // Test default configuration
            let config = SerpAPIConfig::default();
            
            // Check default values
            assert_eq!(config.base_url, "https://serpapi.com");
            assert_eq!(config.api_key, "");
            assert_eq!(config.timeout_seconds, 30);
            assert_eq!(config.default_engine, "google");
        }
        
        #[test]
        fn test_serpapi_config_from_provider() {
            // Set up environment variables
            env::set_var("SERPAPI_API_KEY", "serp_api_key_123");
            env::set_var("SERPAPI_TIMEOUT", "45");
            env::set_var("SERPAPI_DEFAULT_ENGINE", "bing");
            
            // Create provider
            let env_provider = EnvConfigProvider::new()
                .with_prefix("SERPAPI");
            
            // Create config from provider
            let config = SerpAPIConfig::from_provider(&env_provider).unwrap();
            
            // Verify values were loaded correctly
            assert_eq!(config.api_key, "serp_api_key_123");
            assert_eq!(config.timeout_seconds, 45);
            assert_eq!(config.default_engine, "bing");
            
            // Clean up
            env::remove_var("SERPAPI_API_KEY");
            env::remove_var("SERPAPI_TIMEOUT");
            env::remove_var("SERPAPI_DEFAULT_ENGINE");
        }
    }
    
    /// Tests for configuration validation
    mod config_validation_tests {
        use super::*;
        
        #[test]
        fn test_openai_config_validation() {
            // Test empty API key validation
            let mut config = OpenAIConfig::default();
            let result = config.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("OpenAI API key"));
            
            // Test valid config
            config.api_key = "valid_key".to_string();
            assert!(config.validate().is_ok());
            
            // Test timeout validation
            config.timeout_seconds = 0;
            let result = config.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("timeout"));
            
            config.timeout_seconds = 180; // Valid timeout
            assert!(config.validate().is_ok());
            
            // Test base URL validation
            config.base_url = "".to_string();
            let result = config.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("base URL"));
            
            config.base_url = "https://custom.openai.api".to_string(); // Valid URL
            assert!(config.validate().is_ok());
        }
        
        #[test]
        fn test_serpapi_config_validation() {
            // Test empty API key validation
            let mut config = SerpAPIConfig::default();
            let result = config.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("SerpAPI API key"));
            
            // Test valid config
            config.api_key = "valid_key".to_string();
            assert!(config.validate().is_ok());
            
            // Test timeout validation
            config.timeout_seconds = 0;
            let result = config.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("timeout"));
            
            config.timeout_seconds = 180; // Valid timeout
            assert!(config.validate().is_ok());
            
            // Test default engine validation
            config.default_engine = "".to_string();
            let result = config.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("default engine"));
            
            // Test with valid engine
            config.default_engine = "google".to_string();
            assert!(config.validate().is_ok());
        }
    }
    
    /// Tests for config loading from environment
    mod env_config_loading_tests {
        use super::*;
        
        #[test]
        fn test_env_provider_with_namespaces() {
            // Set up environment variables with different namespaces
            env::set_var("TOOL_SDK_OPENAI_API_KEY", "openai_key_from_namespaced_env");
            env::set_var("TOOL_SDK_SERPAPI_API_KEY", "serpapi_key_from_namespaced_env");
            
            // Create provider with prefix and namespace
            let provider = EnvConfigProvider::new()
                .with_prefix("TOOL_SDK");
            
            // Load different configs using the same provider but different namespaces
            let openai_config = OpenAIConfig::from_provider(&provider).unwrap();
            let serpapi_config = SerpAPIConfig::from_provider(&provider).unwrap();
            
            // Verify correct values were loaded
            assert_eq!(openai_config.api_key, "openai_key_from_namespaced_env");
            assert_eq!(serpapi_config.api_key, "serpapi_key_from_namespaced_env");
            
            // Clean up
            env::remove_var("TOOL_SDK_OPENAI_API_KEY");
            env::remove_var("TOOL_SDK_SERPAPI_API_KEY");
        }
        
        #[test]
        fn test_env_provider_fallback() {
            // Set up environment variables
            env::set_var("OPENAI_API_KEY", "default_key");
            
            // Set up a provider without the required values
            let provider = EnvConfigProvider::new()
                .with_prefix("MISSING");
            
            // Create config with defaults
            let config = OpenAIConfig::from_provider(&provider).unwrap_or_else(|_| {
                // Should fall back to this default config
                let mut default_config = OpenAIConfig::default();
                default_config.api_key = "fallback_key".to_string();
                default_config
            });
            
            // Verify fallback values were used
            assert_eq!(config.api_key, "fallback_key");
            
            // Clean up
            env::remove_var("OPENAI_API_KEY");
        }
        
        #[test]
        fn test_composite_provider_for_service_configs() {
            // Set up memory and environment providers with different values
            let mut memory_provider = MemoryConfigProvider::new();
            memory_provider.set("OPENAI_API_KEY", "memory_key");
            memory_provider.set("OPENAI_TIMEOUT", "60");
            
            env::set_var("OPENAI_ORG_ID", "env_org_id");
            let env_provider = EnvConfigProvider::new();
            
            // Create composite provider that prioritizes memory over env
            let mut composite = CompositeConfigProvider::new();
            composite.add_provider(memory_provider);
            composite.add_provider(env_provider);
            
            // Load config from composite provider
            let config = OpenAIConfig::from_provider(&composite).unwrap();
            
            // Verify values from both providers were correctly used
            assert_eq!(config.api_key, "memory_key"); // From memory provider
            assert_eq!(config.timeout_seconds, 60);   // From memory provider
            assert_eq!(config.org_id, Some("env_org_id".to_string())); // From env provider
            
            // Clean up
            env::remove_var("OPENAI_ORG_ID");
        }
    }
    
    /// Tests for custom configurations
    mod custom_config_tests {
        use super::*;
        
        /// Custom configuration example
        #[derive(Debug, Clone)]
        struct CustomServiceConfig {
            service_name: String,
            endpoint_url: String,
            request_timeout: Duration,
            max_connections: usize,
            features: HashMap<String, bool>,
        }
        
        impl Default for CustomServiceConfig {
            fn default() -> Self {
                let mut features = HashMap::new();
                features.insert("caching".to_string(), true);
                features.insert("metrics".to_string(), false);
                
                Self {
                    service_name: "custom_service".to_string(),
                    endpoint_url: "https://api.custom-service.com".to_string(),
                    request_timeout: Duration::from_secs(30),
                    max_connections: 10,
                    features,
                }
            }
        }
        
        impl ServiceConfig for CustomServiceConfig {
            fn from_provider(provider: &dyn ConfigProvider) -> Result<Self, ServiceError> {
                let mut config = Self::default();
                
                // Load basic string settings
                if let Ok(name) = provider.get_string("CUSTOM_SERVICE_NAME") {
                    config.service_name = name;
                }
                
                if let Ok(url) = provider.get_string("CUSTOM_ENDPOINT_URL") {
                    config.endpoint_url = url;
                }
                
                // Load numeric settings
                if let Ok(timeout) = provider.get_int("CUSTOM_TIMEOUT") {
                    config.request_timeout = Duration::from_secs(timeout as u64);
                }
                
                if let Ok(connections) = provider.get_int("CUSTOM_MAX_CONNECTIONS") {
                    config.max_connections = connections as usize;
                }
                
                // Load feature flags
                if let Ok(caching) = provider.get_bool("CUSTOM_FEATURE_CACHING") {
                    config.features.insert("caching".to_string(), caching);
                }
                
                if let Ok(metrics) = provider.get_bool("CUSTOM_FEATURE_METRICS") {
                    config.features.insert("metrics".to_string(), metrics);
                }
                
                // Add any other features found with CUSTOM_FEATURE_ prefix
                // This is a more advanced pattern for dynamic feature discovery
                
                Ok(config)
            }
            
            fn validate(&self) -> Result<(), ServiceError> {
                // Basic validation
                if self.service_name.is_empty() {
                    return Err(ServiceError::validation("Service name cannot be empty"));
                }
                
                if self.endpoint_url.is_empty() {
                    return Err(ServiceError::validation("Endpoint URL cannot be empty"));
                }
                
                if self.request_timeout.as_secs() == 0 {
                    return Err(ServiceError::validation("Request timeout must be greater than zero"));
                }
                
                if self.max_connections == 0 {
                    return Err(ServiceError::validation("Max connections must be greater than zero"));
                }
                
                Ok(())
            }
        }
        
        #[test]
        fn test_custom_service_config() {
            // Set up environment variables
            env::set_var("CUSTOM_SERVICE_NAME", "test_service");
            env::set_var("CUSTOM_ENDPOINT_URL", "https://test.api.com");
            env::set_var("CUSTOM_TIMEOUT", "45");
            env::set_var("CUSTOM_MAX_CONNECTIONS", "5");
            env::set_var("CUSTOM_FEATURE_CACHING", "false");
            env::set_var("CUSTOM_FEATURE_METRICS", "true");
            
            // Create provider
            let provider = EnvConfigProvider::new();
            
            // Create config from provider
            let config = CustomServiceConfig::from_provider(&provider).unwrap();
            
            // Verify values were loaded correctly
            assert_eq!(config.service_name, "test_service");
            assert_eq!(config.endpoint_url, "https://test.api.com");
            assert_eq!(config.request_timeout, Duration::from_secs(45));
            assert_eq!(config.max_connections, 5);
            assert_eq!(config.features.get("caching"), Some(&false));
            assert_eq!(config.features.get("metrics"), Some(&true));
            
            // Clean up
            env::remove_var("CUSTOM_SERVICE_NAME");
            env::remove_var("CUSTOM_ENDPOINT_URL");
            env::remove_var("CUSTOM_TIMEOUT");
            env::remove_var("CUSTOM_MAX_CONNECTIONS");
            env::remove_var("CUSTOM_FEATURE_CACHING");
            env::remove_var("CUSTOM_FEATURE_METRICS");
        }
        
        #[test]
        fn test_custom_config_validation() {
            // Create valid config
            let config = CustomServiceConfig::default();
            assert!(config.validate().is_ok());
            
            // Test invalid service name
            let mut invalid_config = config.clone();
            invalid_config.service_name = "".to_string();
            assert!(invalid_config.validate().is_err());
            
            // Test invalid endpoint URL
            let mut invalid_config = config.clone();
            invalid_config.endpoint_url = "".to_string();
            assert!(invalid_config.validate().is_err());
            
            // Test invalid timeout
            let mut invalid_config = config.clone();
            invalid_config.request_timeout = Duration::from_secs(0);
            assert!(invalid_config.validate().is_err());
            
            // Test invalid max connections
            let mut invalid_config = config;
            invalid_config.max_connections = 0;
            assert!(invalid_config.validate().is_err());
        }
    }
}