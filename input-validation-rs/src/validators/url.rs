//! URL validators
//!
//! This module provides validators for URL inputs, helping prevent
//! URL-based attacks and ensuring URL inputs are correctly formatted.

use crate::errors::{ValidationError, ValidationResult};
use url::{Url, Host};
use std::collections::HashSet;

/// Validate that a string is a valid URL
pub fn is_url(s: &str) -> ValidationResult<()> {
    match Url::parse(s) {
        Ok(_) => Ok(()),
        Err(e) => Err(ValidationError::InvalidUrl(format!(
            "Invalid URL: {}", e
        )))
    }
}

/// Validate that a URL uses an allowed protocol/scheme
pub fn allowed_protocol(s: &str, allowed: &[&str]) -> ValidationResult<()> {
    match Url::parse(s) {
        Ok(url) => {
            let scheme = url.scheme();
            if allowed.contains(&scheme) {
                Ok(())
            } else {
                Err(ValidationError::InvalidUrl(format!(
                    "URL uses disallowed protocol '{}'. Allowed protocols: {:?}",
                    scheme, allowed
                )))
            }
        },
        Err(e) => Err(ValidationError::InvalidUrl(format!(
            "Invalid URL: {}", e
        )))
    }
}

/// Validate that a URL's host is in the allowed domains list
pub fn allowed_domain(s: &str, allowed: &[&str]) -> ValidationResult<()> {
    match Url::parse(s) {
        Ok(url) => {
            if let Some(host) = url.host_str() {
                if allowed.iter().any(|&allowed_domain| {
                    // Allow exact match or subdomains of allowed domains
                    host == allowed_domain || 
                    (allowed_domain.starts_with(".") && host.ends_with(allowed_domain))
                }) {
                    Ok(())
                } else {
                    Err(ValidationError::InvalidUrl(format!(
                        "URL domain '{}' is not in the allowed domains list", host
                    )))
                }
            } else {
                Err(ValidationError::InvalidUrl(
                    "URL has no host component".to_string()
                ))
            }
        },
        Err(e) => Err(ValidationError::InvalidUrl(format!(
            "Invalid URL: {}", e
        )))
    }
}

/// Validate that a URL's host is not in the denied domains list
pub fn denied_domain(s: &str, denied: &[&str]) -> ValidationResult<()> {
    match Url::parse(s) {
        Ok(url) => {
            if let Some(host) = url.host_str() {
                if denied.iter().any(|&denied_domain| {
                    // Check exact match or subdomains of denied domains
                    host == denied_domain || 
                    (denied_domain.starts_with(".") && host.ends_with(denied_domain))
                }) {
                    Err(ValidationError::InvalidUrl(format!(
                        "URL domain '{}' is in the denied domains list", host
                    )))
                } else {
                    Ok(())
                }
            } else {
                Err(ValidationError::InvalidUrl(
                    "URL has no host component".to_string()
                ))
            }
        },
        Err(e) => Err(ValidationError::InvalidUrl(format!(
            "Invalid URL: {}", e
        )))
    }
}

/// Validate that a URL has no credentials (username/password)
pub fn no_credentials(s: &str) -> ValidationResult<()> {
    match Url::parse(s) {
        Ok(url) => {
            if url.username().is_empty() && url.password().is_none() {
                Ok(())
            } else {
                Err(ValidationError::SecurityThreat(
                    "URL contains credentials (username/password) which is not allowed".to_string()
                ))
            }
        },
        Err(e) => Err(ValidationError::InvalidUrl(format!(
            "Invalid URL: {}", e
        )))
    }
}

/// Validate that a URL does not contain certain path segments
pub fn no_path_segments(s: &str, denied_segments: &[&str]) -> ValidationResult<()> {
    match Url::parse(s) {
        Ok(url) => {
            let path_segments: Vec<&str> = url.path().split('/').collect();
            for segment in path_segments {
                if !segment.is_empty() && denied_segments.contains(&segment) {
                    return Err(ValidationError::InvalidUrl(format!(
                        "URL contains denied path segment '{}'", segment
                    )));
                }
            }
            Ok(())
        },
        Err(e) => Err(ValidationError::InvalidUrl(format!(
            "Invalid URL: {}", e
        )))
    }
}

/// Validate that a URL has a valid port in the allowed range
pub fn valid_port(s: &str, allowed_ports: Option<&[u16]>) -> ValidationResult<()> {
    match Url::parse(s) {
        Ok(url) => {
            if let Some(port) = url.port() {
                if let Some(ports) = allowed_ports {
                    if ports.contains(&port) {
                        Ok(())
                    } else {
                        Err(ValidationError::InvalidUrl(format!(
                            "URL port {} is not in the allowed ports list", port
                        )))
                    }
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        },
        Err(e) => Err(ValidationError::InvalidUrl(format!(
            "Invalid URL: {}", e
        )))
    }
}

/// Validate that a URL has only allowed query parameters
pub fn allowed_query_params(s: &str, allowed: &[&str]) -> ValidationResult<()> {
    match Url::parse(s) {
        Ok(url) => {
            if let Some(query) = url.query() {
                let params: HashSet<String> = query
                    .split('&')
                    .filter_map(|p| p.split('=').next())
                    .map(|s| s.to_string())
                    .collect();
                
                let allowed_set: HashSet<String> = allowed
                    .iter()
                    .map(|&s| s.to_string())
                    .collect();
                
                let disallowed: Vec<String> = params
                    .difference(&allowed_set)
                    .cloned()
                    .collect();
                
                if disallowed.is_empty() {
                    Ok(())
                } else {
                    Err(ValidationError::InvalidUrl(format!(
                        "URL contains disallowed query parameters: {:?}", disallowed
                    )))
                }
            } else {
                Ok(())
            }
        },
        Err(e) => Err(ValidationError::InvalidUrl(format!(
            "Invalid URL: {}", e
        )))
    }
}

/// Validate that a URL does not have denylisted IP hosts
pub fn no_ip_hosts(s: &str) -> ValidationResult<()> {
    match Url::parse(s) {
        Ok(url) => {
            if let Some(host) = url.host() {
                match host {
                    Host::Domain(_) => Ok(()),
                    Host::Ipv4(_) | Host::Ipv6(_) => Err(ValidationError::InvalidUrl(
                        "URLs with IP addresses as hosts are not allowed".to_string()
                    )),
                }
            } else {
                Err(ValidationError::InvalidUrl(
                    "URL has no host component".to_string()
                ))
            }
        },
        Err(e) => Err(ValidationError::InvalidUrl(format!(
            "Invalid URL: {}", e
        )))
    }
}

/// Validate that a URL has a file extension in the allowed list
pub fn allowed_file_extension(s: &str, allowed: &[&str]) -> ValidationResult<()> {
    match Url::parse(s) {
        Ok(url) => {
            let path = url.path();
            if let Some(extension) = path.split('.').last() {
                if allowed.contains(&extension) {
                    Ok(())
                } else {
                    Err(ValidationError::InvalidUrl(format!(
                        "URL has disallowed file extension '{}'. Allowed extensions: {:?}",
                        extension, allowed
                    )))
                }
            } else {
                // No file extension
                Err(ValidationError::InvalidUrl(
                    "URL path has no file extension".to_string()
                ))
            }
        },
        Err(e) => Err(ValidationError::InvalidUrl(format!(
            "Invalid URL: {}", e
        )))
    }
}

/// Validate that a URL's path does not contain parent directory traversal
pub fn no_path_traversal(s: &str) -> ValidationResult<()> {
    match Url::parse(s) {
        Ok(url) => {
            let path = url.path();
            
            // Check for parent directory traversal
            if path.contains("/../") || path.ends_with("/..") || path == ".." || path.starts_with("../") {
                Err(ValidationError::SecurityThreat(
                    "URL contains directory traversal patterns".to_string()
                ))
            } else {
                Ok(())
            }
        },
        Err(e) => Err(ValidationError::InvalidUrl(format!(
            "Invalid URL: {}", e
        )))
    }
}

/// Convenience validator that combines multiple common URL validations
pub fn is_safe_url(s: &str) -> ValidationResult<()> {
    // Safe protocols
    allowed_protocol(s, &["http", "https"])?;
    
    // No credentials
    no_credentials(s)?;
    
    // No path traversal
    no_path_traversal(s)?;
    
    // No IP hosts (require domain names)
    no_ip_hosts(s)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_url() {
        assert!(is_url("https://example.com").is_ok());
        assert!(is_url("not a url").is_err());
    }

    #[test]
    fn test_allowed_protocol() {
        assert!(allowed_protocol("https://example.com", &["http", "https"]).is_ok());
        assert!(allowed_protocol("ftp://example.com", &["http", "https"]).is_err());
    }

    #[test]
    fn test_allowed_domain() {
        assert!(allowed_domain("https://example.com", &["example.com", "test.com"]).is_ok());
        assert!(allowed_domain("https://subdomain.example.com", &[".example.com"]).is_ok());
        assert!(allowed_domain("https://other.com", &["example.com"]).is_err());
    }

    #[test]
    fn test_denied_domain() {
        assert!(denied_domain("https://example.com", &["evil.com", "bad.com"]).is_ok());
        assert!(denied_domain("https://subdomain.evil.com", &[".evil.com"]).is_err());
    }

    #[test]
    fn test_no_credentials() {
        assert!(no_credentials("https://example.com").is_ok());
        assert!(no_credentials("https://user:pass@example.com").is_err());
    }

    #[test]
    fn test_valid_port() {
        assert!(valid_port("https://example.com:443", Some(&[443, 80])).is_ok());
        assert!(valid_port("https://example.com:8080", Some(&[443, 80])).is_err());
        assert!(valid_port("https://example.com", None).is_ok());
    }

    #[test]
    fn test_no_ip_hosts() {
        assert!(no_ip_hosts("https://example.com").is_ok());
        assert!(no_ip_hosts("https://127.0.0.1").is_err());
        assert!(no_ip_hosts("https://[::1]").is_err());
    }

    #[test]
    fn test_no_path_traversal() {
        assert!(no_path_traversal("https://example.com/path/to/file").is_ok());
        assert!(no_path_traversal("https://example.com/path/../to/file").is_err());
        assert!(no_path_traversal("https://example.com/../file").is_err());
    }

    #[test]
    fn test_is_safe_url() {
        assert!(is_safe_url("https://example.com/path").is_ok());
        assert!(is_safe_url("ftp://example.com").is_err());
        assert!(is_safe_url("https://user:pass@example.com").is_err());
        assert!(is_safe_url("https://example.com/../file").is_err());
        assert!(is_safe_url("https://127.0.0.1").is_err());
    }
}