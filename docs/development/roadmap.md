# Argonaut Roadmap

Forward-looking work only. Completed work is in [CHANGELOG.md](../../CHANGELOG.md); live
state (version, binary size, suite count, dep pins) is in [`state.md`](state.md).

Each release below is pinned to the work it ships. The order is the dependency order: a
later release may assume everything above it. An item whose start depends on someone
else is in [Gated](#gated--1x-pinned-when-the-gate-opens). It gets a release number when
that gate opens.

---

## Release plan

| Release | Theme | Can start |
|---|---|---|
| [1.17.0](#1170--next-cyrius-pin-and-the-announced-removals) | Next cyrius pin; remove the deprecated APIs; remove or wire the inert helpers | When the pin target is chosen |
| [1.17.1](#1171--negative-literals-mechanical) | `0 - N` → `-N`, no functional change | Any time after 1.17.0 |
| [1.17.2](#1172--ci-native-aarch64-and-the-state-file) | Native aarch64 CI; `state.md` automation; CI bench fix; doc-tree conformance | Once a runner is chosen |
| [1.18.0](#1180--process-identity-and-the-exec-paths) | pidfd process identity; health-check commands under the pre-exec hook | Now |

**Every release:**
- the security-hardening pass and the benchmark gate (CLAUDE.md);
- a closeout pass before each `X.Y.0`.

---

## 1.17.0 — next cyrius pin, and the announced removals

**The pin target is open.** The tree holds at cyrius 6.7.9 with libro 2.11.0. 6.7.10 and
6.7.11 are out, but the target (6.7.11 or a later release) is chosen when this work
starts. Start from cyrius's `docs/development/ecosystem-migration.md`, at the first section
after 6.7.9, and from the libro release that matches the new fold.

The removals do not depend on the pin. The deprecation messages name 1.17.0. If the pin
is still waiting when the removals are wanted, ship them alone as 1.17.0, and the pin
takes the next free release number.

Before tagging, run the 1.16.x closeout pass (CLAUDE.md).

**Toolchain move:**
- [ ] Read the migration notes for every release in range. List the changes that alter
      what already-compiling code computes. Scan `src/` and `tests/` for each shape, with
      comments and strings stripped. A green suite proves nothing about them; the method
      is in CHANGELOG [1.16.0] Audited.
- [ ] Move `cyrius` in `cyrius.cyml` and `qemu/helpers/cyrius.cyml`, and move `[deps.libro]`.
      Set `[deps.sigil]` to the sigil tag the new libro pins.
- [ ] **Re-diff the scaffolding fns that `sigil-argon2` and `sigil-mldsa` both define.**
      There are 14 at 3.13.12 (`state.md` Toolchain). They must stay identical for
      last-definition binding to be harmless.
- [ ] **Delete `cyrius.lock` and `lib/` before `cyrius deps`.** Otherwise old commit pins,
      and leaves the new toolchain no longer needs, survive into the lock.
- [ ] Check every resolved library against its upstream tag byte for byte.
- [ ] Re-cut the qemu L3 helper (`qemu/helpers/`), then delete the `cyrius.lock` its
      resolve writes.
- [ ] **Re-check the two stdlib layouts argonaut reads directly** (`state.md` Toolchain):
      - the hashmap entry slots walked in `src/init.cyr`;
      - the `str_builder` length that `src/health.cyr` resets.
- [ ] **Re-check `src/syscall_compat.cyr`.** Where the stdlib now wraps `getsockopt`,
      `getsid`, `setpgid`, `symlinkat` or `clock_gettime`, move to the wrapper. Check each
      move with a before/after `qemu -strace` trace.
- [ ] State the consumer floor: the oldest toolchain argonaut's own calls need (≥ 6.6.8
      today).
- [ ] Run the full gates, plus the bench gate against `1.16.2-deferred-fixes`.

**Removals (breaking):**
- [ ] **`notify_try_recv` / `notify_drain`.** They skip sender authentication.
      `notify_try_recv_authenticated` replaces both.
- [ ] **`spawn_service`.** It is a forwarder to `fork_exec_service`.
- [ ] **Check kybernet's tree for callers first.** At kybernet 1.7.9 there are none.

**Inert helpers: remove each, or wire it (breaking either way).** Each of these suggests
a feature argonaut does not have, and the 2026-10-14 doc sweep found no caller in `src/`
for any of them:
- [ ] `to_prlimit_commands` / `resource_limits_secure` and the `svc_def_rlimits` field.
      Nothing applies resource limits (ADR-010).
- [ ] `LogConfig` / `svc_def_log_config`. Nothing rotates logs; service output goes to
      `/dev/null`.
- [ ] `load_env_file` / `load_env_files` and `svc_def_env_files`. The spawn path never
      loads env files; kybernet does.
- [ ] The `socket_activation` field and `socket_env_listen_fds`. Nothing creates or
      passes sockets.
- [ ] `capability_setpriv_cmd`, `validate_service_name` and `is_localhost_target`.
- [ ] **Check kybernet for each one before removing it.**

---

## 1.17.1 — negative literals (mechanical)

- [ ] Rewrite the `0 - N` literal sites in `src/` (104 today) as `-N`. This form works since
      cyrius 3.10.3.
- [ ] Ship this patch alone so it cannot hide a functional change. The proof is a
      byte-identical DCE binary, or a diff whose every byte is explained.

---

## 1.17.2 — CI: native aarch64, and the state file

The qemu-user sweep (`scripts/aarch64-sweep.sh`) catches codegen regressions, not
real-hardware behaviour. `scripts/aarch64-pi-smoke.sh` proves the hardware path by hand:
argonaut initialises in ~536 µs on an RPi4.

- [ ] **Pick the runner:** `ubuntu-24.04-arm`, or a self-hosted Pi. Match agnos and
      kybernet.
- [ ] **CI job:** an `aarch64-native` build and the full `.tcyr` suite, plus the qemu PID-1
      harness when KVM is available.
- [ ] **Release gate:** once that job is green, the `argonaut-<VER>-aarch64-linux` artifact
      becomes required, not best-effort.
- [ ] **The release workflow bumps `docs/development/state.md`.** Today it is edited by
      hand every release.
- [ ] **Run the CI bench step with `bash`.** `ci.yml` runs `sh scripts/bench-history.sh`,
      but the script needs bash (`pipefail`, `BASH_SOURCE`, `[[`), and ubuntu's `sh` is
      dash. `|| true` and `continue-on-error` hide the failure.
- [ ] **Delete `scripts/version.sh`.** It edits `Cargo.toml` (Rust-era), fails under
      `set -e`, and nothing calls it; the manifest reads `${file:VERSION}`.
- [ ] **Doc-tree conformance** with the first-party documentation standard: add
      `CODE_OF_CONDUCT.md` (and add it to the CI required-files list), a `README.md` in
      `docs/adrs/` and in `docs/architecture/`, and `docs/examples/`.

---

## 1.18.0 — process identity, and the exec paths

**pidfd.** The 2026-10-14 audit's B-1 closed the PID-file hole (symlinks, FIFOs, loose
parsing). The window between reading a pid and signalling it is still open: a pid can be
recycled in that gap.

- [ ] **Open a pidfd for every spawned service.** Use `pidfd_open`, which the stdlib wraps
      as `sys_pidfd_open` (Linux 5.3+).
- [ ] **Signal through the pidfd** in the stop, kill and watchdog paths. Use
      `pidfd_send_signal`, which the stdlib does not wrap, so it is an `ag_sys_` entry.
      Verify its number under `qemu -strace` on both arches (the ESYSXLAT rule in that
      file's header).
- [ ] **Decide the fallback** for a kernel without pidfd: refuse, or today's B-7
      probe-then-signal.
- [ ] **Tests:** a recycled pid is never signalled, and a pidfd closes when its service is
      reaped.

**Health-check commands bypass the pre-exec hook.** `fork_exec_service` runs the
consumer's pre-exec hook (1.9.0), which is where kybernet confines a service. A
`HC_COMMAND` check runs through `check_command` → `run_safe_cmd_timeout` instead. That
path has no service definition and no hook, so a confined service's health-check
command runs unconfined, with PID 1's root privileges.

- [ ] **Run the check under the service's hook** (pass the definition through), or make
      the unconfined run an explicit, documented choice.
- [ ] **Tests:** with a denying hook, a service's command check runs confined or is
      refused. It must not run unconfined.

---

## Gated — 1.x, pinned when the gate opens

| Item | Gate | What lands |
|---|---|---|
| **Retire the legacy emergency credential** (64 lowercase hex, unsalted SHA-256) | kybernet moves operators to v1 Argon2id records; its legacy arm calls `verify_emergency_auth` | `emergency_cred_kind` returns `ECRED_INVALID` for a legacy digest. Breaking, so a minor |
| **cgroup v2 membership per service** | kybernet owns cgroup setup today (`svc_def_cgroup_limits` is opaque to argonaut) | Kill-by-cgroup, and membership as the identity invariant beside the pidfd |
| **WitnessAnchor publishing** | Consumer demand and the AGNOS federation protocol | libro's anchor primitive, for cross-snapshot trust pins |
| **Durable signing-key rotation** | A key-management surface in kybernet | Today `audit_log_keygen()` makes an ephemeral key per call. argonaut's API stays stable |
| **Abstract-namespace notify socket** | A consumer that needs it | `notify_bind` on an abstract address |
| **dm-verity FEC** (`--fec-device`) | Edge storage that needs error correction | An optional FEC device in `verify_rootfs_integrity` |
| **sd_notify `MAINPID=` / `STATUS=` / `RELOADING=1` / `STOPPING=1`** | A service that relies on them | These are parsed today, but only `READY=1` and `WATCHDOG=1` change state |

---

## Kybernet-side (separate repo)

[kybernet](https://github.com/MacCracken/kybernet) consumes argonaut's `src/` as a
library. These items are kybernet's to do; each touches argonaut's surface.

- [ ] **Take 1.16.x.** It needs cyrius ≥ 6.6.8.
      - **CHANGELOG [1.16.1] Breaking (behaviour):** the start refusal, the 90 s start
        timeout, the restart window, the reused `init_poll_health` result, the bounded
        HTTP check and the strict PID-file reader.
      - **CHANGELOG [1.16.2]:**
        - `init_stop_all` is now ordered.
        - The health tick writes far fewer audit records.
        - `test_emergency_auth` (`src/test.cyr:1391-1393`) pins the old open
          `emergency_shell_default()`.
- [ ] **TPM enrolment per [architecture note 005](../architecture/005-edge-boot-tpm-key-policy.md):**
      measure the verity root hash into PCR 15 before unlocking, bind the token to PCR 7
      + 15, and cap PCR 15 afterwards.
- [ ] **Move operators to v1 Argon2id emergency credentials.** This opens the
      legacy-credential gate above.
- [ ] A control socket for agnoshi runtime commands.
- [ ] Real-hardware boot validation (RPi4, NUC).

---

## Standing rules

- **Policy enforcement belongs to the consumer.** By the 1.9.0 design argonaut does not
  apply seccomp / Landlock / capabilities. The `seccomp`, `landlock` and `capabilities`
  fields are opaque, and `fork_exec_service` hands each definition to the pre-exec hook,
  which fails closed (exit 126). kybernet's `service_sandbox.cyr` is that hook. With no
  hook, a service runs with PID 1's root privileges. Do not describe argonaut itself as
  sandboxing services. Moving enforcement into argonaut would be a new design decision
  (an ADR), not a roadmap item.

- **Re-audit trigger.** Several severities (2026-08-24 and 2026-10-14) stay bounded only
  because the shipped PID-1 loop (`src/main.cyr`) never calls the service-lifecycle
  machinery. Re-rate them the day it calls `init_start_service`, `init_poll_health` or
  `init_notify_bind`.
- **Consumer floor.** A release that raises the toolchain a consumer needs says so under
  Breaking in the CHANGELOG.

---

## Non-goals

- **Container orchestration** — that's stiva
- **Package installation** — that's ark
- **Agent lifecycle** — that's daimon (argonaut manages the process, daimon manages the
  agent)
- **Scheduling and timer-based services** — that's samay (argonaut starts and stops; samay
  decides when)
- **D-Bus interface** — only if the AGNOS desktop requires it
