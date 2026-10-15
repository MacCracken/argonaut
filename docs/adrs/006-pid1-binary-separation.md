# ADR-006: PID 1 Binary Crate Separation

**Status**: Accepted — amended 1.6.0: kybernet is the production PID 1; argonaut also ships a minimal PID-1 loop
**Date**: 2026-04-02

## Context

Research into s6, dinit, and systemd confirms that production init systems separate the PID 1 process from the service manager. PID 1 has extreme constraints:

- Must never exit (kernel panic)
- Must never panic (kernel panic)
- Must reap ALL child processes (zombies)
- Must handle signals (SIGCHLD, SIGTERM, SIGPWR)
- Must mount essential filesystems before anything else
- Requires `unsafe` for: `pre_exec`, `signalfd`, `waitpid`, `mount`, `setuid`/`setgid`

The argonaut library is kept free of direct OS primitives to remain independently testable.

## Decision

Create a separate `kybernet` binary (Cyrius project) that:

1. Acts as PID 1 (or systemd delegate)
2. Depends on the `argonaut` library for orchestration logic
3. Handles the low-level OS integration that argonaut deliberately omits
4. Is kept minimal to minimize the unsafe/OS-call surface area

The two-process pattern follows industry practice:
- s6: `s6-svscan` (PID 1) + `s6-rc` (service manager)
- dinit: single process but with explicit unsafe for OS primitives
- systemd: PID 1 with internal separation between init and manager

## Consequences

- **Positive**: Library remains free of OS primitives — 607 assertions across 26 suites run without root.
- **Positive**: kybernet is minimal — small audit surface for OS-level operations.
- **Positive**: Library is reusable by other consumers (containers, playbooks, agents).
- **Negative**: Integration testing requires kybernet.
- **Negative**: Two projects to maintain, version, and release.

## Post-Port Update (v0.95.0)

The `argonaut-init` binary crate was renamed/evolved into **kybernet**, a separate Cyrius project. Kybernet boots QEMU with real AGNOS binaries and integrates argonaut as a Cyrius library. The separation rationale — keeping the library testable and the unsafe surface minimal — is unchanged.

## Update (2026-10-14)

- **The library is not free of OS primitives** (see the ADR-001 update). What stayed out
  of argonaut is service *policy*: privilege drop, sandboxing and cgroups, which the
  consumer applies through the pre-exec hook (ADR-013).
- **kybernet links argonaut in-process**, as one process with its own event loop. It is
  not a separate manager process.
- **argonaut's own binary is a minimal PID 1** (since 1.6.0); see the ADR-003 update.
- **The Rust `unsafe` / `pre_exec` points in this ADR no longer apply.**
