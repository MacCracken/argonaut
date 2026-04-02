# ADR-006: PID 1 Binary Crate Separation

**Status**: Accepted
**Date**: 2026-04-02

## Context

Research into s6, dinit, and systemd confirms that production init systems separate the PID 1 process from the service manager. PID 1 has extreme constraints:

- Must never exit (kernel panic)
- Must never panic (kernel panic)
- Must reap ALL child processes (zombies)
- Must handle signals (SIGCHLD, SIGTERM, SIGPWR)
- Must mount essential filesystems before anything else
- Requires `unsafe` for: `pre_exec`, `signalfd`, `waitpid`, `mount`, `setuid`/`setgid`

The argonaut library crate uses `#![forbid(unsafe_code)]`, which prevents implementing these OS primitives directly.

## Decision

Create a separate `argonaut-init` binary crate that:

1. Acts as PID 1 (or systemd delegate)
2. Depends on the `argonaut` library for orchestration logic
3. May use `unsafe` where Linux APIs require it
4. Is kept minimal (~500 lines) to minimize unsafe surface area

The two-process pattern follows industry practice:
- s6: `s6-svscan` (PID 1) + `s6-rc` (service manager)
- dinit: single process but with explicit unsafe for OS primitives
- systemd: PID 1 with internal separation between init and manager

## Consequences

- **Positive**: Library remains `forbid(unsafe_code)` — 256 tests run without root.
- **Positive**: PID 1 binary is tiny — small audit surface for unsafe code.
- **Positive**: Library is reusable by other consumers (containers, playbooks, agents).
- **Negative**: Integration testing requires the binary crate.
- **Negative**: Two crates to maintain, version, and release.
