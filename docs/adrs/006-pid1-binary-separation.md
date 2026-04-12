# ADR-006: PID 1 Binary Crate Separation

**Status**: Accepted — PID 1 binary is now kybernet (Cyrius); argonaut is the Cyrius library
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

- **Positive**: Library remains free of OS primitives — 545 assertions across 22 suites run without root.
- **Positive**: kybernet is minimal — small audit surface for OS-level operations.
- **Positive**: Library is reusable by other consumers (containers, playbooks, agents).
- **Negative**: Integration testing requires kybernet.
- **Negative**: Two projects to maintain, version, and release.

## Post-Port Update (v0.95.0)

The `argonaut-init` binary crate was renamed/evolved into **kybernet**, a separate Cyrius project. Kybernet boots QEMU with real AGNOS binaries and integrates argonaut as a Cyrius library. The separation rationale — keeping the library testable and the unsafe surface minimal — is unchanged.
