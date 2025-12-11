// safety-service-rs/src/main.rs
// Main Entry Point for safety-service-rs
// Implements the SafetyService gRPC server with Enhanced Policy Engine
// Includes improved validation, sanitization, and ReDoS protection

use input_validation_rs::prelude::*;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tonic::{Request, Response, Status, transport::Server};

// Track service start time for uptime reporting
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

// Import Generated Code and Types
pub mod agi_core {
    tonic::include_proto!("agi_core");
}

// Enhanced Threat Filter Module
mod threat_filter;
mod validation;

use agi_core::{
    HealthRequest, HealthResponse, ThreatCheck, ThreatResponse, ValidationRequest,
    ValidationResponse,
    health_service_server::{HealthService, HealthServiceServer},
    safety_service_server::{SafetyService, SafetyServiceServer},
};

// Policy Engine Configuration
#[derive(Debug)]
pub struct PolicyEngine {
    // Blocked keywords (high severity)
    blocked_keywords: HashSet<String>,
    // Warning keywords (medium severity)
    warning_keywords: HashSet<String>,
    // Blocked operations
    blocked_operations: HashSet<String>,
    // Max risk threshold for approval
    max_risk_threshold: i32,
    // Filter sensitivity (1-10)
    filter_sensitivity: u8,
    // Block unsafe links
    block_unsafe_links: bool,
    // Maximum consecutive failures
    max_consecutive_failures: u8,
    // Use enhanced validation
    use_enhanced_validation: bool,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        let mut blocked_keywords = HashSet::new();
        let mut warning_keywords = HashSet::new();
        let mut blocked_operations = HashSet::new();

        // High severity blocked keywords
        for kw in [
            "rm -rf",
            "format c:",
            "del /s /q",
            "shutdown",
            "reboot",
            "drop database",
            "delete from",
            "truncate table",
            "password",
            "credit card",
            "ssn",
            "api_key",
            "secret",
        ] {
            blocked_keywords.insert(kw.to_lowercase());
        }

        // Medium severity warning keywords
        for kw in [
            "sudo", "admin", "root", "system32", "registry", "install", "download", "execute",
            "eval",
        ] {
            warning_keywords.insert(kw.to_lowercase());
        }

        // Blocked operations
        for op in [
            "delete_all",
            "format_disk",
            "shutdown_system",
            "modify_registry",
            "disable_security",
        ] {
            blocked_operations.insert(op.to_string());
        }

        // Additional blocked keywords from environment
        if let Ok(additional_keywords) = env::var("AGENT_SAFETY_ADDITIONAL_BLOCKED_KEYWORDS") {
            for kw in additional_keywords.split(',') {
                if !kw.trim().is_empty() {
                    blocked_keywords.insert(kw.trim().to_lowercase());
                }
            }
        }

        // Additional blocked operations from environment
        if let Ok(additional_ops) = env::var("AGENT_SAFETY_ADDITIONAL_BLOCKED_OPERATIONS") {
            for op in additional_ops.split(',') {
                if !op.trim().is_empty() {
                    blocked_operations.insert(op.trim().to_string());
                }
            }
        }

        // Read customized safety parameters from environment variables
        let max_risk_threshold = Self::get_env_var("AGENT_SAFETY_RISK_THRESHOLD", 5);
        let filter_sensitivity = Self::get_env_var("AGENT_SAFETY_FILTER_SENSITIVITY", 7);
        let block_unsafe_links = Self::get_env_var("AGENT_SAFETY_BLOCK_UNSAFE_LINKS", true);
        let max_consecutive_failures =
            Self::get_env_var("AGENT_SAFETY_MAX_CONSECUTIVE_FAILURES", 3);

        // Check if we should use enhanced validation (default to true)
        let use_enhanced = Self::get_env_var::<bool>("AGENT_SAFETY_USE_ENHANCED_VALIDATION", true);

        Self {
            blocked_keywords,
            warning_keywords,
            blocked_operations,
            max_risk_threshold,
            filter_sensitivity,
            block_unsafe_links,
            max_consecutive_failures,
            use_enhanced_validation: use_enhanced,
        }
    }
}

impl PolicyEngine {
    // Helper function to read environment variables with default values
    fn get_env_var<T: FromStr>(name: &str, default: T) -> T {
        env::var(name)
            .ok()
            .and_then(|v| v.parse::<T>().ok())
            .unwrap_or(default)
    }

    pub fn evaluate(&self, content: &str, operation: Option<&str>) -> (bool, i32, String) {
        let content_lower = content.to_lowercase();
        let mut risk_level = 0;
        let mut violations = Vec::new();

        // Apply filter sensitivity to adjust severity based on configuration
        let sensitivity_multiplier = self.filter_sensitivity as f32 / 7.0;

        // Check blocked keywords (high severity: +5 risk)
        for kw in &self.blocked_keywords {
            if content_lower.contains(kw) {
                let adjusted_risk = (5.0 * sensitivity_multiplier).round() as i32;
                risk_level += adjusted_risk;
                violations.push(format!("Blocked keyword: '{}'", kw));
            }
        }

        // Check warning keywords (medium severity: +2 risk)
        for kw in &self.warning_keywords {
            if content_lower.contains(kw) {
                let adjusted_risk = (2.0 * sensitivity_multiplier).round() as i32;
                risk_level += adjusted_risk;
                violations.push(format!("Warning keyword: '{}'", kw));
            }
        }

        // Check blocked operations
        if let Some(op) = operation {
            if self.blocked_operations.contains(op) {
                let adjusted_risk = (10.0 * sensitivity_multiplier).round() as i32;
                risk_level += adjusted_risk;
                violations.push(format!("Blocked operation: '{}'", op));
            }
        }

        // Check for links if block_unsafe_links is enabled
        if self.block_unsafe_links
            && (content_lower.contains("http://") || content_lower.contains("https://"))
        {
            // Simple check for potentially unsafe links
            let unsafe_link_patterns = [
                "download.",
                ".exe",
                ".msi",
                ".dmg",
                ".apk",
                "torrent",
                "onion.",
            ];
            for pattern in unsafe_link_patterns {
                if content_lower.contains(pattern) {
                    let adjusted_risk = (3.0 * sensitivity_multiplier).round() as i32;
                    risk_level += adjusted_risk;
                    violations.push(format!("Potentially unsafe link pattern: '{}'", pattern));
                }
            }
        }

        // Determine approval
        let approved = risk_level <= self.max_risk_threshold;
        let reason = if violations.is_empty() {
            "Policy check passed: no violations detected.".to_string()
        } else {
            format!("Policy violations: {}", violations.join("; "))
        };

        (approved, risk_level, reason)
    }

    pub fn detect_threat(&self, content: &str, source: &str) -> (bool, String, f32) {
        let content_lower = content.to_lowercase();
        let mut threat_score = 0.0;
        let mut threat_types = Vec::new();

        // Apply sensitivity factor to threat detection
        let sensitivity_factor = self.filter_sensitivity as f32 / 7.0;

        // Injection patterns
        let injection_patterns = [
            "'; drop",
            "\" or 1=1",
            "<script>",
            "javascript:",
            "onclick=",
            "onerror=",
            "../",
            "..\\",
        ];
        for pattern in injection_patterns {
            if content_lower.contains(pattern) {
                threat_score += 0.3 * sensitivity_factor;
                threat_types.push("injection");
            }
        }

        // Malware indicators
        let malware_patterns = [
            "exec(",
            "eval(",
            "base64_decode",
            "cmd.exe",
            "powershell -",
            "wget ",
            "curl -o",
        ];
        for pattern in malware_patterns {
            if content_lower.contains(pattern) {
                threat_score += 0.25 * sensitivity_factor;
                threat_types.push("malware");
            }
        }

        // Suspicious source patterns
        let suspicious_sources = ["unknown", "external", "untrusted"];
        for src in suspicious_sources {
            if source.to_lowercase().contains(src) {
                threat_score += 0.15 * sensitivity_factor;
                threat_types.push("suspicious_source");
            }
        }

        // Cap threat score at 1.0
        let threat_score = f32::min(threat_score, 1.0);

        // Adjust threshold based on sensitivity
        let threshold = 0.5 / sensitivity_factor;
        let adjusted_threshold = f32::max(0.3, f32::min(threshold, 0.7)); // Keep the threshold in a reasonable range

        let is_threat = threat_score > adjusted_threshold;
        let threat_type = if threat_types.is_empty() {
            "none".to_string()
        } else {
            threat_types
                .into_iter()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(",")
        };

        (is_threat, threat_type, threat_score)
    }
}

// Define the Safety Server Structure
#[derive(Debug)]
pub struct SafetyServer {
    policy_engine: PolicyEngine,
}

impl Default for SafetyServer {
    fn default() -> Self {
        Self {
            policy_engine: PolicyEngine::default(),
        }
    }
}

// Implement the SafetyService Trait
#[tonic::async_trait]
impl SafetyService for SafetyServer {
    async fn check_policy(
        &self,
        request: Request<ValidationRequest>,
    ) -> Result<Response<ValidationResponse>, Status> {
        let req_data = request.into_inner();

        let request_id = req_data
            .request
            .as_ref()
            .map(|r| r.id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        log::info!("Received CheckPolicy request: id={}", request_id);

        let request_payload = req_data
            .request
            .as_ref()
            .and_then(|r| String::from_utf8(r.payload.clone()).ok())
            .unwrap_or_default();

        let operation = req_data.request.as_ref().map(|r| r.method.as_str());

        let (approved, risk_level, reason) =
            self.policy_engine.evaluate(&request_payload, operation);

        log::info!(
            "Policy check result for {}: approved={}, risk_level={}",
            request_id,
            approved,
            risk_level
        );

        let reply = ValidationResponse {
            approved,
            reason,
            risk_level,
        };

        Ok(Response::new(reply))
    }

    async fn validate_request(
        &self,
        request: Request<ValidationRequest>,
    ) -> Result<Response<ValidationResponse>, Status> {
        let req_data = request.into_inner();

        let request_id = req_data
            .request
            .as_ref()
            .map(|r| r.id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        log::info!("Received ValidateRequest: id={}", request_id);

        // Basic structure validation
        let has_request = req_data.request.is_some();
        let has_payload = req_data
            .request
            .as_ref()
            .map(|r| !r.payload.is_empty())
            .unwrap_or(false);

        let (approved, risk_level, reason) = if !has_request {
            (false, 10, "Missing request object".to_string())
        } else if !has_payload {
            (false, 5, "Empty payload".to_string())
        } else {
            // Get payload
            let payload = req_data
                .request
                .as_ref()
                .and_then(|r| String::from_utf8(r.payload.clone()).ok())
                .unwrap_or_default();

            // SANITIZE the payload to handle malformed input
            let sanitized_payload = validation::sanitize_input(&payload);

            // FAST NLP PRE-CHECK: Run threat filter BEFORE any LLM call
            let threat_detection = if self.policy_engine.use_enhanced_validation {
                // Use the enhanced/safe detection method
                threat_filter::detect_threat_safe(&sanitized_payload)
            } else {
                // Fallback to original detection method
                threat_filter::detect_threat(&sanitized_payload)
            };

            if threat_detection.is_suspicious {
                log::warn!(
                    "Threat detected in request {}: {:?} (severity: {})",
                    request_id,
                    threat_detection.threat_type,
                    threat_detection.severity.as_str()
                );
                (
                    false,
                    10, // Max risk
                    format!(
                        "Security threat detected: {} - pattern: {:?}",
                        threat_detection.threat_type.unwrap_or_default(),
                        threat_detection.matched_pattern
                    ),
                )
            } else {
                // Validate input with enhanced validation if enabled
                if self.policy_engine.use_enhanced_validation {
                    // Check for validation errors
                    if let Err(e) = validation::validate_content(&sanitized_payload, None) {
                        log::warn!("Validation error in request {}: {}", request_id, e);
                        return Ok(Response::new(ValidationResponse {
                            approved: false,
                            risk_level: 8,
                            reason: format!("Input validation failed: {}", e),
                        }));
                    }
                }

                // No immediate threat - proceed with policy engine
                self.policy_engine.evaluate(&sanitized_payload, None)
            }
        };

        log::info!(
            "Request validation result for {}: approved={}, risk_level={}",
            request_id,
            approved,
            risk_level
        );

        let reply = ValidationResponse {
            approved,
            reason,
            risk_level,
        };

        Ok(Response::new(reply))
    }

    async fn check_threat(
        &self,
        request: Request<ThreatCheck>,
    ) -> Result<Response<ThreatResponse>, Status> {
        let req_data = request.into_inner();

        log::info!(
            "Received CheckThreat: source={}, content_length={}",
            req_data.source,
            req_data.content.len()
        );

        let (is_threat, threat_type, confidence) = self
            .policy_engine
            .detect_threat(&req_data.content, &req_data.source);

        log::info!(
            "Threat check result: is_threat={}, type={}, confidence={:.2}",
            is_threat,
            threat_type,
            confidence
        );

        let reply = ThreatResponse {
            is_threat,
            threat_type,
            confidence,
        };

        Ok(Response::new(reply))
    }
}

// Main function to start the gRPC server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Read address from environment variable or use the default port 50055
    let addr_str = env::var("SAFETY_SERVICE_ADDR").unwrap_or_else(|_| "0.0.0.0:50055".to_string());

    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str
            .strip_prefix("http://")
            .unwrap_or(&addr_str)
            .parse()?
    } else {
        addr_str.parse()?
    };

    let safety_server = SafetyServer::default();

    log::info!("SafetyService (Policy Engine) starting on {}", addr);
    log::info!(
        "Policy Engine initialized with {} blocked keywords, {} warning keywords",
        safety_server.policy_engine.blocked_keywords.len(),
        safety_server.policy_engine.warning_keywords.len()
    );

    log::info!(
        "Safety Configuration: Filter Sensitivity={}, Risk Threshold={}, Block Unsafe Links={}, Max Consecutive Failures={}, Enhanced Validation={}",
        safety_server.policy_engine.filter_sensitivity,
        safety_server.policy_engine.max_risk_threshold,
        safety_server.policy_engine.block_unsafe_links,
        safety_server.policy_engine.max_consecutive_failures,
        safety_server.policy_engine.use_enhanced_validation
    );

    println!("SafetyService listening on {}", addr);

    // Initialize start time
    let _ = *START_TIME;

    let safety_server = Arc::new(safety_server);
    let safety_for_health = safety_server.clone();

    Server::builder()
        .add_service(SafetyServiceServer::from_arc(safety_server))
        .add_service(HealthServiceServer::from_arc(safety_for_health))
        .serve(addr)
        .await?;

    Ok(())
}

// Implement HealthService for SafetyServer
#[tonic::async_trait]
impl HealthService for SafetyServer {
    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;

        let mut dependencies = HashMap::new();
        dependencies.insert("policy_engine".to_string(), "ACTIVE".to_string());

        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "safety-service".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}
