# Argonaut Roadmap

Completed items live in [CHANGELOG.md](../../CHANGELOG.md). Live state
(version, binary size, suite count, dep pins) lives in
[`docs/development/state.md`](state.md). This file is forward-looking
work only.

---

## Current — v1.4.0 (shipped 2026-04-26)

P(-1) hardening minor. Eight audit findings landed with regression
tests, three deferred to 1.4.x patches with helpers in place. CLAUDE.md
split into durable rules + `state.md` volatile snapshot. See
[`docs/audit/2026-04-26-audit.md`](../audit/2026-04-26-audit.md) for
the full report and [CHANGELOG 1.4.0](../../CHANGELOG.md#140--2026-04-26)
for the disposition.

---

## Next — v1.5.0 — PID-1 readiness

Theme: graduate from systemd-delegate to a credible PID-1 candidate
for the AGNOS boot path. This rolls up the 1.4.x audit deferrals
into one coherent minor and unblocks the AGNOS-boot consumer.

### Audit deferrals (from 2026-04-26 audit)

- [ ] **MEDIUM-1** — wire `notify_recv_authenticated`
  (`SO_PEERCRED` / `SCM_CREDENTIALS`) into `init_poll_health`. Helper
  shipped unwired in 1.4.0; consumer adoption is the remaining work.
- [ ] **MEDIUM-3** — `proc_table_reap_orphans` +
  `prctl(PR_SET_CHILD_SUBREAPER)` enrol in `argonaut_init_new`.
  Gated on the QEMU PID-1 boot test harness (below).
- [ ] **LOW-3** — `setsid` + stdout/stderr `dup2` in
  `fork_exec_service`. Same QEMU-harness gating as MEDIUM-3.

### Infrastructure

- [ ] **QEMU PID-1 boot harness** — minimal initramfs that runs
  argonaut as PID 1, asserts service waves come up, asserts orphan
  reap behaviour. Unlocks MEDIUM-3 / LOW-3 testing.

### Re-audit

- [ ] Per the 2026-04-26 audit's re-audit triggers, graduating to
  true PID 1 is one of the four. Schedule a fresh P(-1) pass before
  tagging 1.5.0.

---

## v1.6.0 — health-check resolver

- [ ] **HIGH-1 follow-up** — replace the localhost-only health-check
  gate (introduced in 1.4.0) with a real host resolver. Either
  dotted-quad parser + AAAA literal handling, or a minimal
  `gethostbyname`/`getaddrinfo` consumer. Restores the feature surface
  the 1.4.0 fix had to drop to preserve the safety property.

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
