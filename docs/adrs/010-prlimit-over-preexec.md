# ADR-010: Resource Limits via prlimit(1) Instead of pre_exec

**Status**: Accepted — carried forward in Cyrius port; prlimit approach unchanged
**Date**: 2026-04-02

## Context

Services need resource limits (RLIMIT_NOFILE, RLIMIT_AS, RLIMIT_NPROC, RLIMIT_CORE). The standard approach is `setrlimit(2)` called in a pre-exec hook — but the argonaut library deliberately avoids direct syscalls, keeping low-level OS operations in kybernet (the PID 1 binary).

## Decision

Resource limits are applied **post-spawn** via the `prlimit(1)` CLI tool:

```
prlimit --pid=<child_pid> --nofile=65536:65536
prlimit --pid=<child_pid> --core=0:0
```

`resource_limits_to_prlimit_commands()` generates a list of `SafeCommand` from the configured limits. These are executed via `run_command_sequence()` after the child process is spawned. Failure is **best-effort** — a prlimit failure logs a warning but does not kill the service.

A `ResourceLimits::secure_defaults()` constructor provides `core: Some(0)` (disable core dumps) as a baseline.

## Consequences

- **Positive**: No direct syscalls in the library. `prlimit(1)` is a standard Linux util-linux tool, available on all distributions.
- **Positive**: Limits can be changed at runtime (re-run prlimit on the same PID).
- **Positive**: Same SafeCommand pattern used throughout the library.
- **Negative**: Brief window between spawn and limit application where the child runs without limits. Acceptable for an init system — the child is trusted code, and the window is milliseconds.
- **Negative**: Requires `prlimit` binary on the system. Standard on all Linux distributions (part of util-linux).
- **Negative**: kybernet (the PID 1 binary) could apply limits tighter via a pre-exec hook. This is complementary — the library provides the configuration and SafeCommand fallback; kybernet provides the optimal path.

## Alternatives Considered

- **Pre-exec hook with `setrlimit`**: Tightest enforcement (limits applied before exec). Deferred to kybernet — the library does not make direct syscalls.
- **Feature-gated syscall path**: Add a conditional compilation path for direct `setrlimit`. Rejected — Cyrius has no feature gates; deferred to kybernet regardless.
- **`setrlimit` via agnosys**: Still requires a pre-exec context. Same architectural boundary.
