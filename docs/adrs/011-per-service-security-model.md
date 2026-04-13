# ADR-011: Per-Service Security Model (Seccomp, Landlock, Capabilities)

**Status**: Accepted — carried forward in Cyrius port; dual-path architecture unchanged
**Date**: 2026-04-02

## Context

Modern init systems enforce security policies per service — syscall filtering (seccomp), filesystem restrictions (Landlock), and capability dropping. These reduce the blast radius of a compromised service.

The challenge: applying these policies requires Linux syscalls that the argonaut library deliberately avoids (direct syscalls belong in kybernet). The AGNOS ecosystem provides `agnosys` which wraps these syscalls, and the library integrates with agnosys for enforcement.

## Decision

Security enforcement uses a **dual-path architecture**:

### Configuration Layer (always available)

`ServiceDefinition` gains four optional fields (all optional/0 when unused):
- `seccomp` (`SeccompConfig`) — Basic (20-syscall allowlist) or Custom (named syscalls)
- `landlock` (`LandlockConfig`) — per-path access rules (NoAccess/ReadOnly/ReadWrite)
- `capabilities` (`CapabilityConfig`) — capabilities to drop from bounding set
- `socket_activation` (`SocketActivationConfig`) — LISTEN_FDS protocol

These are serializable configuration types. All consumers get:
- `seccomp_description()` / `landlock_description()` — human-readable summaries for logging
- `to_capability_commands()` → `SafeCommand` using `setpriv --no-new-privs --bounding-set=-<caps>` (no shell interpretation)
- `socket_activation_listen_fds_env()` — LISTEN_FDS environment variable

### Enforcement Layer (agnosys integration)

With agnosys included (via `include "src/security.cyr"` which wraps agnosys):
- `apply_seccomp(config)` — builds BPF filter via agnosys seccomp functions and loads it
- `apply_landlock(config)` — converts rules to agnosys FilesystemRule and calls apply_landlock
- Syscall name → number mapping via agnosys syscall_name_to_nr

### Capability Approach: setpriv over capsh

Capabilities use `setpriv` (from util-linux) instead of `capsh`:
- `setpriv --no-new-privs --bounding-set=-cap_sys_admin /usr/bin/myapp --flag`
- Binary and arguments are separate SafeCommand args — no shell interpretation
- `capsh -- -c "..."` was rejected because the `-c` flag passes through a shell, creating an injection vector

## Consequences

- **Positive**: Security config is declarative and serializable — can be loaded from TOML/JSON config files.
- **Positive**: SafeCommand fallbacks work with standard Linux tools regardless of agnosys availability.
- **Positive**: AGNOS deployments get native syscall enforcement via agnosys with zero CLI overhead.
- **Positive**: `setpriv` eliminates the shell injection risk that `capsh -c` would introduce.
- **Negative**: seccomp/Landlock application functions operate on the *calling* process, not a child. In practice, these are called by the PID 1 binary in a `pre_exec` context or by the service itself. The library provides the building blocks.
- **Negative**: Seccomp filters are irreversible once loaded. A malformed filter can brick a process.

## Alternatives Considered

- **Global security policy only**: Single system-wide seccomp filter. Rejected — per-service policies are more precise and follow the principle of least privilege.
- **External policy files (seccomp JSON profiles)**: More flexible but adds a file format dependency. Rejected for initial implementation — can be added later as a config loading layer.
- **Direct `prctl` syscalls in the library**: The library avoids direct syscalls. Rejected — delegated to agnosys which encapsulates the OS-level operations.
