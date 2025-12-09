// red-team-rs/src/main.rs
// RED Team Agent - Ethical Adversary for Vulnerability Scanning and Attack Simulation
// Port 50068

use tonic::{transport::Server, Request, Response, Status};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use std::net::SocketAddr;
use std::env;
use std::collections::HashMap;
use once_cell::sync::Lazy;

static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

pub mod agi_core {
    tonic::include_proto!("agi_core");
}

use agi_core::{
    red_team_service_server::{RedTeamService, RedTeamServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    agent_registry_service_client::AgentRegistryServiceClient,
    ScanRequest,
    ScanResult,
    Vulnerability,
    AttackSimulationRequest,
    AttackSimulationResult,
    ReportRequest,
    SecurityReport,
    RegisterAgentRequest,
    HealthRequest,
    HealthResponse,
};

#[derive(Debug)]
pub struct RedTeamServer {
    scans_completed: AtomicU64,
    simulations_run: AtomicU64,
}

impl Default for RedTeamServer {
    fn default() -> Self {
        Self {
            scans_completed: AtomicU64::new(0),
            simulations_run: AtomicU64::new(0),
        }
    }
}

#[tonic::async_trait]
impl RedTeamService for RedTeamServer {
    async fn scan_vulnerabilities(
        &self,
        request: Request<ScanRequest>,
    ) -> Result<Response<ScanResult>, Status> {
        let req = request.into_inner();
        let scan_id = uuid::Uuid::new_v4().to_string();
        
        log::info!("ScanVulnerabilities: target='{}', type='{}'", req.target, req.scan_type);
        
        self.scans_completed.fetch_add(1, Ordering::Relaxed);
        
        // Simulated vulnerability scan results
        let vulnerabilities = vec![
            Vulnerability {
                id: "CVE-2024-0001".to_string(),
                name: "Example Vulnerability".to_string(),
                severity: "MEDIUM".to_string(),
                description: format!("Simulated vulnerability found in {}", req.target),
                remediation: "Apply latest security patches".to_string(),
            },
        ];
        
        let risk_score = match req.scan_type.as_str() {
            "vuln_scan" => 0.45,
            "port_scan" => 0.25,
            "config_audit" => 0.55,
            _ => 0.3,
        };
        
        let mut metadata = HashMap::new();
        metadata.insert("scan_type".to_string(), req.scan_type);
        metadata.insert("target".to_string(), req.target);
        
        Ok(Response::new(ScanResult {
            scan_id,
            vulnerabilities,
            summary: "Scan completed. 1 vulnerability found.".to_string(),
            risk_score,
            metadata,
        }))
    }
    
    async fn simulate_attack(
        &self,
        request: Request<AttackSimulationRequest>,
    ) -> Result<Response<AttackSimulationResult>, Status> {
        let req = request.into_inner();
        let simulation_id = uuid::Uuid::new_v4().to_string();
        
        log::info!("SimulateAttack: target='{}', type='{}', dry_run={}",
            req.target, req.attack_type, req.dry_run);
        
        self.simulations_run.fetch_add(1, Ordering::Relaxed);
        
        // Simulated attack path
        let attack_path = vec![
            "Initial Access: Phishing email".to_string(),
            "Execution: Malicious macro".to_string(),
            "Persistence: Registry modification".to_string(),
            "Exfiltration: C2 channel".to_string(),
        ];
        
        let success = match req.attack_type.as_str() {
            "phishing" => true,
            "brute_force" => false,
            "injection" => true,
            _ => false,
        };
        
        let mut metadata = HashMap::new();
        metadata.insert("dry_run".to_string(), req.dry_run.to_string());
        metadata.insert("attack_type".to_string(), req.attack_type);
        
        Ok(Response::new(AttackSimulationResult {
            simulation_id,
            success,
            attack_path,
            impact_assessment: "Potential data exfiltration risk".to_string(),
            recommendations: vec![
                "Enable MFA for all users".to_string(),
                "Implement email filtering".to_string(),
                "Deploy EDR solution".to_string(),
            ],
            metadata,
        }))
    }
    
    async fn generate_report(
        &self,
        request: Request<ReportRequest>,
    ) -> Result<Response<SecurityReport>, Status> {
        let req = request.into_inner();
        let report_id = uuid::Uuid::new_v4().to_string();
        
        log::info!("GenerateReport: type='{}', range='{}'", req.report_type, req.time_range);
        
        let scans = self.scans_completed.load(Ordering::Relaxed);
        let sims = self.simulations_run.load(Ordering::Relaxed);
        
        let content = format!(
            "# Security Report: {}\n\n\
            ## Summary\n\
            - Scans completed: {}\n\
            - Attack simulations: {}\n\
            - Time range: {}\n\n\
            ## Key Findings\n\
            1. Network perimeter requires hardening\n\
            2. Phishing susceptibility is medium\n\
            3. Patch management needs improvement\n",
            req.report_type, scans, sims, req.time_range
        );
        
        let mut metadata = HashMap::new();
        metadata.insert("generated_by".to_string(), "RED_TEAM_SHADOW".to_string());
        
        Ok(Response::new(SecurityReport {
            report_id,
            report_type: req.report_type,
            content,
            key_findings: vec![
                "Network hardening needed".to_string(),
                "Phishing awareness training recommended".to_string(),
            ],
            overall_risk_score: 0.42,
            metadata,
        }))
    }
}

#[tonic::async_trait]
impl HealthService for RedTeamServer {
    async fn get_health(&self, _request: Request<HealthRequest>) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let scans = self.scans_completed.load(Ordering::Relaxed);
        let sims = self.simulations_run.load(Ordering::Relaxed);
        
        let mut dependencies = HashMap::new();
        dependencies.insert("scan_engine".to_string(), "ACTIVE".to_string());
        dependencies.insert("scans_completed".to_string(), scans.to_string());
        dependencies.insert("simulations_run".to_string(), sims.to_string());
        
        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "red-team-agent".to_string(),
            uptime_seconds: uptime,
            status: "SERVING".to_string(),
            dependencies,
        }))
    }
}

async fn register_with_agent_registry() {
    let registry_addr = env::var("AGENT_REGISTRY_ADDR")
        .unwrap_or_else(|_| "http://127.0.0.1:50067".to_string());
    
    log::info!("Attempting to register with Agent Registry at {}", registry_addr);
    
    match AgentRegistryServiceClient::connect(registry_addr.clone()).await {
        Ok(mut client) => {
            let request = RegisterAgentRequest {
                name: "RED_TEAM_SHADOW".to_string(),
                port: 50068,
                role: "Ethical Adversary, Vulnerability Scanning, Attack Path Simulation".to_string(),
                capabilities: vec![
                    "vulnerability_scanning".to_string(),
                    "exploit_chaining".to_string(),
                    "risk_simulation".to_string(),
                    "penetration_testing".to_string(),
                    "attack_surface_mapping".to_string(),
                ],
                metadata: HashMap::new(),
            };
            
            match client.register_agent(tonic::Request::new(request)).await {
                Ok(resp) => {
                    log::info!("Registered with Agent Registry: id={}", resp.into_inner().agent_id);
                }
                Err(e) => {
                    log::warn!("Failed to register with Agent Registry: {}", e);
                }
            }
        }
        Err(e) => {
            log::info!("Agent Registry not available (will retry later): {}", e);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let addr_str = env::var("RED_TEAM_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50068".to_string());
    
    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str.strip_prefix("http://").unwrap_or(&addr_str).parse()?
    } else {
        addr_str.parse()?
    };

    let _ = *START_TIME;

    let red_team_server = Arc::new(RedTeamServer::default());
    let rt_for_health = red_team_server.clone();

    // Attempt to register with Agent Registry (non-blocking)
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        register_with_agent_registry().await;
    });

    log::info!("RED Team Agent (RED_TEAM_SHADOW) starting on {}", addr);
    println!("RED Team Agent listening on {}", addr);

    Server::builder()
        .add_service(RedTeamServiceServer::from_arc(red_team_server))
        .add_service(HealthServiceServer::from_arc(rt_for_health))
        .serve(addr)
        .await?;

    Ok(())
}
