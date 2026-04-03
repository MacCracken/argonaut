//! Shared API response types for argonaut consumers.
//!
//! This module provides serializable types that external crates use to
//! interact with argonaut's service management and boot orchestration.
//!
//! # Consumers
//!
//! - **agnoshi** (CLI): imports [`ServiceStatus`], [`ServiceListResponse`]
//!   for `service status` / `service list` commands.
//! - **daimon** (HTTP API): imports [`ServiceCreateRequest`],
//!   [`ServiceStatus`], [`ServiceListResponse`] for `/v1/services` CRUD.
//! - **MCP tools**: imports [`SystemStatusResponse`], [`ServiceListResponse`],
//!   [`BootLogResponse`] for `argonaut_status`, `argonaut_services`,
//!   `argonaut_boot_log` tool handlers.
//! - **nazar** (metrics): imports [`SystemMetrics`], [`ServiceMetrics`]
//!   for the `/v1/services` metrics scrape endpoint.
//!
//! # Example
//!
//! ```rust,no_run
//! use argonaut::{ArgonautInit, ArgonautConfig};
//!
//! let init = ArgonautInit::new(ArgonautConfig::default());
//! let status = init.system_status();
//! let json = serde_json::to_string_pretty(&status).unwrap();
//! println!("{json}");
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::types::{
    ArgonautStats, BootMode, BootStep, HealthCheck, ManagedService, ReadyCheck, RestartConfig,
    RestartPolicy, ServiceDefinition, ServiceState,
};

// ---------------------------------------------------------------------------
// Service status
// ---------------------------------------------------------------------------

/// Status summary for a single service.
///
/// Returned by [`ArgonautInit::service_status`] and included in
/// [`ServiceListResponse`]. Serializes to JSON for CLI, REST, and MCP
/// consumers.
///
/// # Example (agnoshi CLI)
///
/// ```rust,no_run
/// # use argonaut::{ArgonautInit, ArgonautConfig};
/// let init = ArgonautInit::new(ArgonautConfig::default());
/// if let Some(status) = init.service_status("daimon") {
///     println!("{}: {}", status.name, status.state);
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub description: String,
    pub state: ServiceState,
    pub pid: Option<u32>,
    pub enabled: bool,
    /// Milliseconds since the service started, computed at call time.
    pub uptime_ms: Option<u64>,
    pub restart_count: u32,
    pub last_health_check: Option<DateTime<Utc>>,
    pub depends_on: Vec<String>,
    pub restart_policy: RestartPolicy,
}

/// Response for listing all managed services.
///
/// # Example (daimon REST)
///
/// ```rust,no_run
/// # use argonaut::{ArgonautInit, ArgonautConfig};
/// let init = ArgonautInit::new(ArgonautConfig::default());
/// let list = init.list_services();
/// // Serialize to JSON for GET /v1/services
/// let json = serde_json::to_string(&list).unwrap();
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceListResponse {
    pub services: Vec<ServiceStatus>,
    pub total: usize,
    pub running: usize,
    pub starting: usize,
    pub stopping: usize,
    pub failed: usize,
    pub stopped: usize,
}

// ---------------------------------------------------------------------------
// System status
// ---------------------------------------------------------------------------

/// Full system status response.
///
/// Combines boot state, service list, and aggregate statistics. Used by
/// MCP `argonaut_status` tool and agnoshi `status` command.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    pub boot_mode: BootMode,
    pub boot_complete: bool,
    pub boot_duration_ms: Option<u64>,
    pub services: ServiceListResponse,
    pub stats: ArgonautStats,
}

// ---------------------------------------------------------------------------
// Boot log
// ---------------------------------------------------------------------------

/// Boot log response for MCP `argonaut_boot_log` tool.
///
/// Contains the full boot sequence with per-step timing and status.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootLogResponse {
    pub boot_mode: BootMode,
    pub steps: Vec<BootStep>,
    pub boot_started: Option<DateTime<Utc>>,
    pub boot_completed: Option<DateTime<Utc>>,
    pub boot_duration_ms: Option<u64>,
}

// ---------------------------------------------------------------------------
// Service creation (daimon REST API)
// ---------------------------------------------------------------------------

/// Request body for creating/registering a service via the daimon REST API.
///
/// # Example (daimon handler)
///
/// ```rust,no_run
/// # use argonaut::{ArgonautInit, ArgonautConfig};
/// # use argonaut::api::ServiceCreateRequest;
/// # use std::path::PathBuf;
/// let mut init = ArgonautInit::new(ArgonautConfig::default());
/// let req = ServiceCreateRequest {
///     name: "my-service".into(),
///     description: "Custom service".into(),
///     binary_path: PathBuf::from("/usr/bin/my-service"),
///     args: vec![],
///     environment: Default::default(),
///     depends_on: vec![],
///     restart_policy: argonaut::RestartPolicy::OnFailure,
///     restart_config: None,
///     health_check: None,
///     ready_check: None,
///     enabled: true,
/// };
/// let status = init.create_service_from_request(req).unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCreateRequest {
    pub name: String,
    pub description: String,
    pub binary_path: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub restart_policy: RestartPolicy,
    pub restart_config: Option<RestartConfig>,
    pub health_check: Option<HealthCheck>,
    pub ready_check: Option<ReadyCheck>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Metrics (nazar integration)
// ---------------------------------------------------------------------------

/// Per-service metrics for nazar scraping.
///
/// # Example (nazar metrics endpoint)
///
/// ```rust,no_run
/// # use argonaut::{ArgonautInit, ArgonautConfig};
/// let init = ArgonautInit::new(ArgonautConfig::default());
/// let metrics = init.system_metrics();
/// for svc in &metrics.service_metrics {
///     println!("argonaut_service_restarts{{service=\"{}\"}} {}", svc.name, svc.restart_count);
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub name: String,
    pub state: ServiceState,
    pub uptime_ms: Option<u64>,
    pub restart_count: u32,
    pub last_health_check: Option<DateTime<Utc>>,
    pub pid: Option<u32>,
}

/// System-wide metrics for nazar scraping.
///
/// Richer than [`ArgonautStats`] — includes per-service breakdown.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub boot_mode: BootMode,
    pub boot_duration_ms: Option<u64>,
    pub boot_complete: bool,
    pub services_running: usize,
    pub services_failed: usize,
    pub services_stopped: usize,
    pub total_restarts: u32,
    pub service_metrics: Vec<ServiceMetrics>,
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Compute uptime in milliseconds from an optional `started_at` timestamp.
#[must_use]
fn uptime_ms(started_at: Option<DateTime<Utc>>) -> Option<u64> {
    started_at.map(|t| {
        Utc::now()
            .signed_duration_since(t)
            .num_milliseconds()
            .max(0) as u64
    })
}

/// Build a [`ServiceStatus`] from a [`ManagedService`].
#[must_use]
fn service_to_status(svc: &ManagedService) -> ServiceStatus {
    ServiceStatus {
        name: svc.definition.name.clone(),
        description: svc.definition.description.clone(),
        state: svc.state.clone(),
        pid: svc.pid,
        enabled: svc.definition.enabled,
        uptime_ms: if svc.state == ServiceState::Running || svc.state == ServiceState::Starting {
            uptime_ms(svc.started_at)
        } else {
            None
        },
        restart_count: svc.restart_count,
        last_health_check: svc.last_health_check,
        depends_on: svc.definition.depends_on.clone(),
        restart_policy: svc.definition.restart_policy,
    }
}

// ---------------------------------------------------------------------------
// ArgonautInit methods
// ---------------------------------------------------------------------------

impl crate::ArgonautInit {
    /// Get the status of a single service by name.
    ///
    /// Returns `None` if the service is not registered.
    #[must_use]
    pub fn service_status(&self, name: &str) -> Option<ServiceStatus> {
        self.services.get(name).map(service_to_status)
    }

    /// List all managed services with their current status.
    #[must_use]
    pub fn list_services(&self) -> ServiceListResponse {
        let mut services: Vec<ServiceStatus> =
            self.services.values().map(service_to_status).collect();
        // Sort by name for stable output
        services.sort_by(|a, b| a.name.cmp(&b.name));

        let running = services
            .iter()
            .filter(|s| s.state == ServiceState::Running)
            .count();
        let starting = services
            .iter()
            .filter(|s| s.state == ServiceState::Starting)
            .count();
        let stopping = services
            .iter()
            .filter(|s| s.state == ServiceState::Stopping)
            .count();
        let failed = services
            .iter()
            .filter(|s| matches!(s.state, ServiceState::Failed(_)))
            .count();
        let stopped = services
            .iter()
            .filter(|s| s.state == ServiceState::Stopped)
            .count();
        let total = services.len();

        ServiceListResponse {
            services,
            total,
            running,
            starting,
            stopping,
            failed,
            stopped,
        }
    }

    /// Get full system status: boot info, services, and statistics.
    ///
    /// Used by MCP `argonaut_status` tool and agnoshi `status` command.
    #[must_use]
    pub fn system_status(&self) -> SystemStatusResponse {
        SystemStatusResponse {
            boot_mode: self.config.boot_mode,
            boot_complete: self.is_boot_complete(),
            boot_duration_ms: self.boot_duration_ms(),
            services: self.list_services(),
            stats: self.stats(),
        }
    }

    /// Get the boot log: all boot steps with timing and status.
    ///
    /// Used by MCP `argonaut_boot_log` tool.
    #[must_use]
    pub fn boot_log(&self) -> BootLogResponse {
        BootLogResponse {
            boot_mode: self.config.boot_mode,
            steps: self.boot_sequence.clone(),
            boot_started: self.boot_started,
            boot_completed: self.boot_completed,
            boot_duration_ms: self.boot_duration_ms(),
        }
    }

    /// Get system-wide metrics with per-service breakdown.
    ///
    /// Used by nazar for the `/v1/services` metrics scrape endpoint.
    /// Each [`ServiceMetrics`] entry can be converted to Prometheus
    /// gauges/counters by the nazar exporter.
    #[must_use]
    pub fn system_metrics(&self) -> SystemMetrics {
        let mut service_metrics: Vec<ServiceMetrics> = self
            .services
            .values()
            .map(|svc| ServiceMetrics {
                name: svc.definition.name.clone(),
                state: svc.state.clone(),
                uptime_ms: if svc.state == ServiceState::Running
                    || svc.state == ServiceState::Starting
                {
                    uptime_ms(svc.started_at)
                } else {
                    None
                },
                restart_count: svc.restart_count,
                last_health_check: svc.last_health_check,
                pid: svc.pid,
            })
            .collect();
        service_metrics.sort_by(|a, b| a.name.cmp(&b.name));

        let services_running = service_metrics
            .iter()
            .filter(|s| s.state == ServiceState::Running)
            .count();
        let services_failed = service_metrics
            .iter()
            .filter(|s| matches!(s.state, ServiceState::Failed(_)))
            .count();
        let services_stopped = service_metrics
            .iter()
            .filter(|s| s.state == ServiceState::Stopped)
            .count();
        let total_restarts: u32 = service_metrics.iter().map(|s| s.restart_count).sum();

        SystemMetrics {
            boot_mode: self.config.boot_mode,
            boot_duration_ms: self.boot_duration_ms(),
            boot_complete: self.is_boot_complete(),
            services_running,
            services_failed,
            services_stopped,
            total_restarts,
            service_metrics,
        }
    }

    /// Create and register a service from a [`ServiceCreateRequest`].
    ///
    /// Validates the request, converts it to a [`ServiceDefinition`],
    /// registers it, and returns the new service's status.
    ///
    /// # Errors
    ///
    /// Returns an error if the service name is invalid or the service
    /// is already registered.
    pub fn create_service_from_request(
        &mut self,
        req: ServiceCreateRequest,
    ) -> Result<ServiceStatus> {
        // Validate name
        if req.name.is_empty() {
            bail!("service name cannot be empty");
        }
        if !req
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            bail!(
                "service name '{}' contains invalid characters (allowed: alphanumeric, -, _, .)",
                req.name
            );
        }
        if req.name.contains("..") {
            bail!(
                "service name '{}' contains path traversal sequence",
                req.name
            );
        }
        // Validate binary_path is absolute — prevent relative path exploits
        if !req.binary_path.is_absolute() {
            bail!(
                "binary_path must be absolute, got '{}'",
                req.binary_path.display()
            );
        }
        if self.services.contains_key(&req.name) {
            bail!("service '{}' is already registered", req.name);
        }

        let definition = ServiceDefinition {
            name: req.name.clone(),
            description: req.description,
            binary_path: req.binary_path,
            args: req.args,
            environment: req.environment,
            depends_on: req.depends_on,
            required_for_modes: vec![],
            restart_policy: req.restart_policy,
            restart_config: req.restart_config.unwrap_or_default(),
            health_check: req.health_check,
            ready_check: req.ready_check,
            enabled: req.enabled,
        };

        info!(service = %req.name, "creating service from API request");
        self.register_service(definition);

        self.service_status(&req.name)
            .ok_or_else(|| anyhow::anyhow!("service registration failed for '{}'", req.name))
    }
}
