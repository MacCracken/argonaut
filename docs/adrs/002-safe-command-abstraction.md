# ADR-002: SafeCommand Abstraction for Shell Injection Prevention

**Status**: Accepted — carried forward in Cyrius port unchanged
**Date**: 2026-04-02

## Context

Init systems execute commands on behalf of the system — mounting filesystems, starting services, running health checks, unlocking encrypted volumes. Constructing these commands as shell strings (`"mount -o ro /"`) is a classic injection vector (CWE-78).

## Decision

All command execution goes through `SafeCommand` — a structured `{binary, args}` pair that is never passed through a shell interpreter.

- `SafeCommand` holds a binary path and argument vector separately.
- The Cyrius `exec_argv()` primitive is used for execution — arguments are passed as an array, not concatenated into a shell string.
- No function in the library accepts a command as a single string to be shell-interpreted.
- Device paths are validated via `validate_device_path()` which rejects empty paths, paths without `/dev/` prefix, path traversal (`..`), and non-alphanumeric characters.

## Consequences

- **Positive**: CWE-78 (OS Command Injection) is structurally eliminated. There is no code path where user-controlled input reaches a shell interpreter.
- **Positive**: Commands are loggable as structured data (binary + args) for audit trails.
- **Negative**: Commands that genuinely need shell features (pipes, redirects, globbing) cannot be expressed. This is intentional — an init system should not need them.
- **Negative**: `SafeCommand` uses heap-allocated strings for binary and args. In a hot loop this would matter, but command execution is inherently I/O-bound.

## Alternatives Considered

- **Shell strings with escaping**: Error-prone, impossible to audit comprehensively, violates defense-in-depth.
- **Sandboxed shell execution**: Adds complexity and attack surface for no benefit in an init system.

## Post-Port Update (v0.95.0)

The `SafeCommand` pattern was carried forward unchanged into the Cyrius port. In Cyrius, `SafeCommand` is a struct with `binary` (null-terminated string) and `args` (vec of strings), executed via `exec_argv()`. The structural guarantee — no shell interpreter in the execution path — is identical to the Rust implementation. See `src/safe_command.cyr`.
