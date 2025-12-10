//! Tests for configuration management functionality
//!
//! These tests verify that the configuration providers in the SDK work correctly.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::env;
    
    use crate::config::{ConfigProvider, EnvConfigProvider, MemoryConfigProvider, CompositeConfigProvider};
    
    #[test]
    fn test_memory_config_provider() {
        let mut provider = MemoryConfigProvider::new();
        provider.set("api_key", "test_key");
        provider.set("timeout", "30");
        provider.set("retry_enabled", "true");
        
        // Test string retrieval
        assert_eq!(provider.get_string("api_key").unwrap(), "test_key");
        
        // Test int retrieval
        assert_eq!(provider.get_int("timeout").unwrap(), 30);
        
        // Test bool retrieval
        assert_eq!(provider.get_bool("retry_enabled").unwrap(), true);
        
        // Test default values
        assert_eq!(provider.get_string_or("missing", "default"), "default");
        assert_eq!(provider.get_int_or("missing", 60), 60);
        assert_eq!(provider.get_bool_or("missing", false), false);
        
        // Test error case
        assert!(provider.get_string("missing").is_err());
        assert!(provider.get_int("api_key").is_err()); // Not an integer
    }
    
    #[test]
    fn test_env_config_provider() {
        // Set test environment variables
        env::set_var("TEST_SERVICE_API_KEY", "env_test_key");
        env::set_var("TEST_SERVICE_MAX_RETRIES", "5");
        env::set_var("TEST_SERVICE_DEBUG_MODE", "true");
        
        let provider = EnvConfigProvider::new()
            .with_prefix("TEST")
            .with_namespace("SERVICE");
        
        // Test string retrieval
        assert_eq!(provider.get_string("API_KEY").unwrap(), "env_test_key");
        
        // Test int retrieval
        assert_eq!(provider.get_int("MAX_RETRIES").unwrap(), 5);
        
        // Test bool retrieval
        assert_eq!(provider.get_bool("DEBUG_MODE").unwrap(), true);
        
        // Test key formatting
        assert_eq!(provider.format_key("api-key"), "TEST_SERVICE_API_KEY");
        
        // Test error case
        assert!(provider.get_string("NON_EXISTENT").is_err());
        
        // Clean up
        env::remove_var("TEST_SERVICE_API_KEY");
        env::remove_var("TEST_SERVICE_MAX_RETRIES");
        env::remove_var("TEST_SERVICE_DEBUG_MODE");
    }
    
    #[test]
    fn test_composite_config_provider() {
        // Create the first provider (memory-based)
        let mut memory_provider = MemoryConfigProvider::new();
        memory_provider.set("key1", "memory_value");
        memory_provider.set("common", "memory_value");
        
        // Set an environment variable for the second provider
        env::set_var("TEST_COMPOSITE_KEY2", "env_value");
        env::set_var("TEST_COMPOSITE_COMMON", "env_value");
        
        let env_provider = EnvConfigProvider::new()
            .with_prefix("TEST")
            .with_namespace("COMPOSITE");
        
        // Create a composite provider with memory provider first
        let mut composite = CompositeConfigProvider::new();
        composite.add_provider(memory_provider);
        composite.add_provider(env_provider);
        
        // Test that values from first provider are retrieved
        assert_eq!(composite.get_string("key1").unwrap(), "memory_value");
        
        // Test that values from second provider are retrieved when not in first
        assert_eq!(composite.get_string("KEY2").unwrap(), "env_value");
        
        // Test that values from first provider take precedence
        assert_eq!(composite.get_string("COMMON").unwrap(), "memory_value");
        
        // Test error case
        assert!(composite.get_string("NON_EXISTENT").is_err());
        
        // Clean up
        env::remove_var("TEST_COMPOSITE_KEY2");
        env::remove_var("TEST_COMPOSITE_COMMON");
    }
}