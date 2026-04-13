# ADR-007: Feature-Gated AGNOS Ecosystem Dependencies

**Status**: Superseded — Cyrius has no feature gates; AGNOS dependencies are included directly
**Date**: 2026-04-02

## Context

Argonaut is part of the AGNOS ecosystem but should also be usable as a standalone init library by anyone building a Linux-based OS. Two AGNOS crates provide capabilities argonaut benefits from: **libro** (tamper-proof audit logging) and **agnosys** (seccomp, Landlock, capability management). Adding these as hard dependencies would force all consumers to pull in the full AGNOS stack.

## Decision (original — Rust)

AGNOS ecosystem dependencies were optional, gated behind Cargo features:

- `audit` feature enabled `libro` — tamper-proof service event audit chains.
- `security` feature enabled `agnosys` — direct seccomp BPF loading, Landlock enforcement.

Without these features, the library still provided:
- All configuration types (`SeccompConfig`, `LandlockConfig`, `AuditLog` types, etc.)
- SafeCommand generation for CLI tool fallbacks (`prlimit`, `setpriv`)
- Human-readable descriptions for logging

With features enabled, the library could directly apply security policies via syscalls (via agnosys) or record events to a cryptographic audit chain (via libro).

## Consequences (original — Rust)

- **Positive**: Non-AGNOS consumers got a fully functional init library with zero AGNOS coupling.
- **Positive**: AGNOS deployments got tight native integration without runtime overhead of CLI shelling.
- **Negative**: Feature-gated code paths (`#[cfg(feature = "...")]`) increased maintenance surface.

## Alternatives Considered (original — Rust)

- **Hard dependencies on libro/agnosys**: Forces all consumers into the AGNOS stack. Rejected — violates reusability.
- **Separate crates (argonaut-audit, argonaut-security)**: More crates to maintain, version, and publish. Rejected — feature gates achieve the same with less overhead.
- **Runtime plugin system**: Adds complexity and indirection. Rejected — compile-time feature selection is simpler and has zero runtime cost.

## Post-Port Update (v0.95.0)

Cyrius has no equivalent of Cargo feature gates (`#[cfg(feature = "...")]`). In the Cyrius port, AGNOS ecosystem dependencies are handled via `include` directives:

- **libro 1.0.3** is integrated directly — 8 modules (error, hasher, entry, verify, query, retention, chain, export) are always compiled in via `cyrius deps`. No conditional compilation. The full SHA-256 audit chain is active; the historical shim was retired at v0.97.0.
- **agnosys** security functions are included directly where needed. The dual-path (SafeCommand fallback vs. native syscall) is preserved via runtime logic rather than compile-time feature selection.

The reusability concern from the original decision is addressed differently in Cyrius: consumers include only the modules they need by selecting which `.cyr` files to compile into their project. The "always compiled in" approach is acceptable for AGNOS-first deployments.
