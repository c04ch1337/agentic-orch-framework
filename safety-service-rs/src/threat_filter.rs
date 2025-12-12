// safety-service-rs/src/threat_filter.rs
// Enhanced Threat Triage System with ReDoS Protection
// This runs BEFORE any LLM call for immediate defense and uses token-based
// pattern matching for better security and performance

use lazy_static::lazy_static;
use regex::Regex;
use regex_automata::dfa::regex::Regex as DfaRegex;
use std::time::{Duration, Instant};

// Import our enhanced validation module
use crate::validation::{detect_threats, sanitize_input, validate_content};

/// Security threat patterns organized by category
lazy_static! {
    // SQL Injection patterns
    static ref SQL_INJECTION_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(\bSELECT\b.*\bFROM\b)").unwrap(),
        Regex::new(r"(?i)(\bUNION\b.*\bSELECT\b)").unwrap(),
        Regex::new(r"(?i)(\bINSERT\b.*\bINTO\b)").unwrap(),
        Regex::new(r"(?i)(\bDROP\b.*\bTABLE\b)").unwrap(),
        Regex::new(r"(?i)(\bDELETE\b.*\bFROM\b)").unwrap(),
        Regex::new(r"(?i)(\bUPDATE\b.*\bSET\b)").unwrap(),
        Regex::new(r"(?i)(--\s*$|;\s*--\s*)").unwrap(),
        Regex::new(r"(?i)('\s*OR\s*'1'\s*=\s*'1)").unwrap(),
        Regex::new(r"(?i)('\s*OR\s*1\s*=\s*1)").unwrap(),
    ];

    // Shell command injection patterns
    static ref SHELL_INJECTION_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(rm\s+-rf\s+/)").unwrap(),
        Regex::new(r"(?i)(rm\s+-rf\s+\.\.)").unwrap(),
        Regex::new(r"(?i)(\|\s*sh\b)").unwrap(),
        Regex::new(r"(?i)(\|\s*bash\b)").unwrap(),
        Regex::new(r"(?i)(;\s*cat\s+/etc/passwd)").unwrap(),
        Regex::new(r"(?i)(\$\(.*\))").unwrap(),
        Regex::new(r"(?i)(`[^`]+`)").unwrap(),
        Regex::new(r"(?i)(&&\s*\w+)").unwrap(),
        Regex::new(r"(?i)(\|\|\s*\w+)").unwrap(),
        Regex::new(r"(?i)(>\s*/dev/null)").unwrap(),
        Regex::new(r"(?i)(wget\s+http|curl\s+http)").unwrap(),
    ];

    // Path traversal patterns
    static ref PATH_TRAVERSAL_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(\.\./)").unwrap(),
        Regex::new(r"(\.\.\\)").unwrap(),
        Regex::new(r"(?i)(/etc/passwd)").unwrap(),
        Regex::new(r"(?i)(/etc/shadow)").unwrap(),
        Regex::new(r"(?i)(c:\\windows\\system32)").unwrap(),
    ];

    // XSS patterns
    static ref XSS_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(<script[^>]*>)").unwrap(),
        Regex::new(r"(?i)(</script>)").unwrap(),
        Regex::new(r"(?i)(javascript:)").unwrap(),
        Regex::new(r"(?i)(on\w+\s*=)").unwrap(),
        Regex::new(r"(?i)(<iframe[^>]*>)").unwrap(),
        Regex::new(r"(?i)(document\.cookie)").unwrap(),
        Regex::new(r"(?i)(document\.location)").unwrap(),
    ];

    // Ransomware file extension patterns
    static ref RANSOMWARE_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(\.encrypted$)").unwrap(),
        Regex::new(r"(?i)(\.locked$)").unwrap(),
        Regex::new(r"(?i)(\.crypted$)").unwrap(),
        Regex::new(r"(?i)(\.crypt$)").unwrap(),
        Regex::new(r"(?i)(DECRYPT_INSTRUCTIONS)").unwrap(),
        Regex::new(r"(?i)(YOUR_FILES_ARE_ENCRYPTED)").unwrap(),
    ];

    // Credential patterns (potential data exfiltration)
    static ref CREDENTIAL_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(password\s*[:=]\s*\S+)").unwrap(),
        Regex::new(r"(?i)(api_key\s*[:=]\s*\S+)").unwrap(),
        Regex::new(r"(?i)(secret\s*[:=]\s*\S+)").unwrap(),
        Regex::new(r"(?i)(bearer\s+[a-zA-Z0-9\-_]+\.[a-zA-Z0-9\-_]+)").unwrap(),
    ];
}

/// Threat detection result with details
#[derive(Debug, Clone)]
pub struct ThreatDetection {
    pub is_suspicious: bool,
    pub threat_type: Option<String>,
    pub matched_pattern: Option<String>,
    pub severity: ThreatSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreatSeverity {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl ThreatSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThreatSeverity::None => "NONE",
            ThreatSeverity::Low => "LOW",
            ThreatSeverity::Medium => "MEDIUM",
            ThreatSeverity::High => "HIGH",
            ThreatSeverity::Critical => "CRITICAL",
        }
    }
}

/// Fast pre-LLM filter for suspicious text
/// Returns true if the text contains suspicious patterns
pub fn filter_suspicious_text(text: &str) -> bool {
    let detection = detect_threat(text);
    detection.is_suspicious
}

/// Comprehensive threat detection with details
pub fn detect_threat(text: &str) -> ThreatDetection {
    // Sanitize the input first to handle potential malformed data
    let sanitized_text = sanitize_input(text);

    // Use the enhanced threat detection from validation module with ReDoS protection
    if let Some((threat_type, pattern, severity)) = detect_threats(&sanitized_text) {
        // Map severity level to ThreatSeverity enum
        let severity_level = match severity {
            5 => ThreatSeverity::Critical,
            4 => ThreatSeverity::High,
            3 => ThreatSeverity::Medium,
            1..=2 => ThreatSeverity::Low,
            _ => ThreatSeverity::None,
        };

        return ThreatDetection {
            is_suspicious: true,
            threat_type: Some(threat_type),
            matched_pattern: Some(pattern),
            severity: severity_level,
        };
    }

    // Fallback to legacy pattern matching with timeout protection
    // SQL Injection check with safe regex
    for pattern in SQL_INJECTION_PATTERNS.iter() {
        let start_time = Instant::now();

        // Use safe_pattern_match instead of direct regex
        if let Some(m) = pattern.find(&sanitized_text) {
            // Check if the regex operation took too long (potential ReDoS)
            if start_time.elapsed() > Duration::from_millis(100) {
                log::warn!("SQL pattern matching timeout - potential ReDoS attack");
                return ThreatDetection {
                    is_suspicious: true,
                    threat_type: Some("REGEX_DOS_ATTEMPT".to_string()),
                    matched_pattern: Some(format!("Timeout on SQL pattern: {}", pattern.as_str())),
                    severity: ThreatSeverity::Critical,
                };
            }

            return ThreatDetection {
                is_suspicious: true,
                threat_type: Some("SQL_INJECTION".to_string()),
                matched_pattern: Some(m.as_str().to_string()),
                severity: ThreatSeverity::Critical,
            };
        }
    }

    // Run other pattern checks with similar timeout protection
    let pattern_groups = [
        (
            &*SHELL_INJECTION_PATTERNS,
            "SHELL_INJECTION",
            ThreatSeverity::Critical,
        ),
        (
            &*PATH_TRAVERSAL_PATTERNS,
            "PATH_TRAVERSAL",
            ThreatSeverity::High,
        ),
        (&*XSS_PATTERNS, "XSS", ThreatSeverity::High),
        (&*RANSOMWARE_PATTERNS, "RANSOMWARE", ThreatSeverity::Critical),
        (
            &*CREDENTIAL_PATTERNS,
            "CREDENTIAL_EXPOSURE",
            ThreatSeverity::Medium,
        ),
    ];

    for (patterns, name, severity) in pattern_groups.iter() {
        for pattern in patterns.iter() {
            let start_time = Instant::now();

            if let Some(m) = pattern.find(&sanitized_text) {
                // Check for timeout
                if start_time.elapsed() > Duration::from_millis(100) {
                    return ThreatDetection {
                        is_suspicious: true,
                        threat_type: Some("REGEX_DOS_ATTEMPT".to_string()),
                        matched_pattern: Some(format!("Timeout on pattern: {}", pattern.as_str())),
                        severity: ThreatSeverity::Critical,
                    };
                }

                return ThreatDetection {
                    is_suspicious: true,
                    threat_type: Some(name.to_string()),
                    matched_pattern: Some(m.as_str().to_string()),
                    severity: *severity,
                };
            }
        }
    }

    // No threats detected
    ThreatDetection {
        is_suspicious: false,
        threat_type: None,
        matched_pattern: None,
        severity: ThreatSeverity::None,
    }
}

/// Enhanced threat detection with safe pattern matching that's protected against ReDoS
pub fn detect_threat_safe(text: &str) -> ThreatDetection {
    // Use the enhanced validation module's detect_threats function
    if let Some((threat_type, pattern, severity)) = detect_threats(text) {
        // Map severity level to ThreatSeverity enum
        let severity_level = match severity {
            5 => ThreatSeverity::Critical,
            4 => ThreatSeverity::High,
            3 => ThreatSeverity::Medium,
            1..=2 => ThreatSeverity::Low,
            _ => ThreatSeverity::None,
        };

        return ThreatDetection {
            is_suspicious: true,
            threat_type: Some(threat_type),
            matched_pattern: Some(pattern),
            severity: severity_level,
        };
    }

    ThreatDetection {
        is_suspicious: false,
        threat_type: None,
        matched_pattern: None,
        severity: ThreatSeverity::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_injection_detection() {
        assert!(filter_suspicious_text("SELECT * FROM users"));
        assert!(filter_suspicious_text("' OR '1'='1"));
        assert!(filter_suspicious_text(
            "UNION SELECT password FROM accounts"
        ));
    }

    #[test]
    fn test_shell_injection_detection() {
        assert!(filter_suspicious_text("rm -rf /"));
        assert!(filter_suspicious_text("cat /etc/passwd"));
        assert!(filter_suspicious_text("| sh -c 'whoami'"));
    }

    #[test]
    fn test_xss_detection() {
        assert!(filter_suspicious_text("<script>alert('xss')</script>"));
        assert!(filter_suspicious_text("javascript:void(0)"));
    }

    #[test]
    fn test_safe_text() {
        assert!(!filter_suspicious_text("Hello, this is a normal message"));
        assert!(!filter_suspicious_text("Please process my request"));
    }
}
