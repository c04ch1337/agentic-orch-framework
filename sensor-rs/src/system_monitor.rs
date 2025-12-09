// sensor-rs/src/system_monitor.rs
// System state polling logic for Digital Twin perception

use serde::{Deserialize, Serialize};
use sysinfo::{Networks, System};

/// Represents the current state of the host system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    /// Timestamp when this state was captured (Unix epoch seconds)
    pub timestamp: u64,
    /// Overall CPU usage percentage (0-100)
    pub cpu_usage_percent: f32,
    /// Total RAM in bytes
    pub total_memory_bytes: u64,
    /// Used RAM in bytes
    pub used_memory_bytes: u64,
    /// RAM usage percentage (0-100)
    pub memory_usage_percent: f32,
    /// Network bytes received since last poll
    pub network_rx_bytes: u64,
    /// Network bytes transmitted since last poll
    pub network_tx_bytes: u64,
    /// Active window title (if available)
    pub active_window_title: Option<String>,
    /// Active window process name (if available)
    pub active_window_process: Option<String>,
}

/// System monitor that polls host state
pub struct SystemMonitor {
    system: System,
    networks: Networks,
}

impl SystemMonitor {
    /// Create a new system monitor
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        let networks = Networks::new_with_refreshed_list();
        
        Self { system, networks }
    }

    /// Poll the current system state
    pub fn poll_state(&mut self) -> SystemState {
        // Refresh system information
        self.system.refresh_all();
        self.networks.refresh(true);

        // Calculate CPU usage (average across all cores)
        let cpu_usage_percent = self.system.global_cpu_usage();

        // Memory information
        let total_memory_bytes = self.system.total_memory();
        let used_memory_bytes = self.system.used_memory();
        let memory_usage_percent = if total_memory_bytes > 0 {
            (used_memory_bytes as f32 / total_memory_bytes as f32) * 100.0
        } else {
            0.0
        };

        // Network statistics (sum across all interfaces)
        let (network_rx_bytes, network_tx_bytes) = self
            .networks
            .iter()
            .fold((0u64, 0u64), |(rx, tx), (_, data)| {
                (rx + data.received(), tx + data.transmitted())
            });

        // Active window information
        let (active_window_title, active_window_process) = get_active_window_info();

        // Current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        SystemState {
            timestamp,
            cpu_usage_percent,
            total_memory_bytes,
            used_memory_bytes,
            memory_usage_percent,
            network_rx_bytes,
            network_tx_bytes,
            active_window_title,
            active_window_process,
        }
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Get active window information using platform-specific API
fn get_active_window_info() -> (Option<String>, Option<String>) {
    match active_win_pos_rs::get_active_window() {
        Ok(window) => {
            let title = if window.title.is_empty() {
                None
            } else {
                Some(window.title)
            };
            let process = if window.process_path.to_string_lossy().is_empty() {
                None
            } else {
                // Extract just the process name from the full path
                window
                    .process_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
            };
            (title, process)
        }
        Err(e) => {
            log::debug!("Could not get active window: {:?}", e);
            (None, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_monitor_creation() {
        let _monitor = SystemMonitor::new();
    }

    #[test]
    fn test_poll_state() {
        let mut monitor = SystemMonitor::new();
        let state = monitor.poll_state();
        
        // Basic sanity checks
        assert!(state.timestamp > 0);
        assert!(state.cpu_usage_percent >= 0.0 && state.cpu_usage_percent <= 100.0);
        assert!(state.total_memory_bytes > 0);
        assert!(state.memory_usage_percent >= 0.0 && state.memory_usage_percent <= 100.0);
    }
}
