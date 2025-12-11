//! Security validators
//!
//! This module provides validators focused on security concerns,
//! helping to protect against common attack vectors.

use super::utils::get_regex;
use crate::errors::{ValidationError, ValidationResult};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // SQL Injection patterns
    static ref SQL_INJECTION_PATTERNS: Vec<&'static str> = vec![
        r"(?i)'\s*OR\s*'1'\s*=\s*'1",
        r"(?i)'\s*OR\s*1\s*=\s*1",
        r"(?i)'\s*OR\s*'\w+'\s*=\s*'\w+'",
        r"(?i)--",
        r"(?i);\s*DROP\s+TABLE",
        r"(?i);\s*DELETE\s+FROM",
        r"(?i);\s*INSERT\s+INTO",
        r"(?i)UNION\s+SELECT",
        r"(?i)SELECT\s+.*\s+FROM\s+\w+",
        r"(?i)EXEC\s*\(",
        r"(?i)EXECUTE\s*\(",
    ];

    // XSS patterns
    static ref XSS_PATTERNS: Vec<&'static str> = vec![
        r"(?i)<script",
        r"(?i)</script",
        r"(?i)<img[^>]*\bonerror=",
        r"(?i)<iframe",
        r"(?i)<object",
        r"(?i)<embed",
        r"(?i)javascript:",
        r"(?i)data:text/html",
        r"(?i)\bon\w+=",
        r"(?i)document\.cookie",
        r"(?i)eval\(",
        r"(?i)setTimeout\(",
        r"(?i)setInterval\(",
    ];

    // Command Injection patterns
    static ref CMD_INJECTION_PATTERNS: Vec<&'static str> = vec![
        r"(?i)[;&|`]\s*\w+",
        r"(?i)^\s*[;&|]\s*\w+",
        r";ls",
        r"&ls",
        r"\|ls",
        r"\|\|ls",
        r"&&ls",
        r"`ls`",
        r"\$\([^)]+\)", // $(command)
        r"\$\{[^}]+\}", // ${command}
        r"\bping\b",
        r"\bnmap\b",
        r"\bcurl\b",
        r"\bwget\b",
        r"\brm\b",
        r"\bmkdir\b",
        r"\bchmod\b",
        r"\bcat\b",
    ];

    // Path traversal patterns
    static ref PATH_TRAVERSAL_PATTERNS: Vec<&'static str> = vec![
        r"\.\./",
        r"\.\.\\",
        r"\.\.%2f",
        r"\.\.%5c",
        r"/%2e%2e/",
        r"\\%2e%2e\\",
        r"/etc/passwd",
        r"C:\\Windows\\system32",
        r"/bin/sh",
        r"/dev/null",
        r"%00",  // Null byte injection
    ];

    // LDAP injection
    static ref LDAP_INJECTION_PATTERNS: Vec<&'static str> = vec![
        r"\*\)",
        r"\(\|\(",
        r"&\|",
        r"\|\|",
        r"=\*",
        r"cn=",
        r"objectClass=",
    ];

    // NoSQL injection
    static ref NOSQL_INJECTION_PATTERNS: Vec<&'static str> = vec![
        r"\{\$",
        r"\$eq:",
        r"\$gt:",
        r"\$lt:",
        r"\$where:",
        r"\$elemMatch:",
        r"\$ne:",
        r";\s*sleep\s*\(",
        r";\s*while\s*\(",
    ];

    // JWT tampering patterns
    static ref JWT_TAMPERING_PATTERNS: Vec<&'static str> = vec![
        r#"alg.?:.?"none""#,
        r#"alg.?:.?"HS256".*kid"#,
        r"eyJhbGciOiJub25lI", // base64 encoded header with "none" algorithm
    ];

    // Server-side template injection patterns
    static ref SSTI_PATTERNS: Vec<&'static str> = vec![
        r"\{\{.+\}\}",      // Handlebars, Mustache, Angular, etc.
        r"\${.+}",          // JSP EL, Spring
        r"<%.+%>",          // JSP
        r"\$\{.+\}",        // JSP EL, Spring, Struts
        r"#\{.+\}",         // Ruby ERB
        r"\{\%.+\%\}",      // Twig, Liquid, Django
        r"\{\#.+\#\}",      // Jinja2 comments
    ];

    // Prototype pollution patterns
    static ref PROTO_POLLUTION_PATTERNS: Vec<&'static str> = vec![
        r"__proto__",
        r"constructor.prototype",
        r"__defineGetter__",
        r"__defineSetter__",
    ];

    // Format string vulnerabilities
    static ref FORMAT_STRING_PATTERNS: Vec<&'static str> = vec![
        r"%[n|x|s|d]",
        r"%\d+\$[n|x|s|d]",
        r"%p",
        r"%n",
    ];
}

/// Check for SQL injection patterns in a string
pub fn no_sql_injection(input: &str) -> ValidationResult<()> {
    for pattern in SQL_INJECTION_PATTERNS.iter() {
        if let Ok(re) = get_regex(pattern) {
            if re.is_match(input) {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potential SQL injection detected: matched pattern '{}'",
                    pattern
                )));
            }
        }
    }
    Ok(())
}

/// Check for XSS (Cross-Site Scripting) patterns in a string
pub fn no_xss(input: &str) -> ValidationResult<()> {
    for pattern in XSS_PATTERNS.iter() {
        if let Ok(re) = get_regex(pattern) {
            if re.is_match(input) {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potential XSS attack detected: matched pattern '{}'",
                    pattern
                )));
            }
        }
    }
    Ok(())
}

/// Check for command injection patterns in a string
pub fn no_command_injection(input: &str) -> ValidationResult<()> {
    for pattern in CMD_INJECTION_PATTERNS.iter() {
        if let Ok(re) = get_regex(pattern) {
            if re.is_match(input) {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potential command injection detected: matched pattern '{}'",
                    pattern
                )));
            }
        }
    }
    Ok(())
}

/// Check for path traversal patterns in a string
pub fn no_path_traversal(input: &str) -> ValidationResult<()> {
    for pattern in PATH_TRAVERSAL_PATTERNS.iter() {
        if let Ok(re) = get_regex(pattern) {
            if re.is_match(input) {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potential path traversal attack detected: matched pattern '{}'",
                    pattern
                )));
            }
        }
    }
    Ok(())
}

/// Check for LDAP injection patterns in a string
pub fn no_ldap_injection(input: &str) -> ValidationResult<()> {
    for pattern in LDAP_INJECTION_PATTERNS.iter() {
        if let Ok(re) = get_regex(pattern) {
            if re.is_match(input) {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potential LDAP injection detected: matched pattern '{}'",
                    pattern
                )));
            }
        }
    }
    Ok(())
}

/// Check for NoSQL injection patterns in a string
pub fn no_nosql_injection(input: &str) -> ValidationResult<()> {
    for pattern in NOSQL_INJECTION_PATTERNS.iter() {
        if let Ok(re) = get_regex(pattern) {
            if re.is_match(input) {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potential NoSQL injection detected: matched pattern '{}'",
                    pattern
                )));
            }
        }
    }
    Ok(())
}

/// Check for JWT tampering patterns in a string
pub fn no_jwt_tampering(input: &str) -> ValidationResult<()> {
    for pattern in JWT_TAMPERING_PATTERNS.iter() {
        if let Ok(re) = get_regex(pattern) {
            if re.is_match(input) {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potential JWT tampering detected: matched pattern '{}'",
                    pattern
                )));
            }
        }
    }
    Ok(())
}

/// Check for Server-Side Template Injection (SSTI) patterns in a string
pub fn no_ssti(input: &str) -> ValidationResult<()> {
    for pattern in SSTI_PATTERNS.iter() {
        if let Ok(re) = get_regex(pattern) {
            if re.is_match(input) {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potential server-side template injection detected: matched pattern '{}'",
                    pattern
                )));
            }
        }
    }
    Ok(())
}

/// Check for prototype pollution patterns in a string
pub fn no_prototype_pollution(input: &str) -> ValidationResult<()> {
    for pattern in PROTO_POLLUTION_PATTERNS.iter() {
        if let Ok(re) = get_regex(pattern) {
            if re.is_match(input) {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potential prototype pollution detected: matched pattern '{}'",
                    pattern
                )));
            }
        }
    }
    Ok(())
}

/// Check for format string vulnerability patterns in a string
pub fn no_format_string_vulnerabilities(input: &str) -> ValidationResult<()> {
    for pattern in FORMAT_STRING_PATTERNS.iter() {
        if let Ok(re) = get_regex(pattern) {
            if re.is_match(input) {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potential format string vulnerability detected: matched pattern '{}'",
                    pattern
                )));
            }
        }
    }
    Ok(())
}

/// Check for content that might be trying to exfiltrate data
pub fn no_data_exfiltration(input: &str) -> ValidationResult<()> {
    // Patterns for credential/sensitive data detection
    let sensitive_patterns = [
        r#"(?i)pass(word)?[=:'"]"#,
        r#"(?i)secret[=:'"]"#,
        r#"(?i)api_?key[=:'"]"#,
        r#"(?i)token[=:'"]"#,
        r#"(?i)auth(entication)?[=:'"]"#,
        r"(?i)private_?key",
        r"(?i)secret_?key",
        r"(?i)access_?key",
        r#"(?i)cred(ential)?[=:'"]"#,
    ];

    for pattern in &sensitive_patterns {
        if let Ok(re) = get_regex(pattern) {
            if re.is_match(input) {
                return Err(ValidationError::SecurityThreat(format!(
                    "Potential sensitive data exfiltration detected: matched pattern '{}'",
                    pattern
                )));
            }
        }
    }
    Ok(())
}

/// Run a comprehensive security scan on input
pub fn security_scan(input: &str) -> ValidationResult<()> {
    // List of security validators to run
    let validators: Vec<&dyn Fn(&str) -> ValidationResult<()>> = vec![
        &no_sql_injection,
        &no_xss,
        &no_command_injection,
        &no_path_traversal,
        &no_ldap_injection,
        &no_nosql_injection,
        &no_jwt_tampering,
        &no_ssti,
        &no_prototype_pollution,
        &no_format_string_vulnerabilities,
        &no_data_exfiltration,
    ];

    let mut errors = Vec::new();

    // Run all validators and collect errors
    for validator in validators {
        if let Err(err) = validator(input) {
            errors.push(err);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::composite(errors))
    }
}

/// Default comprehensive security scan without JWT tampering check
/// (useful for validating JWTs themselves)
pub fn default_security_scan(input: &str) -> ValidationResult<()> {
    // List of security validators to run
    let validators: Vec<&dyn Fn(&str) -> ValidationResult<()>> = vec![
        &no_sql_injection,
        &no_xss,
        &no_command_injection,
        &no_path_traversal,
        &no_ldap_injection,
        &no_nosql_injection,
        &no_ssti,
        &no_prototype_pollution,
        &no_format_string_vulnerabilities,
    ];

    let mut errors = Vec::new();

    // Run all validators and collect errors
    for validator in validators {
        if let Err(err) = validator(input) {
            errors.push(err);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::composite(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_injection() {
        assert!(no_sql_injection("normal query").is_ok());
        assert!(no_sql_injection("' OR '1'='1").is_err());
        assert!(no_sql_injection("SELECT * FROM users").is_err());
        assert!(no_sql_injection("username'; DROP TABLE users; --").is_err());
    }

    #[test]
    fn test_xss() {
        assert!(no_xss("normal text").is_ok());
        assert!(no_xss("<script>alert(1)</script>").is_err());
        assert!(no_xss("<img src=x onerror=alert(1)>").is_err());
        assert!(no_xss("javascript:alert(1)").is_err());
    }

    #[test]
    fn test_command_injection() {
        assert!(no_command_injection("normal command").is_ok());
        assert!(no_command_injection("command; ls").is_err());
        assert!(no_command_injection("command | ls").is_err());
        assert!(no_command_injection("command && ls").is_err());
    }

    #[test]
    fn test_path_traversal() {
        assert!(no_path_traversal("normal/path").is_ok());
        assert!(no_path_traversal("../../../etc/passwd").is_err());
        assert!(no_path_traversal("..\\..\\Windows\\system32").is_err());
        assert!(no_path_traversal("/etc/passwd").is_err());
    }

    #[test]
    fn test_security_scan() {
        assert!(security_scan("Hello, this is a normal message!").is_ok());
        assert!(security_scan("<script>alert('xss');</script>").is_err());
        assert!(security_scan("' OR 1=1; --").is_err());
        assert!(security_scan("login; rm -rf /").is_err());
    }
}
