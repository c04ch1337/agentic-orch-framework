use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use once_cell::sync::Lazy;

static RECOVERY_COUNTER: Lazy<Arc<AtomicU32>> = Lazy::new(|| Arc::new(AtomicU32::new(0)));
static LAST_RESET_TIME: Lazy<Arc<AtomicU64>> = Lazy::new(|| Arc::new(AtomicU64::new(current_timestamp())));

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn log_recovery_attempt(error: &str) {
    let current_time = current_timestamp();
    let last_reset = LAST_RESET_TIME.load(Ordering::Relaxed);
    
    // Reset counter if 24 hours have passed
    if current_time - last_reset >= 86400 {
        RECOVERY_COUNTER.store(0, Ordering::SeqCst);
        LAST_RESET_TIME.store(current_time, Ordering::SeqCst);
        log::info!("Recovery counter reset after 24 hours");
    }
    
    let attempt = RECOVERY_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    
    log::error!(
        "Service recovery attempt {} of {}: Error: {}",
        attempt,
        super::MAX_RESTART_ATTEMPTS,
        error
    );
    
    if attempt >= super::MAX_RESTART_ATTEMPTS {
        log::error!(
            "Maximum recovery attempts ({}) reached. Manual intervention required.",
            super::MAX_RESTART_ATTEMPTS
        );
    }
}

pub fn get_recovery_stats() -> (u32, u64) {
    (
        RECOVERY_COUNTER.load(Ordering::Relaxed),
        LAST_RESET_TIME.load(Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_recovery_counter() {
        // Reset counter
        RECOVERY_COUNTER.store(0, Ordering::SeqCst);
        
        // Log multiple recovery attempts
        log_recovery_attempt("Test error 1");
        log_recovery_attempt("Test error 2");
        
        assert_eq!(RECOVERY_COUNTER.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_counter_reset() {
        // Set last reset time to 25 hours ago
        LAST_RESET_TIME.store(current_timestamp() - 90000, Ordering::SeqCst);
        RECOVERY_COUNTER.store(2, Ordering::SeqCst);
        
        // This should trigger a reset
        log_recovery_attempt("Test error after reset period");
        
        assert_eq!(RECOVERY_COUNTER.load(Ordering::Relaxed), 1);
    }
}