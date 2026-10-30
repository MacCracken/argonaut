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
| [1.18.0](#1180--process-identity-and-the-exec-paths) | pidfd process identity; health-check commands under the pre-exec hook | Now |

**Every release:**
- the security-hardening pass and the benchmark gate (CLAUDE.md);
- a closeout pass before each `X.Y.0`.

**The next cyrius pin has no release number.** 1.16.3 moved to 6.7.18, the latest release,
with every dependency at its latest tag. The next move takes the next free number when a
newer cyrius is wanted; [its checklist](#every-cyrius-pin-move) is below.

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
command runs unconfined, with PID 1's root privileges. Since 1.17.0 it also runs without
the service's resource limits and env files, which `fork_exec_service` applies.

- [ ] **Run the check under the service's hook** (pass the definition through), or make
      the unconfined run an explicit, documented choice. Decide the limits and env files
      in the same change.
- [ ] **Tests:** with a denying hook, a service's command check runs confined or is
      refused. It must not run unconfined.

---

## Every cyrius pin move

The procedure 1.16.0 and 1.16.3 followed. It ships as its own release, or with whatever
release is next.

- [ ] Read cyrius's `docs/development/ecosystem-migration.md` for every release in range.
      List the changes that alter what already-compiling code computes. Scan `src/`,
      `tests/` and `qemu/helpers/` for each shape, with comments and strings stripped. A green
      suite proves nothing about them; the method is in CHANGELOG [1.16.0] and [1.16.3]
      Audited.
- [ ] Move `cyrius` in `cyrius.cyml` and `qemu/helpers/cyrius.cyml`, and move `[deps.libro]`
      to the libro release that matches the new fold. Set `[deps.sigil]` to the sigil tag that
      libro pins.
- [ ] **Re-diff the scaffolding fns that `sigil-argon2` and `sigil-mldsa` both define.**
      There are 23 at 3.13.15 (`state.md` Toolchain). They must stay identical for
      last-definition binding to be harmless.
- [ ] **Delete `cyrius.lock` and `lib/` before `cyrius deps`.** Otherwise old commit pins,
      and leaves the new toolchain no longer needs, survive into the lock.
- [ ] Check every resolved library against its upstream tag byte for byte.
- [ ] **Check that the new release ships both linux tarballs and a signed `SHA256SUMS`.**
      `scripts/ci-install-cyrius.sh` installs x86_64-linux for the CI and release jobs and
      aarch64-linux for the arm64 job, and refuses a release whose signature does not verify
      against its pinned key.
- [ ] **Check that the native and cross aarch64 builds still match** (`cmp`), before CI
      does. Build `src/main.cyr` with `--aarch64` on x86_64, and natively with the
      aarch64-linux tarball's `cycc` (on arm64, or under `qemu-aarch64`).
- [ ] Re-cut the qemu L3 helper (`qemu/helpers/`) with `CYRIUS_DCE=1`, as
      `qemu/build-initramfs.sh` builds it. Then delete the `cyrius.lock` its resolve writes.
- [ ] **Re-check the two stdlib layouts argonaut reads directly** (`state.md` Toolchain):
      - the hashmap entry slots walked in `src/init.cyr`;
      - the `str_builder` length that `src/health.cyr` resets.
- [ ] **Re-check `src/syscall_compat.cyr`.** Where the stdlib now wraps `getsockopt`,
      `getsid`, `setpgid`, `symlinkat` or `clock_gettime`, move to the wrapper. Check each
      move with a before/after `qemu -strace` trace. As of 6.7.18 it wraps none of them.
- [ ] State the consumer floor: the oldest toolchain argonaut's own calls need (≥ 6.7.9
      today, for `sys_ppoll`). Measure it: build the tree on that toolchain.
- [ ] **Attribute any binary-size change** by building the old pins under the new toolchain.
- [ ] Run the full gates, plus the bench gate against the previous release's label
      (`1.16.3-cyrius-6.7.18` today).

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
| **Service log capture and rotation** (`LogConfig`, `log_config_default`, `svc_def_log_config`) | A design: where service output goes instead of `/dev/null` (a journal fd, files under a log dir), and who rotates. Kept, not removed, by the 1.17.0 decision | `fork_exec_service` wires stdout / stderr to the chosen sink, and `LogConfig` drives rotation |
| **Socket activation** (the `socket_activation` field, `socket_env_listen_fds`) | A design for the listening-socket protocol (it is not a scalar config key) and a service that needs it. Kept by the 1.17.0 decision | argonaut creates and passes the sockets, and sets `LISTEN_FDS` / `LISTEN_PID` |
| **`capability_setpriv_cmd`, `is_localhost_target`** | A call site. ADR-013 puts capability drops in the consumer's hook, and the health checks deliberately accept any IPv4 literal or `/etc/hosts` name. Kept by the 1.17.0 decision | Each is wired where a caller needs it |
| **Shed bayan's Unicode tables** (~307 KB of the binary since 1.16.3) | Upstream: `sigil-mldsa` stops including the whole of bayan, or cyrius DCE drops unread global initializers | A pin move that takes the fix. Measure it with the binary-size attribution step above |

---

## Kybernet-side (separate repo)

[kybernet](https://github.com/MacCracken/kybernet) consumes argonaut's `src/` as a
library. These items are kybernet's to do; each touches argonaut's surface.

- [ ] **Take 1.16.x and 1.17.0.** They need cyrius ≥ 6.7.9 (`sys_ppoll`, since 1.16.1;
      1.16.1–1.16.3 said 6.6.8 in error). On 6.7.18 a scratch kybernet passes 888 / 890
      against 1.17.0, the two being the `test_emergency_auth` assertions below.
      - **CHANGELOG [1.17.0] Breaking:** the removed notify receivers and `spawn_service`
        (kybernet calls none of them), names refused at registration (`a..b` is, though
        kybernet accepts it), and the 126 exit's two new causes.
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
- **Automating `docs/development/state.md`** — it is refreshed by hand as part of every
  release; no generator, CI gate or workflow commit
