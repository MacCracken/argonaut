# Argonaut Security Posture

## Overview

Argonaut is an init system — it runs as PID 1 with root privileges. Compromise of the init process means full system compromise. This document describes the security measures built into argonaut and maps them to industry standards.

## Defense Layers

### 1. Memory Safety

| Measure | Detail |
|---------|--------|
| No direct syscalls in library | The argonaut library contains no direct OS syscalls. Low-level operations are delegated to kybernet (PID 1 binary). Memory safety bugs are mitigated by Cyrius's type system and explicit allocation model. |
| Type-encoded state machines | State machine transitions (e.g., `ServiceState`) are encoded in the type system. Invalid transitions return errors. |
| Integer overflow protection | PID casts use explicit bounds checks. Backoff delays use saturating arithmetic. No truncation, no wrapping. |

### 2. Injection Prevention

| Measure | Detail |
|---------|--------|
| `SafeCommand` abstraction | All command execution uses structured `{binary, args}` pairs passed directly to `exec_argv()`. No shell interpretation, no string concatenation. Structurally eliminates CWE-78. |
| Device path validation | `validate_device_path()` rejects: empty paths, missing `/dev/` prefix, `..` traversal components, non-alphanumeric characters. |
| dm-verity parameter validation | Root hash validated for exact 64-char length and hex-only characters. LUKS mapped names validated against character whitelist. |
| Service name validation | `register_service()` and `create_service_from_request()` reject empty names, `..` traversal sequences, and non-alphanumeric characters (except `-`, `_`, `.`). |
| Binary path validation | `create_service_from_request()` requires absolute `binary_path` — rejects relative path exploits. |
| systemd unit injection prevention | `generate_unit()` sanitizes newlines from description and environment values. `$` escaped to `$$` to prevent systemd variable expansion. |
| tmpfiles path validation | `validate_tmpfile_entries()` requires absolute paths, rejects `..` traversal, validates device types and mode ranges. |

### 3. Boot Integrity

| Measure | Detail |
|---------|--------|
| dm-verity | Rootfs integrity verified against SHA-256 Merkle tree at block level. Tampered blocks return I/O errors. `--restart-on-corruption` enabled for edge mode. Verity failure is fatal in edge mode. |
| Read-only rootfs | Root partition remounted `ro`. Writable state isolated to tmpfs overlays with size limits and `noexec`/`nosuid` flags. |
| LUKS2 encryption | Data partition encrypted. Unlock via TPM2-backed tokens (`--token-id=0`, `--tries=1`) with PCR binding support. |
| Boot time budget | `EdgeBootConfig.max_boot_time_ms` detects anomalous boot delays that may indicate tampering. |
| tmpfiles.d validation | Boot-time filesystem entries validated before execution — absolute paths, no traversal, valid modes and device types. |

### 4. Runtime Protection

| Measure | Detail |
|---------|--------|
| Health check timeouts | All health checks enforce deadlines. Command checks use spawn+poll+kill — no infinite hangs. |
| Restart limits | `RestartConfig.max_restarts` with exponential backoff (100ms minimum floor) prevents restart storms. |
| Watchdog enforcement | Services exceeding startup or runtime watchdog windows are killed. Stale health check timestamps trigger watchdog. sd_notify `WATCHDOG=1` keepalive messages are handled. |
| sd_notify security | `SO_PASSCRED` enabled via `enable_credentials()` for kernel-validated sender identity. Drain limit prevents DoS. `RELOADING=1`/`STOPPING=1` lifecycle fields supported. |
| Graceful shutdown | SIGTERM → configurable wait → SIGKILL. Signal field respected. 500ms cap on post-SIGKILL wait. |
| Privilege separation | `spawn_process()` errors if `uid`/`gid` are set (library limitation). Prevents silent no-op privilege drop. Actual drop deferred to kybernet (PID 1 binary). |
| Resource limits | `ResourceLimits` applied post-spawn via `prlimit(1)`: NOFILE, AS, NPROC, CORE. `secure_defaults()` provides core dump restriction (RLIMIT_CORE=0). |
| Log rotation | Size-based rotation prevents disk exhaustion. Configurable `max_size_bytes` and `max_files` per service. |

### 5. Per-Service Security (v0.9.0)

| Measure | Detail |
|---------|--------|
| Seccomp BPF filtering | `SeccompConfig` supports basic (20-syscall allowlist) and custom filters with named syscalls. With `security` feature: applied via agnosys. Without: configuration documented for external tooling. |
| Landlock filesystem restrictions | `LandlockConfig` with per-path access control (NoAccess, ReadOnly, ReadWrite). With `security` feature: applied via agnosys (kernel 5.13+, graceful degradation). |
| Capability bounding set | `CapabilityConfig` specifies capabilities to drop. Generated `setpriv` commands avoid shell interpretation (no `capsh -c` injection risk). |
| Socket activation | `SocketActivationConfig` with LISTEN_FDS protocol. LISTEN_PID set by kybernet post-fork (not baked into library to avoid PID mismatch). |
| Emergency shell authentication | `verify_emergency_auth()` validates password against stored hash with constant-time comparison. `require_auth` + `auth_password_hash` on `EmergencyShellConfig`. |

### 6. Audit Trail

| Measure | Detail |
|---------|--------|
| Structured logging | `tracing` instrumentation on all operations — service start/stop, health checks, state transitions, boot stages, security config application. |
| Tamper-proof audit chain | With `audit` feature: libro integration provides hash-linked audit entries for all `ServiceEvent` types. Chain integrity verifiable via `AuditLog::verify()`. |
| Service event recording | `Enabled`, `Disabled`, `Starting`, `Started`, `Stopped`, `CrashDetected`, `TimeoutKilled` and 8 other event types recorded with timestamps. |

### 7. Supply Chain Security

| Measure | Detail |
|---------|--------|
| Cyrius build toolchain | The Cyrius compiler (cc3) and stdlib are AGNOS-maintained. No third-party package registry — dependencies are path-based includes. |
| Minimal dependencies | argonaut's direct dependencies are: cyrius stdlib (`lib/`), and optional AGNOS libraries (libro, agnosys). No HTTP client library. Health checks use raw socket operations from stdlib. |
| No direct syscalls in library | First-party library code contains no direct syscalls. OS-level operations are confined to kybernet and audited AGNOS libraries (agnosys for security syscalls). |
| AGNOS-owned stack | Security primitives (seccomp, landlock) provided by `agnosys` — an AGNOS-maintained library, not a third-party dependency. |

## CWE Coverage

| CWE | Name | Mitigation |
|-----|------|------------|
| CWE-78 | OS Command Injection | `SafeCommand` — no shell invocation anywhere. `setpriv` for capabilities (no `capsh -c`). |
| CWE-22 | Path Traversal | `validate_device_path()`, `validate_tmpfile_entries()`, service name `..` rejection, absolute binary_path requirement. |
| CWE-20 | Improper Input Validation | Hash length/charset validation, device path validation, mapped name validation, environment file parsing, tmpfile mode validation. |
| CWE-190 | Integer Overflow | `i32::try_from(pid)`, `saturating_mul`, `saturating_pow`. |
| CWE-676 | Use of Dangerous Function | No direct syscalls in library — OS operations delegated to kybernet and agnosys. |
| CWE-400 | Uncontrolled Resource Consumption | Restart limits, backoff floor, health check timeouts, watchdog kills, log rotation, sd_notify drain limits. |
| CWE-362 | TOCTOU Race Condition | Atomic socket removal (remove + ignore NotFound). PID file race documented (acceptable for init). |
| CWE-269 | Improper Privilege Management | Explicit error on unimplemented uid/gid drop. Capability drop via `setpriv`. Seccomp/Landlock enforcement. |
| CWE-835 | Infinite Loop | Deadline-based health checks, bounded stop waits, SIGKILL escalation. |
| CWE-459 | Incomplete Cleanup | `Drop` on NotifyListener, shutdown plan includes sync/unmount/LUKS close, log rotation prevents disk fill. |
| CWE-754 | Unusual Conditions | Error returns (-1 convention) throughout; no unchecked panics in library code. |
| CWE-696 | Incorrect Behavior Order | Kahn's algorithm for dependency resolution, cycle detection, wave-based parallel startup. |
| CWE-829 | Untrusted Control Sphere | No external package registry. All dependencies are path-based AGNOS libraries or Cyrius stdlib. |
| CWE-250 | Unnecessary Privileges | `CapabilityConfig` drops unneeded capabilities. `ResourceLimits::secure_defaults()` disables core dumps. |
| CWE-284 | Improper Access Control | Landlock filesystem restrictions per service. Seccomp syscall filtering. Emergency shell authentication. |

## Known Gaps

| Gap | Severity | Status |
|-----|----------|--------|
| uid/gid privilege drop | High | Deferred to kybernet (PID 1 binary) — requires pre-exec hook at the OS level |
| Cgroup-per-service isolation | High | Deferred to kybernet (PID 1 binary) — requires cgroup v2 filesystem manipulation |
| Password hashing (emergency auth) | Medium | Current fallback uses non-cryptographic hash. Production deployments must enable `security` feature for proper crypto via agnosys. |
| SBOM generation | Low | No Software Bill of Materials produced |
| Reproducible builds | Low | No SLSA provenance attestation |

## Module Capability Matrix

Cyrius uses `include`-based compilation rather than feature gates. All modules are compiled in. The table below reflects functional availability in the current Cyrius build.

| Capability | Status | Module |
|------------|--------|--------|
| Service lifecycle management | Always available | `src/init.cyr` |
| Health checks & watchdog | Always available | `src/health.cyr` |
| Boot sequencing & shutdown | Always available | `src/boot.cyr`, `src/shutdown.cyr` |
| SafeCommand generation | Always available | `src/safe_command.cyr` |
| systemd unit generation | Always available | `src/systemd.cyr` |
| API response types | Always available | `src/api.cyr` |
| Tamper-proof audit chain (libro) | Shim (pending libro port) | `src/audit.cyr` |
| Seccomp application (agnosys) | Via agnosys integration | `src/security.cyr` |
| Landlock application (agnosys) | Via agnosys integration | `src/security.cyr` |
| Cryptographic auth hashing | Via agnosys integration | `src/security.cyr` |
