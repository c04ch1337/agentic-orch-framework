//! config-rs/lib.rs
//! Shared configuration utilities for consistent service configuration
//! Provides standardized functions for port/address management

use std::env;
use std::net::SocketAddr;

/// Get service port from environment variables with proper fallback
/// 
/// # Arguments
/// * `service_name` - The name of the service (e.g., "ORCHESTRATOR", "LLM")
/// * `default_port` - The default port to use if not specified in environment
/// 
/// # Returns
/// The port number to use for the service
pub fn get_service_port(service_name: &str, default_port: u16) -> u16 {
    let var_name = format!("{}_SERVICE_PORT", service_name.to_uppercase());
    env::var(&var_name)
        .unwrap_or_else(|_| default_port.to_string())
        .parse::<u16>()
        .unwrap_or_else(|_| {
            log::warn!("Invalid port in {}, using default {}", var_name, default_port);
            default_port
        })
}

/// Create a SocketAddr for binding a service
/// 
/// # Arguments
/// * `service_name` - The name of the service (e.g., "ORCHESTRATOR", "LLM")
/// * `default_port` - The default port to use if not specified in environment
/// 
/// # Returns
/// A SocketAddr configured with the appropriate bind address and port
pub fn get_bind_address(service_name: &str, default_port: u16) -> SocketAddr {
    let var_name = format!("{}_SERVICE_ADDR", service_name.to_uppercase());
    
    // Check if there's a full address override
    if let Ok(addr_str) = env::var(&var_name) {
        if let Ok(addr) = addr_str.parse::<SocketAddr>() {
            return addr;
        } else {
            // Check if it's in http://host:port format
            if addr_str.starts_with("http://") || addr_str.starts_with("https://") {
                let addr_parts = addr_str.split("://").collect::<Vec<&str>>();
                if addr_parts.len() > 1 {
                    if let Ok(addr) = addr_parts[1].parse::<SocketAddr>() {
                        return addr;
                    }
                }
            }
            log::warn!("Invalid address format in {}, using default", var_name);
        }
    }
    
    // Use the port from environment or default
    let port = get_service_port(service_name, default_port);
    format!("0.0.0.0:{}", port).parse().unwrap()
}

/// Get client connection address for connecting to a service
/// 
/// # Arguments
/// * `service_name` - The name of the service (e.g., "ORCHESTRATOR", "LLM")
/// * `default_port` - The default port to use if not specified in environment
/// * `host` - Optional host to use if not specified in environment (default: "localhost")
/// 
/// # Returns
/// A connection string for the client to connect to the service
pub fn get_client_address(service_name: &str, default_port: u16, host: Option<&str>) -> String {
    let addr_var_name = format!("{}_SERVICE_ADDR", service_name.to_uppercase());
    let port_var_name = format!("{}_SERVICE_PORT", service_name.to_uppercase());
    
    // First check if there's a full address override
    if let Ok(addr) = env::var(&addr_var_name) {
        return addr;
    }
    
    // Then check for port override
    let port = env::var(&port_var_name)
        .unwrap_or_else(|_| default_port.to_string())
        .parse::<u16>()
        .unwrap_or(default_port);
    
    // Build the address with the host (default to localhost if not provided)
    let host = host.unwrap_or("localhost");
    format!("http://{}:{}", host, port)
}

/// Get service name for logging and monitoring
/// 
/// # Arguments
/// * `service_name` - The name of the service (e.g., "ORCHESTRATOR", "LLM")
/// 
/// # Returns
/// A formatted service name suitable for logging
pub fn get_formatted_service_name(service_name: &str) -> String {
    match service_name {
        "ORCHESTRATOR" => "orchestrator-service".to_string(),
        "LLM" => "llm-service".to_string(),
        "DATA_ROUTER" => "data-router-service".to_string(),
        "TOOLS" => "tools-service".to_string(),
        "SAFETY" => "safety-service".to_string(),
        "LOGGING" => "logging-service".to_string(),
        "MIND_KB" => "mind-kb-service".to_string(),
        "BODY_KB" => "body-kb-service".to_string(),
        "HEART_KB" => "heart-kb-service".to_string(),
        "SOCIAL_KB" => "social-kb-service".to_string(),
        "SOUL_KB" => "soul-kb-service".to_string(),
        "EXECUTOR" => "executor-service".to_string(),
        "CONTEXT_MANAGER" => "context-manager-service".to_string(),
        "REFLECTION" => "reflection-service".to_string(),
        "SCHEDULER" => "scheduler-service".to_string(),
        "AGENT_REGISTRY" => "agent-registry-service".to_string(),
        "RED_TEAM" => "red-team-service".to_string(),
        "BLUE_TEAM" => "blue-team-service".to_string(),
        "SECRETS" => "secrets-service".to_string(),
        "AUTH" => "auth-service".to_string(),
        "API_GATEWAY" => "api-gateway".to_string(),
        _ => format!("{}-service", service_name.to_lowercase()),
    }
}

/// Service definition with port information
#[derive(Debug, Clone)]
pub struct ServiceDefinition {
    pub name: String,
    pub default_port: u16,
    pub display_name: String,
}

/// Get default port for a specific service
/// 
/// # Arguments
/// * `service_name` - The name of the service (e.g., "ORCHESTRATOR", "LLM")
/// 
/// # Returns
/// The default port for the service
pub fn get_default_port(service_name: &str) -> u16 {
    match service_name.to_uppercase().as_str() {
        "ORCHESTRATOR" => 50051,
        "DATA_ROUTER" => 50052,
        "LLM" => 50053,
        "TOOLS" => 50054,
        "SAFETY" => 50055,
        "LOGGING" => 50056,
        "MIND_KB" => 50057,
        "BODY_KB" => 50058,
        "HEART_KB" => 50059,
        "SOCIAL_KB" => 50060,
        "SOUL_KB" => 50061,
        "EXECUTOR" => 50062,
        "CONTEXT_MANAGER" => 50064,
        "REFLECTION" => 50065,
        "SCHEDULER" => 50066,
        "AGENT_REGISTRY" => 50067,
        "RED_TEAM" => 50068,
        "BLUE_TEAM" => 50069,
        "SECRETS" => 50080,
        "AUTH" => 50090,
        "API_GATEWAY" => 8282,
        _ => 50100, // Unknown services start at 50100
    }
}

/// Get all service definitions
pub fn get_all_services() -> Vec<ServiceDefinition> {
    vec![
        ServiceDefinition {
            name: "ORCHESTRATOR".to_string(),
            default_port: 50051,
            display_name: "Orchestrator Service".to_string(),
        },
        ServiceDefinition {
            name: "DATA_ROUTER".to_string(),
            default_port: 50052,
            display_name: "Data Router Service".to_string(),
        },
        ServiceDefinition {
            name: "LLM".to_string(),
            default_port: 50053,
            display_name: "LLM Service".to_string(),
        },
        ServiceDefinition {
            name: "TOOLS".to_string(),
            default_port: 50054,
            display_name: "Tools Service".to_string(),
        },
        ServiceDefinition {
            name: "SAFETY".to_string(),
            default_port: 50055,
            display_name: "Safety Service".to_string(),
        },
        ServiceDefinition {
            name: "LOGGING".to_string(),
            default_port: 50056,
            display_name: "Logging Service".to_string(),
        },
        ServiceDefinition {
            name: "MIND_KB".to_string(),
            default_port: 50057,
            display_name: "Mind KB Service".to_string(),
        },
        ServiceDefinition {
            name: "BODY_KB".to_string(),
            default_port: 50058,
            display_name: "Body KB Service".to_string(),
        },
        ServiceDefinition {
            name: "HEART_KB".to_string(),
            default_port: 50059,
            display_name: "Heart KB Service".to_string(),
        },
        ServiceDefinition {
            name: "SOCIAL_KB".to_string(),
            default_port: 50060,
            display_name: "Social KB Service".to_string(),
        },
        ServiceDefinition {
            name: "SOUL_KB".to_string(),
            default_port: 50061,
            display_name: "Soul KB Service".to_string(),
        },
        ServiceDefinition {
            name: "EXECUTOR".to_string(),
            default_port: 50062,
            display_name: "Executor Service".to_string(),
        },
        ServiceDefinition {
            name: "CONTEXT_MANAGER".to_string(),
            default_port: 50064,
            display_name: "Context Manager Service".to_string(),
        },
        ServiceDefinition {
            name: "REFLECTION".to_string(),
            default_port: 50065,
            display_name: "Reflection Service".to_string(),
        },
        ServiceDefinition {
            name: "SCHEDULER".to_string(),
            default_port: 50066,
            display_name: "Scheduler Service".to_string(),
        },
        ServiceDefinition {
            name: "AGENT_REGISTRY".to_string(),
            default_port: 50067,
            display_name: "Agent Registry Service".to_string(),
        },
        ServiceDefinition {
            name: "RED_TEAM".to_string(),
            default_port: 50068,
            display_name: "Red Team Service".to_string(),
        },
        ServiceDefinition {
            name: "BLUE_TEAM".to_string(),
            default_port: 50069,
            display_name: "Blue Team Service".to_string(),
        },
        ServiceDefinition {
            name: "SECRETS".to_string(),
            default_port: 50080,
            display_name: "Secrets Service".to_string(),
        },
        ServiceDefinition {
            name: "AUTH".to_string(),
            default_port: 50090,
            display_name: "Auth Service".to_string(),
        },
        ServiceDefinition {
            name: "API_GATEWAY".to_string(),
            default_port: 8282,
            display_name: "API Gateway".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_service_port() {
        // Test with environment variable
        std::env::set_var("TEST_SERVICE_PORT", "9000");
        assert_eq!(get_service_port("TEST", 8000), 9000);

        // Test with default
        std::env::remove_var("UNKNOWN_SERVICE_PORT");
        assert_eq!(get_service_port("UNKNOWN", 8000), 8000);
    }

    #[test]
    fn test_get_client_address() {
        // Test with full address override
        std::env::set_var("TEST_SERVICE_ADDR", "http://example.com:9000");
        assert_eq!(get_client_address("TEST", 8000, None), "http://example.com:9000");

        // Test with port override 
        std::env::remove_var("TEST_SERVICE_ADDR");
        std::env::set_var("TEST_SERVICE_PORT", "9000");
        assert_eq!(get_client_address("TEST", 8000, None), "http://localhost:9000");

        // Test with default
        std::env::remove_var("UNKNOWN_SERVICE_ADDR");
        std::env::remove_var("UNKNOWN_SERVICE_PORT");
        assert_eq!(get_client_address("UNKNOWN", 8000, None), "http://localhost:8000");

        // Test with custom host
        assert_eq!(get_client_address("UNKNOWN", 8000, Some("service.local")), "http://service.local:8000");
    }
}