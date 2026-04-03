# ADR-009: Service Type Dispatch (Simple, Forking, Oneshot)

**Status**: Accepted
**Date**: 2026-04-02

## Context

All services were treated as long-running daemons (spawn, supervise, restart on crash). This doesn't model two common service patterns:

1. **Forking daemons** (e.g., PostgreSQL): the spawned parent exits after daemonizing; the real service runs as a child process with a different PID.
2. **Oneshot tasks** (e.g., filesystem setup, database migration): run to completion, no supervision needed.

## Decision

Added `ServiceType` enum with three variants:

- **`Simple`** (default): Long-running daemon. Spawned process IS the service. Supervised, health-checked, restarted on crash. Existing behavior.
- **`Forking`**: Parent spawns, then exits. Argonaut waits for parent exit, reads child PID from a configured `pid_file`, and tracks the child. The child is supervised via PID liveness checks (`kill(pid, 0)`).
- **`Oneshot`**: Spawns, blocks until exit, transitions to Stopped (exit 0) or Failed (non-zero). NOT inserted into the process table. No health checks, no supervision.

`start_service()` dispatches to `start_simple_service()`, `start_forking_service()`, or `start_oneshot_service()` based on `ServiceDefinition.service_type`.

`SpawnedProcess.child` changed from `Child` to `Option<Child>` to support forked processes where the original `Child` handle is consumed when the parent exits.

## Consequences

- **Positive**: PostgreSQL, Redis (if configured as forking), and boot-time setup tasks can be modeled correctly.
- **Positive**: Oneshot services complete synchronously during wave execution — subsequent waves don't start until oneshots finish.
- **Positive**: `start_service()` API unchanged — callers don't need to know the service type.
- **Negative**: Forking services require a `pid_file` configuration. sd_notify MAINPID is parsed but not yet wired as an alternative PID source (future enhancement).
- **Negative**: `Option<Child>` adds a branch to every `try_wait`/`wait`/`signal` call. Negligible — these are I/O-bound operations.

## Alternatives Considered

- **Separate start functions exposed publicly**: `start_simple()`, `start_forking()`, etc. Rejected — leaks service type to callers who shouldn't need to know.
- **Trait-based polymorphism**: `dyn ServiceRunner` with vtable dispatch. Over-engineered for 3 variants — enum dispatch is simpler and zero-cost.
- **sd_notify-only PID tracking for forking**: Would require polling the notify socket during startup. PID file is simpler and more widely supported.
