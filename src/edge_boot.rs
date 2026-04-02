//! Edge boot helpers — read-only rootfs and dm-verity verification.
//!
//! These free functions are used during Edge boot to lock down the root
//! partition and verify its integrity before mounting.

use tracing::{info, warn};

use super::types::SafeCommand;

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
