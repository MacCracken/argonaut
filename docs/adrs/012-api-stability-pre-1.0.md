# ADR-012: API Stability Approach Pre-1.0

**Status**: Superseded — argonaut is now at v1.2.0; API stability guarantees are in effect
**Date**: 2026-04-02

## Context

Argonaut was at version 0.90.0 (pre-1.0, SemVer 0.D.M) when this ADR was written. The public API had grown from ~15 types in v0.1 to ~45 types across v0.7–v0.9. Each version added fields to `ServiceDefinition` (the most frequently constructed type), requiring updates to 13+ literal construction sites across the library and breaking external consumers.

Consumers: AGNOS boot (PID 1), stiva, sutra, daimon — all AGNOS-internal. No known external consumers yet.

## Decision

### Pre-1.0 (historical)

- **`ServiceDefinition` remained non-exhaustive** and was constructed via struct literals. Adding fields was a breaking change but acceptable for pre-1.0 since all consumers were AGNOS-internal.
- **Output-only structs** (`ServiceStatus`, `ServiceListResponse`, `SystemMetrics`, etc.) were read-only by consumers — never constructed externally.
- **Input structs** (`ServiceCreateRequest`) were constructable by consumers.
- **`HealthTracker` and `SafeCommand`** intentionally omitted serialization — they are runtime/execution primitives, not configuration types.

### At 1.0 (current)

Argonaut has reached v1.0.0. The API is now stable. The Cyrius port completed at v0.95.0 (Rust source removed v0.96.1), and the API was finalized through the v0.96–v0.99 stabilization cycle.

**API stability guarantees (v1.0.0+):**
- `ServiceDefinition` field set is stable — no new required fields without a major version bump
- All public function signatures are stable
- `ServiceCreateRequest` covers all fields consumers need
- Builder pattern (`service_definition_builder()`) is available for constructing `ServiceDefinition`

### Stability checklist — resolved at 1.0

- [x] No new fields added to `ServiceDefinition` for 2+ release cycles
- [x] `ServiceCreateRequest` covers all fields consumers need
- [x] All public function signatures stable
- [x] Public API surface finalized
- [x] Include-based dependency approach finalized (feature gates not applicable in Cyrius)

## Consequences

- **Positive**: Simple, direct API for internal consumers during rapid development.
- **Positive**: External consumers know pre-1.0 means field additions are expected.
- **Negative**: Each new `ServiceDefinition` field required ~13 site updates during pre-1.0. This was mechanical but time-consuming.
- **Negative**: External consumers (if any) broke on pre-1.0 minor version bumps.

## Alternatives Considered

- **Builder pattern during pre-1.0**: `service_definition_builder()` with defaults. Would have eliminated the field update problem but added API complexity before the type was stable. Deferred — adopted at 1.0.
- **Exhaustive construction only**: Would require all consumers to update on every new field. Rejected — builder pattern preferred at 1.0.
- **Optional fields throughout**: Requiring `name` and `binary_path` to be `Option` weakens type safety at the call site. Rejected — builder validates required fields at `build()` time.
