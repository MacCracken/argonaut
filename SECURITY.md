# Security Policy

## Scope

Argonaut is a PID 1 init system library for AGNOS. It manages service lifecycle, boot sequencing, health checks, shutdown orchestration, and security enforcement (seccomp, Landlock, capabilities). It runs as the first userspace process with full system privileges.

## Attack Surface

| Area | Risk | Mitigation |
|------|------|------------|
| Service execution | Command injection via service definitions | SafeCommand abstraction — no shell expansion |
| Process management | Wrong-process-group signals | PID validated via i32 safe conversion |
| Emergency shell | Unauthorized access | Password hash verification (`verify_emergency_auth`) |
| Systemd unit generation | Injection via service name/env | Newline sanitization, `$` escaping, sorted env vars |
| Service creation API | Path traversal in binary_path | Rejects `..` components and relative paths |
| Edge boot (dm-verity) | Rootfs integrity bypass | Device path validation, hash length checks |
| Edge boot (LUKS) | Key material exposure | Input validation only — key handling delegated to cryptsetup |
| Tmpfiles | Symlink/traversal attacks | Path validation rejects `..` and non-absolute paths |
| Audit chain | Tamper-evident logging | Hash-linked entries with FNV-1a integrity verification |
| Seccomp/Landlock | Policy bypass | Command generation only — enforcement delegated to kernel |

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.95.x | Yes |
| 0.90.x | No |

## Reporting a Vulnerability

Please report security issues to **security@agnos.dev**.

- You will receive acknowledgement within 48 hours
- We follow a 90-day coordinated disclosure timeline
- Please do not open public issues for security vulnerabilities

## Design Principles

- SafeCommand for all process execution — no shell interpolation
- Constant-time comparisons for security-sensitive operations (emergency auth)
- No secret material in log output
- Device path validation on all dm-verity and LUKS operations
- Service name validation prevents path traversal
- Audit chain integrity verification on query
