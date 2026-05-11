# Argonaut Roadmap

Completed items live in [CHANGELOG.md](../../CHANGELOG.md). Live state
(version, binary size, suite count, dep pins) lives in
[`docs/development/state.md`](state.md). This file is forward-looking
work only.

---

## Current — v1.5.5 (shipped 2026-05-10) — 1.5.x arc CLOSED

Arc-closing P(-1) audit per CLAUDE.md procedure. Findings in
[`docs/audit/2026-05-10-audit.md`](../audit/2026-05-10-audit.md):
**0 CRITICAL / 0 HIGH**; 3 MEDIUM closed with regression tests
(`lookup_etc_hosts` heap leak; persistent log silent disk-fail;
persistent log replay accepted tampered chain); LOW-1/2/3 closed
(TCP pre-resolve split, HTTP port-range gate, `Host:` header
sanitize gate); LOW-4 documented. 2 UPSTREAM (sigil dist tag
instability mitigated via permanent `src/compat.cyr` shim;
sigil Ed25519-aarch64 verify quirk filed at 1.5.4 pre-audit).
Orphan `src/test_*.cyr` stubs removed (predate `tests/tcyr/`).
kybernet BC-clean against the 1.5.5 surface.

The 1.5.x arc is CLOSED. 1.6.x picks up the QEMU PID-1 harness
+ PID-1 graduation re-audit + native aarch64 CI runner +
carry-forward cleanups (`audit_log_new` rename, anchor publish,
durable signing-key rotation).

---

## Next — v1.6.x arc — PID-1 graduation

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
