# ADR-008: Wave-Based Parallel Service Startup

**Status**: Accepted
**Date**: 2026-04-02

## Context

Sequential service startup wastes time when independent services could start concurrently. For Desktop mode with 8+ services, the dependency graph forms a DAG where multiple services at the same depth level have no ordering constraints between them (e.g., PostgreSQL and Redis have no mutual dependency).

The existing `resolve_service_order()` produces a linear topological ordering via Kahn's algorithm. This preserves correctness but serializes independent services.

## Decision

Added `resolve_service_waves()` — a modified Kahn's algorithm that groups services into parallel **waves** by dependency depth:

- **Wave 0**: All services with in-degree 0 (no dependencies)
- **Wave N**: Services whose dependencies are all in waves 0..N-1
- Services within each wave are sorted alphabetically for deterministic ordering

The library provides the wave grouping (vec of vec of string) via `resolve_service_waves()` and `boot_execution_plan_waves()` which returns waves of `(name, ProcessSpec)` pairs. The **consumer controls parallelism** — the library does not spawn threads or use async. This keeps the library runtime-agnostic.

Example for Desktop mode:
```
Wave 0: [postgres, redis]         — parallel
Wave 1: [daimon]                  — after databases
Wave 2: [aethersafha, llm-gateway] — after daimon
Wave 3: [agnoshi, synapse]        — after their deps
```

## Consequences

- **Positive**: Desktop boot can start databases in parallel, cutting ~1-2s from boot time.
- **Positive**: Consumer chooses threading model (tokio, rayon, std::thread, or sequential).
- **Positive**: Original `resolve_service_order()` preserved for consumers that need linear ordering.
- **Negative**: Wave algorithm duplicates dependency validation logic from `resolve_service_order`. Acceptable — they're never called together, and extracting a common base would add complexity for no user benefit.

## Alternatives Considered

- **Async executor in library**: Would force a runtime (tokio/async-std) on all consumers. Rejected — library should be runtime-agnostic.
- **Thread pool in library**: Same coupling problem. Rejected.
- **Depth annotation on linear sort**: Return `Vec<(String, usize)>` with depth. Workable but less ergonomic than pre-grouped waves.

## Amendment — 1.15.6 (2026-10-14): the sort, implemented

The Decision says each wave is sorted alphabetically. Until 1.15.6 nothing sorted. Both functions took ready services in `map_keys` order. cyrius's hashmap has seeded itself per process since cyrius 6.5.39, so that order, and therefore the boot order of unordered services, changed from boot to boot.

From 1.15.6:

- **Each wave is sorted bytewise by service name.** The comparison is byte order, not locale order: `Zed` sorts before `alpha`, and a name sorts before its extensions (`a`, `ab`, `abc`).
- **`resolve_service_order` uses the same levels and the same sort.** It is a level-synchronous Kahn's algorithm: every service a level releases joins the next level, and each level is sorted before it runs. So the linear order is exactly the wave plan read wave by wave. The two functions still compute their result independently (the Negative above stands), and `tests/tcyr/resolve_order.tcyr` asserts that they agree for every default profile.
- **The desktop profile today:**
  ```
  Wave 0: [agnos-init, postgres, redis]
  Wave 1: [daimon]
  Wave 2: [aethersafha, llm-gateway]
  Wave 3: [agnoshi, ifran]
  ```
  The same test pins this plan and the matching linear order. Neither depends on the order services were defined in.

## Amendment — 1.16.1 (2026-10-14): one implementation

The Negative above is resolved. Both functions now call one level-synchronous Kahn's pass, `_svc_kahn_levels` in `src/services.cyr`. It returns the levels as waves for `resolve_service_waves`, or reads them flat for `resolve_service_order`. The waves version used to rescan every key of the in-degree map once per wave, which made it O(waves × services). Sharing the pass halved `resolve_waves_chain_20` (63.9 → 32.7 µs) and cut `resolve_waves_desktop` by a fifth (CHANGELOG [1.16.1] Performance). The output is unchanged: `tests/tcyr/resolve_order.tcyr` passes as it did at 1.15.6.

## Amendment — 1.16.2 (2026-10-14): shutdown uses the same waves

The API is `init_boot_execution_plan_waves(init)`, which returns waves of
`(name Str, ServiceDefinition)` pairs; the consumer chooses how to run each wave.

Shutdown uses the same waves in reverse (`init_execute_shutdown`):
- each wave gets a share of one overall budget;
- SIGTERM first, then a process-group SIGKILL;
- a dependency cycle stops everything as one wave.
