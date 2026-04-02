# ADR-001: Rust as Implementation Language

**Status**: Accepted
**Date**: 2026-04-02

## Context

Argonaut is an init system that runs as PID 1 (or a systemd delegate). Init systems have extreme reliability requirements — a crash means the entire system goes down. Memory safety bugs in init systems have historically caused kernel panics, security vulnerabilities, and data loss.

## Decision

Implement argonaut in Rust with `#![forbid(unsafe_code)]`.

- Rust's ownership model eliminates use-after-free, double-free, and data races at compile time.
- `forbid(unsafe_code)` ensures no unsafe blocks exist anywhere in the crate. All unsafe operations are delegated to audited dependencies (`nix`, `std`).
- The type system encodes state machine invariants (e.g., `ServiceState` transitions) that would be runtime bugs in C.

## Consequences

- **Positive**: Memory safety without garbage collection. Zero-cost abstractions for hot paths. Compile-time guarantee of no undefined behavior in our code.
- **Positive**: `cargo audit` + `cargo deny` provide automated supply chain security.
- **Negative**: Privilege dropping (`setuid`/`setgid`) requires unsafe or a pre-exec hook — currently blocked by `forbid(unsafe_code)`. Deferred to the PID 1 binary crate.
- **Negative**: Smaller ecosystem of Linux system programming libraries compared to C.

## Alternatives Considered

- **C**: Maximum control, but manual memory management is the #1 source of init system CVEs.
- **Go**: GC pauses are unacceptable for PID 1. Runtime startup overhead conflicts with <3s boot target.
- **Zig**: Promising but ecosystem maturity insufficient for production init system.
