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

The library provides the wave grouping (`Vec<Vec<String>>`) and a `boot_execution_plan_waves()` method that returns `Vec<Vec<(String, ProcessSpec)>>`. The **consumer controls parallelism** — the library does not spawn threads or use async. This keeps the library runtime-agnostic.

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
