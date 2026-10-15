# Argonaut Roadmap

Completed items live in [CHANGELOG.md](../../CHANGELOG.md). Live state
(version, binary size, suite count, dep pins) lives in
[`docs/development/state.md`](state.md). This file is forward-looking
work only.

---

## Toolchain — holding at cyrius 6.7.9

**The pin stays at cyrius 6.7.9 (libro 2.11.0) until further cyrius releases.**
The next move starts from cyrius's `docs/development/ecosystem-migration.md`,
at its 6.7.10 section, and from libro's release that matches the new fold.

Checklist for that move (each step was needed at 1.15.5 or 1.16.0):

- [ ] Read the migration notes for every release in range. List the changes that
      alter what already-compiling code computes, and scan `src/` and `tests/` for
      each shape with comments and strings stripped. A green suite proves nothing
      about them; the method is in CHANGELOG [1.16.0] Audited.
- [ ] Move `cyrius` in `cyrius.cyml` and `qemu/helpers/cyrius.cyml`, and `[deps.libro]`.
      Keep `[deps.sigil]` at the sigil tag the new libro pins, and re-diff the 14
      scaffolding fns `sigil-argon2` and `sigil-mldsa` both define (`state.md`
      Toolchain). They must stay identical for last-definition binding to be harmless.
- [ ] **Delete `cyrius.lock` and `lib/` before `cyrius deps`.** Otherwise old commit
      pins and leaves the new toolchain no longer needs survive into the lock.
- [ ] Check every resolved library against its upstream tag byte-for-byte.
- [ ] Re-cut the qemu L3 helper (`qemu/helpers/`), then delete the `cyrius.lock`
      its resolve writes.
- [ ] **Re-check the two stdlib layouts argonaut reads directly** (`state.md`
      Toolchain): hashmap entry slots walked in `src/init.cyr`, and the
      `str_builder` length that `src/health.cyr` resets.
- [ ] Re-check `src/syscall_compat.cyr`. When the stdlib gains a wrapper for
      `getsockopt`, `getsid`, `setpgid`, `symlinkat` or `clock_gettime`, move to it, with the
      `qemu -strace` before/after check that 1.15.6–1.16.1 used.
- [ ] State the consumer floor. A consumer that compiles argonaut's `src/` needs
      at least the toolchain argonaut's own calls require (≥ 6.6.8 today).
- [ ] Run the full gates plus the bench gate against `1.16.2-deferred-fixes`.

---

## Security enforcement — filed by the 2026-08-24 P(-1) audit

**seccomp / Landlock / capabilities are modelled but NOT enforced.**
`src/security.cyr` contains zero syscalls: it renders description strings
and builds one `setpriv` SafeCommand that nothing executes. No spawn path
reads those fields, so every service `fork_exec_service` starts inherits
PID 1's full root privileges.

- [ ] **`PR_SET_NO_NEW_PRIVS` + seccomp BPF filter** applied in the
      `fork_exec_service` child between `reset_child_signal_mask()` and
      `sys_execve`. Needs a BPF program assembler; `SeccompConfig` already
      carries the allow/deny lists.
- [ ] **Landlock ruleset** (`landlock_create_ruleset` /
      `landlock_add_rule` / `landlock_restrict_self`) from the existing
      `LandlockConfig`.
- [ ] **Capability drop + uid/gid switch.** Ordering trap: setgid and
      setgroups must precede setuid, or the drop is incomplete. There is no
      uid/gid field on `ServiceDef` yet.
- [ ] **cgroup membership (or pidfd) as the PID-identity invariant.** This
      closes the PID-reuse window between reading a PID and signalling it. The
      2026-10-14 audit's B-1 closed the PID-file side (symlinks, FIFOs, loose
      parsing); the reuse window is open by design until this lands.
- [ ] Regression tests per mechanism, plus a qemu-harness assertion that a
      spawned service genuinely cannot perform a denied syscall.

Until this lands, **do not describe argonaut as sandboxing services.**

---

## 1.17.0 — removals

Every finding the 2026-10-14 audit deferred was fixed at 1.16.2. The record is in
[`docs/audit/2026-10-14-audit.md`](../audit/2026-10-14-audit.md), "Fixed at 1.16.2". What
it left for the next minor:

- [ ] **Remove the deprecated APIs.**
      - `notify_try_recv` / `notify_drain` skip sender authentication. Their deprecation
        message names 1.17.0.
      - `spawn_service` is a forwarder to `fork_exec_service`.

      Check kybernet's tree for callers first; at 1.7.9 it has none.
- [ ] **Retire the legacy emergency credential** (64 lowercase hex, unsalted SHA-256).
      This waits on kybernet: its legacy arm calls `verify_emergency_auth`, so kybernet
      migrates operators to v1 Argon2id records first. Retiring it then means
      `emergency_cred_kind` returns `ECRED_INVALID` for a legacy digest.

---

## Native aarch64 CI

The qemu-user sweep (`scripts/aarch64-sweep.sh`) catches codegen regressions,
not real-hardware behaviour. `scripts/aarch64-pi-smoke.sh` proves the real-hardware
path by hand (argonaut initialises in ~536 µs on an RPi4); CI would make it
automatic. (The sigil Ed25519 aarch64 quirk this item once waited on was
resolved upstream on 2026-05-21.)

- [ ] **A native aarch64 runner:** `ubuntu-24.04-arm` or a self-hosted Pi.
      Choose to match agnos / kybernet.
- [ ] **CI job:** an `aarch64-native` build and the full `.tcyr` suite, plus the
      qemu PID-1 harness when KVM is available.
- [ ] **Release gate:** once that job is green, the `argonaut-<VER>-aarch64-linux`
      artifact becomes a hard requirement rather than best-effort.

---

## Gated on external work

- [ ] **WitnessAnchor publishing** — libro's anchor primitive for
      cross-snapshot trust pins. Gated on consumer demand and the AGNOS
      federation protocol.
- [ ] **Durable signing-key rotation** — `audit_log_keygen()` generates an
      ephemeral key per call. Lands when kybernet has a key-management surface;
      argonaut's API stays stable.

---

## Kybernet integration (separate repo)

Tracked in [kybernet](https://github.com/MacCracken/kybernet), which consumes
argonaut's `src/` as a library. kybernet-side work follows the project-boundaries
rule. Open items that involve argonaut:

- [ ] **Take 1.16.x.** It needs cyrius ≥ 6.6.8.
      - **Review CHANGELOG [1.16.1] Breaking (behaviour):** the start refusal, the 90 s
        start timeout, the restart window, the reused `init_poll_health` result, the
        bounded HTTP check and the strict PID-file reader.
      - **1.16.2's changes:**
        - The existing `init_stop_all` call becomes an ordered shutdown with no code
          change.
        - The health tick writes far fewer audit records.
        - `test_emergency_auth` (`src/test.cyr:1391-1393`) pins the old open
          `emergency_shell_default()` and must be updated.
- [ ] **TPM enrolment per `docs/architecture/005-edge-boot-tpm-key-policy.md`:**
      - measure the verity root hash into PCR 15 before unlocking;
      - bind the token to PCR 7 + 15;
      - cap PCR 15 afterwards.
- [ ] **Move operators to v1 Argon2id emergency credentials**, so argonaut can retire
      the legacy digest (1.17.0, above).
- [ ] Seccomp / Landlock application in `pre_exec`, with the enforcement work
      above.
- [ ] A control socket for agnoshi runtime commands.
- [ ] Real-hardware boot validation (RPi4, NUC).

---

## Hygiene

- [ ] **`0 - N` → `-N`.** 80 sites in `src/` still use the pre-3.10.3 form.
      Cosmetic, so batch it as a patch of its own where it cannot hide a
      functional change.
- [ ] **The release workflow should bump `state.md`.** It is still edited by
      hand every release.

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
