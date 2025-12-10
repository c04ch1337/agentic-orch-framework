use std::time::Duration;
use tokio::time::sleep;
use anyhow::Result;
use phoenix_orch::{
    PhoenixOrch,
    resilience::{
        CriticalEvent,
        Severity,
        FailureCategory,
    },
};
use chrono::Utc;
use uuid::Uuid;
use windows_service::{
    service::{ServiceAccess, ServiceState},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

// Helper function to check service status
async fn check_service_status(service_name: &str) -> Result<ServiceState> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(
        service_name,
        ServiceAccess::QUERY_STATUS,
    )?;
    let status = service.query_status()?;
    Ok(status.current_state)
}

#[tokio::test]
async fn test_service_lifecycle() -> Result<()> {
    // Initialize Phoenix ORCH
    let orch = PhoenixOrch::new().await?;
    
    // Start services
    orch.start().await?;
    
    // Verify all services are healthy
    let health = orch.get_service_health().await;
    assert!(health.values().all(|&healthy| healthy), "Not all services are healthy");
    
    // Stop services
    orch.stop().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_process_watchdog() -> Result<()> {
    let orch = PhoenixOrch::new().await?;
    orch.start().await?;
    
    // Create a CPU-intensive operation
    let code = r#"
    while True:
        pass
    "#;
    
    // Execute code that should trigger watchdog
    let result = orch.executor.execute_python(
        code,
        &std::collections::HashMap::new(),
    ).await;
    
    // Verify process was terminated
    assert!(result.is_err(), "Process should have been terminated by watchdog");
    
    // Check that executor service was marked unhealthy
    let health = orch.get_service_health().await;
    assert!(!health.get("executor").unwrap(), "Executor should be marked unhealthy");
    
    orch.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_data_integrity_rollback() -> Result<()> {
    let orch = PhoenixOrch::new().await?;
    orch.start().await?;
    
    // Simulate a critical failure
    let event = CriticalEvent {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        severity: Severity::Critical,
        category: FailureCategory::DataCorruption,
        description: "Data corruption detected".to_string(),
        affected_services: vec!["body-kb".to_string()],
    };
    
    // Handle critical failure
    orch.handle_critical_failure(event).await?;
    
    // Verify service was restored
    sleep(Duration::from_secs(1)).await;
    let health = orch.get_service_health().await;
    assert!(health.get("body-kb").unwrap(), "Service should be restored after rollback");
    
    orch.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_secret_rotation() -> Result<()> {
    let orch = PhoenixOrch::new().await?;
    orch.start().await?;
    
    // Simulate a security breach
    let event = CriticalEvent {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        severity: Severity::Emergency,
        category: FailureCategory::SecurityBreach,
        description: "Security breach detected".to_string(),
        affected_services: vec!["auth-service".to_string()],
    };
    
    // Handle critical failure
    orch.handle_critical_failure(event).await?;
    
    // Verify service is healthy after secret rotation
    sleep(Duration::from_secs(1)).await;
    let health = orch.get_service_health().await;
    assert!(health.get("auth-service").unwrap(), "Service should be healthy after secret rotation");
    
    orch.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_windows_service_integration() -> Result<()> {
    // Check initial service state
    let state = check_service_status("PhoenixOrchService").await?;
    assert_eq!(state, ServiceState::Running, "Service should be running");
    
    // Simulate service stop
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(
        "PhoenixOrchService",
        ServiceAccess::STOP,
    )?;
    service.stop()?;
    
    // Wait for service to stop
    sleep(Duration::from_secs(2)).await;
    let state = check_service_status("PhoenixOrchService").await?;
    assert_eq!(state, ServiceState::Stopped, "Service should be stopped");
    
    Ok(())
}

#[tokio::test]
async fn test_emergency_resilience_integration() -> Result<()> {
    let orch = PhoenixOrch::new().await?;
    orch.start().await?;
    
    // Test process watchdog
    let cpu_intensive_code = r#"
    while True:
        pass
    "#;
    let result = orch.executor.execute_python(
        cpu_intensive_code,
        &std::collections::HashMap::new(),
    ).await;
    assert!(result.is_err(), "Process should be terminated");
    
    // Test data integrity
    let corruption_event = CriticalEvent {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        severity: Severity::Critical,
        category: FailureCategory::DataCorruption,
        description: "Data corruption".to_string(),
        affected_services: vec!["body-kb".to_string()],
    };
    orch.handle_critical_failure(corruption_event).await?;
    
    // Test secret rotation
    let security_event = CriticalEvent {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        severity: Severity::Emergency,
        category: FailureCategory::SecurityBreach,
        description: "Security breach".to_string(),
        affected_services: vec!["auth-service".to_string()],
    };
    orch.handle_critical_failure(security_event).await?;
    
    // Verify all services recovered
    sleep(Duration::from_secs(2)).await;
    let health = orch.get_service_health().await;
    assert!(health.values().all(|&healthy| healthy), "All services should recover");
    
    orch.stop().await?;
    Ok(())
}