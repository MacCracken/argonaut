# ADR-003: Library Crate + Separate PID 1 Binary

**Status**: Accepted
**Date**: 2026-04-02

## Context

An init system has two distinct roles: the "brain" (boot orchestration, service management, health checks, dependency resolution) and the "body" (PID 1, signal handling, process reaping, cgroup management). These have different constraints — the brain should be testable and reusable, the body requires low-level OS integration.

## Decision

Argonaut is a library crate that provides the orchestration logic. The actual PID 1 binary lives in a separate crate that depends on argonaut.

- `argonaut` (this crate): types, boot sequences, service lifecycle, health checks, shutdown planning, edge boot. Fully testable with `cargo test`. `#![forbid(unsafe_code)]`.
- PID 1 binary (separate crate): process reaping via `waitpid`, signal mask setup, cgroup delegation, console I/O, `setuid`/`setgid` privilege dropping. May use `unsafe` where Linux APIs require it.

## Consequences

- **Positive**: The library is testable in CI without root privileges or a real boot environment. 256 tests run in milliseconds.
- **Positive**: Multiple consumers can use the library (AGNOS boot, stiva containers, sutra playbooks, daimon agent).
- **Positive**: `forbid(unsafe_code)` is achievable because unsafe OS operations are pushed to the binary crate.
- **Negative**: Some features (uid/gid dropping, cgroup management) cannot be implemented in the library and must be deferred.
- **Negative**: Integration testing requires the binary crate — the library alone cannot boot a real system.

## Alternatives Considered

- **Monolithic binary**: Simpler build, but untestable without root. `unsafe` would be required throughout.
- **Dynamic linking**: Library as a `.so` loaded by the binary. Adds deployment complexity for zero benefit in a static-linked init system.
