//! Runlevel management — shutdown planning and runlevel switching.

use std::time::Duration;

use anyhow::Result;
use tracing::{debug, error, info, warn};

use super::process::run_command;
use super::types::{
    BootMode, Runlevel, RunlevelSwitchPlan, SafeCommand, ServiceState, ServiceTarget,
    ShutdownAction, ShutdownPlan, ShutdownStep, ShutdownStepStatus, ShutdownType,
};

impl super::ArgonautInit {
    /// Build a shutdown plan for the given shutdown type.
    /// Services are stopped in reverse dependency order.
    pub fn plan_shutdown(&self, shutdown_type: ShutdownType) -> Result<ShutdownPlan> {
        let service_order = self.shutdown_order()?;
        let mut steps = Vec::new();

        // Step 1: Wall message
        let wall_msg = format!(
            "AGNOS system {} in {} seconds",
            shutdown_type,
            self.config.shutdown_timeout_ms / 1000
        );
        steps.push(ShutdownStep {
            description: "Broadcast shutdown warning".into(),
            action: ShutdownAction::WallMessage(wall_msg.clone()),
            timeout_ms: 1000,
            status: ShutdownStepStatus::Pending,
        });

        // Step 2: Notify agents to save state
        steps.push(ShutdownStep {
            description: "Notify agents to save state".into(),
            action: ShutdownAction::NotifyAgents,
            timeout_ms: 5000,
            status: ShutdownStepStatus::Pending,
        });

        // Step 3: Stop services in reverse dependency order
        for svc_name in &service_order {
            if let Some(svc) = self.services.get(svc_name)
                && (svc.state == ServiceState::Running || svc.state == ServiceState::Starting)
            {
                steps.push(ShutdownStep {
                    description: format!("Stop service: {}", svc_name),
                    action: ShutdownAction::StopService {
                        name: svc_name.clone(),
                        signal: 15, // SIGTERM
                    },
                    timeout_ms: 5000,
                    status: ShutdownStepStatus::Pending,
                });
            }
        }

        // Step 4: Sync filesystems
        steps.push(ShutdownStep {
            description: "Sync filesystem buffers".into(),
            action: ShutdownAction::SyncFilesystems,
            timeout_ms: 3000,
            status: ShutdownStepStatus::Pending,
        });

        // Step 5: Unmount
        steps.push(ShutdownStep {
            description: "Unmount filesystems".into(),
            action: ShutdownAction::UnmountFilesystems,
            timeout_ms: 5000,
            status: ShutdownStepStatus::Pending,
        });

        // Step 6: Deactivate swap
        steps.push(ShutdownStep {
            description: "Deactivate swap".into(),
            action: ShutdownAction::SwapOff,
            timeout_ms: 2000,
            status: ShutdownStepStatus::Pending,
        });

        // Step 7: Close LUKS volumes
        steps.push(ShutdownStep {
            description: "Close encrypted volumes".into(),
            action: ShutdownAction::CloseLuks,
            timeout_ms: 3000,
            status: ShutdownStepStatus::Pending,
        });

        // Step 8: Final kernel action
        steps.push(ShutdownStep {
            description: format!("Execute {}", shutdown_type),
            action: ShutdownAction::KernelAction(shutdown_type),
            timeout_ms: 1000,
            status: ShutdownStepStatus::Pending,
        });

        let plan = ShutdownPlan {
            shutdown_type,
            steps,
            timeout_ms: self.config.shutdown_timeout_ms,
            wall_message: Some(wall_msg),
        };
        info!(
            shutdown_type = %shutdown_type,
            step_count = plan.steps.len(),
            timeout_ms = plan.timeout_ms,
            "shutdown plan created"
        );
        Ok(plan)
    }

    /// Compute which services need to start/stop when switching runlevels.
    #[must_use]
    pub fn plan_runlevel_switch(
        &self,
        target: Runlevel,
        targets: &[ServiceTarget],
    ) -> RunlevelSwitchPlan {
        let mut services_to_start = Vec::new();
        let mut services_to_stop = Vec::new();

        // Determine which services should be running at the target runlevel
        let mut desired: std::collections::HashSet<String> = std::collections::HashSet::new();
        for tgt in targets {
            if tgt.is_active_in(target) {
                for svc in &tgt.requires {
                    desired.insert(svc.clone());
                }
                for svc in &tgt.wants {
                    desired.insert(svc.clone());
                }
            }
        }

        // Services that need to start (desired but not running)
        for svc_name in &desired {
            if let Some(svc) = self.services.get(svc_name) {
                if svc.state != ServiceState::Running && svc.state != ServiceState::Starting {
                    services_to_start.push(svc_name.clone());
                }
            } else {
                // Service not registered, still mark it as needed
                services_to_start.push(svc_name.clone());
            }
        }

        // Services that need to stop (running but not desired)
        for (name, svc) in &self.services {
            if (svc.state == ServiceState::Running || svc.state == ServiceState::Starting)
                && !desired.contains(name)
            {
                services_to_stop.push(name.clone());
            }
        }

        // Emergency/rescue: stop everything except basic shell
        if target == Runlevel::Emergency {
            services_to_stop.clear();
            services_to_start.clear();
            for (name, svc) in &self.services {
                if svc.state == ServiceState::Running || svc.state == ServiceState::Starting {
                    services_to_stop.push(name.clone());
                }
            }
        }

        let plan = RunlevelSwitchPlan {
            from: Runlevel::from_boot_mode(self.config.boot_mode),
            to: target,
            services_to_start,
            services_to_stop,
            drop_to_shell: target == Runlevel::Emergency || target == Runlevel::Rescue,
        };
        info!(
            from = %plan.from,
            to = %plan.to,
            starting = plan.services_to_start.len(),
            stopping = plan.services_to_stop.len(),
            drop_to_shell = plan.drop_to_shell,
            "runlevel switch plan created"
        );
        plan
    }

    /// Determine which boot mode to use for a given runlevel.
    #[must_use]
    pub fn runlevel_boot_mode(runlevel: Runlevel) -> Option<BootMode> {
        let mode = runlevel.to_boot_mode();
        debug!(runlevel = %runlevel, boot_mode = ?mode, "resolved runlevel to boot mode");
        mode
    }

    /// Execute a shutdown plan. Walks each step in order, updating
    /// step status as it goes. Service stops use SIGTERM → wait → SIGKILL.
    ///
    /// Returns the completed plan with updated step statuses.
    pub fn execute_shutdown(&mut self, mut plan: ShutdownPlan) -> ShutdownPlan {
        info!(
            shutdown_type = %plan.shutdown_type,
            steps = plan.steps.len(),
            "executing shutdown plan"
        );

        for step in &mut plan.steps {
            step.status = ShutdownStepStatus::InProgress;
            let timeout = Duration::from_millis(step.timeout_ms);

            let result = match &step.action {
                ShutdownAction::WallMessage(msg) => {
                    info!(message = %msg, "shutdown wall message");
                    // In a real system this would write to /dev/console or
                    // use the `wall` command. For now, just log it.
                    Ok(())
                }
                ShutdownAction::NotifyAgents => {
                    info!("notifying agents to save state");
                    // Agent notification is handled by the consumer (daimon).
                    Ok(())
                }
                ShutdownAction::StopService { name, signal: _ } => {
                    info!(service = %name, "stopping service for shutdown");
                    match self.stop_service(name, timeout) {
                        Ok(code) => {
                            debug!(service = %name, exit_code = code, "service stopped");
                            Ok(())
                        }
                        Err(e) => {
                            warn!(service = %name, error = %e, "failed to stop service");
                            Err(e.to_string())
                        }
                    }
                }
                ShutdownAction::ForceKillService { name } => {
                    warn!(service = %name, "force killing service");
                    if let Some(mut proc) = self.processes.remove(name) {
                        let _ = proc.kill();
                        let _ = proc.wait();
                    }
                    if let Some(svc) = self.services.get_mut(name) {
                        svc.pid = None;
                        svc.state = ServiceState::Stopped;
                    }
                    Ok(())
                }
                ShutdownAction::SyncFilesystems => {
                    info!("syncing filesystem buffers");
                    let cmd = SafeCommand {
                        binary: "sync".to_string(),
                        args: vec![],
                    };
                    match run_command(&cmd) {
                        Ok(0) => Ok(()),
                        Ok(code) => Err(format!("sync exited with code {code}")),
                        Err(e) => Err(e.to_string()),
                    }
                }
                ShutdownAction::UnmountFilesystems => {
                    info!("unmounting filesystems");
                    // In a real init system, this would iterate mount points.
                    // For now, log only — actual unmount requires root.
                    Ok(())
                }
                ShutdownAction::SwapOff => {
                    info!("deactivating swap");
                    let cmd = SafeCommand {
                        binary: "swapoff".to_string(),
                        args: vec!["-a".to_string()],
                    };
                    match run_command(&cmd) {
                        Ok(0) => Ok(()),
                        Ok(code) => {
                            warn!(exit_code = code, "swapoff returned non-zero");
                            Ok(()) // non-fatal
                        }
                        Err(e) => {
                            warn!(error = %e, "swapoff failed");
                            Ok(()) // non-fatal
                        }
                    }
                }
                ShutdownAction::CloseLuks => {
                    info!("closing LUKS volumes");
                    // LUKS close requires cryptsetup and root.
                    // Log only for now.
                    Ok(())
                }
                ShutdownAction::KernelAction(action) => {
                    info!(action = %action, "executing final kernel action");
                    // In a real init system, this would call reboot(2).
                    // We log it; the actual syscall is the consumer's job.
                    Ok(())
                }
            };

            match result {
                Ok(()) => {
                    step.status = ShutdownStepStatus::Complete;
                    debug!(description = %step.description, "shutdown step complete");
                }
                Err(msg) => {
                    error!(description = %step.description, error = %msg, "shutdown step failed");
                    step.status = ShutdownStepStatus::Failed(msg);
                    // Continue with remaining steps — shutdown should be best-effort
                }
            }
        }

        info!(
            shutdown_type = %plan.shutdown_type,
            "shutdown plan execution complete"
        );
        plan
    }
}
