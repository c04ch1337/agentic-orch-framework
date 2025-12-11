//! Monitoring integration stubs for error-handling-rs
//!
//! This module provides minimal, non-invasive stubs so the crate compiles
//! without enforcing a specific metrics backend beyond the `metrics` crate
//! that is already in use.

use crate::types::{Error, Result, ErrorKind};
use metrics::{counter, gauge, histogram};
use tracing::warn;

/// High-level health/monitoring event that can be emitted by services.
#[derive(Debug, Clone)]
pub struct MonitoringEvent {
    pub name: String,
    pub details: Option<String>,
}

impl MonitoringEvent {
    /// Create a new monitoring event with the given name.
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            details: None,
        }
    }

    /// Attach additional human-readable details to the event.
    pub fn details<S: Into<String>>(mut self, details: S) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Emit a lightweight monitoring event as a metric and optional log entry.
///
/// This keeps behavior minimal and side-effect free beyond metrics/logging,
/// so it is safe to call from anywhere without introducing new dependencies.
pub fn emit_event(event: MonitoringEvent) -> Result<()> {
    let key = format!("monitoring.event.{}", event.name);
    counter!(key, 1);

    if let Some(details) = event.details {
        warn!(event = %event.name, details = %details, "Monitoring event");
    }

    Ok(())
}