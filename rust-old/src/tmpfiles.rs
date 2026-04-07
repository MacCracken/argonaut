//! Boot-time filesystem setup — tmpfiles.d equivalent.
//!
//! Generates [`SafeCommand`]s for creating directories, symlinks, and
//! device nodes at system boot (before services start). Executed during
//! the `StartSecurity` boot stage.
//!
//! # Example
//!
//! ```rust
//! use argonaut::tmpfiles::generate_tmpfile_commands;
//! use argonaut::types::TmpfileEntry;
//! use std::path::PathBuf;
//!
//! let entries = vec![
//!     TmpfileEntry::Directory {
//!         path: PathBuf::from("/run/myapp"),
//!         mode: 0o755,
//!         uid: None,
//!         gid: None,
//!     },
//! ];
//! let cmds = generate_tmpfile_commands(&entries);
//! assert!(!cmds.is_empty());
//! ```

use std::path::Path;

use anyhow::{Result, bail};
use tracing::{debug, warn};

use crate::types::{SafeCommand, TmpfileEntry};

/// Generate [`SafeCommand`]s to create the given filesystem entries.
///
/// Each entry produces one or more commands:
/// - **Directory**: `mkdir -p -m <mode> <path>` + optional `chown <uid>:<gid> <path>`
/// - **Symlink**: `ln -sf <target> <path>`
/// - **Device**: `mknod <path> <type> <major> <minor>` + `chmod <mode> <path>`
#[must_use]
pub fn generate_tmpfile_commands(entries: &[TmpfileEntry]) -> Vec<SafeCommand> {
    let mut cmds = Vec::new();

    for entry in entries {
        match entry {
            TmpfileEntry::Directory {
                path,
                mode,
                uid,
                gid,
            } => {
                let mode_str = format!("{mode:04o}");
                cmds.push(SafeCommand {
                    binary: "mkdir".to_string(),
                    args: vec![
                        "-p".to_string(),
                        "-m".to_string(),
                        mode_str,
                        path.display().to_string(),
                    ],
                });
                if uid.is_some() || gid.is_some() {
                    let uid_str = uid.map_or_else(String::new, |u| u.to_string());
                    let gid_str = gid.map_or_else(String::new, |g| g.to_string());
                    let owner = format!("{uid_str}:{gid_str}");
                    cmds.push(SafeCommand {
                        binary: "chown".to_string(),
                        args: vec![owner, path.display().to_string()],
                    });
                }
                debug!(path = %path.display(), mode = %format!("{mode:04o}"), "tmpfile: directory");
            }
            TmpfileEntry::Symlink { path, target } => {
                cmds.push(SafeCommand {
                    binary: "ln".to_string(),
                    args: vec![
                        "-sf".to_string(),
                        target.display().to_string(),
                        path.display().to_string(),
                    ],
                });
                debug!(
                    path = %path.display(),
                    target = %target.display(),
                    "tmpfile: symlink"
                );
            }
            TmpfileEntry::Device {
                path,
                dev_type,
                major,
                minor,
                mode,
            } => {
                cmds.push(SafeCommand {
                    binary: "mknod".to_string(),
                    args: vec![
                        path.display().to_string(),
                        dev_type.to_string(),
                        major.to_string(),
                        minor.to_string(),
                    ],
                });
                cmds.push(SafeCommand {
                    binary: "chmod".to_string(),
                    args: vec![format!("{mode:04o}"), path.display().to_string()],
                });
                debug!(
                    path = %path.display(),
                    dev_type = %dev_type,
                    major = major,
                    minor = minor,
                    "tmpfile: device"
                );
            }
            // non_exhaustive: skip unknown variants
            #[allow(unreachable_patterns)]
            _ => {
                warn!("unknown tmpfile entry type, skipping");
            }
        }
    }

    cmds
}

/// Validate tmpfile entries for correctness and safety.
///
/// Checks:
/// - All paths are absolute
/// - No path traversal (`..` components)
/// - Device types are `'b'` (block) or `'c'` (character)
/// - Modes are valid (≤ 0o7777)
///
/// # Errors
///
/// Returns an error describing the first invalid entry found.
pub fn validate_tmpfile_entries(entries: &[TmpfileEntry]) -> Result<()> {
    for entry in entries {
        match entry {
            TmpfileEntry::Directory { path, mode, .. } => {
                validate_path(path)?;
                validate_mode(*mode)?;
            }
            TmpfileEntry::Symlink { path, target } => {
                validate_path(path)?;
                validate_path(target)?;
            }
            TmpfileEntry::Device {
                path,
                dev_type,
                mode,
                ..
            } => {
                validate_path(path)?;
                validate_mode(*mode)?;
                if *dev_type != 'b' && *dev_type != 'c' {
                    bail!(
                        "invalid device type '{}' for {} (expected 'b' or 'c')",
                        dev_type,
                        path.display()
                    );
                }
            }
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
    Ok(())
}

/// Validate a path is absolute and contains no traversal.
fn validate_path(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("tmpfile path must be absolute: {}", path.display());
    }
    let path_str = path.display().to_string();
    if path_str.contains("..") {
        bail!(
            "tmpfile path contains traversal sequence: {}",
            path.display()
        );
    }
    Ok(())
}

/// Validate a mode is within valid range.
fn validate_mode(mode: u32) -> Result<()> {
    if mode > 0o7777 {
        bail!("invalid mode {:04o} (max 7777)", mode);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::types::TmpfileEntry;

    #[test]
    fn generate_directory_commands() {
        let entries = vec![TmpfileEntry::Directory {
            path: PathBuf::from("/run/myapp"),
            mode: 0o755,
            uid: None,
            gid: None,
        }];
        let cmds = generate_tmpfile_commands(&entries);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "mkdir");
        assert!(cmds[0].args.contains(&"0755".to_string()));
        assert!(cmds[0].args.contains(&"/run/myapp".to_string()));
    }

    #[test]
    fn generate_directory_with_chown() {
        let entries = vec![TmpfileEntry::Directory {
            path: PathBuf::from("/var/lib/db"),
            mode: 0o700,
            uid: Some(999),
            gid: Some(999),
        }];
        let cmds = generate_tmpfile_commands(&entries);
        assert_eq!(cmds.len(), 2); // mkdir + chown
        assert_eq!(cmds[1].binary, "chown");
        assert!(cmds[1].args.contains(&"999:999".to_string()));
    }

    #[test]
    fn generate_symlink_commands() {
        let entries = vec![TmpfileEntry::Symlink {
            path: PathBuf::from("/run/current"),
            target: PathBuf::from("/var/lib/data"),
        }];
        let cmds = generate_tmpfile_commands(&entries);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].binary, "ln");
        assert!(cmds[0].args.contains(&"-sf".to_string()));
    }

    #[test]
    fn generate_device_commands() {
        let entries = vec![TmpfileEntry::Device {
            path: PathBuf::from("/dev/mydev"),
            dev_type: 'c',
            major: 10,
            minor: 200,
            mode: 0o660,
        }];
        let cmds = generate_tmpfile_commands(&entries);
        assert_eq!(cmds.len(), 2); // mknod + chmod
        assert_eq!(cmds[0].binary, "mknod");
        assert_eq!(cmds[1].binary, "chmod");
        assert!(cmds[1].args.contains(&"0660".to_string()));
    }

    #[test]
    fn validate_absolute_paths() {
        let entries = vec![TmpfileEntry::Directory {
            path: PathBuf::from("relative/path"),
            mode: 0o755,
            uid: None,
            gid: None,
        }];
        assert!(validate_tmpfile_entries(&entries).is_err());
    }

    #[test]
    fn validate_path_traversal_rejected() {
        let entries = vec![TmpfileEntry::Directory {
            path: PathBuf::from("/var/../etc/shadow"),
            mode: 0o755,
            uid: None,
            gid: None,
        }];
        let err = validate_tmpfile_entries(&entries).unwrap_err();
        assert!(err.to_string().contains("traversal"));
    }

    #[test]
    fn validate_invalid_device_type() {
        let entries = vec![TmpfileEntry::Device {
            path: PathBuf::from("/dev/test"),
            dev_type: 'x',
            major: 1,
            minor: 1,
            mode: 0o660,
        }];
        let err = validate_tmpfile_entries(&entries).unwrap_err();
        assert!(err.to_string().contains("invalid device type"));
    }

    #[test]
    fn validate_invalid_mode() {
        let entries = vec![TmpfileEntry::Directory {
            path: PathBuf::from("/tmp/test"),
            mode: 0o77777, // Too large
            uid: None,
            gid: None,
        }];
        assert!(validate_tmpfile_entries(&entries).is_err());
    }

    #[test]
    fn validate_valid_entries() {
        let entries = vec![
            TmpfileEntry::Directory {
                path: PathBuf::from("/run/myapp"),
                mode: 0o755,
                uid: Some(0),
                gid: Some(0),
            },
            TmpfileEntry::Symlink {
                path: PathBuf::from("/run/link"),
                target: PathBuf::from("/var/data"),
            },
            TmpfileEntry::Device {
                path: PathBuf::from("/dev/test"),
                dev_type: 'c',
                major: 10,
                minor: 200,
                mode: 0o660,
            },
        ];
        assert!(validate_tmpfile_entries(&entries).is_ok());
    }

    #[test]
    fn generate_empty_entries() {
        let cmds = generate_tmpfile_commands(&[]);
        assert!(cmds.is_empty());
    }
}
