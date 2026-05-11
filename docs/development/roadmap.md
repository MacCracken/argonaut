# Argonaut Roadmap

Completed items live in [CHANGELOG.md](../../CHANGELOG.md). Live state
(version, binary size, suite count, dep pins) lives in
[`docs/development/state.md`](state.md). This file is forward-looking
work only.

---

## Current — v1.6.3 (shipped 2026-05-11) — 1.6.x arc CLOSED

L3 end-to-end lands via `qemu/helpers/l3-helper.cyr` — a 12 KB
statically-linked cyrius helper that writes `sid=N pid=M\n` to
`/l3.marker` via raw syscalls. No shell, no dyn-loader,
sidesteps the busybox-shell blockers from 1.6.2. Spawned by
`fork_exec_service`; the harness wrapper greps the marker +
asserts `sid == pid` (proves setsid ran before exec). 1.6.x
closeout P(-1) audit per CLAUDE.md procedure:
**0 CRITICAL / 0 HIGH**; 2 MEDIUM closed with regression tests
(`fork_exec_service` child inherited the PID-1 sigmask block;
empty envp dropped PATH for spawned services); 3 LOW (1
closed, 2 documented). Closes the 2026-04-26 audit's PID-1
graduation re-audit trigger. See
[CHANGELOG 1.6.3](../../CHANGELOG.md#163--2026-05-11) and
[`docs/audit/2026-05-11-audit.md`](../audit/2026-05-11-audit.md).

The 1.6.x arc is CLOSED. PID-1 surface is now
production-grade: boot smoke (1.6.0), clean SIGTERM/SIGINT
shutdown via signalfd (1.6.2), orphan reap under real-PID-1
reparenting (M3, 1.6.2), `fork_exec_service` controlling-TTY
decoupling validated via setsid (L3, 1.6.3). The static
helper pattern unlocks future end-to-end tests for any
service-lifecycle behaviour that needs PID-1 validation.

---

## Open — gated on external work

### Native aarch64

- [ ] **Native aarch64 CI runner** — close the qemu-user
  emulation gap from 1.5.4 (`audit-m3-reaper-orphans`,
  `audit-l3-fork-setsid` require real fork/setsid semantics).
  Gated on runner allocation; the aarch64 binary is ready.
  Note: M3 + L3 are now also covered end-to-end under x86_64
  PID-1 via `qemu/pid1-harness-test.sh`, so the aarch64
  blocker is per-arch validation, not per-finding.
- [ ] **Real-hardware smoke** — RPi4, Apple Silicon (Asahi
  Linux), Graviton / Ampere cloud aarch64. Once a runner exists
  the matrix is just `boot + audit_findings + audit_extended +
  pid1-harness`.

### Per-service env override

- [ ] **`fork_exec_service` map → flat-cstrs** — 1.6.3
  shipped a default envp containing only PATH. Consumers
  needing per-service env (HOME, locale, LD_LIBRARY_PATH, etc.)
  surface a need for the full `svc_def_env` map → flat
  `KEY=VAL` cstr conversion. Lands when the first such
  consumer appears.

### Gated on external work

- [ ] **WitnessAnchor publishing** — libro's anchor primitive
  for cross-snapshot trust pins. Gated on consumer demand +
  AGNOS federation protocol (libro's own roadmap calls this
  out under "Ecosystem-blocked").
- [ ] **Durable signing-key rotation** — current 1.5.3 shape
  generates ephemeral signing keys per `audit_log_keygen()`
  call. Long-running supervisor sessions across boots want a
  persisted key. Lands when kybernet wires a real
  key-management surface; argonaut's API stays stable.

---

## Kybernet integration (separate repo)

Tracked in [kybernet](https://github.com/MacCracken/kybernet). Argonaut
keeps the API stable; consumer wiring is kybernet-side work per the
project-boundaries rule. Argonaut 1.0+ exposes everything kybernet
needs.

- [ ] Wire kybernet to `init_start_service` / `init_stop_service`
- [ ] Wire kybernet to `boot_execution_plan_waves`
- [ ] Wire kybernet to `init_plan_shutdown`
- [ ] Seccomp/Landlock application in `pre_exec`
- [ ] Control socket for agnoshi runtime commands
- [ ] Real-hardware boot validation (RPi4, NUC)

---

## v1.0.0 Criteria — All Met (2026-04-12)

Retained for historical context.

- [x] All P0 / P1 library gaps closed
- [x] API stable (ADR-012)
- [x] 12 ADRs for major design decisions
- [x] Security posture documented and reviewed
- [x] QEMU boot: minimal < 3s (2.98s); desktop < 3s (2.9s with real daimon)
- [x] Crash recovery (exponential backoff, restart limit, GiveUp)
- [x] Shutdown ordering (clean stop → sync → poweroff)
- [x] Sakshi tracing integrated
- [x] Cyrius port complete, rust-old removed (v0.96.1)
- [x] Libro 1.0.3 → 2.0.5 audit chain (real SHA-256, not shim)
- [x] Lifecycle audit recording

---

## Non-goals

- **Container orchestration** — that's stiva
- **Package installation** — that's ark
- **Agent lifecycle** — that's daimon (argonaut manages the process,
  daimon manages the agent)
- **Scheduling** — that's samay (argonaut starts/stops, samay decides
  when)
- **D-Bus interface** — only if AGNOS desktop requires it
- **Timer-based services** — that's samay's domain
