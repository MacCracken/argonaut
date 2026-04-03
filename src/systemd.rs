//! systemd unit file generation from argonaut service definitions.
//!
//! Generates `.service` unit files for hybrid installs where argonaut
//! services may also be managed by systemd. The generated units use
//! `Type=simple` with `NotifyAccess=main` for sd_notify integration.
//!
//! # Example
//!
//! ```rust
//! use argonaut::systemd::generate_unit;
//! # use argonaut::types::*;
//! # use std::collections::HashMap;
//! # use std::path::PathBuf;
//! # let svc = ServiceDefinition {
//! #     name: "example".into(),
//! #     description: "Example service".into(),
//! #     binary_path: PathBuf::from("/usr/bin/example"),
//! #     args: vec![],
//! #     environment: HashMap::new(),
//! #     depends_on: vec![],
//! #     required_for_modes: vec![],
//! #     restart_policy: RestartPolicy::OnFailure,
//! #     restart_config: RestartConfig::default(),
//! #     health_check: None,
//! #     ready_check: None,
//! #     enabled: true,
//! # };
//! let unit = generate_unit(&svc);
//! assert!(unit.contains("[Service]"));
//! ```

use std::fmt::Write;

use crate::types::{BootMode, RestartPolicy, ServiceDefinition};

/// Generate a systemd `.service` unit file from a [`ServiceDefinition`].
///
/// Returns the full unit file content as a string. The caller is
/// responsible for writing it to disk (e.g. `/etc/systemd/system/`).
#[must_use]
pub fn generate_unit(svc: &ServiceDefinition) -> String {
    let mut out = String::with_capacity(512);

    // [Unit] section
    let _ = writeln!(out, "[Unit]");
    let _ = writeln!(out, "Description={}", svc.description);

    if !svc.depends_on.is_empty() {
        let deps: Vec<String> = svc
            .depends_on
            .iter()
            .map(|d| format!("{d}.service"))
            .collect();
        let dep_str = deps.join(" ");
        let _ = writeln!(out, "After={dep_str}");
        let _ = writeln!(out, "Requires={dep_str}");
    }

    let _ = writeln!(out);

    // [Service] section
    let _ = writeln!(out, "[Service]");
    let _ = writeln!(out, "Type=notify");

    // ExecStart
    let exec_start = if svc.args.is_empty() {
        svc.binary_path.display().to_string()
    } else {
        let args = svc
            .args
            .iter()
            .map(|a| escape_systemd_value(a))
            .collect::<Vec<_>>()
            .join(" ");
        format!("{} {}", svc.binary_path.display(), args)
    };
    let _ = writeln!(out, "ExecStart={exec_start}");

    // Environment
    for (key, value) in &svc.environment {
        let _ = writeln!(out, "Environment=\"{key}={value}\"");
    }

    // Restart policy
    let restart = match svc.restart_policy {
        RestartPolicy::Always => "always",
        RestartPolicy::OnFailure => "on-failure",
        RestartPolicy::Never => "no",
    };
    let _ = writeln!(out, "Restart={restart}");

    // Restart delay (convert ms to seconds, minimum 1s)
    let restart_sec = (svc.restart_config.base_delay_ms / 1000).max(1);
    let _ = writeln!(out, "RestartSec={restart_sec}");

    // Restart limit
    if svc.restart_config.max_restarts > 0 {
        let _ = writeln!(out, "StartLimitBurst={}", svc.restart_config.max_restarts);
    }

    // Watchdog from health check
    if let Some(ref hc) = svc.health_check {
        let watchdog_sec = (hc.interval_ms * u64::from(hc.retries)) / 1000;
        if watchdog_sec > 0 {
            let _ = writeln!(out, "WatchdogSec={watchdog_sec}");
        }
    }

    let _ = writeln!(out, "NotifyAccess=main");
    let _ = writeln!(out);

    // [Install] section
    let _ = writeln!(out, "[Install]");
    let has_graphical = svc.required_for_modes.contains(&BootMode::Desktop);
    if has_graphical {
        let _ = writeln!(out, "WantedBy=graphical.target");
    } else {
        let _ = writeln!(out, "WantedBy=multi-user.target");
    }

    out
}

/// Generate the systemd unit filename for a service.
///
/// Returns `"{name}.service"`.
#[must_use]
pub fn generate_unit_filename(svc: &ServiceDefinition) -> String {
    format!("{}.service", svc.name)
}

/// Escape a value for systemd unit files.
///
/// Wraps in quotes if the value contains spaces or special characters.
#[must_use]
fn escape_systemd_value(value: &str) -> String {
    if value.contains(' ') || value.contains('"') || value.contains('\\') {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::types::{
        BootMode, HealthCheck, HealthCheckType, RestartConfig, RestartPolicy, ServiceDefinition,
    };

    use super::*;

    fn test_service(name: &str) -> ServiceDefinition {
        ServiceDefinition {
            name: name.into(),
            description: format!("Test service {name}"),
            binary_path: PathBuf::from(format!("/usr/bin/{name}")),
            args: vec![],
            environment: HashMap::new(),
            depends_on: vec![],
            required_for_modes: vec![BootMode::Server],
            restart_policy: RestartPolicy::OnFailure,
            restart_config: RestartConfig::default(),
            health_check: None,
            ready_check: None,
            enabled: true,
        }
    }

    #[test]
    fn generate_unit_simple_service() {
        let svc = test_service("myapp");
        let unit = generate_unit(&svc);
        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("Description=Test service myapp"));
        assert!(unit.contains("[Service]"));
        assert!(unit.contains("Type=notify"));
        assert!(unit.contains("ExecStart=/usr/bin/myapp"));
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("NotifyAccess=main"));
        assert!(unit.contains("[Install]"));
        assert!(unit.contains("WantedBy=multi-user.target"));
    }

    #[test]
    fn generate_unit_with_dependencies() {
        let mut svc = test_service("backend");
        svc.depends_on = vec!["postgres".into(), "redis".into()];
        let unit = generate_unit(&svc);
        assert!(unit.contains("After=postgres.service redis.service"));
        assert!(unit.contains("Requires=postgres.service redis.service"));
    }

    #[test]
    fn generate_unit_with_environment() {
        let mut svc = test_service("envapp");
        svc.environment
            .insert("PGDATA".into(), "/var/lib/postgres".into());
        let unit = generate_unit(&svc);
        assert!(unit.contains("Environment=\"PGDATA=/var/lib/postgres\""));
    }

    #[test]
    fn generate_unit_restart_always() {
        let mut svc = test_service("always");
        svc.restart_policy = RestartPolicy::Always;
        let unit = generate_unit(&svc);
        assert!(unit.contains("Restart=always"));
    }

    #[test]
    fn generate_unit_restart_never() {
        let mut svc = test_service("never");
        svc.restart_policy = RestartPolicy::Never;
        let unit = generate_unit(&svc);
        assert!(unit.contains("Restart=no"));
    }

    #[test]
    fn generate_unit_with_health_check_watchdog() {
        let mut svc = test_service("watched");
        svc.health_check = Some(HealthCheck {
            check_type: HealthCheckType::HttpGet("http://127.0.0.1:8080/health".into()),
            interval_ms: 10_000,
            timeout_ms: 2000,
            retries: 3,
        });
        let unit = generate_unit(&svc);
        assert!(unit.contains("WatchdogSec=30"));
    }

    #[test]
    fn generate_unit_with_args() {
        let mut svc = test_service("withargs");
        svc.args = vec!["--port".into(), "8080".into()];
        let unit = generate_unit(&svc);
        assert!(unit.contains("ExecStart=/usr/bin/withargs --port 8080"));
    }

    #[test]
    fn generate_unit_desktop_target() {
        let mut svc = test_service("guiapp");
        svc.required_for_modes = vec![BootMode::Desktop];
        let unit = generate_unit(&svc);
        assert!(unit.contains("WantedBy=graphical.target"));
    }

    #[test]
    fn generate_unit_escapes_args_with_spaces() {
        let mut svc = test_service("spacey");
        svc.args = vec!["--config".into(), "/path/with spaces/config.toml".into()];
        let unit = generate_unit(&svc);
        assert!(unit.contains("\"/path/with spaces/config.toml\""));
    }

    #[test]
    fn generate_unit_restart_limit() {
        let mut svc = test_service("limited");
        svc.restart_config.max_restarts = 5;
        let unit = generate_unit(&svc);
        assert!(unit.contains("StartLimitBurst=5"));
    }

    #[test]
    fn generate_unit_filename_format() {
        let svc = test_service("myapp");
        assert_eq!(generate_unit_filename(&svc), "myapp.service");
    }
}
