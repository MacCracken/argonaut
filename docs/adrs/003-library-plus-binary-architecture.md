# ADR-003: Library + Separate PID 1 Binary

**Status**: Accepted — architecture carried forward in Cyrius port; PID 1 binary is now kybernet
**Date**: 2026-04-02

## Context

An init system has two distinct roles: the "brain" (boot orchestration, service management, health checks, dependency resolution) and the "body" (PID 1, signal handling, process reaping, cgroup management). These have different constraints — the brain should be testable and reusable, the body requires low-level OS integration.

## Decision

Argonaut is a Cyrius library that provides the orchestration logic. The actual PID 1 binary lives in a separate project (kybernet) that depends on argonaut.

- `argonaut` (this library): types, boot sequences, service lifecycle, health checks, shutdown planning, edge boot. Fully testable with `cyrius test`.
- `kybernet` (separate Cyrius binary): process reaping via `waitpid`, signal mask setup, cgroup delegation, console I/O, `setuid`/`setgid` privilege dropping. Contains the low-level OS integration that argonaut deliberately omits.

## Consequences

- **Positive**: The library is testable in CI without root privileges or a real boot environment. 545 assertions across 22 suites run in milliseconds.
- **Positive**: Multiple consumers can use the library (AGNOS boot via kybernet, stiva containers, sutra playbooks, daimon agent).
- **Positive**: Low-level OS operations are isolated to kybernet, keeping the library auditable and independently testable.
- **Negative**: Some features (uid/gid dropping, cgroup management) cannot be implemented in the library and must be deferred to kybernet.
- **Negative**: Integration testing requires kybernet — the library alone cannot boot a real system.

## Alternatives Considered

- **Monolithic binary**: Simpler build, but untestable without root. Low-level OS calls would be interleaved with orchestration logic.
- **Dynamic linking**: Library as a `.so` loaded by the binary. Adds deployment complexity for zero benefit in a static-linked init system.

## Post-Port Update (v0.95.0)

The library/binary split was preserved and clarified in the Cyrius port. The PID 1 binary is now **kybernet** (a separate Cyrius project). Argonaut is included by kybernet as a Cyrius library dependency. The separation of concerns — orchestration logic in argonaut, OS primitives in kybernet — is identical to the original design intent. See the kybernet project for the integration point.
