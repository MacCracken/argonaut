//! Edge boot helpers — read-only rootfs and dm-verity verification.
//!
//! These free functions are used during Edge boot to lock down the root
//! partition and verify its integrity before mounting.

use std::time::Instant;

use tracing::{debug, error, info, warn};

use super::process::run_command_sequence;
use super::types::{EdgeBootConfig, SafeCommand};

/// Generate mount commands to configure a read-only root filesystem with
/// writable tmpfs overlays for directories that require writes at runtime.
///
/// This is used during Edge boot to lock down the root partition, ensuring
/// integrity while still allowing ephemeral writes to `/tmp`, `/var/run`,
/// `/var/log`, and `/var/tmp`.
#[must_use]
pub fn configure_readonly_rootfs() -> Vec<SafeCommand> {
    info!(
        overlays = "/tmp, /var/run, /var/log, /var/tmp",
        "configuring read-only rootfs with tmpfs overlays"
    );
    vec![
        SafeCommand {
            binary: "mount".to_string(),
            args: vec!["-o".to_string(), "remount,ro".to_string(), "/".to_string()],
        },
        SafeCommand {
            binary: "mount".to_string(),
            args: vec![
                "-t".to_string(),
                "tmpfs".to_string(),
                "tmpfs".to_string(),
                "/tmp".to_string(),
                "-o".to_string(),
                "size=64M,noexec,nosuid".to_string(),
            ],
        },
        SafeCommand {
            binary: "mount".to_string(),
            args: vec![
                "-t".to_string(),
                "tmpfs".to_string(),
                "tmpfs".to_string(),
                "/var/run".to_string(),
                "-o".to_string(),
                "size=16M,nosuid".to_string(),
            ],
        },
        SafeCommand {
            binary: "mount".to_string(),
            args: vec![
                "-t".to_string(),
                "tmpfs".to_string(),
                "tmpfs".to_string(),
                "/var/log".to_string(),
                "-o".to_string(),
                "size=32M,nosuid".to_string(),
            ],
        },
        SafeCommand {
            binary: "mount".to_string(),
            args: vec![
                "-t".to_string(),
                "tmpfs".to_string(),
                "tmpfs".to_string(),
                "/var/tmp".to_string(),
                "-o".to_string(),
                "size=16M,noexec,nosuid".to_string(),
            ],
        },
    ]
}

/// Validate a device path to prevent injection.
fn validate_device_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        warn!("device path validation failed: empty path");
        return Err("device path cannot be empty".to_string());
    }
    if path.contains("..") {
        warn!(
            path = &path[..path.len().min(20)],
            "device path validation failed: path traversal attempt"
        );
        return Err("device path must not contain '..' components".to_string());
    }
    if !path.starts_with("/dev/") {
        warn!(
            path = &path[..path.len().min(20)],
            "device path validation failed: missing /dev/ prefix"
        );
        return Err(format!(
            "device path must start with /dev/, got: {}",
            &path[..path.len().min(20)]
        ));
    }
    if !path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'))
    {
        warn!(
            path = &path[..path.len().min(20)],
            "device path validation failed: invalid characters"
        );
        return Err("device path contains invalid characters".to_string());
    }
    Ok(())
}

/// Generate dm-verity verification and activation commands for a root device.
///
/// Validates the integrity of a root filesystem partition using dm-verity by
/// checking the provided SHA-256 root hash against the hash device, then
/// activating a read-only verified mapping at `/dev/mapper/verified-root`.
///
/// # Errors
///
/// Returns an error if any parameter is empty or the root hash is not exactly
/// 64 hex characters (SHA-256 digest length).
pub fn verify_rootfs_integrity(
    root_device: &str,
    hash_device: &str,
    root_hash: &str,
) -> Result<Vec<SafeCommand>, String> {
    if root_device.is_empty() || hash_device.is_empty() || root_hash.is_empty() {
        warn!("dm-verity verification failed: empty parameters");
        return Err("dm-verity parameters cannot be empty".to_string());
    }
    if root_hash.len() != 64 {
        warn!(
            hash_len = root_hash.len(),
            "dm-verity verification failed: invalid hash length"
        );
        return Err("Root hash must be 64 hex characters (SHA-256)".to_string());
    }
    if !root_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        warn!("dm-verity verification failed: non-hex characters in root hash");
        return Err("Root hash must contain only hex characters (0-9, a-f)".to_string());
    }
    validate_device_path(root_device)?;
    validate_device_path(hash_device)?;

    info!(
        root_device = root_device,
        hash_device = hash_device,
        root_hash_prefix = &root_hash[..8],
        "dm-verity verification commands generated"
    );
    Ok(vec![
        SafeCommand {
            binary: "veritysetup".to_string(),
            args: vec![
                "verify".to_string(),
                root_device.to_string(),
                hash_device.to_string(),
                root_hash.to_string(),
            ],
        },
        SafeCommand {
            binary: "veritysetup".to_string(),
            args: vec![
                "open".to_string(),
                root_device.to_string(),
                "verified-root".to_string(),
                hash_device.to_string(),
                root_hash.to_string(),
            ],
        },
        SafeCommand {
            binary: "mount".to_string(),
            args: vec![
                "-o".to_string(),
                "ro".to_string(),
                "/dev/mapper/verified-root".to_string(),
                "/mnt/root".to_string(),
            ],
        },
    ])
}

/// Generate LUKS unlock commands for the data partition.
///
/// Uses `cryptsetup luksOpen` to unlock the encrypted data partition.
/// The key slot is expected to be unlocked via TPM or kernel keyring
/// (no passphrase prompt in edge mode).
pub fn unlock_luks(encrypted_device: &str, mapped_name: &str) -> Result<Vec<SafeCommand>, String> {
    validate_device_path(encrypted_device)?;

    if mapped_name.is_empty() {
        return Err("mapped name cannot be empty".to_string());
    }
    if !mapped_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err("mapped name contains invalid characters".to_string());
    }

    info!(
        device = encrypted_device,
        mapped_name = mapped_name,
        "generating LUKS unlock commands"
    );

    Ok(vec![
        // Try TPM2-backed unlock first (systemd-cryptenroll compatible)
        SafeCommand {
            binary: "cryptsetup".to_string(),
            args: vec![
                "open".to_string(),
                "--type".to_string(),
                "luks2".to_string(),
                "--token-only".to_string(),
                encrypted_device.to_string(),
                mapped_name.to_string(),
            ],
        },
    ])
}

/// Generate LUKS close commands for shutdown.
#[must_use]
pub fn close_luks(mapped_name: &str) -> Vec<SafeCommand> {
    vec![SafeCommand {
        binary: "cryptsetup".to_string(),
        args: vec!["close".to_string(), mapped_name.to_string()],
    }]
}

// ---------------------------------------------------------------------------
// Edge boot execution
// ---------------------------------------------------------------------------

/// Result of executing the edge boot sequence.
#[derive(Debug)]
pub struct EdgeBootResult {
    /// Whether rootfs was locked down successfully.
    pub rootfs_locked: bool,
    /// Whether dm-verity verification passed.
    pub verity_verified: bool,
    /// Whether LUKS was unlocked successfully.
    pub luks_unlocked: bool,
    /// Total boot time in milliseconds.
    pub boot_time_ms: u64,
    /// Whether boot completed within the time budget.
    pub within_budget: bool,
    /// Any errors encountered (boot continues best-effort).
    pub errors: Vec<String>,
}

/// Execute the full edge boot sequence:
/// 1. Lock down rootfs (read-only + tmpfs overlays)
/// 2. Verify rootfs integrity via dm-verity (if configured)
/// 3. Unlock LUKS data partition (if configured)
/// 4. Check boot time budget
///
/// This is called during the `VerifyRootfs` boot stage for Edge mode.
pub fn execute_edge_boot(
    config: &EdgeBootConfig,
    root_device: &str,
    hash_device: &str,
    root_hash: &str,
    luks_device: &str,
) -> EdgeBootResult {
    let start = Instant::now();
    let mut result = EdgeBootResult {
        rootfs_locked: false,
        verity_verified: false,
        luks_unlocked: false,
        boot_time_ms: 0,
        within_budget: true,
        errors: Vec::new(),
    };

    // Step 1: Read-only rootfs with tmpfs overlays
    if config.readonly_rootfs {
        info!("edge boot: locking down rootfs");
        let cmds = configure_readonly_rootfs();
        match run_command_sequence(&cmds) {
            Ok(()) => {
                result.rootfs_locked = true;
                debug!("edge boot: rootfs locked successfully");
            }
            Err(e) => {
                error!(error = %e, "edge boot: rootfs lockdown failed");
                result.errors.push(format!("rootfs lockdown: {e}"));
            }
        }
    } else {
        debug!("edge boot: readonly_rootfs disabled, skipping");
        result.rootfs_locked = true;
    }

    // Step 2: dm-verity verification
    if !root_device.is_empty() && !hash_device.is_empty() && !root_hash.is_empty() {
        info!("edge boot: verifying rootfs integrity via dm-verity");
        match verify_rootfs_integrity(root_device, hash_device, root_hash) {
            Ok(cmds) => match run_command_sequence(&cmds) {
                Ok(()) => {
                    result.verity_verified = true;
                    debug!("edge boot: dm-verity verification passed");
                }
                Err(e) => {
                    error!(error = %e, "edge boot: dm-verity commands failed");
                    result.errors.push(format!("dm-verity exec: {e}"));
                }
            },
            Err(e) => {
                error!(error = %e, "edge boot: dm-verity validation failed");
                result.errors.push(format!("dm-verity validation: {e}"));
            }
        }
    } else {
        debug!("edge boot: dm-verity params not provided, skipping");
    }

    // Step 3: LUKS unlock
    if config.luks_enabled && !luks_device.is_empty() {
        info!("edge boot: unlocking LUKS data partition");
        match unlock_luks(luks_device, "agnos-data") {
            Ok(cmds) => match run_command_sequence(&cmds) {
                Ok(()) => {
                    result.luks_unlocked = true;
                    debug!("edge boot: LUKS unlocked successfully");
                }
                Err(e) => {
                    error!(error = %e, "edge boot: LUKS unlock failed");
                    result.errors.push(format!("luks unlock: {e}"));
                }
            },
            Err(e) => {
                error!(error = %e, "edge boot: LUKS validation failed");
                result.errors.push(format!("luks validation: {e}"));
            }
        }
    } else {
        debug!("edge boot: LUKS disabled or no device, skipping");
        result.luks_unlocked = true;
    }

    // Step 4: Boot time budget
    result.boot_time_ms = start.elapsed().as_millis() as u64;
    result.within_budget = result.boot_time_ms <= config.max_boot_time_ms;

    if result.within_budget {
        info!(
            boot_time_ms = result.boot_time_ms,
            budget_ms = config.max_boot_time_ms,
            "edge boot: completed within time budget"
        );
    } else {
        warn!(
            boot_time_ms = result.boot_time_ms,
            budget_ms = config.max_boot_time_ms,
            "edge boot: EXCEEDED time budget"
        );
    }

    info!(
        rootfs_locked = result.rootfs_locked,
        verity_verified = result.verity_verified,
        luks_unlocked = result.luks_unlocked,
        boot_time_ms = result.boot_time_ms,
        errors = result.errors.len(),
        "edge boot sequence complete"
    );

    result
}

/// Validate that the system meets the minimal edge boot profile.
///
/// Checks:
/// - Total memory usage is under the given limit
/// - Boot time is within budget
///
/// Returns a list of violations (empty = pass).
#[must_use]
pub fn validate_edge_profile(boot_result: &EdgeBootResult, max_memory_mb: u64) -> Vec<String> {
    let mut violations = Vec::new();

    if !boot_result.within_budget {
        violations.push(format!(
            "boot time {}ms exceeds budget",
            boot_result.boot_time_ms
        ));
    }

    if !boot_result.rootfs_locked {
        violations.push("rootfs not locked to read-only".to_string());
    }

    if !boot_result.errors.is_empty() {
        violations.push(format!("{} errors during boot", boot_result.errors.len()));
    }

    // Memory check via /proc/meminfo (best-effort)
    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo")
        && let Some(line) = meminfo.lines().find(|l| l.starts_with("MemTotal:"))
    {
        let kb: u64 = line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let mb = kb / 1024;
        if mb > max_memory_mb {
            violations.push(format!(
                "total memory {mb}MB exceeds limit {max_memory_mb}MB"
            ));
        }
        debug!(
            total_memory_mb = mb,
            limit_mb = max_memory_mb,
            "edge profile memory check"
        );
    }

    if violations.is_empty() {
        info!("edge profile validation passed");
    } else {
        warn!(violations = ?violations, "edge profile validation failed");
    }

    violations
}

// ---------------------------------------------------------------------------
// Fleet auto-registration
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

/// Configuration for fleet auto-registration on first boot.
///
/// On first boot, an edge device registers itself with the fleet
/// management server (daimon) to receive its configuration, identity,
/// and update channel. The actual HTTP call is the consumer's
/// responsibility — argonaut provides the registration payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetRegistration {
    /// Machine ID (typically from `/etc/machine-id`).
    pub machine_id: String,
    /// Hostname of the edge device.
    pub hostname: String,
    /// Boot mode the device started in.
    pub boot_mode: String,
    /// Whether dm-verity is active.
    pub verity_active: bool,
    /// Whether LUKS is active.
    pub luks_active: bool,
    /// Firmware / kernel version string.
    pub kernel_version: String,
    /// Total memory in MB.
    pub total_memory_mb: u64,
}

impl FleetRegistration {
    /// Build a registration payload from the current system state.
    ///
    /// Reads machine-id, hostname, and kernel version from the filesystem.
    /// Returns `None` for fields that can't be read.
    #[must_use]
    pub fn from_system(boot_result: &EdgeBootResult) -> Self {
        let machine_id = std::fs::read_to_string("/etc/machine-id")
            .unwrap_or_default()
            .trim()
            .to_string();

        let hostname = std::fs::read_to_string("/etc/hostname")
            .unwrap_or_default()
            .trim()
            .to_string();

        let kernel_version = std::fs::read_to_string("/proc/version")
            .unwrap_or_default()
            .split_whitespace()
            .nth(2)
            .unwrap_or("unknown")
            .to_string();

        let total_memory_mb = std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|m| {
                m.lines()
                    .find(|l| l.starts_with("MemTotal:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|kb| kb / 1024)
            })
            .unwrap_or(0);

        info!(
            machine_id = %machine_id,
            hostname = %hostname,
            "built fleet registration payload"
        );

        Self {
            machine_id,
            hostname,
            boot_mode: "edge".to_string(),
            verity_active: boot_result.verity_verified,
            luks_active: boot_result.luks_unlocked,
            kernel_version,
            total_memory_mb,
        }
    }

    /// Serialize the registration payload to JSON for sending to the
    /// fleet management server.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
