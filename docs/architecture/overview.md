# Argonaut Architecture Overview

## System Context

Argonaut is the init system and service manager for AGNOS. It takes the system from
"the kernel handed off control" to "every service running and healthy", and back down
again.

It ships two things:
- **A library** (`src/*.cyr`). AGNOS's production PID 1, kybernet, links it in-process
  and drives the service lifecycle from its own event loop.
- **A minimal PID-1 binary** (`src/main.cyr`). It mounts, blocks signals onto a
  signalfd, waits in poll, reaps, syncs and powers off. It does not run the service
  lifecycle, and the qemu harness boots it as PID 1 (architecture note 002).

```
┌──────────────────────────────────────────────────────┐
│                   AGNOS userspace                    │
│  daimon · llm-gateway (hoosh) · aethersafha · agnoshi │
├──────────────────────────────────────────────────────┤
│  kybernet (PID 1): event loop, config, policy hook   │
│   └─ argonaut library                                │
│        boot sequence │ service lifecycle │ health    │
│        runlevels     │ ordered shutdown  │ watchdog  │
│        edge boot     │ process table     │ sd_notify │
│        audit chain   │ unit / tmpfiles   │ pre-exec  │
├──────────────────────────────────────────────────────┤
│  Linux kernel: processes, signals, filesystems,      │
│  dm-verity, LUKS                                     │
└──────────────────────────────────────────────────────┘
```

## Module Structure

```
argonaut
├── syscall_compat.cyr — per-arch wrappers for syscalls the stdlib does not wrap
├── types.cyr          — core enums, structs, configs and accessors
├── boot.cyr           — boot sequence per BootMode
├── services.cyr       — default services; dependency resolution (one Kahn's pass → waves)
├── process_mgmt.cyr   — fork/exec, the pre-exec hook, process table, signals, PID files
├── resolver.cyr       — IPv4 literals and /etc/hosts lookups for health checks
├── health.cyr         — HTTP / TCP / command / process-alive checks, state, history
├── edge_boot.cyr      — read-only rootfs, dm-verity, LUKS, fleet registration
├── notify.cyr         — sd_notify server (authenticated) and client
├── security.cyr       — policy descriptions (not enforced); emergency-shell credential
├── systemd.cyr        — systemd unit generation
├── tmpfiles.cyr       — tmpfiles entries: native symlink-safe apply, command rendering
├── audit.cyr          — the libro bridge (ServiceEventType → audit chain)
├── audit_ext.cyr      — persistence (patra), signed snapshots, merkle proofs
├── init.cyr           — the ArgonautInit orchestrator: lifecycle, health, shutdown
├── pid1_harness.cyr   — the qemu M3 / L3 self-tests
├── bench_main.cyr     — the release benchmark gate
└── main.cyr           — entry point; the minimal PID-1 loop
```

## Key Design Decisions

| Decision | ADR | Rationale |
|----------|-----|-----------|
| Language | [ADR-001](../adrs/001-language-choice.md) | Originally Rust, ported to Cyrius at 0.95.0: a sovereign toolchain and a small static binary |
| SafeCommand abstraction | [ADR-002](../adrs/002-safe-command-abstraction.md) | No shell, so no shell injection (CWE-78) |
| Library + binary split | [ADR-003](../adrs/003-library-plus-binary-architecture.md) | A testable library; kybernet is the production PID 1 |
| Edge boot security model | [ADR-004](../adrs/004-edge-boot-security-model.md) | dm-verity + LUKS + read-only rootfs |
| No HTTP dependencies | [ADR-005](../adrs/005-health-check-no-external-deps.md) | A raw-socket HTTP/1.0 client, no additional deps |
| Wave-based startup and shutdown | [ADR-008](../adrs/008-wave-based-parallel-startup.md) | Parallel waves, deterministic order, reversed for shutdown |
| Policy enforced by the consumer | [ADR-013](../adrs/013-policy-enforcement-in-the-consumer.md) | argonaut owns the fork→exec window; the consumer's hook applies seccomp / Landlock / capabilities |

## Boot Flow (as a consumer drives it)

```
argonaut_init_new(config)
  ├── build_boot_sequence(mode)        → BootSteps
  ├── default_services(mode)           → registered services
  └── audit chain                      → streaming by default; retained or persistent by config
  │
  ▼ boot (the caller drives it)
  init_mark_step_complete(init, STAGE_MOUNT_FS)
  init_mark_step_complete(init, STAGE_VERIFY_ROOTFS)   ← execute_edge_boot() in Edge mode
  …
  for each wave of init_boot_execution_plan_waves(init):
      init_start_service(init, name)   → STARTING; RUNNING after the ready check
                                         (SVC_NOTIFY: after an authenticated READY=1)
  init_mark_step_complete(init, STAGE_BOOT_COMPLETE)
  │
  ▼ supervisor loop (the caller drives it)
  loop {
      init_reap_services(init)         → crash handling, restarts, audit
      init_poll_health(init, tracker)  → audited on change, plus an hourly heartbeat
      init_enforce_watchdog(init)      → kills services that missed WATCHDOG=1
  }
  │
  ▼ shutdown
  init_execute_shutdown(init, budget)  → reverse waves, SIGTERM then process-group SIGKILL
```

## Service State Machine

`valid_transition` in `src/types.cyr` is the table:

```
  Stopped  ──► Starting
  Starting ──► Running    (ready check passes; SVC_NOTIFY: READY=1)
  Starting ──► Failed     (start fails, ready check fails, or start timeout)
  Starting ──► Stopping   (stopped before ready)
  Running  ──► Stopping   (init_stop_service, shutdown)
  Running  ──► Failed     (crash reaped, or watchdog kill)
  Stopping ──► Stopped    (process exits)
  Stopping ──► Failed
  Failed   ──► Starting   (restart policy allows, within the restart window)
  Failed   ──► Stopped    (restart limit reached, or policy = never)
```

Health checks record and audit results; they do not change state themselves.

## Dependencies

| Library | Purpose |
|---------|---------|
| libro | Hash-linked audit chain, persistence, signing, merkle proofs (`dist/libro.cyr`, git tag) |
| sigil | Thin sub-bundles: SHA-256 (+ SHA-NI), hex, ed25519 / ML-DSA, Argon2id (`[deps.sigil]`, libro's tag) |
| patra | Storage behind optional audit persistence (stdlib fold) |
| sakshi | Structured tracing (stdlib fold) |

The full stdlib list is in `cyrius.cyml` `[deps] stdlib`. Current pins, binary sizes and
suite counts are in [`docs/development/state.md`](../development/state.md). The binary is
a statically linked ELF with no libc.
