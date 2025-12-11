//! Resilience Patterns Demo
//!
//! This example demonstrates how to use the resilience patterns
//! (retry, circuit breaker) in the tool-sdk.
//!
//! It creates a mock service that fails intermittently to show
//! how retry and circuit breaker patterns handle failures.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tool_sdk::{
    error::{Result, ServiceError},
    resilience::{CircuitBreakerConfig, Resilience, RetryConfig},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    println!("Resilience Patterns Demo");
    println!("========================\n");

    // Step 1: Demonstrate retry with a temporary failure
    println!("RETRY PATTERN DEMONSTRATION");
    println!("---------------------------");

    // Configure retry with 3 attempts
    let retry_config = RetryConfig {
        max_retries: 3,
        initial_interval: Duration::from_millis(100),
        max_interval: Duration::from_secs(1),
        multiplier: 2.0,
        randomization_factor: 0.1,
        max_elapsed_time: Some(Duration::from_secs(5)),
    };

    // Create an operation that fails twice then succeeds
    let attempt_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&attempt_counter);

    println!("Starting flaky operation with retry...");
    println!("This will fail twice and succeed on the third attempt.\n");

    let resilience = Resilience::new(retry_config, CircuitBreakerConfig::default());

    // Execute with retry
    let result = resilience
        .execute(move || {
            let counter = Arc::clone(&counter_clone);
            async move {
                let attempt = counter.fetch_add(1, Ordering::SeqCst) + 1;
                println!("Attempt {}: Executing flaky operation...", attempt);

                tokio::time::sleep(Duration::from_millis(50)).await;

                if attempt <= 2 {
                    println!("Attempt {}: Operation failed (temporary error)", attempt);
                    Err(ServiceError::network(format!(
                        "Temporary failure in attempt {}",
                        attempt
                    )))
                } else {
                    println!("Attempt {}: Operation succeeded!", attempt);
                    Ok("Success after retries")
                }
            }
        })
        .await;

    match result {
        Ok(message) => println!("\nRetry result: {}\n", message),
        Err(e) => println!("\nRetry failed: {}\n", e),
    }

    // Step 2: Demonstrate circuit breaker
    println!("\nCIRCUIT BREAKER PATTERN DEMONSTRATION");
    println!("------------------------------------");

    // Configure circuit breaker
    let cb_config = CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: Duration::from_secs(2),
        success_threshold: 1,
        sliding_window_size: 10,
        error_threshold_percentage: 0.5,
    };

    let retry_config = RetryConfig {
        max_retries: 1,
        ..RetryConfig::default()
    };

    let resilience = Resilience::new(retry_config, cb_config);

    println!("Starting operations with circuit breaker...");
    println!("Circuit will open after 3 failures and reset after 2 seconds.\n");

    // Execute 5 operations that will all fail
    for i in 1..=5 {
        let result = resilience
            .execute(|| async {
                println!("Operation {}: Executing failing operation...", i);
                tokio::time::sleep(Duration::from_millis(50)).await;
                Err(ServiceError::service("Service is down"))
            })
            .await;

        match result {
            Ok(_) => println!("Operation {}: Succeeded (unexpected)", i),
            Err(e) => println!("Operation {}: Failed: {}", i, e),
        }

        // Small delay between operations
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Print the circuit breaker status
        println!("Circuit status: {:?}", resilience.circuit_breaker_status());
        println!();
    }

    // Wait for circuit breaker to reset
    println!("Waiting for circuit breaker reset timeout (2 seconds)...");
    tokio::time::sleep(Duration::from_secs(3)).await;
    println!(
        "Circuit status after wait: {:?}",
        resilience.circuit_breaker_status()
    );

    // Try again after reset
    let result = resilience
        .execute(|| async {
            println!("Final operation: Executing successful operation...");
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok("Success after circuit reset")
        })
        .await;

    match result {
        Ok(message) => println!("Final result: {}", message),
        Err(e) => println!("Final operation failed: {}", e),
    }

    println!("\nResilience patterns demonstration completed.");

    Ok(())
}
