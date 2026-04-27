# Argonaut Roadmap

Completed items live in [CHANGELOG.md](../../CHANGELOG.md). Live state
(version, binary size, suite count, dep pins) lives in
[`docs/development/state.md`](state.md). This file is forward-looking
work only.

---

## Current — v1.5.0 (shipped 2026-04-27)

PID-1 readiness minor. Closes the three audit deferrals from
1.4.0 (MEDIUM-1, MEDIUM-3, LOW-3) with regression coverage.
QEMU PID-1 boot harness (gating end-to-end M3 / L3 validation
per the audit) and the HIGH-1 host resolver slip to 1.6.0. See
[CHANGELOG 1.5.0](../../CHANGELOG.md#150--2026-04-27) for the
full disposition.

### Audit deferrals — closed in 1.5.0

- [x] **MEDIUM-1** — `notify_try_recv_authenticated` (recvmsg +
  SCM_CREDENTIALS) added to `src/notify.cyr`; `notify_bind`
  enables `SO_PASSCRED`; `init_notify_bind` opt-in + `notify_fd`
  field on `ArgonautInit`; `init_poll_health` drains
  authenticated messages when the fd is registered.
- [x] **MEDIUM-3** — `proc_table_reap_orphans` (bounded
  `waitpid(-1, ..., WNOHANG)` loop) added; `argonaut_init_new`
  calls `prctl(PR_SET_CHILD_SUBREAPER, 1)`; `init_reap_services`
  drains orphans after tracked-PID reaping.
- [x] **LOW-3** — `fork_exec_service` child now `setsid`s and
  `dup2`s stdout / stderr to `/dev/null` after the existing stdin
  redirect.

---

## Next — v1.6.0 — QEMU harness + health-check resolver

Theme: end-to-end validate the PID-1 surface, restore the
non-loopback health-check feature, and clean up the stdlib
quirk that blocks shell-exec testing.

### Infrastructure

- [ ] **QEMU PID-1 boot harness** — minimal initramfs + kernel
  boot + assertion harness that runs argonaut as PID 1.
  Validates M3 (orphan reap under real PID-1 reparenting) and
  L3 (controlling-TTY decoupling) end-to-end. Sibling repo
  [kybernet](https://github.com/MacCracken/kybernet) has the
  shape under `qemu/` — pull pattern, not code.
- [ ] **`lib/process.cyr` `exec_env` Str/cstr fix** — upstream
  stdlib issue. Blocks unit-level shell-exec testing across
  `health_exec.tcyr` and `audit-l3-fork-setsid`. File against
  the cyrius stdlib; consume via toolchain bump.

### Audit follow-ups

- [ ] **HIGH-1 follow-up** — replace the 1.4.0
  reject-non-loopback gate in health checks with a real host
  resolver (dotted-quad parser + `gethostbyname` or
  `getaddrinfo`). Restores the feature surface the 1.4.0 fix
  had to drop.

### Re-audit

- [ ] One of the 2026-04-26 audit's four re-audit triggers is
  argonaut graduating to true PID 1. The QEMU harness lands
  that ground; schedule a fresh P(-1) pass before tagging
  1.6.0.

---

## Libro extended features (Post-1.0, pulled forward as needed)

Libro 2.0.5 ships the signing / anchoring / merkle / streaming
surface; argonaut consumes the audit-chain APIs only. These items
extend that surface:

- [ ] **AuditChain on-disk persistence** — wire libro's PatraStore
  audit-entry persistence (libro 2.0 ships it; argonaut's
  `audit_log_*` wrappers in `src/audit.cyr` currently only persist
  the in-memory chain).
- [ ] **Signed audit entries** — adopt libro's signing module
  (Ed25519 entry signatures) for tamper-evident shutdown / runlevel
  records.
- [ ] **Merkle batching** — libro's merkle module for chain batches;
  cuts verify cost on long-running argonaut sessions.

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

## Cross-arch

- [ ] **aarch64 / Apple Silicon build** — `cc5_aarch64` ships in the
  toolchain since 5.5.x; argonaut hasn't been cross-built since the
  cc3 era. Lift planned for a future minor; gated on a CI runner with
  aarch64 capacity.

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
