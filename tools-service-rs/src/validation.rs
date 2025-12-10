//! Tools Service Input Validation
//!
//! This module provides validation and sanitization for the Tools Service,
//! focusing on command parameter validation and whitelisting to prevent
//! command injection and other security issues.

use input_validation_rs::prelude::*;
use input_validation_rs::validators;
use input_validation_rs::sanitizers;
use std::collections::{HashMap, HashSet};
use once_cell::sync::Lazy;
use thiserror::Error;

/// Maximum argument length for any command
pub const MAX_ARG_LENGTH: usize = 1024;

/// Maximum number of arguments allowed for a command
pub const MAX_ARGS_COUNT: usize = 32;

/// Default allowed commands whitelist
static DEFAULT_ALLOWED_COMMANDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();
    // Basic file operations
    set.insert("ls");
    set.insert("dir");
    set.insert("cat");
    set.insert("head");
    set.insert("tail");
    set.insert("find");
    set.insert("grep");
    set.insert("touch");
    // Directory operations
    set.insert("mkdir");
    set.insert("cd");
    set.insert("pwd");
    // Common utilities
    set.insert("echo");
    set.insert("wc");
    set.insert("sort");
    set.insert("uniq");
    set.insert("date");
    set.insert("zip");
    set
});

/// Default denied commands blacklist
static DEFAULT_DENIED_COMMANDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();
    // System commands
    set.insert("sudo");
    set.insert("su");
    set.insert("systemctl");
    // File system modification
    set.insert("rm");
    set.insert("mv");
    set.insert("cp");
    set.insert("chmod");
    set.insert("chown");
    // Network commands
    set.insert("wget");
    set.insert("curl");
    set.insert("ssh");
    set.insert("telnet");
    set.insert("nc");
    set.insert("netcat");
    // Shells
    set.insert("bash");
    set.insert("sh");
    set.insert("ksh");
    set.insert("csh");
    set.insert("zsh");
    // Interpreters
    set.insert("python");
    set.insert("python3");
    set.insert("perl");
    set.insert("ruby");
    set.insert("node");
    set.insert("npm");
    set
});

/// Tools Service validation error
#[derive(Debug, Error)]
pub enum ToolValidationError {
    #[error("Command not allowed: {0}")]
    CommandNotAllowed(String),
    
    #[error("Argument not allowed: {0}")]
    ArgumentNotAllowed(String),
    
    #[error("Too many arguments: {0} (maximum {1})")]
    TooManyArguments(usize, usize),
    
    #[error("Argument too long: {0} characters (maximum {1})")]
    ArgumentTooLong(usize, usize),
    
    #[error("Invalid input type: {0}")]
    InvalidInputType(String),
    
    #[error("Security threat detected: {0}")]
    SecurityThreat(String),
    
    #[error("Validation error: {0}")]
    Other(String),
}

impl From<ValidationError> for ToolValidationError {
    fn from(err: ValidationError) -> Self {
        match err {
            ValidationError::SecurityThreat(msg) => Self::SecurityThreat(msg),
            ValidationError::TooLong(msg) => Self::ArgumentTooLong(MAX_ARG_LENGTH, MAX_ARG_LENGTH),
            ValidationError::InvalidCharacters(msg) => Self::ArgumentNotAllowed(msg),
            _ => Self::Other(err.to_string()),
        }
    }
}

/// Validate a command name against allowed and denied lists
pub fn validate_command_name(
    cmd: &str,
    allowed: Option<&HashSet<&str>>,
    denied: Option<&HashSet<&str>>,
) -> Result<(), ToolValidationError> {
    // Check denied commands first
    let denied_list = denied.unwrap_or(&DEFAULT_DENIED_COMMANDS);
    if denied_list.contains(cmd) {
        return Err(ToolValidationError::CommandNotAllowed(format!(
            "Command '{}' is explicitly denied", cmd
        )));
    }

    // Then check against allowed commands if provided
    if let Some(allowed_list) = allowed {
        if !allowed_list.contains(cmd) {
            return Err(ToolValidationError::CommandNotAllowed(format!(
                "Command '{}' is not in the allowed list", cmd
            )));
        }
    }

    // Run the command through security validation
    match validators::security::default_security_scan(cmd) {
        Ok(_) => Ok(()),
        Err(e) => Err(ToolValidationError::SecurityThreat(format!(
            "Command contains potential security threat: {}", e
        ))),
    }
}

/// Validate command arguments for safety and proper format
pub fn validate_command_args(args: &[String]) -> Result<(), ToolValidationError> {
    // Check number of arguments
    if args.len() > MAX_ARGS_COUNT {
        return Err(ToolValidationError::TooManyArguments(
            args.len(),
            MAX_ARGS_COUNT,
        ));
    }

    // Validate each argument individually
    for (idx, arg) in args.iter().enumerate() {
        // Check argument length
        if arg.len() > MAX_ARG_LENGTH {
            return Err(ToolValidationError::ArgumentTooLong(
                arg.len(),
                MAX_ARG_LENGTH,
            ));
        }

        // Check for security threats in argument
        if let Err(e) = validators::security::default_security_scan(arg) {
            return Err(ToolValidationError::SecurityThreat(format!(
                "Argument {} contains potential security threat: {}", idx, e
            )));
        }

        // Check for command injection attempts
        if arg.contains(';') || arg.contains('|') || arg.contains('&') || 
           arg.contains('`') || arg.contains('$') || arg.contains('(') {
            return Err(ToolValidationError::ArgumentNotAllowed(format!(
                "Argument {} contains command chaining characters", idx
            )));
        }
    }

    Ok(())
}

/// Sanitize a command to make it safer for execution
pub fn sanitize_command(cmd: &str) -> String {
    sanitizers::command::sanitize_command(cmd).sanitized
}

/// Sanitize command arguments for safer execution
pub fn sanitize_command_args(args: &[String]) -> Vec<String> {
    args.iter()
        .map(|arg| sanitizers::command::sanitize_command(arg).sanitized)
        .collect()
}

/// Validate input type for simulation
pub fn validate_input_type(input_type: &str) -> Result<(), ToolValidationError> {
    // Check if input type is recognized
    match input_type {
        "keyboard" | "mouse" | "touch" | "gamepad" | "network" => Ok(()),
        _ => Err(ToolValidationError::InvalidInputType(format!(
            "Unrecognized input type: {}", input_type
        ))),
    }
}

/// Validate input parameters for simulation
pub fn validate_input_params(
    params: &HashMap<String, String>,
    input_type: &str,
) -> Result<(), ToolValidationError> {
    // Validate parameters based on input type
    match input_type {
        "keyboard" => {
            // Required parameters for keyboard input
            if !params.contains_key("key") {
                return Err(ToolValidationError::Other(
                    "Keyboard input requires 'key' parameter".to_string()
                ));
            }

            // Validate key parameter
            if let Some(key) = params.get("key") {
                if key.len() > 50 {
                    return Err(ToolValidationError::ArgumentTooLong(
                        key.len(),
                        50
                    ));
                }
            }
        },
        "mouse" => {
            // Required parameters for mouse input
            if !params.contains_key("x") || !params.contains_key("y") {
                return Err(ToolValidationError::Other(
                    "Mouse input requires 'x' and 'y' parameters".to_string()
                ));
            }

            // Validate x and y are numeric
            if let Some(x) = params.get("x") {
                if let Err(_) = x.parse::<i32>() {
                    return Err(ToolValidationError::Other(
                        "Mouse 'x' parameter must be a number".to_string()
                    ));
                }
            }

            if let Some(y) = params.get("y") {
                if let Err(_) = y.parse::<i32>() {
                    return Err(ToolValidationError::Other(
                        "Mouse 'y' parameter must be a number".to_string()
                    ));
                }
            }
        },
        // Add validation for other input types as needed
        _ => {}
    }

    // Check all parameters for security issues
    for (key, value) in params {
        // Validate the parameter name
        if let Err(e) = validators::string::allowed_chars(key, "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-") {
            return Err(ToolValidationError::ArgumentNotAllowed(
                format!("Parameter name contains invalid characters: {}", key)
            ));
        }

        // Validate the parameter value
        if let Err(e) = validators::security::default_security_scan(value) {
            return Err(ToolValidationError::SecurityThreat(
                format!("Parameter '{}' contains potential security threat: {}", key, e)
            ));
        }
    }

    Ok(())
}

/// Comprehensive validation of command execution request
pub fn validate_command_execution(
    cmd: &str,
    args: &[String],
) -> Result<(), ToolValidationError> {
    // First, validate the command name
    validate_command_name(cmd, None, None)?;
    
    // Then validate arguments
    validate_command_args(args)?;
    
    Ok(())
}

/// Comprehensive validation of input simulation request
pub fn validate_input_simulation(
    input_type: &str,
    params: &HashMap<String, String>,
) -> Result<(), ToolValidationError> {
    // Validate input type
    validate_input_type(input_type)?;
    
    // Validate parameters
    validate_input_params(params, input_type)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_command_name() {
        // Test allowed command
        assert!(validate_command_name("ls", None, None).is_ok());
        
        // Test denied command
        assert!(validate_command_name("rm", None, None).is_err());
        
        // Test command with security issue
        assert!(validate_command_name("ls;rm -rf /", None, None).is_err());
    }
    
    #[test]
    fn test_validate_command_args() {
        // Test valid arguments
        let args = vec!["file.txt".to_string(), "-l".to_string()];
        assert!(validate_command_args(&args).is_ok());
        
        // Test argument with command injection
        let args = vec!["file.txt;rm -rf /".to_string()];
        assert!(validate_command_args(&args).is_err());
        
        // Test too many arguments
        let too_many_args = (0..MAX_ARGS_COUNT + 1)
            .map(|i| format!("arg{}", i))
            .collect::<Vec<String>>();
        assert!(validate_command_args(&too_many_args).is_err());
    }
    
    #[test]
    fn test_sanitize_command() {
        // Test sanitizing command with potential injection
        let cmd = "ls -la; rm -rf /";
        let sanitized = sanitize_command(cmd);
        assert!(!sanitized.contains(';'));
        
        // Test simple command
        let cmd = "ls -la";
        let sanitized = sanitize_command(cmd);
        assert_eq!(sanitized, cmd);
    }
    
    #[test]
    fn test_validate_input_type() {
        // Test valid input types
        assert!(validate_input_type("keyboard").is_ok());
        assert!(validate_input_type("mouse").is_ok());
        
        // Test invalid input type
        assert!(validate_input_type("unknown").is_err());
    }
    
    #[test]
    fn test_validate_input_params() {
        // Test valid keyboard parameters
        let mut params = HashMap::new();
        params.insert("key".to_string(), "Enter".to_string());
        assert!(validate_input_params(&params, "keyboard").is_ok());
        
        // Test missing required parameter
        let empty_params = HashMap::new();
        assert!(validate_input_params(&empty_params, "keyboard").is_err());
        
        // Test valid mouse parameters
        let mut mouse_params = HashMap::new();
        mouse_params.insert("x".to_string(), "100".to_string());
        mouse_params.insert("y".to_string(), "200".to_string());
        assert!(validate_input_params(&mouse_params, "mouse").is_ok());
        
        // Test invalid mouse parameter (non-numeric)
        let mut invalid_mouse_params = HashMap::new();
        invalid_mouse_params.insert("x".to_string(), "abc".to_string());
        invalid_mouse_params.insert("y".to_string(), "200".to_string());
        assert!(validate_input_params(&invalid_mouse_params, "mouse").is_err());
    }
}