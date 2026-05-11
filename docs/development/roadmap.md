# Argonaut Roadmap

Completed items live in [CHANGELOG.md](../../CHANGELOG.md). Live state
(version, binary size, suite count, dep pins) lives in
[`docs/development/state.md`](state.md). This file is forward-looking
work only.

---

## Current — v1.6.0 (shipped 2026-05-10) — PID-1 graduation

argonaut runs as `/sbin/init` under qemu via the new
`qemu/build-initramfs.sh` + `qemu/boot-test.sh` harness
(scaffold lifted from kybernet's pattern; standalone runtime).
`src/main.cyr` adds a sleep-and-reap supervisor loop on
`getpid() == 1` so the kernel doesn't panic when init returns.
Three boot markers gate the smoke
(`init system ready` → `all systems nominal` → `pid1 loop ready`).
KVM + `+invtsc` required locally (sakshi clock_init panics
without invariant TSC; qemu TCG doesn't expose it).
`docs/architecture/002-qemu-pid1-harness.md` is the canonical
reference. See
[CHANGELOG 1.6.0](../../CHANGELOG.md#160--2026-05-10).

The 1.5.x arc is CLOSED — full audit in
[`docs/audit/2026-05-10-audit.md`](../audit/2026-05-10-audit.md).
1.6.x continues with M3/L3 end-to-end, signal-handled clean
shutdown, the closeout re-audit, and the carry-forwards
(cyrius pin → 5.10.44 + `exec_vec_str` migration; native
aarch64; audit_log_new rename; anchor + key rotation).

---

## Next — v1.6.1 — PID-1 coverage extensions

### End-to-end coverage

- [ ] **M3 end-to-end** — busybox helper inside the initramfs
  forks a grandchild + exits; assert argonaut's
  `proc_table_reap_orphans` collects the grandchild via the
  supervisor-loop tick. Lifts the `audit-m3-reaper-orphans`
  unit shape into real-PID-1 territory.
- [ ] **L3 end-to-end** — invoke `fork_exec_service` against a
  test service that writes `getsid(0)` to a tmpfs marker;
  assert the marker reads the child's own PID
  (controlling-TTY decoupled). Lifts the
  `audit-l3-fork-setsid` unit shape into real-PID-1.
- [ ] **Signal-handled clean shutdown** — install SIGTERM /
  SIGINT handlers in the PID-1 supervisor loop; route through
  `init_plan_shutdown` → `sys_reboot(RB_POWER_OFF)`. Closes
  the "qemu timeout is the only exit path" wart from 1.6.0.

### 1.6.x arc closeout

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

### Carry-forwards from 1.5.x

- [ ] **P1 — Migrate `check_command` to `exec_vec_str`**
  (cyrius v5.10.44 unblock; filed by argonaut, shipped
  upstream 2026-05-11). The `exec_*` family added typed
  Str-shape siblings — `exec_vec_str` /
  `exec_capture_str` / `exec_env_str` — that extract
  `str_data` on the way into execve's argv. Argonaut's
  `src/health.cyr:144` calls `exec_vec(argv)` where
  `argv` was built from `str_split(cmd_str, str_from(" "))`
  parts (Str elements) — the silently-broken case the
  v5.10.44 fix was filed against. Migration is one line:
  `exec_vec(argv)` → `exec_vec_str(argv)`. Then:
  - **tests/tcyr/health_exec.tcyr:17-22, 39-43** — flip
    the determinism-only assertions
    (`assert(cmd_ok == 0 || cmd_ok == 1)`) back to strict
    (`assert_eq(check_command(str_from("/bin/true"), 5000), 1)`
    + `assert_eq(check_command(str_from("/bin/false"), 5000), 0)`).
  - **tests/tcyr/audit_findings.tcyr:262** — remove the
    "the existing exec_env Str/cstr quirk … blocks unit-level
    shell exec testing" comment. End-to-end
    fork_exec_service verification stays gated on the
    QEMU PID-1 harness (that's a separate gate — different
    blocker, not this one).
  - **cyrius.cyml** — bump `cyrius = "5.10.34"` →
    `cyrius = "5.10.44"` (current local build).
  Lands as a 1.6.x slot when the cyrius pin bumps; one
  atomic change (the new API is byte-identical for cstr
  consumers, so the migration doesn't cascade).
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
