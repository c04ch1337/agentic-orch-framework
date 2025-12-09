//! Command sanitization utilities
//!
//! This module provides sanitizers for command strings to prevent
//! command injection and ensure safe execution.

use super::SanitizeResult;
use regex::Regex;
use lazy_static::lazy_static;
use std::collections::HashSet;

lazy_static! {
    /// Regex for dangerous shell command patterns
    static ref SHELL_INJECTION_REGEX: Regex = Regex::new(
        r"(;|\||&|\$\(|\`|\)|\{|\}|\\|\n|\r)"
    ).unwrap();
    
    /// Set of allowed shell commands for whitelisting
    static ref DEFAULT_ALLOWED_COMMANDS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        // Common safe commands
        set.insert("ls");
        set.insert("dir");
        set.insert("find");
        set.insert("grep");
        set.insert("cat");
        set.insert("echo");
        set.insert("cd");
        set.insert("pwd");
        set.insert("mkdir");
        set.insert("touch");
        set.insert("rm");
        set.insert("cp");
        set.insert("mv");
        set
    };
    
    /// Set of denied shell commands
    static ref DEFAULT_DENIED_COMMANDS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        // Potentially dangerous commands
        set.insert("sudo");
        set.insert("su");
        set.insert("chmod");
        set.insert("chown");
        set.insert("wget");
        set.insert("curl");
        set.insert("nc");
        set.insert("netcat");
        set.insert("ssh");
        set.insert("telnet");
        set.insert("ftp");
        set.insert("python");
        set.insert("perl");
        set.insert("ruby");
        set.insert("bash");
        set.insert("sh");
        set.insert("csh");
        set.insert("ksh");
        set.insert("zsh");
        set
    };
}

/// Quote a command argument to make it safe for shell execution
pub fn quote_argument(arg: &str) -> SanitizeResult<String> {
    let has_special_chars = arg.contains(|c: char| {
        c.is_whitespace() || "\"'\\$&|;(){}[]<>*?!#~=".contains(c)
    });
    
    if !has_special_chars && !arg.is_empty() {
        // No special chars, no need to quote
        SanitizeResult::unmodified(arg.to_string())
    } else {
        // Escape quotes and backslashes first
        let mut result = arg.replace('\\', "\\\\").replace('\"', "\\\"");
        
        // Then wrap in quotes
        result = format!("\"{}\"", result);
        
        SanitizeResult::modified(
            result, 
            Some("Quoted command argument".to_string())
        )
    }
}

/// Remove shell metacharacters from a command
pub fn remove_shell_metacharacters(command: &str) -> SanitizeResult<String> {
    let result = SHELL_INJECTION_REGEX.replace_all(command, "").to_string();
    
    if result == command {
        SanitizeResult::unmodified(command.to_string())
    } else {
        SanitizeResult::modified(
            result,
            Some("Removed shell metacharacters".to_string())
        )
    }
}

/// Ensure the command starts with an allowed command
pub fn whitelist_command(command: &str, allowed: Option<&HashSet<&str>>) -> SanitizeResult<String> {
    let whitelist = allowed.unwrap_or(&DEFAULT_ALLOWED_COMMANDS);
    
    // Extract the first word (the command name)
    let args: Vec<&str> = command.trim().split_whitespace().collect();
    
    if args.is_empty() {
        return SanitizeResult::unmodified(command.to_string());
    }
    
    let command_name = args[0].to_lowercase();
    
    if whitelist.contains(command_name.as_str()) {
        SanitizeResult::unmodified(command.to_string())
    } else {
        SanitizeResult::modified(
            "".to_string(),
            Some(format!("Command '{}' is not in the allowed list", command_name))
        )
    }
}

/// Ensure the command doesn't start with a denied command
pub fn blacklist_command(command: &str, denied: Option<&HashSet<&str>>) -> SanitizeResult<String> {
    let blacklist = denied.unwrap_or(&DEFAULT_DENIED_COMMANDS);
    
    // Extract the first word (the command name)
    let args: Vec<&str> = command.trim().split_whitespace().collect();
    
    if args.is_empty() {
        return SanitizeResult::unmodified(command.to_string());
    }
    
    let command_name = args[0].to_lowercase();
    
    if blacklist.contains(command_name.as_str()) {
        SanitizeResult::modified(
            "".to_string(),
            Some(format!("Command '{}' is in the denied list", command_name))
        )
    } else {
        SanitizeResult::unmodified(command.to_string())
    }
}

/// Safe command sanitization with multiple strategies
pub fn sanitize_command(command: &str) -> SanitizeResult<String> {
    // First check against blacklist
    let blacklist_result = blacklist_command(command, None);
    if blacklist_result.was_modified {
        return blacklist_result;
    }
    
    // Then remove shell metacharacters
    let sanitized = remove_shell_metacharacters(command);
    
    sanitized
}

/// Quote all arguments in a command for safe execution
pub fn quote_command_args(command: &str) -> SanitizeResult<String> {
    // Split command into parts while respecting quotes
    let parts = split_command(command);
    
    if parts.is_empty() {
        return SanitizeResult::unmodified(command.to_string());
    }
    
    let mut was_modified = false;
    let mut result = String::new();
    
    // Keep the first part (the command) as is
    result.push_str(&parts[0]);
    
    // Quote all arguments
    for arg in &parts[1..] {
        let quoted = quote_argument(arg);
        result.push(' ');
        result.push_str(&quoted.sanitized);
        
        if quoted.was_modified {
            was_modified = true;
        }
    }
    
    if was_modified {
        SanitizeResult::modified(
            result,
            Some("Quoted command arguments".to_string())
        )
    } else {
        SanitizeResult::unmodified(command.to_string())
    }
}

/// Split a command string into parts, respecting quoted sections
fn split_command(command: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single_quotes = false;
    let mut in_double_quotes = false;
    let mut escaped = false;
    
    for c in command.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }
        
        match c {
            '\\' => escaped = true,
            '\'' if !in_double_quotes => in_single_quotes = !in_single_quotes,
            '"' if !in_single_quotes => in_double_quotes = !in_double_quotes,
            ' ' | '\t' if !in_single_quotes && !in_double_quotes => {
                if !current.is_empty() {
                    parts.push(current);
                    current = String::new();
                }
            },
            _ => current.push(c),
        }
    }
    
    if !current.is_empty() {
        parts.push(current);
    }
    
    parts
}

/// Create a command array for safe execution via process APIs
/// This returns a vector of arguments that can be passed directly to process execution
/// without shell interpretation, which is safer than passing a shell command string
pub fn command_to_args(command: &str) -> SanitizeResult<Vec<String>> {
    let parts = split_command(command);
    
    if parts.is_empty() {
        return SanitizeResult::modified(
            Vec::new(),
            Some("Empty command".to_string())
        );
    }
    
    SanitizeResult::modified(
        parts,
        Some("Converted command to argument array".to_string())
    )
}

/// Advanced command sanitization for comprehensive protection
pub fn strict_command_sanitize(
    command: &str,
    allowed_commands: Option<&HashSet<&str>>,
    allowed_args: Option<&HashSet<&str>>
) -> SanitizeResult<String> {
    // First check against blacklist
    let blacklist_result = blacklist_command(command, None);
    if blacklist_result.was_modified {
        return blacklist_result;
    }
    
    // Then check against whitelist if provided
    if let Some(whitelist) = allowed_commands {
        let whitelist_result = whitelist_command(command, Some(whitelist));
        if whitelist_result.was_modified {
            return whitelist_result;
        }
    }
    
    // Split the command into parts
    let parts = split_command(command);
    
    if parts.is_empty() {
        return SanitizeResult::unmodified(command.to_string());
    }
    
    // Validate arguments if allowed_args is provided
    if let Some(allowed) = allowed_args {
        if parts.len() > 1 {
            for arg in &parts[1..] {
                // Check if argument starts with a dash (option)
                if arg.starts_with('-') {
                    let arg_name = if arg.starts_with("--") {
                        &arg[2..]
                    } else {
                        &arg[1..]
                    };
                    
                    if !allowed.contains(arg_name) {
                        return SanitizeResult::modified(
                            "".to_string(),
                            Some(format!("Argument '{}' is not in the allowed list", arg))
                        );
                    }
                }
            }
        }
    }
    
    // Remove shell metacharacters
    let sanitized = remove_shell_metacharacters(command);
    
    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_argument() {
        // Simple argument doesn't need quoting
        let simple = "simple";
        let result = quote_argument(simple);
        assert!(!result.was_modified);
        assert_eq!(result.sanitized, simple);
        
        // Argument with spaces needs quoting
        let with_spaces = "argument with spaces";
        let result = quote_argument(with_spaces);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "\"argument with spaces\"");
        
        // Argument with special characters needs quoting
        let with_special = "argument;with$special&chars";
        let result = quote_argument(with_special);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "\"argument;with$special&chars\"");
        
        // Argument with quotes needs escaping and quoting
        let with_quotes = "argument\"with'quotes";
        let result = quote_argument(with_quotes);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "\"argument\\\"with'quotes\"");
    }

    #[test]
    fn test_remove_shell_metacharacters() {
        // Simple command without metacharacters
        let simple = "ls -la";
        let result = remove_shell_metacharacters(simple);
        assert!(!result.was_modified);
        assert_eq!(result.sanitized, simple);
        
        // Command with shell metacharacters
        let with_meta = "ls -la; rm -rf /";
        let result = remove_shell_metacharacters(with_meta);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "ls -la rm -rf ");
        
        // Command with multiple metacharacters
        let complex = "echo $(ls) | grep 'test'";
        let result = remove_shell_metacharacters(complex);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "echo ls  grep 'test'");
    }

    #[test]
    fn test_whitelist_command() {
        // Command in the default whitelist
        let allowed = "ls -la";
        let result = whitelist_command(allowed, None);
        assert!(!result.was_modified);
        assert_eq!(result.sanitized, allowed);
        
        // Command not in the default whitelist
        let not_allowed = "wget http://example.com";
        let result = whitelist_command(not_allowed, None);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "");
        
        // Custom whitelist
        let mut custom_whitelist = HashSet::new();
        custom_whitelist.insert("wget");
        
        // Command in the custom whitelist
        let result = whitelist_command(not_allowed, Some(&custom_whitelist));
        assert!(!result.was_modified);
        assert_eq!(result.sanitized, not_allowed);
    }

    #[test]
    fn test_blacklist_command() {
        // Command in the default blacklist
        let denied = "sudo rm -rf /";
        let result = blacklist_command(denied, None);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "");
        
        // Command not in the default blacklist
        let allowed = "ls -la";
        let result = blacklist_command(allowed, None);
        assert!(!result.was_modified);
        assert_eq!(result.sanitized, allowed);
        
        // Custom blacklist
        let mut custom_blacklist = HashSet::new();
        custom_blacklist.insert("ls");
        
        // Command in the custom blacklist
        let result = blacklist_command(allowed, Some(&custom_blacklist));
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "");
    }

    #[test]
    fn test_sanitize_command() {
        // Safe command
        let safe = "ls -la";
        let result = sanitize_command(safe);
        assert!(!result.was_modified);
        assert_eq!(result.sanitized, safe);
        
        // Unsafe command in blacklist
        let unsafe_blacklist = "sudo rm -rf /";
        let result = sanitize_command(unsafe_blacklist);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "");
        
        // Command with shell metacharacters
        let unsage_meta = "ls -la; rm -rf /";
        let result = sanitize_command(unsage_meta);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "ls -la rm -rf ");
    }

    #[test]
    fn test_quote_command_args() {
        // Command with simple arguments
        let simple = "ls -la";
        let result = quote_command_args(simple);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "ls \"-la\"");
        
        // Command with argument containing spaces
        let with_spaces = "echo hello world";
        let result = quote_command_args(with_spaces);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "echo \"hello\" \"world\"");
        
        // Command with already quoted arguments
        let quoted = "echo \"hello world\"";
        let result = quote_command_args(quoted);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "echo \"hello world\"");
    }

    #[test]
    fn test_split_command() {
        // Simple command
        let simple = "ls -la";
        let parts = split_command(simple);
        assert_eq!(parts, vec!["ls", "-la"]);
        
        // Command with quotes
        let quoted = "echo \"hello world\"";
        let parts = split_command(quoted);
        assert_eq!(parts, vec!["echo", "\"hello world\""]);
        
        // Command with single quotes
        let single_quoted = "echo 'hello world'";
        let parts = split_command(single_quoted);
        assert_eq!(parts, vec!["echo", "'hello world'"]);
        
        // Command with escaped characters
        let escaped = "echo hello\\ world";
        let parts = split_command(escaped);
        assert_eq!(parts, vec!["echo", "hello\\ world"]);
    }

    #[test]
    fn test_command_to_args() {
        // Simple command
        let simple = "ls -la";
        let result = command_to_args(simple);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, vec!["ls", "-la"]);
        
        // Command with quotes
        let quoted = "echo \"hello world\"";
        let result = command_to_args(quoted);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, vec!["echo", "\"hello world\""]);
    }

    #[test]
    fn test_strict_command_sanitize() {
        // Safe command
        let safe = "ls -la";
        let result = strict_command_sanitize(safe, None, None);
        assert!(!result.was_modified);
        assert_eq!(result.sanitized, safe);
        
        // Unsafe command in blacklist
        let unsafe_blacklist = "sudo rm -rf /";
        let result = strict_command_sanitize(unsafe_blacklist, None, None);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "");
        
        // Command with custom allowed commands
        let mut allowed_cmds = HashSet::new();
        allowed_cmds.insert("echo");
        
        let allowed = "echo hello";
        let result = strict_command_sanitize(allowed, Some(&allowed_cmds), None);
        assert!(!result.was_modified);
        assert_eq!(result.sanitized, allowed);
        
        let not_allowed = "ls -la";
        let result = strict_command_sanitize(not_allowed, Some(&allowed_cmds), None);
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "");
        
        // Command with custom allowed arguments
        let mut allowed_args = HashSet::new();
        allowed_args.insert("l");
        allowed_args.insert("a");
        
        let allowed = "ls -la";
        let result = strict_command_sanitize(allowed, None, Some(&allowed_args));
        assert!(!result.was_modified);
        assert_eq!(result.sanitized, allowed);
        
        // Command with disallowed argument
        let not_allowed = "ls -lah";
        let result = strict_command_sanitize(not_allowed, None, Some(&allowed_args));
        assert!(result.was_modified);
        assert_eq!(result.sanitized, "");
    }
}