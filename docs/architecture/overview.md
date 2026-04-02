# Argonaut Architecture Overview

## System Context

Argonaut is the init system and service manager for AGNOS. It occupies the layer between the kernel and userspace services — responsible for bringing the system from "kernel handed off control" to "all services running and healthy."

```
┌─────────────────────────────────────────────────────┐
│                    AGNOS Userspace                    │
│  ┌─────────┐ ┌────────┐ ┌───────────┐ ┌──────────┐ │
│  │ daimon  │ │ hoosh  │ │aethersafha│ │ agnoshi  │ │
│  │ (agent) │ │ (LLM)  │ │(compositor)│ │ (shell)  │ │
│  └────┬────┘ └───┬────┘ └─────┬─────┘ └────┬─────┘ │
│       │          │            │             │        │
│  ┌────┴──────────┴────────────┴─────────────┴─────┐  │
│  │              argonaut (init system)             │  │
│  │  boot sequence │ service lifecycle │ health     │  │
│  │  runlevels     │ shutdown planning │ watchdog   │  │
│  │  edge boot     │ process table    │ sd_notify  │  │
│  └────────────────────────────────────────────────┘  │
│                         │                            │
└─────────────────────────┼────────────────────────────┘
                          │
┌─────────────────────────┼────────────────────────────┐
│  Linux Kernel (6.x)     │                            │
│  process management, signals, filesystems, dm-verity │
└──────────────────────────────────────────────────────┘
```

## Module Structure

```
argonaut (library crate)
├── lib.rs          — ArgonautInit orchestrator, service lifecycle methods
├── types.rs        — All public types, enums, configs (~1000 lines)
├── boot.rs         — Boot sequence construction per BootMode
├── services.rs     — Service registry, defaults, dependency resolution (Kahn's)
├── runlevels.rs    — Shutdown/runlevel planning + execution
├── edge_boot.rs    — Read-only rootfs, dm-verity, LUKS, fleet registration
├── process.rs      — Fork/exec, PID tracking, signal delivery, ProcessTable
├── health.rs       — Health check execution, HealthState, HealthHistory
├── notify.rs       — sd_notify protocol listener
└── tests.rs        — 256 tests
```

## Key Design Decisions

| Decision | ADR | Rationale |
|----------|-----|-----------|
| Rust + `forbid(unsafe_code)` | [ADR-001](../adrs/001-language-choice.md) | Memory safety without GC; compile-time state machine guarantees |
| SafeCommand abstraction | [ADR-002](../adrs/002-safe-command-abstraction.md) | Structural elimination of shell injection (CWE-78) |
| Library + binary split | [ADR-003](../adrs/003-library-plus-binary-architecture.md) | Testable brain, unsafe-capable body |
| Edge boot security model | [ADR-004](../adrs/004-edge-boot-security-model.md) | dm-verity + LUKS + read-only rootfs |
| No HTTP dependencies | [ADR-005](../adrs/005-health-check-no-external-deps.md) | Raw TCP health checks, zero additional deps |

## Boot Flow

```
ArgonautInit::new(config)
  │
  ├── build_boot_sequence(mode)    → Vec<BootStep>
  ├── default_services(mode)       → registered in services HashMap
  │
  ▼ Boot execution (caller drives):
  mark_step_complete(MountFilesystems)
  mark_step_complete(StartDeviceManager)
  mark_step_complete(VerifyRootfs)        ← execute_edge_boot() for Edge mode
  mark_step_complete(StartSecurity)
  mark_step_complete(StartDatabaseServices)
  start_service("postgres")               ← spawns process, runs ready check
  start_service("redis")
  start_service("daimon")
  start_service("hoosh")
  mark_step_complete(BootComplete)
  │
  ▼ Runtime loop (caller drives):
  loop {
      reap_services()                     → returns CrashAction per service
      poll_health(&mut tracker)           → returns HealthCheckResult per service
      enforce_watchdog()                  → kills timed-out services
      // handle restart decisions from reap + watchdog
  }
```

## Service State Machine

```
                 ┌─────────┐
                 │ Stopped │
                 └────┬────┘
                      │ start_service()
                      ▼
                 ┌─────────┐
                 │Starting │──── ready check fails ───► Failed
                 └────┬────┘
                      │ ready check passes
                      ▼
                 ┌─────────┐
                 │ Running │──── health check fails ──► Failed
                 └────┬────┘     (via HealthTracker)
                      │ stop_service()
                      ▼
                 ┌─────────┐
                 │Stopping │
                 └────┬────┘
                      │ process exits
                      ▼
                 ┌─────────┐
                 │ Stopped │
                 └─────────┘

  Failed ──► Starting  (if restart policy allows)
  Failed ──► Stopped   (if restart limit exceeded or policy=Never)
```

## Dependencies

| Crate | Version | Purpose | Unsafe? |
|-------|---------|---------|---------|
| anyhow | 1.x | Error handling with context | No |
| chrono | 0.4 | Timestamps for boot/health/events | No |
| nix | 0.31 | Signal delivery, PID management | Yes (internal) |
| serde | 1.x | Serialization for configs and events | No |
| serde_json | 1.x | JSON serialization | No |
| tracing | 0.1 | Structured logging / audit trail | No |

Total transitive dependencies: ~15 (excluding dev-deps).
