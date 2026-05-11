# Argonaut Roadmap

Completed items live in [CHANGELOG.md](../../CHANGELOG.md). Live state
(version, binary size, suite count, dep pins) lives in
[`docs/development/state.md`](state.md). This file is forward-looking
work only.

---

## Current — v1.5.4 (shipped 2026-05-10)

Cross-arch — restores aarch64 builds. cyrius's `cc5_aarch64`
translator converts syscalls + ABI at codegen, so no argonaut
source changes were needed. CI + release publish
`argonaut-<VER>-aarch64-linux` alongside x86_64 as best-effort
(skips without failing when toolchain doesn't bundle the
translator). `scripts/aarch64-sweep.sh` runs the full `.tcyr`
sweep under qemu-user with a documented known-failure budget:
2 suites trip qemu emulation limits + an upstream sigil
Ed25519 aarch64 verify quirk (filed against sigil with minimal
repro). `docs/architecture/001-cross-arch-aarch64.md` is the
canonical reference. See
[CHANGELOG 1.5.4](../../CHANGELOG.md#154--2026-05-10).

The 1.5.x arc closes with the P(-1) audit (1.5.5). 1.6.x picks
up the QEMU PID-1 harness, native aarch64 CI runner (closes
qemu-user gap), and carry-forward cleanups (audit_log_new
rename, anchor publish, durable signing-key rotation).

---

## Next — v1.5.5 — 1.5.x closeout P(-1) audit

Theme: arc-closing security re-pass before 1.6.0 tagging. One of
the 2026-04-26 audit's four re-audit triggers is argonaut
graduating to true PID 1; while that lands in 1.6.x, the 1.5.x
arc still earns its own closeout audit covering the libro
extended surface (persistence, signing, merkle) + the aarch64
cross-arch syscall surface added in 1.5.4.

- [ ] **P(-1) full pass** — per CLAUDE.md's procedure: roadmap
  review → cleanliness gate → bench baseline → internal deep
  review → external research (CVEs, init/service-manager
  0-days) → security audit → regression tests for findings →
  post-audit benches → doc sweep.
- [ ] **Audit report** — `docs/audit/YYYY-MM-DD-audit.md` with
  severity tags. Every MEDIUM+ earns a failing regression test
  before the fix.
- [ ] **Closeout pass** — full test suite, bench snapshot vs
  prior closeout label, dead-code floor, refactor + cleanup
  sweep, downstream consumer check (kybernet builds clean
  against tagged 1.5.5).

---

## v1.6.x arc — PID-1 graduation + carry-forwards

Theme: end-to-end validate argonaut as true PID 1, and clear the
carry-forward items from the 1.5.x arc.

### PID-1 harness

- [ ] **QEMU PID-1 boot harness** — minimal initramfs + kernel
  boot + assertion harness that runs argonaut as PID 1.
  Validates M3 (orphan reap under real PID-1 reparenting) and
  L3 (controlling-TTY decoupling) end-to-end. Sibling repo
  [kybernet](https://github.com/MacCracken/kybernet) has the
  shape under `qemu/` — pull pattern, not code.
- [ ] **Re-audit on PID-1 graduation** — trigger from the
  2026-04-26 audit; runs after the harness lands as the gating
  re-audit for 1.6.x.

### Native aarch64

- [ ] **Native aarch64 CI runner** — close the qemu-user
  emulation gap from 1.5.4 (`audit-m3-reaper-orphans`,
  `audit-l3-fork-setsid` require real fork/setsid semantics).
  Gated on runner allocation; the aarch64 binary is ready.
- [ ] **Real-hardware smoke** — RPi4, Apple Silicon (Asahi
  Linux), Graviton / Ampere cloud aarch64. Once a runner exists
  the matrix is just `boot + audit_findings + audit_extended`.

### Carry-forwards from 1.5.x

- [ ] **Rename `audit_log_new` wrapper** — sigil 3.0.1's dist
  defines `audit_log_new()`; argonaut's `src/audit.cyr:91`
  shadows it (last-wins, benign but noisy at compile time).
  Rename to `argonaut_audit_log_new` once kybernet (the
  consumer) is ready to follow.
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
