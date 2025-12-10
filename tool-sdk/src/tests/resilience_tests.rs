//! Tests for resilience patterns
//!
//! These tests verify that the retry and circuit breaker
//! resilience patterns work correctly.

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    
    use crate::error::{ServiceError, Result};
    use crate::resilience::{
        Resilience, RetryExecutor, RetryConfig, 
        CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStatus
    };
    
    #[tokio::test]
    async fn test_retry_success_first_attempt() {
        let config = RetryConfig {
            max_retries: 3,
            initial_interval: Duration::from_millis(10),
            max_interval: Duration::from_millis(100),
            ..RetryConfig::default()
        };
        
        let retry = RetryExecutor::new(config);
        
        // Operation succeeds on first attempt
        let result = retry.execute(|| async { Ok::<_, ServiceError>(42) }).await;
        
        assert_eq!(result.unwrap(), 42);
    }
    
    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let config = RetryConfig {
            max_retries: 3,
            initial_interval: Duration::from_millis(10),
            max_interval: Duration::from_millis(100),
            ..RetryConfig::default()
        };
        
        let retry = RetryExecutor::new(config);
        
        // Track attempts
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = Arc::clone(&attempts);
        
        // Operation fails twice, then succeeds
        let result = retry.execute(move || {
            let attempts = Arc::clone(&attempts_clone);
            async move {
                let current = attempts.fetch_add(1, Ordering::SeqCst);
                
                if current < 2 {
                    Err(ServiceError::network("Temporary failure"))
                } else {
                    Ok(42)
                }
            }
        }).await;
        
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3); // 3 attempts total
    }
    
    #[tokio::test]
    async fn test_retry_max_retries_exceeded() {
        let config = RetryConfig {
            max_retries: 2,
            initial_interval: Duration::from_millis(10),
            max_interval: Duration::from_millis(100),
            ..RetryConfig::default()
        };
        
        let retry = RetryExecutor::new(config);
        
        // Track attempts
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = Arc::clone(&attempts);
        
        // Operation always fails
        let result = retry.execute(move || {
            let attempts = Arc::clone(&attempts_clone);
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(ServiceError::network("Persistent failure"))
            }
        }).await;
        
        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 3); // Initial + 2 retries
    }
    
    #[tokio::test]
    async fn test_retry_non_retryable_error() {
        let config = RetryConfig::default();
        let retry = RetryExecutor::new(config);
        
        // Track attempts
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = Arc::clone(&attempts);
        
        // Operation fails with a non-retryable error
        let result = retry.execute(move || {
            let attempts = Arc::clone(&attempts_clone);
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(ServiceError::validation("Invalid input")) // Not retryable
            }
        }).await;
        
        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 1); // Only 1 attempt, no retries
    }
    
    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        
        assert_eq!(cb.status(), CircuitBreakerStatus::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.success_count(), 0);
        assert!(cb.check().is_ok());
    }
    
    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..CircuitBreakerConfig::default()
        };
        
        let cb = CircuitBreaker::new(config);
        
        // Record failures
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.status(), CircuitBreakerStatus::Closed);
        assert_eq!(cb.failure_count(), 2);
        
        // Third failure should open the circuit
        cb.record_failure();
        assert_eq!(cb.status(), CircuitBreakerStatus::Open);
        
        // Check should fail when circuit is open
        assert!(cb.check().is_err());
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(50),
            ..CircuitBreakerConfig::default()
        };
        
        let cb = CircuitBreaker::new(config);
        
        // Open the circuit
        cb.record_failure();
        assert_eq!(cb.status(), CircuitBreakerStatus::Open);
        
        // Wait for reset timeout
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Next check should transition to half-open
        assert!(cb.check().is_ok());
        assert_eq!(cb.status(), CircuitBreakerStatus::HalfOpen);
    }
    
    #[test]
    fn test_circuit_breaker_closes_after_successes() {
        let config = CircuitBreakerConfig {
            success_threshold: 2,
            ..CircuitBreakerConfig::default()
        };
        
        let cb = CircuitBreaker::new(config);
        
        // Manually transition to half-open
        let _ = cb.record_failure();
        cb.transition_to_half_open(); // This method is private in actual code, here for testing
        assert_eq!(cb.status(), CircuitBreakerStatus::HalfOpen);
        
        // First success
        cb.record_success();
        assert_eq!(cb.status(), CircuitBreakerStatus::HalfOpen);
        assert_eq!(cb.success_count(), 1);
        
        // Second success should close the circuit
        cb.record_success();
        assert_eq!(cb.status(), CircuitBreakerStatus::Closed);
        assert_eq!(cb.failure_count(), 0);
    }
    
    #[test]
    fn test_circuit_breaker_reopens_on_failure_in_half_open() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        
        // Manually transition to half-open
        let _ = cb.record_failure();
        cb.transition_to_half_open(); // This method is private in actual code, here for testing
        assert_eq!(cb.status(), CircuitBreakerStatus::HalfOpen);
        
        // Failure in half-open state should reopen circuit
        cb.record_failure();
        assert_eq!(cb.status(), CircuitBreakerStatus::Open);
    }
    
    #[test]
    fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        
        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.status(), CircuitBreakerStatus::Open);
        
        // Reset should close the circuit
        cb.reset();
        
        assert_eq!(cb.status(), CircuitBreakerStatus::Closed);
        assert_eq!(cb.failure_count(), 0);
    }
    
    #[tokio::test]
    async fn test_resilience_facade() {
        // Set up resilience with both retry and circuit breaker
        let retry_config = RetryConfig {
            max_retries: 2,
            initial_interval: Duration::from_millis(10),
            ..RetryConfig::default()
        };
        
        let cb_config = CircuitBreakerConfig {
            failure_threshold: 4,
            reset_timeout: Duration::from_millis(50),
            ..CircuitBreakerConfig::default()
        };
        
        let resilience = Resilience::new(retry_config, cb_config);
        
        // Track attempts
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = Arc::clone(&attempts);
        
        // First operation succeeds after retries
        let result = resilience.execute(move || {
            let attempts = Arc::clone(&attempts_clone);
            async move {
                let current = attempts.fetch_add(1, Ordering::SeqCst);
                
                if current < 2 {
                    Err(ServiceError::network("Temporary failure"))
                } else {
                    Ok("Success")
                }
            }
        }).await;
        
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(attempts.load(Ordering::SeqCst), 3); // Initial + 2 retries
        
        // Reset for next test
        let attempts = Arc::new(AtomicUsize::new(0));
        
        // Make operations that always fail to open the circuit breaker
        for _ in 0..5 {
            let attempts_clone = Arc::clone(&attempts);
            let _ = resilience.execute(move || {
                let attempts = Arc::clone(&attempts_clone);
                async move {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    Err(ServiceError::service("Service failure")) // Service errors are retryable
                }
            }).await;
        }
        
        // Circuit should be open now
        assert_eq!(resilience.circuit_breaker_status(), CircuitBreakerStatus::Open);
        
        // Next operation should fail immediately due to open circuit
        let before_count = attempts.load(Ordering::SeqCst);
        let result = resilience.execute(move || {
            async move {
                // This should never be called when circuit is open
                panic!("Should not be called");
            }
        }).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circuit breaker is open"));
        
        // No new attempt should have been made
        assert_eq!(attempts.load(Ordering::SeqCst), before_count);
        
        // Wait for circuit breaker timeout
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Reset the circuit breaker for next test
        resilience.reset_circuit_breaker();
        assert_eq!(resilience.circuit_breaker_status(), CircuitBreakerStatus::Closed);
    }
}