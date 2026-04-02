# ADR-002: SafeCommand Abstraction for Shell Injection Prevention

**Status**: Accepted
**Date**: 2026-04-02

## Context

Init systems execute commands on behalf of the system — mounting filesystems, starting services, running health checks, unlocking encrypted volumes. Constructing these commands as shell strings (`"mount -o ro /"`) is a classic injection vector (CWE-78).

## Decision

All command execution goes through `SafeCommand` — a structured `{binary, args}` pair that is never passed through a shell interpreter.

- `SafeCommand` holds a binary path and argument vector separately.
- `std::process::Command` is used for execution — arguments are passed as an array, not concatenated into a shell string.
- No function in the crate accepts a command as a single string to be shell-interpreted.
- Device paths are validated via `validate_device_path()` which rejects empty paths, paths without `/dev/` prefix, path traversal (`..`), and non-alphanumeric characters.

## Consequences

- **Positive**: CWE-78 (OS Command Injection) is structurally eliminated. There is no code path where user-controlled input reaches a shell interpreter.
- **Positive**: Commands are loggable as structured data (binary + args) for audit trails.
- **Negative**: Commands that genuinely need shell features (pipes, redirects, globbing) cannot be expressed. This is intentional — an init system should not need them.
- **Negative**: `SafeCommand` uses `String` allocations for binary and args. In a hot loop this would matter, but command execution is inherently I/O-bound.

## Alternatives Considered

- **Shell strings with escaping**: Error-prone, impossible to audit comprehensively, violates defense-in-depth.
- **Sandboxed shell execution**: Adds complexity and attack surface for no benefit in an init system.
