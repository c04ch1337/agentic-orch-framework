// blue-team-rs/src/main.rs
// BLUE Team Agent - Autonomous Defense, Incident Triage, Hardening
// Port 50069

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
    blue_team_service_server::{BlueTeamService, BlueTeamServiceServer},
    health_service_server::{HealthService, HealthServiceServer},
    agent_registry_service_client::AgentRegistryServiceClient,
    AnomalyTriageRequest,
    TriageResult,
    ContainmentRequest,
    ContainmentResult,
    HardeningRequest,
    HardeningResult,
    RegisterAgentRequest,
    HealthRequest,
    HealthResponse,
};

#[derive(Debug)]
pub struct BlueTeamServer {
    anomalies_triaged: AtomicU64,
    threats_contained: AtomicU64,
    systems_hardened: AtomicU64,
}

impl Default for BlueTeamServer {
    fn default() -> Self {
        Self {
            anomalies_triaged: AtomicU64::new(0),
            threats_contained: AtomicU64::new(0),
            systems_hardened: AtomicU64::new(0),
        }
    }
}

#[tonic::async_trait]
impl BlueTeamService for BlueTeamServer {
    async fn triage_anomaly(
        &self,
        request: Request<AnomalyTriageRequest>,
    ) -> Result<Response<TriageResult>, Status> {
        let req = request.into_inner();
        let triage_id = uuid::Uuid::new_v4().to_string();
        
        log::info!("TriageAnomaly: id='{}', type='{}', priority={}",
            req.anomaly_id, req.anomaly_type, req.priority);
        
        self.anomalies_triaged.fetch_add(1, Ordering::Relaxed);
        
        // Simulated triage logic based on type and priority
        let (is_threat, classification, severity) = match (req.anomaly_type.as_str(), req.priority) {
            ("network", p) if p >= 4 => (true, "suspicious_traffic", 4),
            ("behavior", p) if p >= 3 => (true, "anomalous_behavior", 3),
            ("access", p) if p >= 4 => (true, "unauthorized_access", 5),
            ("network", _) => (false, "normal_traffic_spike", 1),
            ("behavior", _) => (false, "false_positive", 1),
            _ => (false, "unknown", 2),
        };
        
        let recommended_actions = if is_threat {
            vec![
                "Isolate affected system".to_string(),
                "Collect forensic data".to_string(),
                "Alert security team".to_string(),
            ]
        } else {
            vec!["Continue monitoring".to_string()]
        };
        
        let mut metadata = HashMap::new();
        metadata.insert("anomaly_id".to_string(), req.anomaly_id);
        metadata.insert("anomaly_type".to_string(), req.anomaly_type);
        
        Ok(Response::new(TriageResult {
            triage_id,
            is_threat,
            threat_classification: classification.to_string(),
            severity,
            recommended_actions,
            metadata,
        }))
    }
    
    async fn contain_threat(
        &self,
        request: Request<ContainmentRequest>,
    ) -> Result<Response<ContainmentResult>, Status> {
        let req = request.into_inner();
        let containment_id = uuid::Uuid::new_v4().to_string();
        
        log::info!("ContainThreat: id='{}', type='{}', target='{}', auto={}",
            req.threat_id, req.containment_type, req.target, req.auto_remediate);
        
        self.threats_contained.fetch_add(1, Ordering::Relaxed);
        
        // Simulated containment actions
        let (success, action_taken, status) = match req.containment_type.as_str() {
            "isolate" => (true, "Network isolation applied", "CONTAINED"),
            "block" => (true, "Firewall rule added", "CONTAINED"),
            "quarantine" => (true, "Process quarantined", "CONTAINED"),
            _ => (false, "Unknown containment type", "FAILED"),
        };
        
        let mut metadata = HashMap::new();
        metadata.insert("threat_id".to_string(), req.threat_id);
        metadata.insert("target".to_string(), req.target);
        metadata.insert("auto_remediate".to_string(), req.auto_remediate.to_string());
        
        Ok(Response::new(ContainmentResult {
            containment_id,
            success,
            action_taken: action_taken.to_string(),
            status: status.to_string(),
            metadata,
        }))
    }
    
    async fn harden_system(
        &self,
        request: Request<HardeningRequest>,
    ) -> Result<Response<HardeningResult>, Status> {
        let req = request.into_inner();
        let hardening_id = uuid::Uuid::new_v4().to_string();
        
        log::info!("HardenSystem: target='{}', profile='{}', apply={}",
            req.target, req.hardening_profile, req.apply_changes);
        
        self.systems_hardened.fetch_add(1, Ordering::Relaxed);
        
        // Simulated hardening recommendations based on profile
        let changes_recommended = match req.hardening_profile.as_str() {
            "cis" => vec![
                "Disable unnecessary services".to_string(),
                "Enable audit logging".to_string(),
                "Restrict admin access".to_string(),
                "Enable host-based firewall".to_string(),
            ],
            "nist" => vec![
                "Implement access controls".to_string(),
                "Enable encryption at rest".to_string(),
                "Configure automated patching".to_string(),
            ],
            _ => vec![
                "Review security configuration".to_string(),
            ],
        };
        
        let changes_applied = if req.apply_changes {
            changes_recommended.clone()
        } else {
            vec![]
        };
        
        let compliance_score = match req.hardening_profile.as_str() {
            "cis" => 78,
            "nist" => 82,
            _ => 65,
        };
        
        let mut metadata = HashMap::new();
        metadata.insert("target".to_string(), req.target);
        metadata.insert("profile".to_string(), req.hardening_profile);
        
        Ok(Response::new(HardeningResult {
            hardening_id,
            changes_applied,
            changes_recommended,
            compliance_score,
            metadata,
        }))
    }
}

#[tonic::async_trait]
impl HealthService for BlueTeamServer {
    async fn get_health(&self, _request: Request<HealthRequest>) -> Result<Response<HealthResponse>, Status> {
        let uptime = START_TIME.elapsed().as_secs() as i64;
        let anomalies = self.anomalies_triaged.load(Ordering::Relaxed);
        let contained = self.threats_contained.load(Ordering::Relaxed);
        let hardened = self.systems_hardened.load(Ordering::Relaxed);
        
        let mut dependencies = HashMap::new();
        dependencies.insert("defense_engine".to_string(), "ACTIVE".to_string());
        dependencies.insert("anomalies_triaged".to_string(), anomalies.to_string());
        dependencies.insert("threats_contained".to_string(), contained.to_string());
        dependencies.insert("systems_hardened".to_string(), hardened.to_string());
        
        Ok(Response::new(HealthResponse {
            healthy: true,
            service_name: "blue-team-agent".to_string(),
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
                name: "BLUE_TEAM_SENTINEL".to_string(),
                port: 50069,
                role: "Autonomous Defense, Incident Triage, Hardening".to_string(),
                capabilities: vec![
                    "anomaly_triage".to_string(),
                    "threat_containment".to_string(),
                    "patch_management".to_string(),
                    "security_hardening".to_string(),
                    "log_analysis".to_string(),
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

    let addr_str = env::var("BLUE_TEAM_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50069".to_string());
    
    let addr: SocketAddr = if addr_str.starts_with("http://") {
        addr_str.strip_prefix("http://").unwrap_or(&addr_str).parse()?
    } else {
        addr_str.parse()?
    };

    let _ = *START_TIME;

    let blue_team_server = Arc::new(BlueTeamServer::default());
    let bt_for_health = blue_team_server.clone();

    // Attempt to register with Agent Registry (non-blocking)
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        register_with_agent_registry().await;
    });

    log::info!("BLUE Team Agent (BLUE_TEAM_SENTINEL) starting on {}", addr);
    println!("BLUE Team Agent listening on {}", addr);

    Server::builder()
        .add_service(BlueTeamServiceServer::from_arc(blue_team_server))
        .add_service(HealthServiceServer::from_arc(bt_for_health))
        .serve(addr)
        .await?;

    Ok(())
}
