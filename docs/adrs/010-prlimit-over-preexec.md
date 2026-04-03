# ADR-010: Resource Limits via prlimit(1) Instead of pre_exec

**Status**: Accepted
**Date**: 2026-04-02

## Context

Services need resource limits (RLIMIT_NOFILE, RLIMIT_AS, RLIMIT_NPROC, RLIMIT_CORE). The standard approach is `setrlimit(2)` in a `Command::pre_exec` closure — but `pre_exec` requires `unsafe`, and the library crate enforces `#![forbid(unsafe_code)]`.

## Decision

Resource limits are applied **post-spawn** via the `prlimit(1)` CLI tool:

```
prlimit --pid=<child_pid> --nofile=65536:65536
prlimit --pid=<child_pid> --core=0:0
```

`ResourceLimits::to_prlimit_commands()` generates `Vec<SafeCommand>` from the configured limits. These are executed via `run_command_sequence()` after the child process is spawned. Failure is **best-effort** — a prlimit failure logs a warning but does not kill the service.

A `ResourceLimits::secure_defaults()` constructor provides `core: Some(0)` (disable core dumps) as a baseline.

## Consequences

- **Positive**: Zero unsafe code. `prlimit(1)` is a standard Linux util-linux tool, available on all distributions.
- **Positive**: Limits can be changed at runtime (re-run prlimit on the same PID).
- **Positive**: Same SafeCommand pattern used throughout the crate.
- **Negative**: Brief window between spawn and limit application where the child runs without limits. Acceptable for an init system — the child is trusted code, and the window is milliseconds.
- **Negative**: Requires `prlimit` binary on the system. Standard on all Linux distributions (part of util-linux).
- **Negative**: The PID 1 binary crate could apply limits tighter (in `pre_exec`). This is complementary — the library provides the configuration and fallback; the binary provides the optimal path.

## Alternatives Considered

- **`Command::pre_exec` with `unsafe`**: Tightest enforcement (limits applied before exec). Rejected — violates `forbid(unsafe_code)`. Deferred to binary crate.
- **Feature-gated unsafe**: Add an `unsafe-exec` feature that allows `pre_exec`. Rejected — complicates the safety story and audit surface.
- **nix crate `setrlimit`**: Still requires calling it in `pre_exec` context. Same unsafe problem.
