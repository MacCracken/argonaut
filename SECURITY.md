# Security Policy

## Scope

Argonaut is a PID 1 init system library for AGNOS. It manages service lifecycle, boot sequencing, health checks, shutdown orchestration, and security enforcement (seccomp, Landlock, capabilities). It runs as the first userspace process with full system privileges.

Written in Cyrius, compiled to a 373KB statically linked ELF binary. No libc, no external runtime dependencies.

## Attack Surface

| Area | Risk | Mitigation |
|------|------|------------|
| Service execution | Command injection via service definitions | SafeCommand abstraction — no shell expansion |
| Process management | Wrong-process-group signals | PID validated via safe conversion |
| Emergency shell | Unauthorized access | Password hash verification with constant-time comparison |
| Systemd unit generation | Injection via service name/env | Newline sanitization, `$` escaping, sorted env vars |
| Service creation API | Path traversal in binary_path | Rejects `..` components and relative paths |
| Edge boot (dm-verity) | Rootfs integrity bypass | Device path validation, hash length checks |
| Edge boot (LUKS) | Key material exposure | Input validation only — key handling delegated to cryptsetup |
| Tmpfiles | Symlink/traversal attacks | Path validation rejects `..` and non-absolute paths |
| Audit chain | Tamper-evident logging | SHA-256 hash-linked entries via libro 1.0.2 |
| Seccomp/Landlock | Policy bypass | Command generation only — enforcement delegated to kernel |
| HTTP health checks | Non-http:// schemes | Rejects URLs that don't start with `http://` |

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes |
| 0.9x.x  | No |

## Reporting a Vulnerability

Please report security issues to **security@agnos.dev**.

- You will receive acknowledgement within 48 hours
- We follow a 90-day coordinated disclosure timeline
- Please do not open public issues for security vulnerabilities

## Design Principles

- SafeCommand for all process execution — no shell interpolation
- Constant-time comparisons for security-sensitive operations (emergency auth, audit chain verification)
- No secret material in log output
- Device path validation on all dm-verity and LUKS operations
- Service name validation prevents path traversal
- SHA-256 audit chain integrity verification via libro
- Zombie prevention: all killed children are reaped (PID 1 responsibility)
