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
- [ ] **Delete `cyrius.lock` and `lib/` before `cyrius deps`.** Otherwise old commit
      pins and leaves the new toolchain no longer needs survive into the lock.
- [ ] Check every resolved library against its upstream tag byte-for-byte.
- [ ] Re-cut the qemu L3 helper (`qemu/helpers/`), then delete the `cyrius.lock`
      its resolve writes.
- [ ] **Re-check the two stdlib layouts argonaut reads directly** (`state.md`
      Toolchain): hashmap entry slots walked in `src/init.cyr`, and the
      `str_builder` length that `src/health.cyr` resets.
- [ ] Re-check `src/syscall_compat.cyr`. When the stdlib gains a wrapper for
      `getsockopt`, `getsid`, `setpgid` or `clock_gettime`, move to it, with the
      `qemu -strace` before/after check that 1.15.6–1.16.1 used.
- [ ] State the consumer floor. A consumer that compiles argonaut's `src/` needs
      at least the toolchain argonaut's own calls require (≥ 6.6.8 today).
- [ ] Run the full gates plus the bench gate against `1.16.1-audit-bench-streaming`.

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

## Hardening follow-ups — from the 2026-10-14 audit

Deferred findings; details in [`docs/audit/2026-10-14-audit.md`](../audit/2026-10-14-audit.md).

- [ ] **Ordered shutdown (C-8, LOW).** Today `init_stop_all` SIGTERMs everything
      at once, leaves services RUNNING with stale pids, and writes no audit
      record. kybernet builds `init_plan_shutdown` and then calls only this.
      The proposed fix is an `init_execute_shutdown` that walks
      `resolve_service_waves` in reverse:
      - SIGTERM each wave, then reap it against one deadline per wave;
      - SIGKILL stragglers;
      - set every service STOPPED, audited;
      - finish with a `proc_table_stop_all` sweep for untracked pids and an
        audit flush.

      About 80–100 lines, plus a one-line change in kybernet.
- [ ] **Emergency shell.**
      - Replace `password_hash`'s unsalted single-round SHA-256 with a real KDF
        (needs a dependency the thin sigil surface lacks).
      - Revisit the `require_auth = 0` default. Deciding it needs the recovery
        story, so the two travel together.
- [ ] **Remove or rename the unauthenticated `notify_try_recv` / `notify_drain`.**
      They skip the SO_PEERCRED check and cannot detect a truncated datagram.
      No production caller.
- [ ] **systemd unit generation:**
      - `WatchdogSec=` is emitted even under `Type=simple`.
      - An ExecStart binary beginning with `-`, `@`, `:`, `+` or `!` is read as
        a systemd prefix; require an absolute path.
- [ ] **tmpfiles natively.** 1.16.1 made the external commands symlink-safe
      (`chown -h`, `ln -sfnT`). A native `mkdirat` / `openat(O_NOFOLLOW)` /
      `fchown` / `fchmod` implementation removes the commands entirely.
- [ ] **TPM PCR policy (CVE-2026-32606 class).** argonaut neither enrols nor
      seals keys, so this is a kybernet enrolment question: check that its
      policy cannot hand the LUKS key to a swapped root partition.
- [ ] **PERF: audit only health transitions.** Every passing probe writes an
      audit record (~4 µs and 16 B). Recording only pass↔fail changes would cut
      that about 100×, but it changes what the audit log holds, so it needs a
      decision first.

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

- [ ] **Take 1.16.x.** It needs cyrius ≥ 6.6.8 and a review of CHANGELOG
      [1.16.1] Breaking (behaviour): start refusal, the 90 s start timeout, the
      restart window, the reused `init_poll_health` result, the bounded HTTP
      check and the strict PID-file reader.
- [ ] Call the ordered shutdown once it lands (above), instead of `init_stop_all`.
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
