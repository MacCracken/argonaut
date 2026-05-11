# Argonaut Roadmap

Completed items live in [CHANGELOG.md](../../CHANGELOG.md). Live state
(version, binary size, suite count, dep pins) lives in
[`docs/development/state.md`](state.md). This file is forward-looking
work only.

---

## Current — v1.5.2 (shipped 2026-05-10)

HIGH-1 host-resolver follow-up patch. `src/resolver.cyr` adds a
strict IPv4 dotted-quad parser (rejects CVE-2021-29923-style
leading-zero ambiguity) + `/etc/hosts` scan;
`check_tcp_connect` and HTTP_GET route via `resolve_host_ipv4`
rather than hardcoding 127.0.0.1, with distinct messages for
resolver miss vs. connect failure vs. unreachable. HTTP `Host:`
header echoes the configured host. `exec_env` Str/cstr quirk
filed upstream as a cyrius issue. Side benefit: sigil 3.0.1's
dist was re-published upstream between 1.5.1 ship and 1.5.2
work-start with `ct_eq` restored, so the 1.5.1 `src/compat.cyr`
shim + `[deps.argonaut_compat]` self-dep retire one minor early.
See [CHANGELOG 1.5.2](../../CHANGELOG.md#152--2026-05-10) for
full disposition.

The 1.5.x arc continues: libro extended surface (1.5.3) →
cross-arch (1.5.4) → closeout audit (1.5.5). 1.6.x picks up the
QEMU harness and the carry-forward cleanups.

---

## Next — v1.5.3 — Libro extended surface

Theme: pull forward the libro 2.x audit-chain features argonaut
hasn't adopted yet. Libro 2.6.2 ships the signing / anchoring /
merkle / streaming surface; argonaut's `audit_log_*` wrappers in
`src/audit.cyr` currently consume only the in-memory chain APIs.

- [ ] **AuditChain on-disk persistence** — wire libro's PatraStore
  audit-entry persistence so the chain survives across argonaut
  restarts. Default to off; opt-in via `argonaut_config` flag.
- [ ] **Signed audit entries** — adopt libro's signing module
  (Ed25519 entry signatures) for tamper-evident shutdown /
  runlevel records. Key management via sigil.
- [ ] **Merkle batching** — libro's merkle module for chain
  batches; cuts verify cost on long-running argonaut sessions
  (relevant once persistence lands and chains span boots).
- [ ] Regression coverage in `tests/tcyr/audit_*.tcyr` for each
  feature; bench impact tracked in `bench-history.csv`.

---

## v1.5.4 — Cross-arch

Theme: restore aarch64 builds. `cc5_aarch64` has shipped in the
toolchain since 5.5.x; argonaut hasn't been cross-built since
the cc3 era.

- [ ] **aarch64 cross-build** — `CYRIUS_DCE=1 cyrius build --aarch64
  src/main.cyr build/argonaut-aarch64`. Mirror the pattern
  agnosys / agnostik use in their CI (best-effort if
  `cc5_aarch64` isn't in the toolchain bin dir, hard requirement
  once it is).
- [ ] **CI cross-build step** — add to `.github/workflows/ci.yml`
  after the x86_64 build; verify ELF magic + `file` reports
  aarch64. Release workflow publishes `argonaut-<V>-aarch64-linux`
  alongside x86_64.
- [ ] **aarch64 smoke / test sweep** — gated on a CI runner with
  aarch64 capacity (qemu-user emulation acceptable for the
  smoke; native required for the full `.tcyr` sweep).
- [ ] **Real-hardware validation** — RPi4 + Apple Silicon boot
  smoke once binaries publish.

---

## v1.5.5 — 1.5.x closeout P(-1) audit

Theme: arc-closing security re-pass before 1.6.0 tagging. One of
the 2026-04-26 audit's four re-audit triggers is argonaut
graduating to true PID 1; while that lands in 1.6.x, the 1.5.x
arc still earns its own closeout audit covering the libro
extended surface (persistence, signing, merkle) + cross-arch
syscall surface added in 1.5.4.

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

### Carry-forwards from 1.5.x

- [ ] **Drop `src/compat.cyr` shim** — remove the `ct_eq` alias
  once libro releases a version that calls `ct_eq_bytes_lens`
  directly. Remove the `[deps.argonaut_compat]` self-dep at the
  same time. Track upstream libro releases.
- [ ] **Rename `audit_log_new` wrapper** — sigil 3.0.1's dist
  defines `audit_log_new()`; argonaut's `src/audit.cyr:91`
  shadows it (last-wins, benign but noisy at compile time).
  Rename to `argonaut_audit_log_new` once kybernet (the
  consumer) is ready to follow.

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
