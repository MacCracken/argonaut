//! Audit logging integration via [libro](https://github.com/MacCracken/libro).
//!
//! Provides tamper-proof audit trails for service lifecycle events. Each
//! [`ServiceEvent`] is recorded as a hash-linked [`libro::AuditEntry`]
//! in an [`AuditLog`] backed by libro's [`AuditChain`].
//!
//! This module is only available when the `audit` feature is enabled:
//!
//! ```toml
//! argonaut = { version = "0.90", features = ["audit"] }
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use argonaut::audit::{AuditLog, AuditIntegration};
//! use argonaut::{ArgonautInit, ArgonautConfig};
//! use argonaut::types::ServiceEventType;
//!
//! let init = ArgonautInit::new(ArgonautConfig::default());
//! let mut audit = AuditLog::new();
//!
//! let event = init.record_audited_event(
//!     &mut audit,
//!     "daimon",
//!     ServiceEventType::Starting,
//! );
//! assert_eq!(audit.len(), 1);
//! ```

use libro::{AuditChain, EventSeverity, QueryFilter};
use tracing::debug;

use crate::types::{ExitStatus, ServiceEvent, ServiceEventType};

/// Audit log backed by a libro [`AuditChain`].
///
/// Wraps a hash-linked audit chain for recording service lifecycle
/// events. Each recorded event is a tamper-evident entry that can be
/// verified and queried.
pub struct AuditLog {
    chain: AuditChain,
}

impl AuditLog {
    /// Create a new, empty audit log.
    #[must_use]
    pub fn new() -> Self {
        Self {
            chain: AuditChain::new(),
        }
    }

    /// Record a service lifecycle event in the audit chain.
    ///
    /// Converts the [`ServiceEvent`] to a libro audit entry with
    /// appropriate severity, source, action, and JSON details.
    pub fn record_service_event(&mut self, event: &ServiceEvent) {
        let severity = event_severity(&event.event_type);
        let action = event.event_type.to_string();
        let details = serde_json::to_value(event).unwrap_or_else(|_| {
            serde_json::json!({
                "service": event.service,
                "event": action,
            })
        });

        self.chain
            .append(severity, "argonaut", action.clone(), details);

        debug!(
            service = %event.service,
            action = %action,
            severity = %severity,
            "audit entry recorded"
        );
    }

    /// Query audit entries using a libro [`QueryFilter`].
    #[must_use]
    pub fn query(&self, filter: &QueryFilter) -> Vec<&libro::AuditEntry> {
        self.chain.query(filter)
    }

    /// Number of entries in the audit chain.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chain.len()
    }

    /// Whether the audit chain is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }

    /// Verify the integrity of the audit chain.
    ///
    /// Returns an error if any entry's hash is invalid or the chain
    /// links are broken.
    pub fn verify(&self) -> libro::Result<()> {
        self.chain.verify()
    }

    /// Get a reference to the underlying audit chain.
    #[must_use]
    pub fn chain(&self) -> &AuditChain {
        &self.chain
    }

    /// Get all entries by source (always `"argonaut"` for this log).
    #[must_use]
    pub fn entries_by_source(&self, source: &str) -> Vec<&libro::AuditEntry> {
        self.chain.by_source(source)
    }

    /// Get all entries at or above a severity level.
    #[must_use]
    pub fn entries_by_severity(&self, severity: EventSeverity) -> Vec<&libro::AuditEntry> {
        self.chain.by_severity(severity)
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Map a [`ServiceEventType`] to a libro [`EventSeverity`].
///
/// Severity mapping:
/// - **Info**: Starting, Started, HealthCheckPassed, ReadyCheckPassed,
///   DependencyMet, Stopped (exit 0)
/// - **Warning**: Stopping, Restarting, DependencyWaiting,
///   HealthCheckFailed, ReadyCheckFailed
/// - **Error**: TimeoutKilled, CrashDetected, Stopped (non-zero exit or signal)
#[must_use]
pub fn event_severity(event_type: &ServiceEventType) -> EventSeverity {
    match event_type {
        ServiceEventType::Starting
        | ServiceEventType::Started { .. }
        | ServiceEventType::HealthCheckPassed
        | ServiceEventType::ReadyCheckPassed
        | ServiceEventType::DependencyMet { .. }
        | ServiceEventType::Enabled
        | ServiceEventType::Disabled => EventSeverity::Info,

        ServiceEventType::Stopped { exit_status } => match exit_status {
            ExitStatus::Code(0) => EventSeverity::Info,
            _ => EventSeverity::Error,
        },

        ServiceEventType::Stopping
        | ServiceEventType::Restarting { .. }
        | ServiceEventType::DependencyWaiting { .. }
        | ServiceEventType::HealthCheckFailed { .. }
        | ServiceEventType::ReadyCheckFailed => EventSeverity::Warning,

        ServiceEventType::TimeoutKilled | ServiceEventType::CrashDetected { .. } => {
            EventSeverity::Error
        }

        // non_exhaustive: default to Warning for future variants
        #[allow(unreachable_patterns)]
        _ => EventSeverity::Warning,
    }
}

/// Extension trait for [`ArgonautInit`] when the `audit` feature is enabled.
///
/// Provides a convenience method that records a service event to both
/// tracing (via [`record_event`]) and the audit chain in one call.
///
/// # Example
///
/// ```rust,no_run
/// use argonaut::audit::{AuditLog, AuditIntegration};
/// use argonaut::{ArgonautInit, ArgonautConfig};
/// use argonaut::types::ServiceEventType;
///
/// let init = ArgonautInit::new(ArgonautConfig::default());
/// let mut audit = AuditLog::new();
/// let event = init.record_audited_event(&mut audit, "daimon", ServiceEventType::Starting);
/// ```
pub trait AuditIntegration {
    /// Record a service event to both the tracing log and the audit chain.
    fn record_audited_event(
        &self,
        audit_log: &mut AuditLog,
        service: &str,
        event_type: ServiceEventType,
    ) -> ServiceEvent;
}

impl AuditIntegration for crate::ArgonautInit {
    fn record_audited_event(
        &self,
        audit_log: &mut AuditLog,
        service: &str,
        event_type: ServiceEventType,
    ) -> ServiceEvent {
        let event = self.record_event(service, event_type);
        audit_log.record_service_event(&event);
        event
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use libro::EventSeverity;

    use super::*;
    use crate::types::ExitStatus;

    fn make_event(service: &str, event_type: ServiceEventType) -> ServiceEvent {
        ServiceEvent {
            timestamp: Utc::now(),
            service: service.into(),
            event_type,
            details: None,
        }
    }

    #[test]
    fn audit_log_records_service_event() {
        let mut log = AuditLog::new();
        let event = make_event("daimon", ServiceEventType::Starting);
        log.record_service_event(&event);
        assert_eq!(log.len(), 1);
        assert!(!log.is_empty());
    }

    #[test]
    fn audit_log_multiple_events() {
        let mut log = AuditLog::new();
        log.record_service_event(&make_event("daimon", ServiceEventType::Starting));
        log.record_service_event(&make_event("daimon", ServiceEventType::Started { pid: 42 }));
        log.record_service_event(&make_event("daimon", ServiceEventType::HealthCheckPassed));
        assert_eq!(log.len(), 3);
        assert!(log.verify().is_ok());
    }

    #[test]
    fn audit_severity_info_events() {
        assert_eq!(
            event_severity(&ServiceEventType::Starting),
            EventSeverity::Info
        );
        assert_eq!(
            event_severity(&ServiceEventType::Started { pid: 1 }),
            EventSeverity::Info
        );
        assert_eq!(
            event_severity(&ServiceEventType::HealthCheckPassed),
            EventSeverity::Info
        );
        assert_eq!(
            event_severity(&ServiceEventType::ReadyCheckPassed),
            EventSeverity::Info
        );
        assert_eq!(
            event_severity(&ServiceEventType::DependencyMet {
                dependency: "db".into()
            }),
            EventSeverity::Info
        );
        assert_eq!(
            event_severity(&ServiceEventType::Stopped {
                exit_status: ExitStatus::Code(0)
            }),
            EventSeverity::Info
        );
    }

    #[test]
    fn audit_severity_warning_events() {
        assert_eq!(
            event_severity(&ServiceEventType::Stopping),
            EventSeverity::Warning
        );
        assert_eq!(
            event_severity(&ServiceEventType::Restarting { restart_count: 1 }),
            EventSeverity::Warning
        );
        assert_eq!(
            event_severity(&ServiceEventType::DependencyWaiting {
                dependency: "db".into()
            }),
            EventSeverity::Warning
        );
        assert_eq!(
            event_severity(&ServiceEventType::HealthCheckFailed { consecutive: 2 }),
            EventSeverity::Warning
        );
        assert_eq!(
            event_severity(&ServiceEventType::ReadyCheckFailed),
            EventSeverity::Warning
        );
    }

    #[test]
    fn audit_severity_error_events() {
        assert_eq!(
            event_severity(&ServiceEventType::TimeoutKilled),
            EventSeverity::Error
        );
        assert_eq!(
            event_severity(&ServiceEventType::CrashDetected {
                exit_status: ExitStatus::Signal(9)
            }),
            EventSeverity::Error
        );
        assert_eq!(
            event_severity(&ServiceEventType::Stopped {
                exit_status: ExitStatus::Code(1)
            }),
            EventSeverity::Error
        );
        assert_eq!(
            event_severity(&ServiceEventType::Stopped {
                exit_status: ExitStatus::Signal(15)
            }),
            EventSeverity::Error
        );
    }

    #[test]
    fn audit_query_by_source() {
        let mut log = AuditLog::new();
        log.record_service_event(&make_event("daimon", ServiceEventType::Starting));
        log.record_service_event(&make_event("redis", ServiceEventType::Starting));

        let entries = log.entries_by_source("argonaut");
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn audit_query_by_severity() {
        let mut log = AuditLog::new();
        log.record_service_event(&make_event("daimon", ServiceEventType::Starting));
        log.record_service_event(&make_event("daimon", ServiceEventType::TimeoutKilled));

        let errors = log.entries_by_severity(EventSeverity::Error);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn audit_query_filter() {
        let mut log = AuditLog::new();
        log.record_service_event(&make_event("daimon", ServiceEventType::Starting));
        log.record_service_event(&make_event("daimon", ServiceEventType::TimeoutKilled));

        let filter = QueryFilter::new()
            .source("argonaut")
            .min_severity(EventSeverity::Error);
        let results = log.query(&filter);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn audit_chain_integrity() {
        let mut log = AuditLog::new();
        for i in 0..10 {
            log.record_service_event(&make_event(&format!("svc-{i}"), ServiceEventType::Starting));
        }
        assert_eq!(log.len(), 10);
        assert!(log.verify().is_ok());
    }
}
