//! Extended tests for resilience patterns
//!
//! These tests verify advanced resilience scenarios including
//! retry backoff, circuit breaker transitions, and resilience integration.

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    
    use crate::error::{ServiceError, Result};
    use crate::resilience::{
        Resilience, RetryExecutor, RetryConfig, 
        CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStatus
    };
    
    /// Helper function to measure execution time
    async fn measure_execution<F, Fut, T>(f: F) -> (Result<T>, Duration)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let start = Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        (result, duration)
    }
    
    /// Tests to verify that the retry backoff behavior works correctly
    mod retry_backoff_tests {
        use super::*;
        
        #[tokio::test]
        async fn test_exponential_backoff() {
            let config = RetryConfig {
                max_retries: 3,
                initial_interval: Duration::from_millis(50),
                max_interval: Duration::from_millis(1000),
                backoff_factor: 2.0,
                jitter: false, // Disable jitter for deterministic testing
                ..RetryConfig::default()
            };
            
            let retry = RetryExecutor::new(config.clone());
            
            // Track retry timestamps
            let retry_times = Arc::new(Mutex::new(Vec::new()));
            let retry_times_clone = Arc::clone(&retry_times);
            
            // Create operation that always fails but records timestamps
            let (result, total_duration) = measure_execution(|| {
                let retry_times = Arc::clone(&retry_times_clone);
                retry.execute(move || {
                    let retry_times = Arc::clone(&retry_times);
                    async move {
                        // Record the timestamp of this attempt
                        let mut times = retry_times.lock().unwrap();
                        times.push(Instant::now());
                        
                        // Always fail
                        Err(ServiceError::network("Simulated network error"))
                    }
                })
            }).await;
            
            // Verify operation failed
            assert!(result.is_err());
            
            // Verify correct number of attempts
            let times = retry_times.lock().unwrap();
            assert_eq!(times.len(), 4); // Initial attempt + 3 retries
            
            // Calculate delays between attempts
            let mut delays = Vec::new();
            for i in 1..times.len() {
                let delay = times[i].duration_since(times[i-1]);
                delays.push(delay);
            }
            
            // Verify exponential backoff pattern
            // First delay should be approximately initial_interval
            assert!(
                delays[0] >= Duration::from_millis(40) && 
                delays[0] <= Duration::from_millis(60)
            );
            
            // Second delay should be approximately initial_interval * backoff_factor
            assert!(
                delays[1] >= Duration::from_millis(90) && 
                delays[1] <= Duration::from_millis(110)
            );
            
            // Third delay should be approximately initial_interval * backoff_factor^2
            assert!(
                delays[2] >= Duration::from_millis(190) && 
                delays[2] <= Duration::from_millis(210)
            );
            
            // Total duration should be at least the sum of expected delays
            let expected_min_duration = Duration::from_millis(50 + 100 + 200);
            assert!(total_duration >= expected_min_duration);
        }
        
        #[tokio::test]
        async fn test_max_retry_interval() {
            // Configure retry with a low max interval
            let config = RetryConfig {
                max_retries: 3,
                initial_interval: Duration::from_millis(100),
                max_interval: Duration::from_millis(150), // Cap at 150ms
                backoff_factor: 2.0,
                jitter: false,
                ..RetryConfig::default()
            };
            
            let retry = RetryExecutor::new(config);
            
            // Track retry timestamps
            let retry_times = Arc::new(Mutex::new(Vec::new()));
            let retry_times_clone = Arc::clone(&retry_times);
            
            // Measure total time
            let (_, _) = measure_execution(|| {
                let retry_times = Arc::clone(&retry_times_clone);
                retry.execute(move || {
                    let retry_times = Arc::clone(&retry_times);
                    async move {
                        let mut times = retry_times.lock().unwrap();
                        times.push(Instant::now());
                        
                        // Always fail
                        Err(ServiceError::network("Simulated network error"))
                    }
                })
            }).await;
            
            // Calculate delays between attempts
            let times = retry_times.lock().unwrap();
            let mut delays = Vec::new();
            for i in 1..times.len() {
                let delay = times[i].duration_since(times[i-1]);
                delays.push(delay);
            }
            
            // First delay should be approximately initial_interval
            assert!(
                delays[0] >= Duration::from_millis(90) && 
                delays[0] <= Duration::from_millis(110)
            );
            
            // Second delay would be 200ms without cap, but should be capped at ~150ms
            assert!(
                delays[1] >= Duration::from_millis(140) && 
                delays[1] <= Duration::from_millis(160)
            );
            
            // Third delay would be 400ms without cap, but should still be capped at ~150ms
            assert!(
                delays[2] >= Duration::from_millis(140) && 
                delays[2] <= Duration::from_millis(160)
            );
        }
        
        #[tokio::test]
        async fn test_retry_with_jitter() {
            // Configure retry with jitter
            let config = RetryConfig {
                max_retries: 5,
                initial_interval: Duration::from_millis(100),
                max_interval: Duration::from_millis(1000),
                backoff_factor: 2.0,
                jitter: true, // Enable jitter
                jitter_factor: 0.25, // 25% jitter
                ..RetryConfig::default()
            };
            
            let retry = RetryExecutor::new(config);
            
            // Track retry timestamps for multiple runs
            let all_delays = Arc::new(Mutex::new(Vec::new()));
            
            // Run multiple tests to verify jitter is being applied
            for _ in 0..3 {
                let retry_times = Arc::new(Mutex::new(Vec::new()));
                let retry_times_clone = Arc::clone(&retry_times);
                
                let _ = retry.execute(move || {
                    let retry_times = Arc::clone(&retry_times_clone);
                    async move {
                        let mut times = retry_times.lock().unwrap();
                        times.push(Instant::now());
                        
                        // Always fail
                        Err(ServiceError::network("Simulated network error"))
                    }
                }).await;
                
                // Calculate delays and add to all_delays
                let times = retry_times.lock().unwrap();
                let mut this_run_delays = Vec::new();
                for i in 1..times.len() {
                    let delay = times[i].duration_since(times[i-1]);
                    this_run_delays.push(delay);
                }
                
                all_delays.lock().unwrap().push(this_run_delays);
            }
            
            // Check that the delays are different across runs (due to jitter)
            let delays = all_delays.lock().unwrap();
            let first_run = &delays[0];
            let second_run = &delays[1];
            let third_run = &delays[2];
            
            // At least some of the delays should be different due to jitter
            let mut found_difference = false;
            for i in 0..first_run.len().min(second_run.len()).min(third_run.len()) {
                if first_run[i] != second_run[i] || first_run[i] != third_run[i] {
                    found_difference = true;
                    break;
                }
            }
            
            assert!(found_difference, "Jitter doesn't appear to be working, all retry delays were identical");
        }
    }
    
    /// Tests to verify circuit breaker state transitions
    mod circuit_breaker_transitions_tests {
        use super::*;
        
        // Helper enum to keep track of state transitions
        #[derive(Debug, Clone, PartialEq)]
        enum StateTransition {
            Initial(CircuitBreakerStatus),
            ToOpen(usize), // failure count when it opened
            ToHalfOpen,
            ToClosed,
            WasRejected,
        }
        
        #[tokio::test]
        async fn test_detailed_circuit_breaker_transitions() {
            // Create circuit breaker with low threshold for testing
            let config = CircuitBreakerConfig {
                failure_threshold: 2,
                success_threshold: 1,
                reset_timeout: Duration::from_millis(100),
                ..CircuitBreakerConfig::default()
            };
            
            let cb = Arc::new(Mutex::new(CircuitBreaker::new(config)));
            
            // Track state transitions
            let transitions = Arc::new(Mutex::new(Vec::new()));
            
            // Record initial state
            {
                let cb_guard = cb.lock().unwrap();
                let mut t = transitions.lock().unwrap();
                t.push(StateTransition::Initial(cb_guard.status()));
            }
            
            // Step 1: Circuit starts closed
            assert_eq!(cb.lock().unwrap().status(), CircuitBreakerStatus::Closed);
            
            // Step 2: Record failures until circuit opens
            {
                let mut cb_guard = cb.lock().unwrap();
                cb_guard.record_failure();
                assert_eq!(cb_guard.status(), CircuitBreakerStatus::Closed); // Still closed
                
                cb_guard.record_failure();
                let cur_status = cb_guard.status();
                let failure_count = cb_guard.failure_count();
                
                if cur_status == CircuitBreakerStatus::Open {
                    let mut t = transitions.lock().unwrap();
                    t.push(StateTransition::ToOpen(failure_count));
                }
            }
            
            // Verify circuit is now open
            assert_eq!(cb.lock().unwrap().status(), CircuitBreakerStatus::Open);
            
            // Step 3: Verify requests are rejected when open
            {
                let cb_guard = cb.lock().unwrap();
                let check_result = cb_guard.check();
                assert!(check_result.is_err());
                
                if check_result.is_err() {
                    let mut t = transitions.lock().unwrap();
                    t.push(StateTransition::WasRejected);
                }
            }
            
            // Step 4: Wait for reset timeout to transition to half-open
            tokio::time::sleep(Duration::from_millis(150)).await;
            
            // Step 5: Next check should transition to half-open
            {
                let cb_guard = cb.lock().unwrap();
                let check_result = cb_guard.check();
                assert!(check_result.is_ok());
                let cur_status = cb_guard.status();
                
                if cur_status == CircuitBreakerStatus::HalfOpen {
                    let mut t = transitions.lock().unwrap();
                    t.push(StateTransition::ToHalfOpen);
                }
            }
            
            // Verify circuit is now half-open
            assert_eq!(cb.lock().unwrap().status(), CircuitBreakerStatus::HalfOpen);
            
            // Step 6: Record a success to transition back to closed
            {
                let mut cb_guard = cb.lock().unwrap();
                cb_guard.record_success();
                let cur_status = cb_guard.status();
                
                if cur_status == CircuitBreakerStatus::Closed {
                    let mut t = transitions.lock().unwrap();
                    t.push(StateTransition::ToClosed);
                }
            }
            
            // Verify circuit is now closed again
            assert_eq!(cb.lock().unwrap().status(), CircuitBreakerStatus::Closed);
            
            // Inspect the transitions to ensure they happened in the right order
            let transition_history = transitions.lock().unwrap();
            
            assert!(transition_history.contains(&StateTransition::Initial(CircuitBreakerStatus::Closed)));
            assert!(transition_history.iter().any(|t| matches!(t, StateTransition::ToOpen(_))));
            assert!(transition_history.contains(&StateTransition::WasRejected));
            assert!(transition_history.contains(&StateTransition::ToHalfOpen));
            assert!(transition_history.contains(&StateTransition::ToClosed));
            
            // Verify the correct sequence
            let to_open_idx = transition_history.iter()
                .position(|t| matches!(t, StateTransition::ToOpen(_)))
                .unwrap();
            
            let was_rejected_idx = transition_history.iter()
                .position(|t| matches!(t, StateTransition::WasRejected))
                .unwrap();
            
            let to_half_open_idx = transition_history.iter()
                .position(|t| matches!(t, StateTransition::ToHalfOpen))
                .unwrap();
            
            let to_closed_idx = transition_history.iter()
                .position(|t| matches!(t, StateTransition::ToClosed))
                .unwrap();
            
            // Verify the order of transitions
            assert!(to_open_idx < was_rejected_idx);
            assert!(was_rejected_idx < to_half_open_idx);
            assert!(to_half_open_idx < to_closed_idx);
        }
        
        #[tokio::test]
        async fn test_circuit_breaker_reopen_on_failure_in_half_open() {
            let config = CircuitBreakerConfig {
                failure_threshold: 1,
                success_threshold: 2,
                reset_timeout: Duration::from_millis(50),
                ..CircuitBreakerConfig::default()
            };
            
            let cb = Arc::new(Mutex::new(CircuitBreaker::new(config)));
            
            // Open the circuit
            {
                let mut cb_guard = cb.lock().unwrap();
                cb_guard.record_failure();
                assert_eq!(cb_guard.status(), CircuitBreakerStatus::Open);
            }
            
            // Wait for reset timeout
            tokio::time::sleep(Duration::from_millis(60)).await;
            
            // Transition to half-open
            {
                let cb_guard = cb.lock().unwrap();
                let _ = cb_guard.check(); // This should transition to half-open
            }
            
            assert_eq!(cb.lock().unwrap().status(), CircuitBreakerStatus::HalfOpen);
            
            // Record failure in half-open state
            {
                let mut cb_guard = cb.lock().unwrap();
                cb_guard.record_failure();
                // Verify it went back to open
                assert_eq!(cb_guard.status(), CircuitBreakerStatus::Open);
            }
            
            // Verify circuit is now open again
            assert_eq!(cb.lock().unwrap().status(), CircuitBreakerStatus::Open);
        }
        
        #[tokio::test]
        async fn test_circuit_breaker_multiple_successes_required() {
            let config = CircuitBreakerConfig {
                failure_threshold: 1,
                success_threshold: 3, // Require 3 successes to close
                reset_timeout: Duration::from_millis(50),
                ..CircuitBreakerConfig::default()
            };
            
            let cb = Arc::new(Mutex::new(CircuitBreaker::new(config)));
            
            // Open the circuit
            {
                let mut cb_guard = cb.lock().unwrap();
                cb_guard.record_failure();
                assert_eq!(cb_guard.status(), CircuitBreakerStatus::Open);
            }
            
            // Wait for reset timeout
            tokio::time::sleep(Duration::from_millis(60)).await;
            
            // Transition to half-open
            {
                let cb_guard = cb.lock().unwrap();
                let _ = cb_guard.check();
            }
            
            assert_eq!(cb.lock().unwrap().status(), CircuitBreakerStatus::HalfOpen);
            
            // Record two successes - should still be half-open
            {
                let mut cb_guard = cb.lock().unwrap();
                cb_guard.record_success();
                cb_guard.record_success();
                assert_eq!(cb_guard.status(), CircuitBreakerStatus::HalfOpen);
                assert_eq!(cb_guard.success_count(), 2);
            }
            
            // Record third success - should close the circuit
            {
                let mut cb_guard = cb.lock().unwrap();
                cb_guard.record_success();
                assert_eq!(cb_guard.status(), CircuitBreakerStatus::Closed);
            }
            
            // Check success count was reset
            assert_eq!(cb.lock().unwrap().success_count(), 0);
        }
    }
    
    /// Tests to verify integration of resilience patterns
    mod resilience_integration_tests {
        use super::*;
        
        #[tokio::test]
        async fn test_retry_then_circuit_breaker_open() {
            // Configure resilience with both retry and circuit breaker
            let retry_config = RetryConfig {
                max_retries: 2,
                initial_interval: Duration::from_millis(10),
                ..RetryConfig::default()
            };
            
            let cb_config = CircuitBreakerConfig {
                failure_threshold: 3, // Open after 3 failures (which is what we expect after retries)
                reset_timeout: Duration::from_millis(50),
                ..CircuitBreakerConfig::default()
            };
            
            let resilience = Resilience::new(retry_config, cb_config);
            
            // Track number of attempts over multiple executions
            let total_attempts = Arc::new(AtomicUsize::new(0));
            
            // First execution - should retry but still fail
            {
                let attempts_clone = Arc::clone(&total_attempts);
                let result = resilience.execute(move || {
                    let attempts = Arc::clone(&attempts_clone);
                    async move {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        Err(ServiceError::network("Simulated network error"))
                    }
                }).await;
                
                assert!(result.is_err());
            }
            
            // Expected attempts: 1 initial + 2 retries = 3
            assert_eq!(total_attempts.load(Ordering::SeqCst), 3);
            
            // Circuit breaker should be open now (3 failures)
            assert_eq!(resilience.circuit_breaker_status(), CircuitBreakerStatus::Open);
            
            // Reset attempt counter
            total_attempts.store(0, Ordering::SeqCst);
            
            // Second execution - should immediately fail due to open circuit
            {
                let attempts_clone = Arc::clone(&total_attempts);
                let result = resilience.execute(move || {
                    let attempts = Arc::clone(&attempts_clone);
                    async move {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        Ok("This should not be reached")
                    }
                }).await;
                
                assert!(result.is_err());
                assert!(result.unwrap_err().to_string().contains("Circuit breaker is open"));
            }
            
            // No attempts should have been made
            assert_eq!(total_attempts.load(Ordering::SeqCst), 0);
            
            // Wait for circuit breaker timeout
            tokio::time::sleep(Duration::from_millis(60)).await;
            
            // Third execution - should now attempt again (half-open)
            {
                let attempts_clone = Arc::clone(&total_attempts);
                let result = resilience.execute(move || {
                    let attempts = Arc::clone(&attempts_clone);
                    async move {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        Ok("Success!")
                    }
                }).await;
                
                assert!(result.is_ok());
                assert_eq!(result.unwrap(), "Success!");
            }
            
            // One attempt should have been made
            assert_eq!(total_attempts.load(Ordering::SeqCst), 1);
            
            // Circuit breaker should be closed now
            assert_eq!(resilience.circuit_breaker_status(), CircuitBreakerStatus::Closed);
        }
        
        #[tokio::test]
        async fn test_resilience_with_selective_retry() {
            // Configure resilience
            let retry_config = RetryConfig {
                max_retries: 2,
                initial_interval: Duration::from_millis(10),
                ..RetryConfig::default()
            };
            
            let cb_config = CircuitBreakerConfig::default();
            let resilience = Resilience::new(retry_config, cb_config);
            
            // Test with a retryable error
            let attempts = Arc::new(AtomicUsize::new(0));
            {
                let attempts_clone = Arc::clone(&attempts);
                let result = resilience.execute(move || {
                    let attempts = Arc::clone(&attempts_clone);
                    async move {
                        let current = attempts.fetch_add(1, Ordering::SeqCst);
                        if current < 2 {
                            Err(ServiceError::network("Retryable error"))
                        } else {
                            Ok("Success after retry")
                        }
                    }
                }).await;
                
                assert!(result.is_ok());
                assert_eq!(result.unwrap(), "Success after retry");
                assert_eq!(attempts.load(Ordering::SeqCst), 3); // 1 initial + 2 attempts
            }
            
            // Reset attempt counter
            attempts.store(0, Ordering::SeqCst);
            
            // Test with a non-retryable error
            {
                let attempts_clone = Arc::clone(&attempts);
                let result = resilience.execute(move || {
                    let attempts = Arc::clone(&attempts_clone);
                    async move {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        Err(ServiceError::validation("Non-retryable error"))
                    }
                }).await;
                
                assert!(result.is_err());
                assert_eq!(attempts.load(Ordering::SeqCst), 1); // Only 1 attempt, no retries
            }
        }
        
        #[tokio::test]
        async fn test_resilience_error_categorization() {
            // Configure resilience
            let resilience = Resilience::default();
            
            // Define test cases: error type, expected to be retryable, expected to increment circuit breaker
            let test_cases = vec![
                (ServiceError::network("Network error"), true, true),
                (ServiceError::timeout("Timeout"), true, false),
                (ServiceError::service("Service error"), true, true),
                (ServiceError::rate_limit("Rate limited"), true, false),
                (ServiceError::authentication("Auth error"), false, true),
                (ServiceError::authorization("Auth error"), false, true),
                (ServiceError::validation("Validation error"), false, true),
                (ServiceError::parsing("Parsing error"), false, true),
            ];
            
            for (error, should_retry, should_count_failure) in test_cases {
                // Create a new resilience for each test
                let mut resilience = Resilience::default();
                
                // Check if error is retryable
                assert_eq!(resilience.should_retry(&error), should_retry, 
                           "Error {:?} should_retry={}", error, should_retry);
                
                // Check if error increments circuit breaker
                let counts_as_failure = resilience.counts_as_failure(&error);
                assert_eq!(counts_as_failure, should_count_failure,
                           "Error {:?} counts_as_failure={}", error, should_count_failure);
            }
        }
    }
}