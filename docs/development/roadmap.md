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

The 1.6.x arc is CLOSED for the PID-1 graduation theme: boot
smoke (1.6.0), clean SIGTERM/SIGINT shutdown via signalfd
(1.6.2), orphan reap under real-PID-1 reparenting (M3, 1.6.2),
`fork_exec_service` controlling-TTY decoupling validated via
setsid (L3, 1.6.3). The static helper pattern unlocks future
end-to-end tests for any service-lifecycle behaviour that
needs PID-1 validation.

One arc-extension slot follows before 1.7.0: **1.6.4 — native
aarch64 CI** (scope below). Manual real-hardware Pi smoke
already validated via `scripts/aarch64-pi-smoke.sh` (added
post-1.6.3) — argonaut runs cleanly on a real RPi4, init in
~536 µs. CI is the durable form.

---

## Next — v1.6.4 — Native aarch64 CI

Theme: durable per-arch validation. The 1.5.4 qemu-user sweep
(`scripts/aarch64-sweep.sh`) catches codegen regressions but
not real-hardware behaviour (fork/setsid emulation gaps; the
1.5.4-filed sigil Ed25519-aarch64 quirk reproduces under
qemu-user but its real-hardware status is unknown). The
1.6.3-shipped `scripts/aarch64-pi-smoke.sh` proves the
real-hw path works end-to-end but is manual — a developer has
to run it. CI makes it automatic per push.

### Scope

- [ ] **Native aarch64 GitHub Actions runner** — either
  `ubuntu-24.04-arm` (GitHub-hosted, free for public repos
  since 2024-09) or a self-hosted Raspberry Pi runner the
  AGNOS infra fleet already runs for sibling repos. Decide
  based on agnos / kybernet's choice; consistency across the
  ecosystem outweighs per-repo optimization.
- [ ] **CI job: aarch64 build + test sweep** — adds an
  `aarch64-native` job to `.github/workflows/ci.yml`,
  parallel to the existing x86_64 job. Builds via
  `cycc_aarch64` (was `cc5_aarch64` pre-Cyrius-6.0), runs
  the full `.tcyr` suite natively, runs
  the qemu PID-1 harness if KVM is available (qemu-system-aarch64
  + `-cpu host` on a real aarch64 host). No `qemu-user`
  emulation — this is the real-arch coverage.
- [ ] **`tests/tcyr/aarch64_findings.tcyr`** — new
  arch-specific test file separate from `audit_findings.tcyr`
  (which stays portable / arch-agnostic). Houses the
  real-hw retest assertions for the 1.5.4-filed sigil Ed25519
  aarch64 quirk and any other arch-gated findings as they
  surface. Run by the native-aarch64 CI job and the manual
  `scripts/aarch64-pi-smoke.sh`; skipped under x86_64 sweeps
  (gated by a small arch probe at suite entry).
- [ ] **Re-test the upstream sigil Ed25519 aarch64 quirk** on
  real hardware — the 1.5.4-filed issue (`ed25519_verify`
  accepts wrong pk on aarch64) was reproduced under qemu-user.
  If real-hw passes, file an update against the sigil issue
  ("qemu-user-only; real hardware clean") so sigil can scope
  the fix accordingly. Test lives in
  `tests/tcyr/aarch64_findings.tcyr`.
- [ ] **Per-arch known-failure budget** — `scripts/aarch64-sweep.sh`
  already has the budget pattern (KNOWN-FAIL accounting); the
  native runner won't have the qemu-user emulation
  exceptions (fork/setsid land cleanly on real hardware), so
  the budget should be empty or near-empty. If a CI run
  surfaces a real-arch-only failure, that's the signal to
  file a new sigil / cyrius / libro issue and (if MEDIUM+)
  add a regression assertion to `aarch64_findings.tcyr`.
- [ ] **Release publish gate** — once the CI job is green,
  flip `argonaut-<VER>-aarch64-linux` in release.yml from
  "best-effort if `cycc_aarch64` exists" to "hard
  requirement when the CI job passes" — every release ships
  a tested aarch64 binary, not just a built one.

### Out of scope

- **Apple Silicon (Asahi Linux)**: same arch as the RPi but
  different boot path; would be a third runner. Defer to
  consumer demand.
- **Cross-arch service exec testing**: argonaut is
  always single-arch at runtime (supervisor is on one host;
  it spawns services on the same host), so sign-on-aarch64 →
  verify-on-x86_64 paths aren't reachable in production.

---

## Open — gated on external work

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
