# ADR-007: Feature-Gated AGNOS Ecosystem Dependencies

**Status**: Accepted
**Date**: 2026-04-02

## Context

Argonaut is part of the AGNOS ecosystem but should also be usable as a standalone init library by anyone building a Linux-based OS. Two AGNOS crates provide capabilities argonaut benefits from: **libro** (tamper-proof audit logging) and **agnosys** (seccomp, Landlock, capability management). Adding these as hard dependencies would force all consumers to pull in the full AGNOS stack.

## Decision

AGNOS ecosystem dependencies are optional, gated behind Cargo features:

- `audit` feature enables `libro` — tamper-proof service event audit chains.
- `security` feature enables `agnosys` — direct seccomp BPF loading, Landlock enforcement.

Without these features, the library still provides:
- All configuration types (`SeccompConfig`, `LandlockConfig`, `AuditLog` types, etc.)
- SafeCommand generation for CLI tool fallbacks (`prlimit`, `setpriv`)
- Human-readable descriptions for logging

With features enabled, the library can directly apply security policies via syscalls (wrapped safely by agnosys) or record events to a cryptographic audit chain (via libro).

Both dependencies use `path = "../<crate>"` with a `version` constraint for local development, enabling Cargo workspace-like builds while supporting independent publishing.

## Consequences

- **Positive**: Non-AGNOS consumers get a fully functional init library with zero AGNOS coupling.
- **Positive**: AGNOS deployments get tight native integration without runtime overhead of CLI shelling.
- **Positive**: CI must test all feature combinations (`default`, `audit`, `security`, `all-features`).
- **Negative**: Feature-gated code paths (`#[cfg(feature = "...")]`) increase maintenance surface.
- **Negative**: Consumers must know to enable features for full security enforcement.

## Alternatives Considered

- **Hard dependencies on libro/agnosys**: Forces all consumers into the AGNOS stack. Rejected — violates reusability.
- **Separate crates (argonaut-audit, argonaut-security)**: More crates to maintain, version, and publish. Rejected — feature gates achieve the same with less overhead.
- **Runtime plugin system**: Adds complexity and indirection. Rejected — compile-time feature selection is simpler and has zero runtime cost.
