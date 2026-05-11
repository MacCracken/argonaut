# Argonaut Roadmap

Completed items live in [CHANGELOG.md](../../CHANGELOG.md). Live state
(version, binary size, suite count, dep pins) lives in
[`docs/development/state.md`](state.md). This file is forward-looking
work only.

---

## Current — v1.6.2 (shipped 2026-05-10) — PID-1 harness extensions (partial)

`src/pid1_harness.cyr` adds opt-in self-test mode via
`/proc/cmdline argonaut.harness=1`. **M3 end-to-end** validates
orphan reap under real-PID-1 reparenting inside qemu;
`src/main.cyr` signalfd-blocked SIGTERM/SIGINT/SIGCHLD wires
clean shutdown via `sys_reboot(RB_POWER_OFF)`. Discovered + fixed
a double-fork bug in `fork_exec_service` (pre-1.6.2 the nested
`exec_env_str` fork made `setsid` apply to the wrong process).
**L3 end-to-end deferred to 1.6.3** — prototyped but hit
compounded blockers (dynamically-linked busybox → execve ENOENT
unless ld-linux is bundled; then a parent-side waitpid hang
specific to PID-1). See
[CHANGELOG 1.6.2](../../CHANGELOG.md#162--2026-05-10).

The next slot picks up L3 end-to-end and the arc-closing
P(-1) audit.

---

## Next — v1.6.3 — L3 end-to-end + closeout P(-1) audit

### L3 end-to-end

- [ ] **Static test helper in initramfs** — cleanest path: a
  small statically-linked C/cyrius helper bundled into the
  initramfs that argonaut spawns via `fork_exec_service` and
  that writes `sid pid` to `/l3.marker` directly via syscalls
  (no shell, no dyn-loader, no busybox path). Helper sources
  in `qemu/helpers/` next to `build-initramfs.sh`.
- [ ] **OR — root-cause the parent-side waitpid hang** — the
  1.6.2 prototype with bundled ld-linux had execve succeeding
  but the parent's `sys_waitpid(spid, ...)` not returning.
  Diagnose with strace under qemu (kernel logs to serial)
  or with a custom signal handler that prints SIGCHLD.

### 1.6.x arc closeout (v1.6.3)

- [ ] **P(-1) full pass** — mirrors the 1.5.5 closeout shape.
  Roadmap review → cleanliness gate → bench baseline → internal
  deep review (PID-1 surface, supervisor loop, signal handling
  once landed) → external research → audit report → failing
  regression tests for findings → fixes → post-audit benches
  → cleanup → downstream check → full clean build.
- [ ] **Audit report** — `docs/audit/YYYY-MM-DD-audit.md`;
  every MEDIUM+ earns a failing regression test before the
  fix.

### Native aarch64

- [ ] **Native aarch64 CI runner** — close the qemu-user
  emulation gap from 1.5.4 (`audit-m3-reaper-orphans`,
  `audit-l3-fork-setsid` require real fork/setsid semantics).
  Gated on runner allocation; the aarch64 binary is ready.
- [ ] **Real-hardware smoke** — RPi4, Apple Silicon (Asahi
  Linux), Graviton / Ampere cloud aarch64. Once a runner exists
  the matrix is just `boot + audit_findings + audit_extended`.

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
