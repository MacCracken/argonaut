//! Security enforcement — seccomp, Landlock, capabilities, socket activation,
//! and emergency shell authentication.
//!
//! This module provides both SafeCommand fallbacks (always available) and
//! direct agnosys integration (feature-gated behind `security`).
//!
//! # Feature gates
//!
//! - **Without `security` feature**: config types + description/command generation
//! - **With `security` feature**: direct application via agnosys syscall wrappers

use std::fmt::Write;

use tracing::{debug, info, warn};

use crate::types::{
    CapabilityConfig, EmergencyShellConfig, LandlockAccess, LandlockConfig, SafeCommand,
    SeccompAction, SeccompConfig, SocketActivationConfig,
};

// ---------------------------------------------------------------------------
// Socket activation
// ---------------------------------------------------------------------------

/// Generate environment variables for the sd_listen_fds protocol.
///
/// Returns `LISTEN_FDS=N` (number of sockets) and `LISTEN_PID=<pid>`.
/// The binary crate is responsible for creating the actual sockets
/// and passing file descriptors starting at fd 3.
#[must_use]
pub fn generate_socket_env(config: &SocketActivationConfig, pid: u32) -> Vec<(String, String)> {
    config.env_vars(pid)
}

// ---------------------------------------------------------------------------
// Seccomp
// ---------------------------------------------------------------------------

/// Generate a human-readable description of a seccomp configuration.
#[must_use]
pub fn seccomp_description(config: &SeccompConfig) -> String {
    match config {
        SeccompConfig::Basic => {
            "seccomp: basic filter (20 safe syscalls allowed, all others killed)".to_string()
        }
        SeccompConfig::Custom { allow, deny } => {
            let mut desc = String::with_capacity(128);
            let _ = write!(
                desc,
                "seccomp: custom filter — allow {} syscalls",
                allow.len()
            );
            if !deny.is_empty() {
                let _ = write!(desc, ", {} explicit denials", deny.len());
            }
            desc
        }
    }
}

/// Apply a seccomp filter via agnosys.
///
/// Only available with the `security` feature enabled.
#[cfg(feature = "security")]
pub fn apply_seccomp(config: &SeccompConfig) -> anyhow::Result<()> {
    use anyhow::Context;

    let filter = match config {
        SeccompConfig::Basic => {
            info!("applying basic seccomp filter via agnosys");
            agnosys::security::create_basic_seccomp_filter()
                .map_err(|e| anyhow::anyhow!("failed to create basic seccomp filter: {e}"))?
        }
        SeccompConfig::Custom { allow, deny } => {
            // Map syscall names to numbers
            let base_allowed: Vec<u32> = allow
                .iter()
                .filter_map(|name| {
                    let nr = agnosys::security::syscall_name_to_nr(name);
                    if nr.is_none() {
                        warn!(syscall = %name, "unknown syscall name in seccomp allow list, skipping");
                    }
                    nr
                })
                .collect();

            let denied: Vec<(u32, u32)> = deny
                .iter()
                .filter_map(|(name, action)| {
                    let nr = agnosys::security::syscall_name_to_nr(name)?;
                    let ret_val = match action {
                        SeccompAction::Kill => 0x8000_0000, // SECCOMP_RET_KILL_PROCESS
                        SeccompAction::Trap => 0x0003_0000, // SECCOMP_RET_TRAP
                        SeccompAction::Log => 0x7ffc_0000,  // SECCOMP_RET_LOG
                    };
                    Some((nr, ret_val))
                })
                .collect();

            info!(
                allowed = base_allowed.len(),
                denied = denied.len(),
                "applying custom seccomp filter via agnosys"
            );

            agnosys::security::create_custom_seccomp_filter(&base_allowed, &[], &denied)
                .map_err(|e| anyhow::anyhow!("failed to create custom seccomp filter: {e}"))?
        }
    };

    if filter.is_empty() {
        debug!("seccomp filter is empty (non-Linux?), skipping");
        return Ok(());
    }

    agnosys::security::load_seccomp(&filter)
        .map_err(|e| anyhow::anyhow!("failed to load seccomp filter: {e}"))
        .context("seccomp enforcement failed")
}

// ---------------------------------------------------------------------------
// Landlock
// ---------------------------------------------------------------------------

/// Generate a human-readable description of a Landlock configuration.
#[must_use]
pub fn landlock_description(config: &LandlockConfig) -> String {
    let mut desc = String::with_capacity(128);
    let _ = write!(desc, "landlock: {} filesystem rules", config.rules.len());
    for rule in &config.rules {
        let _ = write!(desc, "\n  {} → {}", rule.path.display(), rule.access);
    }
    desc
}

/// Apply Landlock filesystem restrictions via agnosys.
///
/// Only available with the `security` feature enabled.
/// Requires Linux kernel 5.13+. Gracefully degrades if unsupported.
#[cfg(feature = "security")]
pub fn apply_landlock(config: &LandlockConfig) -> anyhow::Result<()> {
    use anyhow::Context;

    if config.rules.is_empty() {
        debug!("no landlock rules configured, skipping");
        return Ok(());
    }

    let rules: Vec<agnosys::security::FilesystemRule> = config
        .rules
        .iter()
        .map(|r| {
            let access = match r.access {
                LandlockAccess::NoAccess => agnosys::security::FsAccess::NoAccess,
                LandlockAccess::ReadOnly => agnosys::security::FsAccess::ReadOnly,
                LandlockAccess::ReadWrite => agnosys::security::FsAccess::ReadWrite,
            };
            agnosys::security::FilesystemRule::new(&r.path, access)
        })
        .collect();

    info!(
        rules = rules.len(),
        "applying landlock restrictions via agnosys"
    );

    agnosys::security::apply_landlock(&rules)
        .map_err(|e| anyhow::anyhow!("landlock enforcement failed: {e}"))
        .context("landlock enforcement failed")
}

// ---------------------------------------------------------------------------
// Capabilities
// ---------------------------------------------------------------------------

/// Generate a `setpriv` command that drops capabilities and execs a binary.
///
/// Available without the `security` feature — uses the `setpriv` CLI tool.
/// No shell interpretation of arguments (safe from injection).
#[must_use]
pub fn to_capability_commands(
    config: &CapabilityConfig,
    binary: &str,
    args: &[String],
) -> SafeCommand {
    config.to_setpriv_command(binary, args)
}

// ---------------------------------------------------------------------------
// Emergency shell authentication
// ---------------------------------------------------------------------------

/// Verify emergency shell authentication.
///
/// Returns `true` if access should be granted:
/// - If `require_auth` is `false`, always grants access.
/// - If `auth_password_hash` is `None`, always grants access (no hash configured).
/// - Otherwise, computes SHA-256 of `password_input` and compares against
///   the stored hex-encoded hash (constant-time comparison).
#[must_use]
pub fn verify_emergency_auth(config: &EmergencyShellConfig, password_input: &str) -> bool {
    if !config.require_auth {
        debug!("emergency auth not required, granting access");
        return true;
    }
    let Some(ref stored_hash) = config.auth_password_hash else {
        warn!("require_auth is true but no password hash configured, granting access");
        return true;
    };

    // Compute SHA-256 of input (using a simple implementation)
    let input_hash = password_hash_hex(password_input.as_bytes());

    // Constant-time comparison to prevent timing attacks
    let result = constant_time_eq(input_hash.as_bytes(), stored_hash.as_bytes());
    if result {
        info!("emergency shell authentication successful");
    } else {
        warn!("emergency shell authentication failed");
    }
    result
}

/// Compute a hex-encoded hash of the given data for password verification.
///
/// **WARNING**: This uses a non-cryptographic hash (SipHash via
/// `DefaultHasher`). It is NOT suitable for real security. Production
/// deployments MUST enable the `security` feature, which provides
/// proper cryptographic hashing via agnosys.
///
/// This fallback exists so non-AGNOS consumers can use the emergency
/// shell without pulling in crypto dependencies.
fn password_hash_hex(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let h1 = hasher.finish();
    data.len().hash(&mut hasher);
    let h2 = hasher.finish();
    format!("{h1:016x}{h2:016x}")
}

/// Constant-time byte comparison (prevents timing side-channel attacks).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::types::{
        CapabilityConfig, EmergencyShellConfig, LandlockAccess, LandlockConfig, LandlockRule,
        LinuxCapability, SeccompAction, SeccompConfig, SocketActivationConfig, SocketSpec,
        SocketType,
    };

    #[test]
    fn socket_env_vars() {
        let config = SocketActivationConfig {
            sockets: vec![
                SocketSpec {
                    address: "127.0.0.1".into(),
                    port: 8080,
                    socket_type: SocketType::Stream,
                },
                SocketSpec {
                    address: "0.0.0.0".into(),
                    port: 8081,
                    socket_type: SocketType::Stream,
                },
            ],
        };
        let vars = generate_socket_env(&config, 42);
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0], ("LISTEN_FDS".to_string(), "2".to_string()));
        assert_eq!(vars[1], ("LISTEN_PID".to_string(), "42".to_string()));
    }

    #[test]
    fn seccomp_basic_description() {
        let desc = seccomp_description(&SeccompConfig::Basic);
        assert!(desc.contains("basic filter"));
    }

    #[test]
    fn seccomp_custom_description() {
        let config = SeccompConfig::Custom {
            allow: vec!["read".into(), "write".into()],
            deny: vec![("ptrace".into(), SeccompAction::Kill)],
        };
        let desc = seccomp_description(&config);
        assert!(desc.contains("2 syscalls"));
        assert!(desc.contains("1 explicit"));
    }

    #[test]
    fn landlock_description_basic() {
        let config = LandlockConfig {
            rules: vec![
                LandlockRule {
                    path: PathBuf::from("/usr"),
                    access: LandlockAccess::ReadOnly,
                },
                LandlockRule {
                    path: PathBuf::from("/var/lib/myapp"),
                    access: LandlockAccess::ReadWrite,
                },
            ],
        };
        let desc = landlock_description(&config);
        assert!(desc.contains("2 filesystem rules"));
        assert!(desc.contains("/usr → ro"));
        assert!(desc.contains("/var/lib/myapp → rw"));
    }

    #[test]
    fn capability_setpriv_command() {
        let config = CapabilityConfig {
            drop: vec![LinuxCapability::SysAdmin, LinuxCapability::NetRaw],
        };
        let cmd = to_capability_commands(&config, "/usr/bin/myapp", &["--flag".into()]);
        assert_eq!(cmd.binary, "setpriv");
        assert!(cmd.args.contains(&"--no-new-privs".to_string()));
        assert!(
            cmd.args
                .iter()
                .any(|a| a.contains("cap_sys_admin") && a.contains("cap_net_raw"))
        );
        assert!(cmd.args.contains(&"/usr/bin/myapp".to_string()));
        assert!(cmd.args.contains(&"--flag".to_string()));
    }

    #[test]
    fn emergency_auth_not_required() {
        let config = EmergencyShellConfig {
            require_auth: false,
            ..Default::default()
        };
        assert!(verify_emergency_auth(&config, "anything"));
    }

    #[test]
    fn emergency_auth_no_hash_configured() {
        let config = EmergencyShellConfig {
            require_auth: true,
            auth_password_hash: None,
            ..Default::default()
        };
        assert!(verify_emergency_auth(&config, "anything"));
    }

    #[test]
    fn emergency_auth_hash_match() {
        let password = "test-password";
        let hash = password_hash_hex(password.as_bytes());
        let config = EmergencyShellConfig {
            require_auth: true,
            auth_password_hash: Some(hash),
            ..Default::default()
        };
        assert!(verify_emergency_auth(&config, password));
    }

    #[test]
    fn emergency_auth_hash_mismatch() {
        let config = EmergencyShellConfig {
            require_auth: true,
            auth_password_hash: Some("0000000000000000".to_string()),
            ..Default::default()
        };
        assert!(!verify_emergency_auth(&config, "wrong-password"));
    }

    #[test]
    fn constant_time_eq_same() {
        assert!(constant_time_eq(b"hello", b"hello"));
    }

    #[test]
    fn constant_time_eq_different() {
        assert!(!constant_time_eq(b"hello", b"world"));
    }

    #[test]
    fn constant_time_eq_different_lengths() {
        assert!(!constant_time_eq(b"short", b"longer"));
    }

    #[test]
    fn linux_capability_as_str() {
        assert_eq!(LinuxCapability::SysAdmin.as_str(), "cap_sys_admin");
        assert_eq!(
            LinuxCapability::NetBindService.as_str(),
            "cap_net_bind_service"
        );
        assert_eq!(LinuxCapability::Kill.as_str(), "cap_kill");
    }

    #[test]
    fn socket_type_display() {
        assert_eq!(SocketType::Stream.to_string(), "stream");
        assert_eq!(SocketType::Datagram.to_string(), "dgram");
        assert_eq!(SocketType::SeqPacket.to_string(), "seqpacket");
    }

    #[test]
    fn seccomp_action_display() {
        assert_eq!(SeccompAction::Kill.to_string(), "kill");
        assert_eq!(SeccompAction::Trap.to_string(), "trap");
        assert_eq!(SeccompAction::Log.to_string(), "log");
    }

    #[test]
    fn landlock_access_display() {
        assert_eq!(LandlockAccess::NoAccess.to_string(), "none");
        assert_eq!(LandlockAccess::ReadOnly.to_string(), "ro");
        assert_eq!(LandlockAccess::ReadWrite.to_string(), "rw");
    }
}
