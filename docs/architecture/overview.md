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
│  │  security      │ audit trail      │ tracing    │  │
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
argonaut (Cyrius library)
├── types.cyr       — All public types, enums, configs (~1000 lines)
├── boot.cyr        — Boot sequence construction per BootMode
├── services.cyr    — Service registry, defaults, dependency resolution (Kahn's)
├── process_mgmt.cyr — Fork/exec, PID tracking, signal delivery, ProcessTable
├── health.cyr      — Health check execution, HealthState, HealthHistory
├── edge_boot.cyr   — Read-only rootfs, dm-verity, LUKS, fleet registration
├── notify.cyr      — sd_notify protocol listener (READY=1, STATUS, MAINPID)
├── security.cyr    — Seccomp, Landlock, capabilities, socket activation
├── systemd.cyr     — Hybrid systemd unit generation
├── tmpfiles.cyr    — Boot-time filesystem setup (directories, symlinks, devices)
├── audit.cyr       — Libro bridge (ServiceEventType → AuditChain)
├── init.cyr        — ArgonautInit orchestrator, service lifecycle, audit recording
└── main.cyr        — Entry point
```

## Key Design Decisions

| Decision | ADR | Rationale |
|----------|-----|-----------|
| Cyrius language | [ADR-001](../adrs/001-language-choice.md) | Sovereign toolchain, 373KB binary, no external deps |
| SafeCommand abstraction | [ADR-002](../adrs/002-safe-command-abstraction.md) | Structural elimination of shell injection (CWE-78) |
| Library + binary split | [ADR-003](../adrs/003-library-plus-binary-architecture.md) | Testable library (argonaut), PID 1 binary (kybernet) |
| Edge boot security model | [ADR-004](../adrs/004-edge-boot-security-model.md) | dm-verity + LUKS + read-only rootfs |
| No HTTP dependencies | [ADR-005](../adrs/005-health-check-no-external-deps.md) | Raw TCP health checks, zero additional deps |

## Boot Flow

```
ArgonautInit::new(config)
  │
  ├── build_boot_sequence(mode)    → vec of BootStep
  ├── default_services(mode)       → registered in services map
  ├── audit_log_new()              → libro AuditChain
  │
  ▼ Boot execution (caller drives):
  mark_step_complete(MountFilesystems)
  mark_step_complete(StartDeviceManager)
  mark_step_complete(VerifyRootfs)        ← execute_edge_boot() for Edge mode
  mark_step_complete(StartSecurity)
  mark_step_complete(StartDatabaseServices)
  start_service("postgres")               ← spawns process, runs ready check
  start_service("redis")                     audit: STARTING → STARTED
  start_service("daimon")
  start_service("hoosh")
  mark_step_complete(BootComplete)
  │
  ▼ Runtime loop (caller drives):
  loop {
      reap_services()                     → CrashAction, audit: CRASH_DETECTED
      poll_health(&tracker)               → audit: HEALTH_PASSED/FAILED
      enforce_watchdog()                  → audit: TIMEOUT_KILLED
      // handle restart decisions
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

| Library | Purpose |
|---------|---------|
| sigil | SHA-256, hex encoding, constant-time comparison (for libro) |
| bigint | Arbitrary precision integers (for sigil) |
| chrono | Timestamp formatting (RFC 3339) |
| libro 1.0.2 | Cryptographic audit chain (7 core modules) |
| sakshi_full 0.7.0 | Structured tracing (spans, ring buffer, UDP) |

Cyrius stdlib: string, fmt, alloc, vec, str, syscalls, io, fs, process, hashmap, tagged, args, json, freelist.

373KB statically linked ELF x86_64. No libc, no external runtime dependencies.
