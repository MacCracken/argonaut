# Argonaut — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Bumped at release time alongside
> `VERSION` and the CHANGELOG header.

## Version

**1.16.2** (the 2026-10-14 audit's deferred findings, repaired.
- **Shutdown:** `init_execute_shutdown` stops services in reverse dependency waves inside
  one shared budget; `init_stop_all` now runs it, and every service ends STOPPED and audited.
- **Emergency shell:** verifies a v1 Argon2id credential (kybernet's format), with the
  arena in a private `mmap`; `emergency_shell_default()` requires authentication.
- **Health audit:** transitions plus an hourly pass heartbeat.
- **tmpfiles:** native and symlink-safe (`tmpfiles_apply`, `ag_sys_symlinkat`).
- **Notify:** `notify_try_recv` / `notify_drain` are deprecated; descriptors passed over
  the notify socket are closed.
- **Unit generation:** `WatchdogSec=` only for `SVC_NOTIFY`; a non-absolute binary is
  refused.
- **Restart limit:** a refused restart is audited.
- **TPM:** the key policy is documented in architecture note 005.

41 suites / 1,662 assertions on x86_64 and under qemu-aarch64. Bench gate: neutral.
Several behaviour changes; see CHANGELOG [1.16.2].)

Earlier releases are in [CHANGELOG.md](../../CHANGELOG.md). The pinned release plan
from 1.17.0 on is in [roadmap.md](roadmap.md).

## Toolchain

- `cyrius = "6.7.9"` in `cyrius.cyml [package]` and `qemu/helpers/cyrius.cyml`.
  The manifest is the only pin source (no `.cyrius-toolchain`); CI and release
  read it. A consumer that compiles argonaut's `src/` needs cyrius ≥ 6.6.8
  (`sys_fcntl`).
- **6.7.x language rules that bind argonaut's source:** `trait`, `const`, `true`,
  `false`, `do` and `dyn` are reserved; `!` is a real operator (it was silently
  dropped before 6.7.3); every forward call is arity-checked; an unmatched `}` at
  module scope is an error. The 1.16.0 audit found none of the silently changed
  shapes in the tree (CHANGELOG [1.16.0] Audited).
- **A `path` beside `git` / `tag` is a dev override** (6.7.6+), used only with
  `CYRIUS_LOCAL=<dep>` or `--local`; a `path`-only dependency is refused. To build
  a consumer against an argonaut working tree, keep `git` / `tag` and add the
  `path` line, then build with `CYRIUS_LOCAL=argonaut`.
- **`Result` is a `(tag, payload)` register pair** (6.6.0+). Bind both halves
  with `var t, v = f();` — `var r = f();` compiles and silently keeps only the
  tag, and there is no `t, v = f();` reassignment form, so a retry loop binds a
  fresh pair per iteration.
- **A dependency cannot overwrite a declared stdlib leaf** (6.5.39+). cyrius
  keeps the fold's copy and prints `refusing to overwrite stdlib leaf 'NAME'`.
  argonaut gets it for `patra` on every build, because libro resolves patra as a
  package — expected. `cyrius build --check-lib-sync` (CI + release) remains
  the guard against a stale `./lib/`.
- **`cyrius.lock`** is sorted (6.6.3+) and ends with a `cyrius <pin>` record
  (6.6.4+). A stdlib leaf that changes under an unchanged pin is refused until
  `cyrius deps --relock`. ⚠ From 6.6.20 a recorded commit pin survives later
  resolves, so after a dependency's tag moves the old pin stays beside the new
  one. `cyrius deps` also never deletes a stdlib file it no longer needs from
  `lib/`, and the lock hashes whatever is there. At a pin move, delete both `cyrius.lock` and
  `lib/`, then run `cyrius deps`.
- **A stdlib-only project gets a `cyrius.lock` on every resolve** (6.6.9+).
  `qemu/helpers/` writes one when it is re-cut. It is not tracked, because the
  helper's `lib/` is committed.
- **`streq(str_data(..), ..)` warns** (6.6.20+). `streq` reads to a NUL, and a
  Str's bytes are not NUL-terminated at its length. Compare a Str with
  `str_eq_cstr(s, "lit")` or `str_eq(a, b)`.
- **Every call to a redefined fn binds its last definition** (6.6.20+).
  - **Main build:** 14 fns are defined twice. `lib/sigil-argon2.cyr` repeats sigil's
    bank / TLS / once scaffolding (`_sigil_ensure_slot` … `_sigil_wipe64`), which
    `lib/sigil-mldsa.cyr` also defines. At sigil 3.13.12 all 14 bodies are identical, so
    the binding changes nothing. **Re-diff them at every sigil bump**; kybernet's build
    prints the same 14.
  - **Test and bench builds:** they also redefine eight `sakshi_*` fns as stubs (eight
    more warnings). The test builds add two deliberate `#deprecated` warnings, for
    `notify_try_recv` / `notify_drain`.
- **`lib/bench.cyr`** (6.6.5+) takes min/max only from windows the clock
  resolves to 1 %, and otherwise reports the mean. On the dev host (HPET
  clocksource, ~2.3 µs clock error) that is every row, so compare `avg_us` only.
- `cyrius fmt <file>` rewrites in place; the CI gate uses
  `cyrius fmt <file> --check` (see the comment on the fmt step in
  `.github/workflows/ci.yml`).
- `freelist` is thread-safe, at a single-threaded alloc+free cost argonaut pays
  although it spawns no threads.
- **`alloc_init` maps its 256 MB first chunk `MAP_NORESERVE`** (6.6.8+), and
  falls back to a 16 MB grain if the kernel still refuses it. Under KVM,
  argonaut boots as PID 1 at `-m 256M`, `160M` and `128M`. At `-m 96M` the
  kernel panics before argonaut runs.
- **argonaut reads two stdlib layouts directly; re-check both at every pin
  move.** `src/init.cyr` walks a hashmap's entries in place: `map_entries` /
  `map_cap`, 24-byte slots of key, value and state, with state 1 meaning live.
  `src/health.cyr` empties a reused `str_builder` by zeroing its length at +8.
  If `lib/hashmap.cyr` or `lib/str.cyr` changes those layouts, these break
  silently. `stats_desktop` and the `p2-*` assertions in `harden_core.tcyr`
  would show the first, and `harden_net.tcyr`'s HTTP tests the second.
- The `cyrius build`/`test`/`bench` driver is the stable surface; the compiler
  is `cycc` (`cycc_aarch64` for the cross-build). Install layout is
  `~/.cyrius/versions/<V>/{bin,lib}`, symlinked from `~/.cyrius/{bin,lib}`.
  Some toolchain tarballs ship `cycc_aarch64` at the top level rather than
  under `bin/`; both workflows handle either.

## Binary

- **x86_64: 448,448 B**, a statically linked ELF, at 1.16.2
  (`CYRIUS_DCE=1 cyrius build --check-lib-sync src/main.cyr build/argonaut`).
  2,620 fns are unreachable and 887,483 B of dead code is eliminated.
  That is +4,600 over 1.16.1's 443,848 B. Adding the argon2 bundle alone was +4,296: the
  scaffolding that reaches `main` before any argonaut code calls it.
- **aarch64: 1,570,488 B** at 1.16.2 (`CYRIUS_DCE=1 cyrius build --aarch64 src/main.cyr`).
  2,621 fns are unreachable and 1,051,668 B is NOPed. That is +496 over 1.16.1's
  1,569,992 B. aarch64 DCE fills dead code with NOPs instead of compacting, so new code
  usually shows in full. This step is small although 49,380 B more is NOPed; the offset
  was not investigated.
- **L3 helper: 15,080 B**, a static ELF (`qemu/helpers/l3-helper`), re-cut under
  6.7.9 at 1.16.0. It is bundled into the qemu harness
  initramfs as `/bin/l3-helper`. Its build is warning-free.

## Suites

- **x86_64: 41 `.tcyr` suites / 1,662 assertions**, 0 failures (cyrius 6.7.9).
  +326 at 1.16.2: `emergency_kdf` (125), `deferred_fixes` (116) and `shutdown_audit`
  (82), plus 3 in existing suites.
- **aarch64 (qemu-user, `scripts/aarch64-sweep.sh`): 41 / 41, 1,662 assertions**,
  identical to x86_64. Run on the dev host at 1.16.2. CI still runs only the
  aarch64 smoke; native aarch64 CI is roadmap 1.17.2.
- **PID 1 in a pid namespace** (`unshare -Urpf --mount-proc ./build/argonaut`, no
  root needed): the quick check for the supervisor loop. At 1.16.2 it shows 0 idle
  wakeups in 3 s, reaps 300 simultaneous orphans with 0 zombies left, and shuts down
  cleanly on SIGTERM.
- **qemu PID-1 harness** (`qemu/pid1-harness-test.sh`, `qemu/boot-test.sh`,
  KVM + `+invtsc`): at 1.16.2, M3 + L3 end-to-end and the supervisor-loop smoke
  pass at the scripts' committed `-m 256M`. They have passed there since 1.15.5,
  the first release built with a `MAP_NORESERVE` heap (cyrius 6.6.8+).
- **Benches**: `src/bench_main.cyr` — 29 micros, the release gate
  (`scripts/bench-history.sh`); `tests/bcyr/argonaut.bcyr` carries the same 29,
  `tests/bcyr/api.bcyr` 8 more. History in `bench-history.csv`.

### Bench snapshot (1.16.2-deferred-fixes, 2026-10-14)

**Verdict: neutral, no regression** against `1.16.1-audit-bench-streaming`, plus a
24-round interleaved A/B of the 1.16.1 tag and 1.16.2, pinned to one core:

| Bench | 1.16.1 gate | 1.16.2 gate | A/B Δ |
|---|---:|---:|---:|
| generate_unit | 4.315 | 3.908 | −10.8 % |
| resolve_order_chain_100 ⚠ | 230.597 | 222.046 | +1.5 % (ranges overlap) |
| resolve_waves_chain_20 | 29.180 | 40.584 | −1.6 % |
| resolve_order_chain_10 | 11.591 | 14.472 | +0.4 µs |
| resolve_waves_wide_20 | 12.758 | 15.508 | +0.3 µs |
| mark_all_steps_complete | 47.904 | 48.791 | +0.4 % |
| audit_log_record | 4.098 | 4.191 | +0.6 % |

- The three gate-row reads beyond ±2 µs (`resolve_waves_chain_20`, `resolve_order_chain_10`
  and `resolve_waves_wide_20`) are cleared by the A/B. None of their paths changed.
- Every other micro is within the ±2 µs band in both measurements.
- `1.16.2-deferred-fixes` is the next gate's baseline.

Earlier snapshots are in the CHANGELOG's Performance sections; every row is in
`bench-history.csv`.

## Dependencies

- **stdlib (26 modules)**: `string fmt alloc vec str slice syscalls io fs
  process hashmap tagged args bayan fnptr freelist chrono ct keccak thread
  thread_local random assert bench sakshi patra`, resolved by `cyrius deps` into
  the gitignored `lib/`.
- **libro 2.11.0** (`modules = ["dist/libro.cyr"]`). It pins sigil **3.13.12** and
  patra **1.16.0**; with libro these are the three commit pins in `cyrius.lock`.
  - A streaming append is allocation-free: `audit_log_record` costs 16 B / ~4.2 µs.
  - ⚠ On a streaming chain, the head hash `chain_append_nokeep` returns (and
    `audit_log_record` passes through) is valid only until the second subsequent
    append. Every in-tree caller discards it.
  - libro's `dist/libro.deps` sidecar names only `hashmap`, so argonaut's own
    `[deps] stdlib` must list every leaf argonaut calls. It does.
  - `patrastore_open` returns 0 if `patra_init()` has not run. argonaut runs
    `patra_init()` in `audit_ext_init` before it opens a store.
- **patra 1.16.0 and sakshi 2.5.8 come from the 6.7.9 fold**, byte-identical to
  their upstream `dist/` files. libro also resolves patra 1.16.0 as a package,
  and cyrius keeps the fold's copy (see Toolchain). On argonaut's `audit_persist`
  path (default off), patra runs WAL recovery before every statement: correct after
  a crashed writer, and libro measured +5–7 % on appends.
- **sigil 3.13.12 — thin sub-bundles only.**
  - **Declared at the root** (`[deps.sigil]`, 1.16.2) at libro's tag, so it resolves as
    one package. The block repeats libro's four modules and adds `dist/sigil-argon2.cyr`,
    for the emergency-shell credential.
  - **The bundles:** `lib/sigil-mldsa.cyr` (ed25519 + ML-DSA + hybrid + sha512),
    `lib/sigil_sha256.cyr`, `lib/sigil_sha_ni.cyr`, `lib/sigil_hex.cyr` and
    `lib/sigil-argon2.cyr`, 7,769 lines / 326,826 B of source. Argon2 is 1,408 lines /
    62,438 B of that.
  - **Keep the tag equal to libro's sigil pin at every libro bump.**
  - 6.7.9 folds sigil 3.13.11. libro's notes record that the thin set differs from 3.13.11
    only in the version header.
  - ⚠ **Never name `"sigil"` in the stdlib array.** That pulls the monolith (x509, RSA,
    authenticode), which roughly doubles the x86_64 binary and prints hundreds of
    duplicate-fn warnings (measured on cyrius 6.6.6: 403,672 → 900,472 B). Test and bench
    files must not `include "lib/sigil.cyr"` either; that rule lives in
    `tests/test_header.cyr`.
- **`cyrius.lock`** — `cyrius deps --verify` → **61 verified, 0 failed**,
  3 commit-pinned (libro, sigil, patra), sorted, ending with `cyrius 6.7.9`.
  It equals the lock a fresh copy of the tree resolves from nothing.

## In-flight

### Open

Each item is pinned to a release in [roadmap.md](roadmap.md).

- **1.17.0:** the next cyrius pin (the target is open), and removal of the deprecated
  `notify_try_recv`, `notify_drain` and `spawn_service`.
- **1.17.1:** the 104 `0 - N` literal sites in `src/`. Cosmetic; it ships alone.
- **1.17.2:** native aarch64 CI. The release workflow should bump this file; it is
  still edited by hand.
- **1.18.0:**
  - **pidfd process identity.** The window between reading a pid and signalling it is
    still open; the 2026-10-14 audit's B-1 closed only the PID-file side.
  - **Health-check commands and the hook.** `HC_COMMAND` checks run without the
    pre-exec hook, so a confined service's check command runs unconfined, as root.
- **Gated:**
  - the legacy SHA-256 emergency credential (still accepted beside v1 Argon2id), until
    kybernet migrates operators;
  - cgroup membership; WitnessAnchor; signing-key rotation.
- **Standing:** the re-audit trigger (see Audit cadence).

### Closed recently

- **1.16.2:** every item the 2026-10-14 audit deferred, plus two LOWs found while
  fixing them: SCM_RIGHTS fds left open in PID 1, and an unaudited restart-limit
  refusal (`docs/audit/2026-10-14-audit.md`, "Fixed at 1.16.2").
- **1.16.1:** the 2026-10-14 audit's 13 MEDIUM and 18 LOW.
- **1.16.0:** `fcntl` and `sendto` on the stdlib; the `ag_` wrappers removed.

## Pending release

None. 1.16.2 is tagged; the next release is 1.17.0 ([roadmap.md](roadmap.md)).

## Recent shipped

| Releases | Date | Headline |
|---|---|---|
| 1.16.0 – 1.16.2 | 2026-10-14 | cyrius 6.7.9 and libro 2.11.0; the 2026-10-14 audit (13 MEDIUM, 26 LOW, 7 PERF fixed); ordered shutdown; Argon2id emergency credential |
| 1.15.2 – 1.15.6 | 2026-09-22 – 2026-10-14 | cyrius 6.6.6 → 6.6.20; agnostik enum lockstep; PID 1 bootable at 256 MB; deterministic startup order |
| 1.9.0 – 1.15.1 | 2026-08-24 – 2026-09-11 | A pre-exec hook; config-driven service definitions; kernel capability numbers; service environment injection; a cgroup-limit field; aarch64 syscall portability; a working edge-boot path; cyrius 6.6.2 |
| 1.8.6 | 2026-08-24 | The fourth P(-1) audit (9 MEDIUM, 6 LOW, 1 DOC). It corrected the docs: argonaut does not enforce seccomp / Landlock / capabilities. 1.9.0 then added the pre-exec hook that kybernet enforces through |

Everything else is in [CHANGELOG.md](../../CHANGELOG.md).

## Consumers

- **AGNOS boot** — PID 1 / systemd-delegate role
- **kybernet** (AGNOS's PID 1) uses argonaut's `src/` as a library for service
  orchestration. At its 1.7.9 it pins argonaut **1.15.4** on cyrius 6.6.6.
  - **Taking 1.16.x needs cyrius ≥ 6.6.8** (`sys_fcntl`). At 1.16.0, a scratch kybernet on
    6.7.9 built and passed 882 / 882.
  - **Behaviour it will notice** is listed under CHANGELOG [1.16.1] and [1.16.2]
    Downstream. Two assertions in its `test_emergency_auth`
    (`src/test.cyr:1391-1393`) pin the old open `emergency_shell_default()`.
  - **It enforces service policy.** Its pre-exec hook (`kyb_pre_exec`,
    `src/lib/service_sandbox.cyr`) applies cgroup placement, `no_new_privs`,
    capabilities, Landlock and seccomp per service, through argonaut's
    `argonaut_set_pre_exec_hook` (1.9.0). argonaut itself applies none of it.
  - **Not yet rebuilt against 1.16.1 or 1.16.2.** That is kybernet's own update, and the
    roadmap's Kybernet-side section tracks it.

(Track downstream build status against this version when bumping minors.)

## Verification

- Linux x86_64 (Arch, kernel 7.2, AMD Ryzen 7 5800H, HPET clocksource) — primary dev host; CI on `ubuntu-latest`
- aarch64 — cross-build + full `.tcyr` sweep under `qemu-aarch64` on the dev host: **41 / 41, 1,662 assertions at 1.16.2** (cyrius 6.7.9), identical to x86_64. CI runs the aarch64 smoke only. Real-hardware validation (RPi4, Apple Silicon, Graviton / Ampere) is roadmap 1.17.2 (native aarch64 CI). See `docs/architecture/001-cross-arch-aarch64.md` for the surface.
- qemu PID-1 harness under KVM — passes at 1.16.2 at the scripts' committed `-m 256M`; at 1.16.0 the boot test also booted at `-m 128M`.
- PID 1 of an unprivileged pid namespace (`unshare -Urpf`) — idle wakeups, orphan reaping and SIGTERM shutdown, at 1.16.2.

## Audit cadence

- `docs/audit/` — security audit reports, dated `YYYY-MM-DD-audit.md`
- Most recent: **`2026-10-14-audit.md` (hardening and optimization pass for 1.16.1)** — 0 CRITICAL / 0 HIGH / 13 MEDIUM / 18 LOW fixed at 1.16.1. Its 6 deferred LOWs and 1 PERF were fixed at 1.16.2, with 2 more LOWs found on the way. The TPM item is documented in architecture note 005. Nothing from the pass is open. Prior: `2026-08-24` (P(-1) for 1.8.6; 9 MEDIUM / 6 LOW / 1 DOC, all closed), `2026-05-11` (1.6.3 closeout), `2026-05-10` (1.5.5 closeout), `2026-04-26` (1.4.0).
- The 2026-10-14 pass rated reachability three ways (shipped binary, kybernet config, library-only), since kybernet drives most of the lifecycle machinery from PID 1.
- **Re-audit trigger:** several severities (1.8.6 and 2026-10-14) are bounded by the fact that the shipped PID-1 loop does not call the service-lifecycle machinery (it is library surface for kybernet). **Re-rate them the day the supervisor loop is wired to `init_start_service` / `init_poll_health` / `init_notify_bind`.**
- Every release's Security section in `CHANGELOG.md` cites its audit.
