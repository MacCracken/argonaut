# ADR-012: API Stability Approach Pre-1.0

**Status**: Accepted
**Date**: 2026-04-02

## Context

Argonaut is at version 0.90.0 (pre-1.0, SemVer 0.D.M). The public API has grown from ~15 types in v0.1 to ~45 types across v0.7–v0.9. Each version added fields to `ServiceDefinition` (the most frequently constructed type), requiring updates to 13+ literal construction sites across the crate and breaking external consumers.

Consumers: AGNOS boot (PID 1), stiva, sutra, daimon — all AGNOS-internal. No known external consumers yet.

## Decision

### Pre-1.0 (current)

- **`ServiceDefinition` remains non-`#[non_exhaustive]`** and is constructed via struct literals. Adding fields is a breaking change but acceptable for pre-1.0 since all consumers are AGNOS-internal.
- **All public enums use `#[non_exhaustive]`** — new variants can be added without breaking `match` in consumers.
- **Output-only structs** (`ServiceStatus`, `ServiceListResponse`, `SystemMetrics`, etc.) use `#[non_exhaustive]` — consumers read fields but never construct them.
- **Input structs** (`ServiceCreateRequest`) do NOT use `#[non_exhaustive]` — consumers must be able to construct them.
- **`HealthTracker` and `SafeCommand`** intentionally omit `Serialize`/`Deserialize` — they are runtime/execution primitives, not configuration types.

### At 1.0

When the API stabilizes for 1.0, consider:
1. Adding `#[non_exhaustive]` to `ServiceDefinition` with a builder: `ServiceDefinition::builder("name", "/usr/bin/name").depends_on(["db"]).build()`
2. Or: keep direct construction if the field set is stable.

### Stability checklist for 1.0 readiness

- [ ] No new fields added to `ServiceDefinition` for 2+ release cycles
- [ ] `ServiceCreateRequest` covers all fields consumers need
- [ ] All public function signatures stable (no `&mut self` → `&self` changes)
- [ ] Re-export surface (`pub use`) finalized
- [ ] Feature gate names (`audit`, `security`) finalized

## Consequences

- **Positive**: Simple, direct API for internal consumers during rapid development.
- **Positive**: External consumers know pre-1.0 means field additions are expected.
- **Negative**: Each new `ServiceDefinition` field requires ~13 site updates. This is mechanical but time-consuming.
- **Negative**: External consumers (if any) will break on minor version bumps.

## Alternatives Considered

- **Builder pattern now**: `ServiceDefinition::builder()` with defaults. Would eliminate the field update problem but adds API complexity before the type is stable. Premature — the field set is still evolving.
- **`#[non_exhaustive]` on `ServiceDefinition` now**: Would force all construction through `Default` + field mutation. Awkward for a type with 18 fields and no sensible defaults for `name`/`binary_path`.
- **`..Default::default()` pattern**: Requires `Default` impl on `ServiceDefinition`, but `name` and `binary_path` have no sensible defaults. Would require `Option<PathBuf>` for binary_path which weakens type safety.
