# Argonaut — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Bumped at release time alongside
> `VERSION` and the CHANGELOG header.

## Version

**1.16.3** (cyrius 6.7.18, libro 2.11.3, sigil 3.13.15; every dependency at its latest
tag.
- **The bump needed no `src/` change.** `tests/tcyr/serde.tcyr` moves off the two JSON getters
  bayan 1.11 deprecated.
- **One fix (MEDIUM):** `fork_exec_service` no longer closes a std fd that `open("/dev/null")`
  returned as 0, 1 or 2. A service now gets all three, even when PID 1 starts without them.
- **Binary:** the x86_64 binary grows 448,448 → 761,960 B. The cause is bayan's Unicode
  tables, which reach every build through `sigil-mldsa` (see Binary).
- **Docs:** the documentation sweep that followed 1.16.2 ships here too.

42 suites / 1,670 assertions on x86_64 and under qemu-aarch64. Bench gate: neutral. See
CHANGELOG [1.16.3].)

Earlier releases are in [CHANGELOG.md](../../CHANGELOG.md). The pinned release plan
from 1.17.0 on is in [roadmap.md](roadmap.md).

## Toolchain

- `cyrius = "6.7.18"` in `cyrius.cyml [package]` and `qemu/helpers/cyrius.cyml`.
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
  - **Main build:** 23 fns are defined twice. `lib/sigil-argon2.cyr` repeats sigil's
    bank / lane / TLS / once scaffolding (`_sigil_ensure_slot` … `_sigil_wipe64`), which
    `lib/sigil-mldsa.cyr` also defines. At sigil 3.13.15 all 23 bodies are identical, so
    the binding changes nothing. **Re-diff them at every sigil bump**; any build that
    includes both bundles (kybernet's does) prints the same set.
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
  argonaut boots as PID 1 at `-m 256M`, `160M` and `128M`. At 1.16.3 it boots at
  `-m 96M` too, on the dev host's 7.2.11 kernel; an earlier kernel panicked there
  before argonaut ran.
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

- **x86_64: 761,960 B**, a statically linked ELF, at 1.16.3
  (`CYRIUS_DCE=1 cyrius build --check-lib-sync src/main.cyr build/argonaut`).
  2,987 fns are unreachable and 1,118,796 B of dead code is eliminated.
  - **+313,512 over 1.16.2's 448,448 B.** +308,544 B of that is the 6.7.18 toolchain alone
    (the 1.16.2 pins built under 6.7.18).
  - **The cause:** `.rodata` grew 39,778 → 382,342 B. bayan 1.9.0 (folded at 6.7.13) includes
    seven `lib/unicode/` files whose tables are top-level string `var`s, and DCE does not
    remove global initializers. bayan reaches the build through `sigil-mldsa`, so it cannot be
    dropped from argonaut's side.
  - **The fix is upstream** (sigil or cyrius). Inside an initramfs the file sits in RAM, and the
    qemu boot test still boots at `-m 96M`.
- **aarch64: 2,183,024 B** at 1.16.3 (`CYRIUS_DCE=1 cyrius build --aarch64 src/main.cyr`).
  2,987 fns are unreachable and 1,325,172 B is NOPed. That is +612,536 over 1.16.2's
  1,570,488 B. aarch64 DCE fills dead code with NOPs instead of compacting, so new code
  usually shows in full.
- **L3 helper: 15,080 B**, a static ELF (`qemu/helpers/l3-helper`), re-cut under
  6.7.18 at 1.16.3 with `CYRIUS_DCE=1`. It is bundled into the qemu harness
  initramfs as `/bin/l3-helper`. Its build is warning-free.
- **`cyrius capacity`** at 1.16.3: `fn_table` 3,736 / 131,072, `string_data`
  382,342 / 2,097,152.

## Suites

- **x86_64: 42 `.tcyr` suites / 1,670 assertions**, 0 failures (cyrius 6.7.18).
  +8 at 1.16.3: `spawn_std_fds`, the std-fd regression test.
- **aarch64 (qemu-user, `scripts/aarch64-sweep.sh`): 42 / 42, 1,670 assertions**,
  identical to x86_64. Run on the dev host at 1.16.3. CI still runs only the
  aarch64 smoke; native aarch64 CI is roadmap 1.17.2.
- **PID 1 in a pid namespace** (`unshare -Urpf --mount-proc ./build/argonaut`, no
  root needed): the quick check for the supervisor loop. At 1.16.3 it shows 0 idle
  wakeups in 3 s, reaps 300 simultaneous orphans with 0 zombies left, and shuts down
  cleanly on SIGTERM.
- **qemu PID-1 harness** (`qemu/pid1-harness-test.sh`, `qemu/boot-test.sh`,
  KVM + `+invtsc`): at 1.16.3, M3 + L3 end-to-end and the supervisor-loop smoke
  pass at the scripts' committed `-m 256M`. They have passed there since 1.15.5,
  the first release built with a `MAP_NORESERVE` heap (cyrius 6.6.8+). At 1.16.3 the
  boot test also boots at `-m 128M` and `-m 96M`.
- **Benches**: `src/bench_main.cyr` — 29 micros, the release gate
  (`scripts/bench-history.sh`); `tests/bcyr/argonaut.bcyr` carries the same 29,
  `tests/bcyr/api.bcyr` 8 more. History in `bench-history.csv`.

### Bench snapshot (1.16.3-cyrius-6.7.18, 2026-10-29)

**Verdict: neutral, no regression.** The comparison is against `1.16.2-baseline` (the same tree
on 6.7.9, run on the same host just before the pin moved), plus a 24-round interleaved A/B of
the two bench binaries, pinned to one core:

| Bench | 1.16.2-baseline | 1.16.3 gate | A/B Δ |
|---|---:|---:|---:|
| resolve_order_chain_100 ⚠ | 210.370 | 230.147 | −3.1 % (ranges overlap) |
| resolve_waves_chain_20 | 32.914 | 36.200 | −4.3 % |
| resolve_waves_wide_20 | 12.358 | 15.173 | −2.1 % |
| resolve_order_desktop | 11.488 | 11.152 | −4.4 % |
| plan_shutdown_reboot | 13.864 | 13.862 | −3.5 % |
| mark_all_steps_complete | 46.892 | 47.385 | −0.2 % |
| audit_log_record | 3.947 | 3.962 | 0.0 % |

- **Cleared by the A/B:** the three gate-row reads beyond ±2 µs (`resolve_order_chain_100`,
  `resolve_waves_chain_20` and `resolve_waves_wide_20`). None of their paths changed.
- **Sub-µs:** the four `build_boot_seq_*` read +0.03 to +0.06 µs in the A/B, with overlapping
  ranges.
- **Every other micro** is within ±0.6 µs.
- **Next baseline:** `1.16.3-cyrius-6.7.18` is the next gate's baseline.

Earlier snapshots are in the CHANGELOG's Performance sections; every row is in
`bench-history.csv`.

## Dependencies

- **stdlib (26 modules)**: `string fmt alloc vec str slice syscalls io fs
  process hashmap tagged args bayan fnptr freelist chrono ct keccak thread
  thread_local random assert bench sakshi patra`, resolved by `cyrius deps` into
  the gitignored `lib/`.
- **libro 2.11.3** (`modules = ["dist/libro.cyr"]`). It pins sigil **3.13.15** and
  patra **1.16.6**; with libro these are the three commit pins in `cyrius.lock`.
  - A streaming append is allocation-free: `audit_log_record` costs 16 B / ~4.2 µs.
  - ⚠ On a streaming chain, the head hash `chain_append_nokeep` returns (and
    `audit_log_record` passes through) is valid only until the second subsequent
    append. Every in-tree caller discards it.
  - libro's `dist/libro.deps` sidecar names only `hashmap`, so argonaut's own
    `[deps] stdlib` must list every leaf argonaut calls. It does.
  - `patrastore_open` returns 0 if `patra_init()` has not run. argonaut runs
    `patra_init()` in `audit_ext_init` before it opens a store.
- **patra 1.16.6, sakshi 2.5.10 and bayan 1.11.1 come from the 6.7.18 fold.** patra
  and sakshi are byte-identical to their upstream `dist/` files. libro also resolves
  patra 1.16.6 as a package, and cyrius keeps the fold's copy (see Toolchain).
  - **patra on `audit_persist`** (default off): it runs WAL recovery before every
    statement, which is correct after a crashed writer; libro measured +5–7 % on appends.
    Failed `fdatasync`s are reported (1.16.2), and `patra_open` refuses before
    `patra_init` (1.16.1).
  - **bayan:** argonaut calls no bayan function, but `sigil-mldsa` includes it, and
    with it seven `lib/unicode/` table files that DCE cannot remove (see Binary).
- **sigil 3.13.15 — thin sub-bundles only.**
  - **Declared at the root** (`[deps.sigil]`, 1.16.2) at libro's tag, so it resolves as
    one package. The block repeats libro's four modules and adds `dist/sigil-argon2.cyr`,
    for the emergency-shell credential.
  - **The bundles:** `lib/sigil-mldsa.cyr` (ed25519 + ML-DSA + hybrid + sha512),
    `lib/sigil_sha256.cyr`, `lib/sigil_sha_ni.cyr`, `lib/sigil_hex.cyr` and
    `lib/sigil-argon2.cyr`, 7,945 lines / 335,570 B of source. Argon2 is 1,496 lines /
    66,810 B of that.
  - **Keep the tag equal to libro's sigil pin at every libro bump.**
  - 6.7.18 folds sigil 3.13.15, the same tag, so the root pin and the fold agree.
  - **Crypto lanes are a pool** (3.13.14, CYRIUS-2026-0040). argonaut is single-threaded and
    calls no ECDSA, Poly1305 stream or direct EC arithmetic. So the pool's fail-stop (exit 70
    when every lane is held) cannot fire in PID 1.
  - ⚠ **Never name `"sigil"` in the stdlib array.** That pulls the monolith (x509, RSA,
    authenticode), which roughly doubles the x86_64 binary and prints hundreds of
    duplicate-fn warnings (measured on cyrius 6.6.6: 403,672 → 900,472 B). Test and bench
    files must not `include "lib/sigil.cyr"` either; that rule lives in
    `tests/test_header.cyr`.
- **`cyrius.lock`** — `cyrius deps --verify` → **69 verified, 0 failed**,
  3 commit-pinned (libro, sigil, patra), sorted, ending with `cyrius 6.7.18`.
  It equals the lock a fresh copy of the tree resolves from nothing.

## In-flight

### Open

Each item is pinned to a release in [roadmap.md](roadmap.md).

- **1.17.0:** removal of the deprecated `notify_try_recv`, `notify_drain` and
  `spawn_service`, and the inert-helper decision. The next cyrius pin has no release
  number; its target is open.
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

- **1.16.3:** the cyrius pin move the roadmap had planned for 1.17.0 (6.7.18), and every
  dependency at its latest tag. Also a MEDIUM found by its review: `fork_exec_service`
  could start a service with stdin or stdout closed.
- **1.16.2:** every item the 2026-10-14 audit deferred, plus two LOWs found while
  fixing them: SCM_RIGHTS fds left open in PID 1, and an unaudited restart-limit
  refusal (`docs/audit/2026-10-14-audit.md`, "Fixed at 1.16.2").
- **1.16.1:** the 2026-10-14 audit's 13 MEDIUM and 18 LOW.
- **1.16.0:** `fcntl` and `sendto` on the stdlib; the `ag_` wrappers removed.

## Pending release

None after 1.16.3, which is ready to tag. The next release is 1.17.0
([roadmap.md](roadmap.md)).

## Recent shipped

| Releases | Date | Headline |
|---|---|---|
| 1.16.3 | 2026-10-29 | cyrius 6.7.18, libro 2.11.3, sigil 3.13.15; services always get std fds (MEDIUM); the documentation sweep; binary +313 KB of bayan Unicode tables |
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
  - **Not yet rebuilt against 1.16.1, 1.16.2 or 1.16.3.** That is kybernet's own update,
    and the roadmap's Kybernet-side section tracks it. 1.16.3's one `src/` change is the
    std-fd fix in `fork_exec_service`, which kybernet spawns every service through.
  - **When it moves its own pin to 6.7.18,** its binary takes the same bayan Unicode
    tables, through its own `sigil-mldsa`, whichever argonaut it pins.

(Track downstream build status against this version when bumping minors.)

## Verification

- Linux x86_64 (Arch, kernel 7.2, AMD Ryzen 7 5800H, HPET clocksource) — primary dev host; CI on `ubuntu-latest`
- aarch64 — cross-build + full `.tcyr` sweep under `qemu-aarch64` on the dev host: **42 / 42, 1,670 assertions at 1.16.3** (cyrius 6.7.18), identical to x86_64. CI runs the aarch64 smoke only. Real-hardware validation (RPi4, Apple Silicon, Graviton / Ampere) is roadmap 1.17.2 (native aarch64 CI). See `docs/architecture/001-cross-arch-aarch64.md` for the surface.
- qemu PID-1 harness under KVM — passes at 1.16.3 at the scripts' committed `-m 256M`; the boot test also boots at `-m 128M` and `-m 96M`.
- PID 1 of an unprivileged pid namespace (`unshare -Urpf`) — idle wakeups, orphan reaping and SIGTERM shutdown, at 1.16.3.

## Audit cadence

- `docs/audit/` — security audit reports, dated `YYYY-MM-DD-audit.md`
- Most recent: **`2026-10-14-audit.md` (hardening and optimization pass for 1.16.1)** — 0 CRITICAL / 0 HIGH / 13 MEDIUM / 18 LOW fixed at 1.16.1. Its 6 deferred LOWs and 1 PERF were fixed at 1.16.2, with 2 more LOWs found on the way. The TPM item is documented in architecture note 005. Nothing from the pass is open. Prior: `2026-08-24` (P(-1) for 1.8.6; 9 MEDIUM / 6 LOW / 1 DOC, all closed), `2026-05-11` (1.6.3 closeout), `2026-05-10` (1.5.5 closeout), `2026-04-26` (1.4.0).
- The 2026-10-14 pass rated reachability three ways (shipped binary, kybernet config, library-only), since kybernet drives most of the lifecycle machinery from PID 1.
- **Re-audit trigger:** several severities (1.8.6 and 2026-10-14) are bounded by the fact that the shipped PID-1 loop does not call the service-lifecycle machinery (it is library surface for kybernet). **Re-rate them the day the supervisor loop is wired to `init_start_service` / `init_poll_health` / `init_notify_bind`.**
- Every release's Security section in `CHANGELOG.md` cites its audit.
