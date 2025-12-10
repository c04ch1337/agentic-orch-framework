use std::time::Duration;
use tokio::time::sleep;
use anyhow::Result;
use executor::{
    Executor,
    JobObjectManager,
    ResourceMonitor,
};
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_cpu_limit_recovery() -> Result<()> {
    let executor = Executor::new();
    
    // Create CPU-intensive code
    let cpu_code = r#"
    while True:
        pass
    "#;
    
    // First execution should be terminated by watchdog
    let result = executor.execute_python(cpu_code, &HashMap::new()).await;
    assert!(result.is_err(), "Process should be terminated due to CPU limit");
    
    // Wait for recovery period
    sleep(Duration::from_secs(5)).await;
    
    // Second execution should work with normal code
    let normal_code = r#"
    print("Hello, World!")
    "#;
    let result = executor.execute_python(normal_code, &HashMap::new()).await;
    assert!(result.is_ok(), "Service should recover after CPU limit breach");
    
    Ok(())
}

#[tokio::test]
async fn test_memory_limit_recovery() -> Result<()> {
    let executor = Executor::new();
    
    // Create memory-intensive code
    let memory_code = r#"
    x = ' ' * (1024 * 1024 * 1024)  # Allocate 1GB
    "#;
    
    // First execution should be terminated by watchdog
    let result = executor.execute_python(memory_code, &HashMap::new()).await;
    assert!(result.is_err(), "Process should be terminated due to memory limit");
    
    // Wait for recovery period
    sleep(Duration::from_secs(5)).await;
    
    // Second execution should work with normal code
    let normal_code = r#"
    x = 'small string'
    print(x)
    "#;
    let result = executor.execute_python(normal_code, &HashMap::new()).await;
    assert!(result.is_ok(), "Service should recover after memory limit breach");
    
    Ok(())
}

#[tokio::test]
async fn test_process_limit_recovery() -> Result<()> {
    let executor = Executor::new();
    let job_manager = JobObjectManager::new()?;
    
    // Try to spawn more than allowed processes
    let mut handles = Vec::new();
    for _ in 0..10 {
        let handle = tokio::spawn(async move {
            let code = r#"
            import time
            time.sleep(1)
            "#;
            let mut env = HashMap::new();
            executor.execute_python(code, &env).await
        });
        handles.push(handle);
    }
    
    // Wait for all processes to complete
    for handle in handles {
        let result = handle.await?;
        // Some should fail due to process limit
        if result.is_err() {
            println!("Process failed as expected: {:?}", result.err());
        }
    }
    
    // Wait for recovery period
    sleep(Duration::from_secs(5)).await;
    
    // Should be able to spawn new process now
    let result = executor.execute_python(
        "print('recovered')",
        &HashMap::new()
    ).await;
    assert!(result.is_ok(), "Service should recover after process limit breach");
    
    Ok(())
}

#[tokio::test]
async fn test_execution_timeout_recovery() -> Result<()> {
    let executor = Executor::new();
    
    // Create long-running code
    let long_code = r#"
    import time
    time.sleep(20)  # Sleep for 20 seconds
    "#;
    
    // First execution should timeout
    let result = executor.execute_python(long_code, &HashMap::new()).await;
    assert!(result.is_err(), "Process should be terminated due to timeout");
    
    // Wait for recovery period
    sleep(Duration::from_secs(5)).await;
    
    // Second execution should work with quick code
    let quick_code = r#"
    print("Quick execution")
    "#;
    let result = executor.execute_python(quick_code, &HashMap::new()).await;
    assert!(result.is_ok(), "Service should recover after timeout");
    
    Ok(())
}

#[tokio::test]
async fn test_resource_monitor_recovery() -> Result<()> {
    let monitor = ResourceMonitor::new();
    monitor.start();
    
    // Simulate high CPU usage
    let executor = Executor::new();
    let cpu_code = r#"
    while True:
        pass
    "#;
    
    let result = executor.execute_python(cpu_code, &HashMap::new()).await;
    assert!(result.is_err(), "Process should be terminated");
    
    // Check that resource monitor detected the spike
    let metrics = monitor.get_metrics().await;
    assert!(metrics.cpu_usage > 45.0, "Should detect high CPU usage");
    
    // Wait for recovery
    sleep(Duration::from_secs(5)).await;
    
    // Metrics should return to normal
    let metrics = monitor.get_metrics().await;
    assert!(metrics.cpu_usage < 45.0, "CPU usage should recover");
    
    monitor.stop();
    Ok(())
}

#[tokio::test]
async fn test_concurrent_resource_limits() -> Result<()> {
    let executor = Executor::new();
    let monitor = ResourceMonitor::new();
    monitor.start();
    
    // Spawn multiple resource-intensive processes
    let mut handles = Vec::new();
    
    // CPU-intensive process
    handles.push(tokio::spawn(async move {
        executor.execute_python(
            "while True: pass",
            &HashMap::new()
        ).await
    }));
    
    // Memory-intensive process
    handles.push(tokio::spawn(async move {
        executor.execute_python(
            "x = ' ' * (1024 * 1024 * 512)",  // 512MB
            &HashMap::new()
        ).await
    }));
    
    // Long-running process
    handles.push(tokio::spawn(async move {
        executor.execute_python(
            "import time; time.sleep(15)",
            &HashMap::new()
        ).await
    }));
    
    // Wait for all processes to complete or be terminated
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_err(), "Process should be terminated");
    }
    
    // Wait for recovery period
    sleep(Duration::from_secs(5)).await;
    
    // Should be able to run normal process
    let result = executor.execute_python(
        "print('recovered')",
        &HashMap::new()
    ).await;
    assert!(result.is_ok(), "Service should recover after multiple breaches");
    
    monitor.stop();
    Ok(())
}

#[tokio::test]
async fn test_graceful_shutdown_recovery() -> Result<()> {
    let executor = Executor::new();
    let job_manager = JobObjectManager::new()?;
    
    // Start some processes
    let mut handles = Vec::new();
    for i in 0..3 {
        let code = format!(
            r#"
            import time
            print("Process {}")
            time.sleep(5)
            "#,
            i
        );
        handles.push(tokio::spawn(async move {
            executor.execute_python(&code, &HashMap::new()).await
        }));
    }
    
    // Initiate graceful shutdown
    job_manager.initiate_shutdown().await?;
    
    // Wait for processes to complete
    for handle in handles {
        let result = handle.await?;
        // Processes should complete or be terminated gracefully
        println!("Process result: {:?}", result);
    }
    
    // Wait for recovery period
    sleep(Duration::from_secs(5)).await;
    
    // Should be able to start new process
    let result = executor.execute_python(
        "print('recovered after shutdown')",
        &HashMap::new()
    ).await;
    assert!(result.is_ok(), "Service should recover after graceful shutdown");
    
    Ok(())
}