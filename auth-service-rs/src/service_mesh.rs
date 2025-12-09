// auth-service-rs/src/service_mesh.rs
//
// Service Mesh integration for secure service-to-service communication
// Provides:
// - Service discovery
// - Load balancing
// - Circuit breaking
// - Secure mTLS connections between services
// - Connection pooling and backoff

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use anyhow::{Result, anyhow, Context};
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use once_cell::sync::Lazy;
use async_trait::async_trait;

use crate::certificates::CertificateManager;

// Global service mesh instance
static SERVICE_MESH: Lazy<RwLock<Option<Arc<ServiceMesh>>>> = Lazy::new(|| {
    RwLock::new(None)
});

/// Service status enum
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Healthy,     // Service is healthy and handling requests normally
    Degraded,    // Service is working but with reduced capabilities
    Unhealthy,   // Service is responding but not working correctly
    Unknown,     // Service status not known
    Offline,     // Service is not responding
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceStatus::Healthy => write!(f, "healthy"),
            ServiceStatus::Degraded => write!(f, "degraded"),
            ServiceStatus::Unhealthy => write!(f, "unhealthy"),
            ServiceStatus::Unknown => write!(f, "unknown"),
            ServiceStatus::Offline => write!(f, "offline"),
        }
    }
}

/// Service endpoint information
#[derive(Debug, Clone)]
pub struct ServiceEndpoint {
    pub service_id: String,
    pub address: String,
    pub status: ServiceStatus,
    pub last_checked: Instant,
    pub use_tls: bool,
    pub weight: u32,  // For load balancing
    pub metadata: HashMap<String, String>,
    pub channel: Option<Arc<Channel>>,
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation, requests allowed
    Open,     // Circuit is open, all requests fail fast
    HalfOpen, // Testing if service recovered
}

/// Circuit breaker for a service endpoint
#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    failure_threshold: u32,
    reset_timeout: Duration,
    last_failure: Option<Instant>,
    half_open_allowed: bool,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold: threshold,
            reset_timeout,
            last_failure: None,
            half_open_allowed: true,
        }
    }
    
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => {
                // Reset failure counter on success
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                // Service recovered, close circuit
                self.state = CircuitState::Closed;
                self.failure_count = 0;
                debug!("Circuit breaker returned to closed state after successful test request");
            }
            CircuitState::Open => {
                // This shouldn't happen - no requests in open state
            }
        }
    }
    
    pub fn record_failure(&mut self) {
        self.last_failure = Some(Instant::now());
        
        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                    debug!("Circuit breaker tripped to open state after {} failures", 
                           self.failure_count);
                }
            }
            CircuitState::HalfOpen => {
                // Failed test request, back to open
                self.state = CircuitState::Open;
                self.half_open_allowed = false;
                debug!("Circuit breaker returned to open state after failed test request");
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }
    
    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if it's time to try a test request
                if let Some(last_failure) = self.last_failure {
                    if last_failure.elapsed() >= self.reset_timeout {
                        self.state = CircuitState::HalfOpen;
                        self.half_open_allowed = true;
                        debug!("Circuit breaker moving to half-open state to test service");
                    }
                }
                
                match self.state {
                    CircuitState::HalfOpen => {
                        if self.half_open_allowed {
                            self.half_open_allowed = false;  // Only allow one test request
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            CircuitState::HalfOpen => {
                if self.half_open_allowed {
                    self.half_open_allowed = false;  // Only allow one test request
                    true
                } else {
                    false
                }
            }
        }
    }
    
    pub fn get_state(&self) -> CircuitState {
        self.state
    }
    
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.last_failure = None;
        self.half_open_allowed = true;
    }
}

/// Main service mesh management
pub struct ServiceMesh {
    // All known services and their endpoints
    services: RwLock<HashMap<String, Vec<ServiceEndpoint>>>,
    
    // Circuit breakers for each endpoint
    circuit_breakers: RwLock<HashMap<String, CircuitBreaker>>,  // key: "service_id/address"
    
    // Configuration
    use_mtls: bool,
    cert_manager: Arc<CertificateManager>,
    
    // Service discovery options
    discovery_interval: Duration,
    discovery_source_url: Option<String>,
    
    // Refresh channels automatically
    channel_refresh_interval: Duration,
    
    // Local service identity
    service_id: String,
    
    // Load balancing strategy
    load_balancing_policy: LoadBalancingPolicy,
}

/// Load balancing policies
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadBalancingPolicy {
    RoundRobin,
    LeastConnections,
    Random,
    WeightedRandom,
}

impl ServiceMesh {
    /// Create a new service mesh
    pub async fn new(
        service_id: &str,
        cert_manager: Arc<CertificateManager>,
        use_mtls: bool,
        discovery_source_url: Option<String>,
        discovery_interval_secs: Option<u64>,
        load_balancing_policy: Option<LoadBalancingPolicy>,
    ) -> Result<Self> {
        let mesh = Self {
            services: RwLock::new(HashMap::new()),
            circuit_breakers: RwLock::new(HashMap::new()),
            use_mtls,
            cert_manager,
            discovery_interval: Duration::from_secs(discovery_interval_secs.unwrap_or(30)),
            discovery_source_url,
            channel_refresh_interval: Duration::from_secs(300),  // 5 minutes
            service_id: service_id.to_string(),
            load_balancing_policy: load_balancing_policy.unwrap_or(LoadBalancingPolicy::RoundRobin),
        };
        
        // Start service discovery if a source is configured
        if mesh.discovery_source_url.is_some() {
            let sm = Arc::new(mesh.clone());
            tokio::spawn(async move {
                sm.service_discovery_loop().await;
            });
        }
        
        Ok(mesh)
    }
    
    /// Initialize and set the global service mesh instance
    pub async fn init_global(
        service_id: &str,
        cert_manager: Arc<CertificateManager>,
        use_mtls: bool,
        discovery_source_url: Option<String>,
        discovery_interval_secs: Option<u64>,
        load_balancing_policy: Option<LoadBalancingPolicy>,
    ) -> Result<()> {
        let mesh = Self::new(
            service_id,
            cert_manager,
            use_mtls,
            discovery_source_url,
            discovery_interval_secs,
            load_balancing_policy,
        ).await?;
        
        let mut service_mesh = SERVICE_MESH.write().await;
        *service_mesh = Some(Arc::new(mesh));
        
        Ok(())
    }
    
    /// Get the global service mesh instance
    pub async fn get_global() -> Result<Arc<ServiceMesh>> {
        let service_mesh = SERVICE_MESH.read().await;
        match &*service_mesh {
            Some(mesh) => Ok(mesh.clone()),
            None => Err(anyhow!("Global service mesh not initialized")),
        }
    }
    
    /// Register a service endpoint with the mesh
    pub async fn register_service(
        &self,
        service_id: &str,
        address: &str,
        use_tls: bool,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<()> {
        let endpoint = ServiceEndpoint {
            service_id: service_id.to_string(),
            address: address.to_string(),
            status: ServiceStatus::Unknown,
            last_checked: Instant::now(),
            use_tls,
            weight: 100,  // Default weight
            metadata: metadata.unwrap_or_default(),
            channel: None,
        };
        
        // Create circuit breaker for this endpoint
        let cb_key = format!("{}/{}", service_id, address);
        let mut circuit_breakers = self.circuit_breakers.write().await;
        circuit_breakers.insert(cb_key, CircuitBreaker::new(3, Duration::from_secs(30)));
        
        // Add the endpoint to the service registry
        let mut services = self.services.write().await;
        
        if let Some(endpoints) = services.get_mut(service_id) {
            // Check if endpoint with this address already exists
            if !endpoints.iter().any(|e| e.address == address) {
                // Add the new endpoint
                endpoints.push(endpoint);
                info!("Registered new endpoint for service {} at {}", service_id, address);
            } else {
                return Err(anyhow!("Endpoint already registered for service {} at {}", 
                                  service_id, address));
            }
        } else {
            // First endpoint for this service
            services.insert(service_id.to_string(), vec![endpoint]);
            info!("Registered new service {} at {}", service_id, address);
        }
        
        Ok(())
    }
    
    /// Deregister a service endpoint
    pub async fn deregister_service(
        &self,
        service_id: &str,
        address: &str,
    ) -> Result<()> {
        let mut services = self.services.write().await;
        
        if let Some(endpoints) = services.get_mut(service_id) {
            let initial_len = endpoints.len();
            endpoints.retain(|e| e.address != address);
            
            // Remove circuit breaker
            let cb_key = format!("{}/{}", service_id, address);
            let mut circuit_breakers = self.circuit_breakers.write().await;
            circuit_breakers.remove(&cb_key);
            
            if endpoints.len() < initial_len {
                info!("Deregistered endpoint for service {} at {}", service_id, address);
                return Ok(());
            }
        }
        
        Err(anyhow!("Endpoint not found for service {} at {}", service_id, address))
    }
    
    /// Update a service endpoint's status
    pub async fn update_service_status(
        &self,
        service_id: &str,
        address: &str,
        status: ServiceStatus,
    ) -> Result<()> {
        let mut services = self.services.write().await;
        
        if let Some(endpoints) = services.get_mut(service_id) {
            for endpoint in endpoints {
                if endpoint.address == address {
                    endpoint.status = status;
                    endpoint.last_checked = Instant::now();
                    
                    // Update circuit breaker based on status
                    let cb_key = format!("{}/{}", service_id, address);
                    let mut circuit_breakers = self.circuit_breakers.write().await;
                    
                    if let Some(cb) = circuit_breakers.get_mut(&cb_key) {
                        match status {
                            ServiceStatus::Healthy => cb.record_success(),
                            ServiceStatus::Degraded => {}, // Do nothing, maintain current state
                            _ => cb.record_failure(),
                        }
                    }
                    
                    debug!("Updated status for service {} at {} to {}", 
                           service_id, address, status);
                    return Ok(());
                }
            }
        }
        
        Err(anyhow!("Endpoint not found for service {} at {}", service_id, address))
    }
    
    /// Get a channel for a service using configured TLS settings and policies
    pub async fn get_service_channel(&self, service_id: &str) -> Result<Arc<Channel>> {
        // Get all endpoints for this service
        let services = self.services.read().await;
        
        let endpoints = services.get(service_id)
            .ok_or_else(|| anyhow!("Service {} not found in service registry", service_id))?;
            
        if endpoints.is_empty() {
            return Err(anyhow!("No endpoints available for service {}", service_id));
        }
        
        // Filter out unhealthy endpoints and check circuit breakers
        let mut available_endpoints = Vec::new();
        let circuit_breakers = self.circuit_breakers.read().await;
        
        for endpoint in endpoints {
            // Skip unhealthy or offline endpoints
            if endpoint.status == ServiceStatus::Unhealthy || 
               endpoint.status == ServiceStatus::Offline {
                continue;
            }
            
            // Check circuit breaker
            let cb_key = format!("{}/{}", service_id, &endpoint.address);
            if let Some(cb) = circuit_breakers.get(&cb_key) {
                // Clone the circuit breaker to check and modify state
                let mut cb_clone = cb.clone();
                if !cb_clone.allow_request() {
                    // Circuit is open, skip this endpoint
                    debug!("Circuit breaker is open for service {} at {}, skipping endpoint",
                           service_id, endpoint.address);
                    continue;
                }
                
                // If we reach here, the circuit breaker allows this request
                // Let's record the breaker state
                let cb_key_clone = cb_key.clone();
                if cb_clone.get_state() != cb.get_state() {
                    // State changed, update the original in a separate task
                    tokio::spawn(async move {
                        if let Ok(mesh) = ServiceMesh::get_global().await {
                            let mut cbs = mesh.circuit_breakers.write().await;
                            if let Some(real_cb) = cbs.get_mut(&cb_key_clone) {
                                *real_cb = cb_clone;
                            }
                        }
                    });
                }
            }
            
            // If we already have a channel for this endpoint, use it
            if let Some(channel) = &endpoint.channel {
                available_endpoints.push((endpoint.clone(), channel.clone()));
            } else {
                // Otherwise, we need to create a new channel
                match self.create_channel_for_endpoint(endpoint).await {
                    Ok(channel) => {
                        // Store the new channel for future use
                        let services_mut = Arc::new(self.services.clone());
                        let endpoint_addr = endpoint.address.clone();
                        let service_id_clone = service_id.to_string();
                        let channel_clone = channel.clone();
                        
                        // Spawn a task to update the channel (so we don't block here)
                        tokio::spawn(async move {
                            let mut services = services_mut.write().await;
                            if let Some(endpoints) = services.get_mut(&service_id_clone) {
                                if let Some(ep) = endpoints.iter_mut().find(|e| e.address == endpoint_addr) {
                                    ep.channel = Some(channel_clone);
                                }
                            }
                        });
                        
                        available_endpoints.push((endpoint.clone(), channel.clone()));
                    },
                    Err(err) => {
                        // Can't create channel to this endpoint
                        warn!("Failed to create channel to service {} at {}: {}",
                              service_id, endpoint.address, err);
                        continue;
                    }
                }
            }
        }
        
        if available_endpoints.is_empty() {
            return Err(anyhow!("No available healthy endpoints for service {}", service_id));
        }
        
        // Apply load balancing policy to select an endpoint
        let selected = match self.load_balancing_policy {
            LoadBalancingPolicy::RoundRobin => {
                // Simple round robin implementation using a random start
                let index = rand::random::<usize>() % available_endpoints.len();
                &available_endpoints[index]
            },
            LoadBalancingPolicy::Random => {
                let index = rand::random::<usize>() % available_endpoints.len();
                &available_endpoints[index]
            },
            LoadBalancingPolicy::WeightedRandom => {
                // Sum up weights
                let total_weight: u32 = available_endpoints.iter()
                    .map(|(ep, _)| ep.weight)
                    .sum();
                    
                // Pick a random weight
                let mut random_weight = rand::random::<u32>() % total_weight;
                
                // Find the corresponding endpoint
                let mut selected_index = 0;
                for (i, (ep, _)) in available_endpoints.iter().enumerate() {
                    if random_weight < ep.weight {
                        selected_index = i;
                        break;
                    }
                    random_weight -= ep.weight;
                }
                
                &available_endpoints[selected_index]
            },
            LoadBalancingPolicy::LeastConnections => {
                // Not implemented in this simple version
                // Would need to track active connections per endpoint
                // For now, fall back to random
                let index = rand::random::<usize>() % available_endpoints.len();
                &available_endpoints[index]
            },
        };
        
        Ok(selected.1.clone())
    }
    
    /// Create a new channel to an endpoint
    async fn create_channel_for_endpoint(&self, endpoint: &ServiceEndpoint) -> Result<Arc<Channel>> {
        let mut channel_builder = Channel::from_shared(endpoint.address.clone())
            .map_err(|e| anyhow!("Invalid service address: {}", e))?;
        
        // Configure TLS if needed
        if endpoint.use_tls {
            if self.use_mtls {
                // Configure mutual TLS
                let (cert_pem, key_pem, _) = self.cert_manager
                    .get_or_create_service_certificate(
                        &self.service_id, 
                        None,  // Default validity
                        None   // No SANs needed for client cert
                    ).await?;
                
                let identity = Identity::from_pem(cert_pem, key_pem);
                
                // Get the CA certificate to validate server
                let ca_cert = self.cert_manager.get_ca_certificate().await?;
                
                // Create TLS config
                let tls_config = ClientTlsConfig::new()
                    .domain_name(endpoint.service_id.clone())  // Verify server as this service ID
                    .identity(identity)
                    .ca_certificate(Certificate::from_pem(&ca_cert));
                
                // Set TLS config
                channel_builder = channel_builder.tls_config(tls_config)
                    .context("Failed to configure TLS")?;
            } else {
                // Standard TLS (not mutual)
                let tls_config = ClientTlsConfig::new()
                    .domain_name(endpoint.service_id.clone());  // Verify server as this service
                
                channel_builder = channel_builder.tls_config(tls_config)
                    .context("Failed to configure TLS")?;
            }
        }
        
        // Connect with timeout
        let channel = tokio::time::timeout(
            Duration::from_secs(5),
            channel_builder.connect()
        ).await
            .context("Connection timeout")??;
        
        Ok(Arc::new(channel))
    }
    
    /// Service discovery loop
    async fn service_discovery_loop(&self) {
        // If no discovery source, nothing to do
        let discovery_url = match &self.discovery_source_url {
            Some(url) => url.clone(),
            None => return,
        };
        
        info!("Starting service discovery with interval {:?}", self.discovery_interval);
        
        loop {
            // Sleep first to allow initial manual registration
            tokio::time::sleep(self.discovery_interval).await;
            
            debug!("Running service discovery from {}", discovery_url);
            
            // Try to fetch service registry from discovery source
            match self.fetch_service_registry(&discovery_url).await {
                Ok(registry) => {
                    self.update_service_registry(registry).await;
                }
                Err(err) => {
                    warn!("Failed to fetch service registry: {}", err);
                }
            }
            
            // Also refresh all channels periodically
            self.refresh_channels().await;
        }
    }
    
    /// Fetch service registry from discovery source
    async fn fetch_service_registry(&self, url: &str) -> Result<Vec<DiscoveredService>> {
        let client = reqwest::Client::new();
        let response = client.get(url)
            .send()
            .await
            .context("Failed to fetch service registry")?;
            
        if !response.status().is_success() {
            return Err(anyhow!("Service discovery failed with status: {}", response.status()));
        }
        
        let registry: Vec<DiscoveredService> = response.json()
            .await
            .context("Failed to parse service registry")?;
            
        Ok(registry)
    }
    
    /// Update local service registry from discovered services
    async fn update_service_registry(&self, discovered_services: Vec<DiscoveredService>) {
        // Get current registry
        let current_services = self.services.read().await.clone();
        
        // Create a map of discovered services for efficient lookup
        let mut discovered_map: HashMap<String, HashMap<String, DiscoveredService>> = HashMap::new();
        
        for service in discovered_services {
            let service_map = discovered_map
                .entry(service.service_id.clone())
                .or_insert_with(HashMap::new);
                
            service_map.insert(service.address.clone(), service);
        }
        
        // Process each current service to find ones that need deregistration
        for (service_id, endpoints) in current_services.iter() {
            let discovered = discovered_map.get(service_id);
            
            for endpoint in endpoints {
                match discovered {
                    Some(service_endpoints) => {
                        if !service_endpoints.contains_key(&endpoint.address) {
                            // Endpoint no longer in discovery, deregister
                            if let Err(err) = self.deregister_service(service_id, &endpoint.address).await {
                                warn!("Failed to deregister service {} at {}: {}",
                                      service_id, endpoint.address, err);
                            }
                        }
                    }
                    None => {
                        // Entire service no longer in discovery, deregister all endpoints
                        if let Err(err) = self.deregister_service(service_id, &endpoint.address).await {
                            warn!("Failed to deregister service {} at {}: {}",
                                  service_id, endpoint.address, err);
                        }
                    }
                }
            }
        }
        
        // Register new services and endpoints
        for (service_id, service_endpoints) in discovered_map {
            for (address, service) in service_endpoints {
                // Check if this endpoint already exists
                let exists = current_services.get(&service_id)
                    .map(|endpoints| endpoints.iter().any(|e| e.address == address))
                    .unwrap_or(false);
                    
                if !exists {
                    // Register the new endpoint
                    if let Err(err) = self.register_service(
                        &service_id,
                        &address,
                        service.use_tls,
                        Some(service.metadata.clone()),
                    ).await {
                        warn!("Failed to register discovered service {} at {}: {}",
                              service_id, address, err);
                    }
                }
                
                // Update status for existing endpoints
                let status = match service.status.as_str() {
                    "healthy" => ServiceStatus::Healthy,
                    "degraded" => ServiceStatus::Degraded,
                    "unhealthy" => ServiceStatus::Unhealthy,
                    "offline" => ServiceStatus::Offline,
                    _ => ServiceStatus::Unknown,
                };
                
                if let Err(err) = self.update_service_status(&service_id, &address, status).await {
                    // May fail if the service was just registered, that's OK
                    debug!("Failed to update status for service {} at {}: {}",
                           service_id, address, err);
                }
            }
        }
    }
    
    /// Refresh all channels periodically
    async fn refresh_channels(&self) {
        debug!("Refreshing all service channels");
        
        let services = self.services.read().await;
        
        for (service_id, endpoints) in services.iter() {
            for endpoint in endpoints {
                if let Some(_) = &endpoint.channel {
                    // Clear the channel to force a refresh next time it's needed
                    let service_id_clone = service_id.clone();
                    let endpoint_addr = endpoint.address.clone();
                    let services_mut = Arc::new(self.services.clone());
                    
                    tokio::spawn(async move {
                        let mut services = services_mut.write().await;
                        if let Some(endpoints) = services.get_mut(&service_id_clone) {
                            if let Some(ep) = endpoints.iter_mut().find(|e| e.address == endpoint_addr) {
                                ep.channel = None;
                            }
                        }
                    });
                }
            }
        }
        
        debug!("All service channels refreshed");
    }
    
    /// Health check a service endpoint
    pub async fn health_check_endpoint(
        &self,
        service_id: &str,
        address: &str,
    ) -> Result<ServiceStatus> {
        let services = self.services.read().await;
        
        let endpoints = services.get(service_id)
            .ok_or_else(|| anyhow!("Service {} not found", service_id))?;
            
        let endpoint = endpoints.iter()
            .find(|e| e.address == address)
            .ok_or_else(|| anyhow!("Endpoint {} not found for service {}", address, service_id))?;
            
        // Get or create channel
        let channel = if let Some(channel) = &endpoint.channel {
            channel.clone()
        } else {
            self.create_channel_for_endpoint(endpoint).await?
        };
        
        // Call health check endpoint
        use crate::proto::agi_core::health_service_client::HealthServiceClient;
        use crate::proto::agi_core::HealthCheckRequest;
        
        let mut client = HealthServiceClient::new(channel);
        
        // Set timeout for health check
        match tokio::time::timeout(
            Duration::from_secs(5),
            client.check(HealthCheckRequest {})
        ).await {
            Ok(result) => {
                match result {
                    Ok(response) => {
                        // Parse response status
                        let status = match response.into_inner().status.as_str() {
                            "SERVING" => ServiceStatus::Healthy,
                            "NOT_SERVING" => ServiceStatus::Unhealthy,
                            "UNKNOWN" => ServiceStatus::Unknown,
                            "SERVICE_UNKNOWN" => ServiceStatus::Unknown,
                            _ => ServiceStatus::Unknown,
                        };
                        
                        // Update the status in the registry
                        self.update_service_status(service_id, address, status).await?;
                        
                        Ok(status)
                    }
                    Err(err) => {
                        // gRPC error
                        warn!("Health check failed for service {} at {}: {}", 
                               service_id, address, err);
                        
                        self.update_service_status(service_id, address, ServiceStatus::Unhealthy).await?;
                        Ok(ServiceStatus::Unhealthy)
                    }
                }
            }
            Err(_) => {
                // Timeout
                warn!("Health check timed out for service {} at {}", service_id, address);
                self.update_service_status(service_id, address, ServiceStatus::Offline).await?;
                Ok(ServiceStatus::Offline)
            }
        }
    }
    
    /// List all services and their endpoints
    pub async fn list_services(&self) -> HashMap<String, Vec<ServiceEndpointInfo>> {
        let services = self.services.read().await;
        let circuit_breakers = self.circuit_breakers.read().await;
        
        let mut result = HashMap::new();
        
        for (service_id, endpoints) in services.iter() {
            let mut endpoint_infos = Vec::new();
            
            for endpoint in endpoints {
                let cb_key = format!("{}/{}", service_id, endpoint.address);
                let circuit_state = circuit_breakers.get(&cb_key)
                    .map(|cb| match cb.get_state() {
                        CircuitState::Closed => "closed",
                        CircuitState::Open => "open",
                        CircuitState::HalfOpen => "half-open",
                    })
                    .unwrap_or("unknown");
                    
                let info = ServiceEndpointInfo {
                    address: endpoint.address.clone(),
                    status: endpoint.status.to_string(),
                    circuit_state: circuit_state.to_string(),
                    use_tls: endpoint.use_tls,
                    last_checked: endpoint.last_checked.elapsed().as_secs(),
                    metadata: endpoint.metadata.clone(),
                };
                
                endpoint_infos.push(info);
            }
            
            result.insert(service_id.clone(), endpoint_infos);
        }
        
        result
    }
}

/// A service discovered from the service registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub service_id: String,
    pub address: String,
    pub status: String,
    pub use_tls: bool,
    pub metadata: HashMap<String, String>,
}

/// Service endpoint information for admin API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpointInfo {
    pub address: String,
    pub status: String,
    pub circuit_state: String,
    pub use_tls: bool,
    pub last_checked: u64,
    pub metadata: HashMap<String, String>,
}

// Convenience functions for the global service mesh
pub async fn register_service(
    service_id: &str,
    address: &str,
    use_tls: bool,
    metadata: Option<HashMap<String, String>>,
) -> Result<()> {
    let mesh = ServiceMesh::get_global().await?;
    mesh.register_service(service_id, address, use_tls, metadata).await
}

pub async fn deregister_service(
    service_id: &str,
    address: &str,
) -> Result<()> {
    let mesh = ServiceMesh::get_global().await?;
    mesh.deregister_service(service_id, address).await
}

pub async fn get_service_channel(service_id: &str) -> Result<Arc<Channel>> {
    let mesh = ServiceMesh::get_global().await?;
    mesh.get_service_channel(service_id).await
}

pub async fn list_services() -> Result<HashMap<String, Vec<ServiceEndpointInfo>>> {
    let mesh = ServiceMesh::get_global().await?;
    Ok(mesh.list_services().await)
}

pub async fn health_check_endpoint(
    service_id: &str,
    address: &str,
) -> Result<ServiceStatus> {
    let mesh = ServiceMesh::get_global().await?;
    mesh.health_check_endpoint(service_id, address).await
}

// Service mesh client trait for dependency injection and testing
#[async_trait]
pub trait ServiceMeshClient: Send + Sync {
    async fn get_channel(&self, service_id: &str) -> Result<Arc<Channel>>;
    async fn register(&self, service_id: &str, address: &str, use_tls: bool) -> Result<()>;
    async fn deregister(&self, service_id: &str, address: &str) -> Result<()>;
    async fn health_check(&self, service_id: &str, address: &str) -> Result<ServiceStatus>;
}

// Default implementation using the global service mesh
pub struct DefaultServiceMeshClient;

#[async_trait]
impl ServiceMeshClient for DefaultServiceMeshClient {
    async fn get_channel(&self, service_id: &str) -> Result<Arc<Channel>> {
        get_service_channel(service_id).await
    }
    
    async fn register(&self, service_id: &str, address: &str, use_tls: bool) -> Result<()> {
        register_service(service_id, address, use_tls, None).await
    }
    
    async fn deregister(&self, service_id: &str, address: &str) -> Result<()> {
        deregister_service(service_id, address).await
    }
    
    async fn health_check(&self, service_id: &str, address: &str) -> Result<ServiceStatus> {
        health_check_endpoint(service_id, address).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::certificates::tests::create_test_cert_manager;
    
    async fn create_test_service_mesh() -> ServiceMesh {
        let cert_manager = Arc::new(create_test_cert_manager().await);
        
        ServiceMesh::new(
            "test-service",
            cert_manager,
            false, // No mTLS
            None,  // No discovery
            Some(60),
            Some(LoadBalancingPolicy::RoundRobin),
        ).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_service_registration() {
        let mesh = create_test_service_mesh().await;
        
        // Register a service
        mesh.register_service(
            "test-target",
            "http://localhost:8080",
            false,
            None,
        ).await.unwrap();
        
        // Check that it's registered
        let services = mesh.list_services().await;
        assert!(services.contains_key("test-target"));
        assert_eq!(services["test-target"].len(), 1);
        assert_eq!(services["test-target"][0].address, "http://localhost:8080");
        
        // Deregister
        mesh.deregister_service("test-target", "http://localhost:8080").await.unwrap();
        
        // Check it's gone
        let services = mesh.list_services().await;
        assert!(services.get("test-target").unwrap().is_empty());
    }
    
    #[tokio::test]
    async fn test_circuit_breaker() {
        let mut cb = CircuitBreaker::new(3, Duration::from_millis(100));
        
        // Initially closed
        assert_eq!(cb.get_state(), CircuitState::Closed);
        assert!(cb.allow_request());
        
        // Record failures
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);
        
        // Request should be denied in open state
        assert!(!cb.allow_request());
        
        // Wait for reset timeout
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Should move to half-open on next request check
        assert!(cb.allow_request());
        assert_eq!(cb.get_state(), CircuitState::HalfOpen);
        
        // Only one request allowed in half-open
        assert!(!cb.allow_request());
        
        // Success should close the circuit
        cb.record_success();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        
        // Failure in half-open should reopen
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);
        
        // Reset should clear everything
        cb.reset();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
    }
}