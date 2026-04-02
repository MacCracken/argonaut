# Argonaut Security Posture

## Overview

Argonaut is an init system — it runs as PID 1 with root privileges. Compromise of the init process means full system compromise. This document describes the security measures built into argonaut and maps them to industry standards.

## Defense Layers

### 1. Memory Safety

| Measure | Detail |
|---------|--------|
| `#![forbid(unsafe_code)]` | No unsafe blocks in the entire crate. Memory safety bugs (buffer overflows, use-after-free, data races) are eliminated at compile time. |
| Rust ownership model | State machine transitions (e.g., `ServiceState`) are encoded in the type system. Invalid transitions are caught at compile time or return errors, never UB. |
| Integer overflow protection | PID casts use `i32::try_from()`. Backoff delays use `saturating_mul`/`saturating_pow`. No truncation, no wrapping. |

### 2. Injection Prevention

| Measure | Detail |
|---------|--------|
| `SafeCommand` abstraction | All command execution uses structured `{binary, args}` pairs passed directly to `std::process::Command`. No shell interpretation, no string concatenation. Structurally eliminates CWE-78 (OS Command Injection). |
| Device path validation | `validate_device_path()` rejects: empty paths, missing `/dev/` prefix, `..` traversal components, non-alphanumeric characters. Prevents CWE-22 (Path Traversal). |
| dm-verity parameter validation | Root hash validated for exact 64-char length and hex-only characters. LUKS mapped names validated against character whitelist. |

### 3. Boot Integrity

| Measure | Detail |
|---------|--------|
| dm-verity | Rootfs integrity verified against SHA-256 Merkle tree at block level. Tampered blocks return I/O errors. |
| Read-only rootfs | Root partition remounted `ro`. Writable state isolated to tmpfs overlays with size limits and `noexec`/`nosuid` flags. |
| LUKS2 encryption | Data partition encrypted. Unlock via TPM2-backed tokens (no passphrase in unattended mode). |
| Boot time budget | `EdgeBootConfig.max_boot_time_ms` detects anomalous boot delays that may indicate tampering. |

### 4. Runtime Protection

| Measure | Detail |
|---------|--------|
| Health check timeouts | All health checks enforce deadlines. Command checks use spawn+poll+kill — no infinite hangs. |
| Restart limits | `RestartConfig.max_restarts` with exponential backoff (100ms minimum floor) prevents restart storms. |
| Watchdog enforcement | Services exceeding startup or runtime watchdog windows are killed. Stale health check timestamps trigger watchdog. |
| Graceful shutdown | SIGTERM → configurable wait → SIGKILL. Signal field respected (SIGKILL for emergency). 500ms cap on post-SIGKILL wait. |
| Privilege separation | `spawn_process()` errors if `uid`/`gid` are set but not implementable (library crate limitation). Prevents silent no-op privilege drop. |

### 5. Supply Chain Security

| Measure | Detail |
|---------|--------|
| `cargo audit` | Checks RustSec Advisory Database for known CVEs in all transitive dependencies. Runs in CI on every push. |
| `cargo deny` | Enforces: license allowlist (MIT, Apache-2.0, GPL-3.0-only, etc.), crates.io-only sources, no unknown registries, no unknown git sources, no wildcard versions. |
| Minimal dependencies | 6 direct runtime dependencies, ~15 transitive. No HTTP client library. Health checks use raw TCP from std. |
| `forbid(unsafe_code)` | First-party code contains zero unsafe blocks. Unsafe is confined to audited dependencies (`nix` for syscalls, `std` for OS primitives). |

## CWE Coverage

| CWE | Name | Mitigation |
|-----|------|------------|
| CWE-78 | OS Command Injection | `SafeCommand` — no shell invocation anywhere |
| CWE-22 | Path Traversal | `validate_device_path()` — prefix, charset, `..` checks |
| CWE-20 | Improper Input Validation | Hash length/charset validation, device path validation, mapped name validation |
| CWE-190 | Integer Overflow | `i32::try_from(pid)`, `saturating_mul`, `saturating_pow` |
| CWE-676 | Use of Dangerous Function | `forbid(unsafe_code)` — no unsafe in crate |
| CWE-400 | Uncontrolled Resource Consumption | Restart limits, backoff floor, health check timeouts, watchdog kills |
| CWE-362 | TOCTOU Race Condition | Atomic socket removal (remove + ignore NotFound) |
| CWE-269 | Improper Privilege Management | Explicit error on unimplemented uid/gid drop |
| CWE-835 | Infinite Loop | Deadline-based health checks, bounded stop waits, SIGKILL escalation |
| CWE-459 | Incomplete Cleanup | `Drop` on NotifyListener, shutdown plan includes sync/unmount/LUKS close |
| CWE-754 | Unusual Conditions | `anyhow::Result` throughout, no `unwrap()`/`panic!()` in library code |
| CWE-696 | Incorrect Behavior Order | Kahn's algorithm for dependency resolution, cycle detection |
| CWE-829 | Untrusted Control Sphere | `cargo deny` restricts to crates.io only |

## Known Gaps

| Gap | Severity | Status |
|-----|----------|--------|
| uid/gid privilege drop | High | Deferred to PID 1 binary crate (requires `unsafe` pre_exec) |
| Seccomp/Landlock policies | High | Boot stage exists (`StartSecurity`) but no implementation |
| sd_notify credential verification (SO_PASSCRED) | Medium | Not implemented — any process can send READY=1 |
| sd_notify WATCHDOG=1 keepalive | Medium | Not implemented |
| Cgroup-per-service isolation | High | Not implemented |
| Emergency shell authentication | Low | `require_auth` defaults to `false` |
| Core dump restriction (RLIMIT_CORE) | Low | Not implemented |
| SBOM generation | Low | No Software Bill of Materials produced |
