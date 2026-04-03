#![forbid(unsafe_code)]
//! Argonaut — Init System for AGNOS
//!
//! Minimal init system that boots AGNOS in under 3 seconds. Manages
//! service startup ordering, health checks, and shutdown sequences.
//! Named after the Greek Argonauts who sailed the Argo — one letter
//! off from AGNOS.
//!
//! This module defines the shared types and boot orchestration logic
//! that agent-runtime uses. The actual PID 1 binary will live in a
//! separate crate; this module provides the brain.
//!
//! Submodules:
//! - **types**: All enums, structs, and configuration types
//! - **boot**: Boot sequence construction per [`BootMode`]
//! - **services**: Service management, default definitions, dependency resolution
//! - **runlevels**: Shutdown planning and runlevel switching
//! - **edge_boot**: Read-only rootfs and dm-verity helpers

pub mod api;
pub mod systemd;
pub mod types;

mod boot;
mod runlevels;
mod services;

pub mod edge_boot;
pub mod health;
pub mod notify;
pub mod process;

#[cfg(feature = "audit")]
pub mod audit;

#[cfg(test)]
mod tests;

// Re-export all public types so external consumers see the same flat API
// as they did when this was a single argonaut.rs file.
pub use api::{
    BootLogResponse, ServiceCreateRequest, ServiceListResponse, ServiceMetrics, ServiceStatus,
    SystemMetrics, SystemStatusResponse,
};
pub use edge_boot::{
    EdgeBootResult, FleetRegistration, close_luks, configure_readonly_rootfs, execute_edge_boot,
    unlock_luks, validate_edge_profile, verify_rootfs_integrity,
};
pub use health::{HealthHistory, HealthState, execute_health_check, execute_ready_check};
pub use notify::{NotifyListener, NotifyMessage, send_notify};
pub use process::{ProcessTable, SpawnedProcess, run_command, run_command_sequence, spawn_process};
pub use systemd::{generate_unit, generate_unit_filename};
pub use types::{
    ArgonautConfig, ArgonautStats, BootMode, BootStage, BootStep, BootStepStatus, CrashAction,
    EdgeBootConfig, EmergencyShellConfig, ExitStatus, HealthCheck, HealthCheckResult,
    HealthCheckType, HealthTracker, ManagedService, ProcessSpec, ReadyCheck, RestartConfig,
    RestartPolicy, Runlevel, RunlevelSwitchPlan, RunlevelSwitchResult, SafeCommand,
    ServiceDefinition, ServiceEvent, ServiceEventType, ServiceState, ServiceTarget, ShutdownAction,
    ShutdownPlan, ShutdownStep, ShutdownStepStatus, ShutdownType,
};

#[cfg(feature = "audit")]
pub use audit::{AuditIntegration, AuditLog, event_severity};

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use tracing::{debug, error, info, warn};

/// The main init-system orchestrator. Holds the boot sequence,
/// all managed services, and the process table for running processes.
pub struct ArgonautInit {
    pub config: ArgonautConfig,
    pub boot_sequence: Vec<BootStep>,
    pub services: HashMap<String, ManagedService>,
    pub boot_started: Option<DateTime<Utc>>,
    pub boot_completed: Option<DateTime<Utc>>,
    /// Tracks all running service processes.
    pub processes: ProcessTable,
}

impl ArgonautInit {
    /// Create a new init system from the given configuration. Builds
    /// the boot sequence and registers default services for the mode.
    pub fn new(config: ArgonautConfig) -> Self {
        let boot_sequence = Self::build_boot_sequence(config.boot_mode);
        let default_svc = Self::default_services(config.boot_mode);
        let mut services = HashMap::new();
        for svc in default_svc {
            let managed = ManagedService {
                definition: svc.clone(),
                state: ServiceState::Stopped,
                pid: None,
                started_at: None,
                restart_count: 0,
                last_health_check: None,
            };
            services.insert(svc.name.clone(), managed);
        }
        // Also register any extra services from config.
        for svc in &config.services {
            if !services.contains_key(&svc.name) {
                let managed = ManagedService {
                    definition: svc.clone(),
                    state: ServiceState::Stopped,
                    pid: None,
                    started_at: None,
                    restart_count: 0,
                    last_health_check: None,
                };
                services.insert(svc.name.clone(), managed);
            }
        }
        info!(mode = %config.boot_mode, steps = boot_sequence.len(), services = services.len(), "argonaut initialized");
        Self {
            config,
            boot_sequence,
            services,
            boot_started: None,
            boot_completed: None,
            processes: ProcessTable::new(),
        }
    }

    /// Mark a boot stage as complete. Returns `true` if the stage was
    /// found and updated.
    pub fn mark_step_complete(&mut self, stage: BootStage) -> bool {
        if let Some(step) = self.boot_sequence.iter_mut().find(|s| s.stage == stage) {
            // Ensure started_at is populated.
            if step.started_at.is_none() {
                step.started_at = Some(Utc::now());
            }
            // Set boot_started on the first step start if not yet set.
            if self.boot_started.is_none() {
                self.boot_started = step.started_at;
            }
            step.status = BootStepStatus::Complete;
            step.completed_at = Some(Utc::now());
            info!(stage = %stage, "boot step complete");

            // If this was the BootComplete stage, mark the overall boot time.
            if stage == BootStage::BootComplete {
                self.boot_completed = Some(Utc::now());
            }
            true
        } else {
            false
        }
    }

    /// Mark a boot stage as failed. Returns `true` if the stage was
    /// found and updated.
    pub fn mark_step_failed(&mut self, stage: BootStage, error: String) -> bool {
        if let Some(step) = self.boot_sequence.iter_mut().find(|s| s.stage == stage) {
            tracing::warn!(stage = %stage, error = %error, "boot step failed");
            // Ensure started_at is populated.
            if step.started_at.is_none() {
                step.started_at = Some(Utc::now());
            }
            // Set boot_started on the first step start if not yet set.
            if self.boot_started.is_none() {
                self.boot_started = step.started_at;
            }
            step.status = BootStepStatus::Failed;
            step.completed_at = Some(Utc::now());
            step.error = Some(error);
            true
        } else {
            false
        }
    }

    /// The first boot step that is not yet complete or failed.
    #[must_use]
    pub fn current_stage(&self) -> Option<&BootStep> {
        self.boot_sequence.iter().find(|s| {
            s.status != BootStepStatus::Complete
                && s.status != BootStepStatus::Failed
                && s.status != BootStepStatus::Skipped
        })
    }

    /// Whether every boot step has completed (or been skipped).
    #[must_use]
    pub fn is_boot_complete(&self) -> bool {
        self.boot_sequence.iter().all(|s| {
            s.status == BootStepStatus::Complete
                || s.status == BootStepStatus::Skipped
                || (!s.required && s.status == BootStepStatus::Failed)
        })
    }

    /// Total boot duration in milliseconds, if boot has completed.
    #[must_use]
    pub fn boot_duration_ms(&self) -> Option<u64> {
        match (self.boot_started, self.boot_completed) {
            (Some(start), Some(end)) => {
                let dur = end.signed_duration_since(start);
                Some(dur.num_milliseconds().max(0) as u64)
            }
            _ => None,
        }
    }

    /// All boot steps that have failed.
    #[must_use]
    pub fn failed_steps(&self) -> Vec<&BootStep> {
        self.boot_sequence
            .iter()
            .filter(|s| s.status == BootStepStatus::Failed)
            .collect()
    }

    /// Collect current statistics.
    #[must_use]
    pub fn stats(&self) -> ArgonautStats {
        let services_running = self
            .services
            .values()
            .filter(|s| s.state == ServiceState::Running)
            .count();
        let services_failed = self
            .services
            .values()
            .filter(|s| matches!(s.state, ServiceState::Failed(_)))
            .count();
        let total_restarts: u32 = self.services.values().map(|s| s.restart_count).sum();

        ArgonautStats {
            boot_mode: self.config.boot_mode,
            boot_duration_ms: self.boot_duration_ms(),
            services_running,
            services_failed,
            total_restarts,
            boot_complete: self.is_boot_complete(),
        }
    }

    /// Determine whether the system should drop to an emergency shell.
    /// Called after a critical boot step failure.
    #[must_use]
    pub fn should_drop_to_emergency(&self) -> bool {
        let failed = self.failed_steps();
        let should_drop = failed
            .iter()
            .any(|step| step.required && step.status == BootStepStatus::Failed);
        if should_drop {
            let failed_stages: Vec<_> = failed
                .iter()
                .filter(|s| s.required)
                .map(|s| s.stage.to_string())
                .collect();
            warn!(
                failed_stages = ?failed_stages,
                "required boot stages failed — dropping to emergency shell"
            );
        }
        should_drop
    }

    /// Get the emergency shell configuration.
    #[must_use]
    pub fn emergency_shell_config(&self) -> EmergencyShellConfig {
        EmergencyShellConfig::default()
    }

    // -------------------------------------------------------------------
    // Process execution
    // -------------------------------------------------------------------

    /// Start a service by name. Spawns the process, transitions state
    /// to Starting → Running, and tracks the PID.
    ///
    /// Returns an error if the service is unknown, dependencies aren't
    /// met, or the process fails to spawn.
    pub fn start_service(&mut self, name: &str) -> Result<u32> {
        // Check if service is enabled
        if let Some(svc) = self.services.get(name)
            && !svc.definition.enabled
        {
            bail!("cannot start service '{}': service is disabled", name);
        }

        // Validate the service exists and transition to Starting
        if !self.set_service_state(name, ServiceState::Starting) {
            bail!(
                "cannot start service '{}': invalid state transition or unmet dependencies",
                name
            );
        }

        let spec = {
            let Some(svc) = self.services.get(name) else {
                bail!("service '{}' not found", name);
            };
            ProcessSpec::from_service(&svc.definition)
        };

        // Grab the ready check config before spawning (avoids borrow issues)
        let ready_check = self
            .services
            .get(name)
            .and_then(|s| s.definition.ready_check.clone());

        match process::spawn_process(&spec, name) {
            Ok(spawned) => {
                let pid = spawned.pid;
                self.processes.insert(spawned);

                // Update PID and started_at
                if let Some(svc) = self.services.get_mut(name) {
                    svc.pid = Some(pid);
                    svc.started_at = Some(Utc::now());
                }

                // Run ready check if configured
                if let Some(ref rc) = ready_check {
                    let result = health::execute_ready_check(name, rc, Some(pid));
                    if !result.passed {
                        warn!(
                            service = name,
                            pid = pid,
                            "ready check failed — marking service as failed"
                        );
                        if let Some(svc) = self.services.get_mut(name) {
                            svc.state = ServiceState::Failed(
                                result
                                    .message
                                    .unwrap_or_else(|| "ready check failed".into()),
                            );
                        }
                        // Kill the process since it never became ready
                        if let Some(mut proc) = self.processes.remove(name) {
                            if let Err(e) = proc.kill() {
                                warn!(service = name, error = %e, "failed to kill process after ready check failure");
                            }
                            if let Err(e) = proc.wait() {
                                warn!(service = name, error = %e, "failed to wait for process after ready check failure");
                            }
                        }
                        bail!(
                            "service '{}' failed ready check after {} retries",
                            name,
                            rc.retries
                        );
                    }
                }

                // Transition to Running
                if let Some(svc) = self.services.get_mut(name) {
                    svc.state = ServiceState::Running;
                    svc.last_health_check = Some(Utc::now());
                }

                info!(service = name, pid = pid, "service started");
                Ok(pid)
            }
            Err(e) => {
                error!(service = name, error = %e, "failed to start service");
                if let Some(svc) = self.services.get_mut(name) {
                    svc.state = ServiceState::Failed(e.to_string());
                }
                Err(e)
            }
        }
    }

    /// Stop a service by name. Sends SIGTERM, waits up to `timeout`,
    /// then SIGKILL if needed.
    ///
    /// Returns the exit code, or an error if the service is unknown or
    /// not running.
    pub fn stop_service(&mut self, name: &str, timeout: Duration) -> Result<i32> {
        if !self.set_service_state(name, ServiceState::Stopping) {
            bail!("cannot stop service '{}': not running or unknown", name);
        }

        let code = if let Some(mut proc) = self.processes.remove(name) {
            proc.stop(timeout)?
        } else {
            warn!(
                service = name,
                "no process tracked for service, marking as stopped"
            );
            0
        };

        // Update managed service state
        if let Some(svc) = self.services.get_mut(name) {
            svc.pid = None;
            svc.state = ServiceState::Stopped;
        }

        info!(service = name, exit_code = code, "service stopped");
        Ok(code)
    }

    /// Restart a service: stop it (if running) then start it again.
    /// Increments the restart counter.
    pub fn restart_service(&mut self, name: &str, stop_timeout: Duration) -> Result<u32> {
        // Check restart limit before proceeding
        if let Some(svc) = self.services.get(name)
            && svc
                .definition
                .restart_config
                .limit_exceeded(svc.restart_count)
        {
            bail!(
                "service '{}' exceeded restart limit ({}/{})",
                name,
                svc.restart_count,
                svc.definition.restart_config.max_restarts
            );
        }
        info!(service = name, "restarting service");

        // Stop if currently running
        let current_state = self.get_service_state(name).cloned();
        if matches!(
            current_state,
            Some(ServiceState::Running) | Some(ServiceState::Starting)
        ) {
            self.stop_service(name, stop_timeout)?;
        }

        // Increment restart count
        if let Some(svc) = self.services.get_mut(name) {
            svc.restart_count += 1;
            debug!(
                service = name,
                restart_count = svc.restart_count,
                "restart counter incremented"
            );
        }

        self.start_service(name)
    }

    /// Reap any service processes that have exited and update their
    /// managed state accordingly. Returns the list of
    /// (service_name, exit_code, crash_action) for services that exited.
    pub fn reap_services(&mut self) -> Vec<(String, i32, CrashAction)> {
        let exited = self.processes.reap_exited();
        let mut results = Vec::new();

        for (name, code) in exited {
            let exit_status = ExitStatus::Code(code);

            // Update state
            if let Some(svc) = self.services.get_mut(&name) {
                svc.pid = None;
                if code == 0 {
                    svc.state = ServiceState::Stopped;
                } else {
                    svc.state = ServiceState::Failed(format!("exit code {}", code));
                }
                info!(
                    service = %name,
                    exit_code = code,
                    state = %svc.state,
                    "reaped exited service"
                );
            }

            // Determine crash action
            let action = self.on_service_crash(&name, &exit_status);
            results.push((name, code, action));
        }

        results
    }

    /// Stop all running services. Used during shutdown.
    pub fn stop_all_services(&mut self, timeout: Duration) -> Vec<(String, i32)> {
        info!(
            timeout_ms = timeout.as_millis() as u64,
            "stopping all services"
        );
        let results = self.processes.stop_all(timeout);

        for (name, code) in &results {
            if let Some(svc) = self.services.get_mut(name) {
                svc.pid = None;
                if *code == 0 {
                    svc.state = ServiceState::Stopped;
                } else {
                    svc.state = ServiceState::Failed(format!(
                        "killed during shutdown (exit code {})",
                        code
                    ));
                }
            }
            debug!(service = %name, exit_code = code, "service stopped during shutdown");
        }

        results
    }

    /// Check for services that have exceeded their watchdog timeout.
    ///
    /// A service is considered timed out if:
    /// - It is in `Starting` state and has been starting longer than
    ///   its ready check timeout (if configured), or
    /// - It is in `Running` state and its health check has been
    ///   failing for longer than `retries * interval_ms`.
    ///
    /// Returns the names of services that should be killed.
    #[must_use]
    pub fn check_watchdog(&self) -> Vec<String> {
        let mut timed_out = Vec::new();

        for (name, proc) in self.processes.iter() {
            let svc = match self.services.get(name) {
                Some(s) => s,
                None => continue,
            };

            let uptime = proc.uptime();

            // Check startup timeout (ready check)
            if svc.state == ServiceState::Starting
                && let Some(ref rc) = svc.definition.ready_check
            {
                let max_startup = Duration::from_millis(rc.timeout_ms);
                if uptime > max_startup {
                    warn!(
                        service = name,
                        uptime_ms = uptime.as_millis() as u64,
                        timeout_ms = rc.timeout_ms,
                        "service exceeded startup timeout"
                    );
                    timed_out.push(name.to_string());
                }
            }

            // Check runtime watchdog (health check)
            if svc.state == ServiceState::Running
                && let Some(ref hc) = svc.definition.health_check
            {
                let watchdog_ms = hc.interval_ms * u64::from(hc.retries) + hc.timeout_ms;
                let watchdog = Duration::from_millis(watchdog_ms);

                let should_trigger = match svc.last_health_check {
                    // No passing health check ever recorded
                    None => uptime > watchdog,
                    // Last passing check is older than the watchdog window
                    Some(last_pass) => {
                        let elapsed = Utc::now()
                            .signed_duration_since(last_pass)
                            .to_std()
                            .unwrap_or_else(|_| {
                                warn!(
                                    service = name,
                                    "negative duration in watchdog check — possible clock skew"
                                );
                                Duration::ZERO
                            });
                        elapsed > watchdog
                    }
                };

                if should_trigger {
                    warn!(
                        service = name,
                        uptime_ms = uptime.as_millis() as u64,
                        watchdog_ms = watchdog_ms,
                        "service exceeded watchdog timeout"
                    );
                    timed_out.push(name.to_string());
                }
            }
        }

        timed_out
    }

    /// Kill services that have exceeded their watchdog timeout and
    /// handle crash action (restart or give up).
    ///
    /// Returns the list of services that were killed.
    pub fn enforce_watchdog(&mut self) -> Vec<String> {
        let timed_out = self.check_watchdog();

        for name in &timed_out {
            info!(service = %name, "killing timed-out service");

            if let Some(mut proc) = self.processes.remove(name) {
                let _ = proc.kill();
                let _ = proc.wait();
            }

            if let Some(svc) = self.services.get_mut(name) {
                svc.pid = None;
                svc.state = ServiceState::Failed("watchdog timeout".into());
            }
        }

        timed_out
    }

    /// Run health checks for all running services that have a health
    /// check configured. Updates health history and returns results.
    ///
    /// This is meant to be called periodically by the main event loop.
    /// It does NOT enforce timing — the caller decides when to poll.
    pub fn poll_health(&mut self, tracker: &mut HealthTracker) -> Vec<HealthCheckResult> {
        let mut results = Vec::new();

        let checks: Vec<(String, HealthCheck, Option<u32>, u32)> = self
            .services
            .iter()
            .filter(|(_, svc)| svc.state == ServiceState::Running)
            .filter_map(|(name, svc)| {
                svc.definition
                    .health_check
                    .as_ref()
                    .map(|hc| (name.clone(), hc.clone(), svc.pid, hc.retries))
            })
            .collect();

        for (name, hc, pid, threshold) in &checks {
            let result = health::execute_health_check(name, hc, *pid);

            // Feed into HealthTracker
            let should_restart = tracker.record(name, result.passed, *threshold);
            if should_restart {
                warn!(
                    service = %name,
                    consecutive_failures = tracker.failure_count(name),
                    "health check threshold exceeded — service should be restarted"
                );
            }

            // Update last_health_check timestamp only on passing checks
            if result.passed
                && let Some(svc) = self.services.get_mut(name.as_str())
            {
                svc.last_health_check = Some(Utc::now());
            }

            results.push(result);
        }

        results
    }
}
