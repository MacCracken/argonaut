# ADR-001: Rust as Implementation Language

**Status**: Superseded — ported to Cyrius at v0.95.0; Rust source removed at v0.96.1
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

## Post-Port Update (v0.95.0 / v0.96.1)

The original Rust implementation was ported to **Cyrius** (the AGNOS self-hosting systems language) at v0.95.0. The Rust source (`rust-old/`) was removed at v0.96.1 once the Cyrius port reached feature parity.

The reasoning above remains valid: memory safety and type-encoded state invariants are still priorities. Cyrius addresses these through its own type system and compile-time checks rather than Rust's ownership model. The `forbid(unsafe_code)` constraint no longer applies as a Rust attribute, but the intent is preserved — all unsafe OS operations are delegated to the PID 1 binary (kybernet), and the argonaut library module itself contains no direct syscalls.

**Rust-specific consequences now superseded:**
- `cargo audit` / `cargo deny` → replaced by Cyrius build toolchain and AGNOS dependency management
- Supply chain security is enforced at the OS layer, not via Cargo
- Privilege dropping (`setuid`/`setgid`) remains deferred to kybernet (PID 1 binary), same constraint, different mechanism
