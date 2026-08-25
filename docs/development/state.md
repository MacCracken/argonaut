# Argonaut — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Bumped at release time alongside
> `VERSION` and the CHANGELOG header.

## Version

**1.12.0** (supervisor-injectable service environment. `argonaut_set_extra_env`
/ `argonaut_extra_env` let an embedder append `KEY=value` cstrs that every
spawned service inherits, appended after PATH by `build_default_envp`.
argonaut assembles the child envp between `fork` and `execve`, so before
this there was no seam at all — kybernet could bind an sd_notify socket but
never publish `$NOTIFY_SOCKET`, leaving the entire readiness/watchdog path
unreachable from the service side. Found by kybernet's 1.5.3 rust-old port
review. Purely additive; default unset reproduces 1.11.0 byte-for-byte.
30 suites / **868 assertions** (was 860). Lint / fmt / vet clean; lockfile
56 verified. Bench gate: two sub-microsecond benches flagged
(`on_service_crash` +22%, `state_transition_check` +20%) — **neither a
regression**; both report `min=0ns`, both land 8% *below* the
`1.8.6-audit-baseline` two labels back, and neither path reaches
`build_default_envp`. The `1.8.6-p-minus-1-audit` baseline was an
unusually fast run. Binary 1137728 bytes.)

> **Gap note:** 1.9.0 (argonaut_set_pre_exec_hook), 1.10.0, 1.10.1 and
> 1.11.0 (kernel capability numbers) shipped without refreshing this file.
> Their details are in `CHANGELOG.md`.

**1.8.6** (P(-1) security / correctness / hardening pass — the fourth, and
the first since 2026-05-11. Full report in
`docs/audit/2026-08-24-audit.md`. **0 CRITICAL / 0 HIGH / 9 MEDIUM / 6 LOW
/ 1 DOC**, all closed; every MEDIUM+ carries a regression test observed
**failing** against the unfixed tree first. Six findings were filed at HIGH
by the sweep; adversarial verification refuted four and corrected both
survivors to MEDIUM — nothing shipped as HIGH. Headline fixes: the **PID-1
supervisor loop leaked 456 bytes per 100 ms tick** (~375 MB/day, ~11
GB/month, in a process that can never restart — now **0 bytes/tick**); two
**NULL-pointer dereferences in the PID-1 health loop** whose tests crashed
the binary outright (an `https://` health URL panicked init); an
**out-of-bounds read on every sd_notify datagram** (a `READY` key hashing
with strlen **65** against a 64-byte allocation, because a cstr-keyed map
walked to a NUL the datagram never had); **self-referential merkle proof
verification** (BREAKING — the verify wrappers now take the trusted root
explicitly; cyrius 6.5.1's hard arity error makes migration loud);
**systemd unit injection** via an unsanitized dependency name plus
unit-filename path traversal; a **fail-open emergency-shell auth**; the
**CVE-2018-16888-class PID-file ownership check** that existed but was
never wired; and **tmpfiles device-type aliasing** that created character
devices as block devices. Also corrected a documentation claim that
seccomp/Landlock/capabilities are enforced — `src/security.cyr` has **zero
syscalls** and every service runs with PID 1's full root privileges.
28 suites / **810 assertions** (was 748). Lint / fmt / vet clean; lockfile
56 verified. Bench gate **zero regressions**; the one attributable movement
is `generate_unit` +0.506 µs (+10.4 %), the deliberate cost of sanitizing
dependency names. Binary 1,120,400 → 1,124,776 bytes.)

**1.8.5** (toolchain pin bump to cyrius **6.5.35** + dependency refresh to
latest tags, with `lib/` deleted and repopulated from scratch, and the
retirement of two dep pins that were silently *downgrading* the toolchain's
own bundled libraries. `cyrius.cyml` + `qemu/helpers/cyrius.cyml` pin
**6.4.62 → 6.5.35**; **libro 2.8.0 → 2.8.12**, carrying **patra 1.12.9 →
1.13.10**, **sigil 3.11.1 → 3.12.9** and **sakshi 2.4.2 → 2.4.11**
transitively. The headline is not size — it is that the `sakshi` and `patra`
git dep blocks were **reverting fixes on every build**. `lib/sakshi.cyr` and
`lib/patra.cyr` have shipped in the cyrius stdlib fold since at least 6.0.1;
`cyrius deps` overlays a git dep on top of that snapshot on *every*
`cyrius build`, so the 1.12.9 patra pin overwrote the fold's 1.12.10 —
**the release that fixed argonaut's own filed P1**, the `''` SQL-escaping
bug in libro's `patrastore_append` — and the 2.4.2 sakshi pin overwrote the
fold's 2.4.6. `deps --verify` cannot see this: the lock is written from
disk, so it records the downgraded hash and agrees with itself. Both blocks
retired; `"sakshi"` + `"patra"` named in the manifest's `stdlib` array
instead; both now verified byte-identical to the 6.5.35 fold. **libro is the
sole remaining git dep** — correctly, it is not folded. **Breaking for
persisted audit chains**: libro 2.8.11 and 2.8.12 each change the entry
preimage and 2.8.12 changes the tree-head signature, so chains written by
≤2.8.10 will not verify (affects `config.audit_persist` deployments only;
default off). **Zero argonaut `src/` change** was needed for the bump: all 28
`.tcyr` suites pass, and the build is **warning-free** for the first time
since 1.8.1. The only test addition is **+5 assertions** pinning a genuinely
new error path — libro 2.8.7 makes `patrastore_append` *reject* an over-255-byte
field with `PATRA_ERR_ROWSZ` where it previously truncated silently, and
argonaut's `details` is `str_from(service)`, so an over-long service name now
drops the persisted record while the in-memory chain still advances (**748
assertions** total). Binary **782,688 →
1,120,400 bytes (+43.1 %)**, entirely upstream and dominated by the *fold*
rather than the git deps — `lib/bayan.cyr` alone grows +496,834 B (bayan
1.1.0 → 1.5.2) of the +660,550 B stdlib delta. `.bss` is 84,336 B; no large
static data warning. Two latent CI defects fixed: `cyrius fmt <file>`
changed between 6.5.0 and 6.5.35 from print-to-stdout to **rewrite-in-place,
print-nothing**, which made CI's `diff -q <(cyrius fmt …)` gate fail
unconditionally *and* silently reformat the checkout — replaced with
`fmt --check`; and 8 files carried real fmt drift that was **already red at
6.4.62**, so 1.8.4's "fmt clean" claim was wrong (fixed; whitespace-only,
`git diff -w` empty). CI + release builds now pass `--check-lib-sync`, the
only automatic detector for the downgrade class. `cyrius.lock`: **56
verified, 0 failed** (was 54). Lint / fmt / vet clean. Mandatory bench gate
recorded as `1.8.5-cyrius-6.5.35` — **net win vs `1.8.4-cyrius-6.4.62`, zero
regressions** — 28 of 29 micros faster, one uptick
(`mark_all_steps_complete` +0.45 µs / +1.0 %, inside the ±2 µs band). The
apparent 85–88 % wins on the five sub-µs micros are a bench-clock **floor**
change, min 907 ns → 0 ns, not a speedup.)

**1.8.4** (toolchain pin bump to cyrius **6.4.62** + dependency refresh to
latest tags, with `lib/` deleted and repopulated from scratch. `cyrius.cyml`
+ `qemu/helpers/cyrius.cyml` pin **6.2.11 → 6.4.62**; **patra 1.11.2 →
1.12.9**, **libro 2.7.4 → 2.8.0**. libro 2.8.0 resolves a **thin sigil
surface** (sha256/ed25519/ML-DSA/hex sub-bundles) rather than the monolithic
`dist/sigil.cyr`, advancing transitive **sigil 3.7.14 → 3.11.1** and
**sakshi 2.2.3 → 2.4.2** and **dropping agnosys** (1.3.2) from the graph
entirely. The headline is a **51.7 % smaller binary**: the thin surface
drops sigil's x509/RSA/authenticode path (~13 MB static `.bss` the audit
chain never linked) — **786,776 bytes** (was 1,629,880 at 1.8.3), 1,683 dead
fns NOPed (was 2,970). Consumer-side migrations: **10 test/bench files dropped
the monolithic `include "lib/sigil.cyr"`** — 6 self-contained suites, both
benches, the shared `tests/test_header.cyr` (~21 suites), and the bench-gate
entry `src/bench_main.cyr` (none call `sigil_*` directly; libro's manifest
resolves the thin sub-bundles — removed the 13 MB static + 234-per-file
`duplicate fn` noise from test builds, and shrank the bench-gate binary from
14.3 MB to ~791 KB so the gate measures the same code as production); **3 suites gained the
`src/resolver.cyr` + `src/audit_ext.cyr` includes** they were missing
(6.4.62's stricter reachability turns the previously-unreachable undefined
refs into hard errors); **`audit_extended.tcyr` fixed to pass service names
as cstr** (a latent `str_from(Str)` misuse that 6.4.62 exposed as
`PATRA_ERR_SYNTAX` when the garbage bytes gained a `'`); **`argonaut.bcyr`
`audit_log_new` → `argonaut_audit_log_new`** (old pre-1.6.1 name, previously
satisfied by the sigil-monolith shadow); **`bench-history.sh` parser
rewritten** for 6.4.x decimal + ns/us/ms bench output (the old integer-`Nus`
regex silently recorded 0 rows). 28 `.tcyr` suites / 0 failures.
`cyrius.lock`: **54 verified, 0 failed** (was 49). Lint / fmt / vet clean.
Mandatory bench gate recorded as `1.8.4-cyrius-6.4.62` — **net win vs
`1.8.3-cyrius-6.2.11`, no regressions** (heavy chain/init micros −14 % to
−51 %; sub-µs upticks on 1 µs-scale micros are noise-floor + integer-rounding
artifacts). See Bench snapshot / `bench-history.csv`.)

**1.8.3** (toolchain pin bump to cyrius **6.2.11** + dependency refresh
to latest tags, with `lib/` deleted and repopulated from scratch.
`cyrius.cyml` + `qemu/helpers/cyrius.cyml` pin **6.0.56 → 6.2.11**;
**patra 1.10.3 → 1.11.2**, **libro 2.7.1 → 2.7.4** (latest), transitive
**sigil 3.6.0 → 3.7.14**. The 6.2.x stdlib reorg forced consumer-side
migrations: **`json` + `bigint` stdlib modules consolidated into
`bayan`** (manifest stdlib list + 9 test/bench include sites migrated to
`lib/bayan.cyr`); **libro sub-module includes collapsed to the single
`dist/libro.cyr` bundle** across the test/bench headers (9 files, per
libro `DEPS-PATTERN.md`); **`thread_local` now an explicit include ahead
of sigil** — 6.2.x's manifest auto-resolver no longer pulls a stdlib
module referenced only by a transitive git dep, and sigil 3.7.x's banked
crypto scratch calls `thread_local_*`, so without it the audit-hash path
SIGILLs. **Benchmark harness ported off the `alloc_reset()`+`alloc_init()`
fresh-chunk idiom** (cyrius 6.1.23 made `alloc_init` idempotent → the old
per-iteration reset SIGSEGV'd); replaced with a heap high-water-mark
rewind, sakshi span tracing silenced via a null emit hook. **x86_64 DCE
build** clean (**1,629,880 bytes**, **+332,136 / +25.6 %** from 1.8.1 —
entirely upstream: sigil 3.7.14 + patra 1.11.2 + libro 2.7.4 + bayan
under 6.2.11; no argonaut-side bloat); 2,970 dead fns NOPed (892,878
bytes). 28 `.tcyr` suites / 0 failures. `cyrius.lock`: **49 verified, 0
failed** (was 45). Lint / fmt clean. Mandatory bench gate recorded as
`1.8.3-cyrius-6.2.11` — net favorable vs `1.8.1-libro-2.7.1`; the five
small dependency-resolution regressions are toolchain + bench-harness-port
artifacts, not argonaut-side logic changes — see Bench snapshot /
`bench-history.csv`.)

**1.8.1** (UNRELEASED — toolchain pin bump to cyrius **6.0.53** + the
long-deferred **libro 2.6.2 → 2.7.1** bump, now unblocked. `cyrius.cyml`
+ `qemu/helpers/cyrius.cyml` both bumped 6.0.26 → 6.0.53, clearing the
`pins 6.0.26 but cycc is 6.0.53` drift warning. **libro 2.7.1** —
deferred since 1.7.1 because libro 2.6.3 tripped a `cycc` 6.0.14 single
compilation-unit limit (silent abort, exit 0); under **6.0.53** the
enlarged unit compiles clean. Pulls newer transitive crypto deps
(**sigil 3.0.1 → 3.6.0**, **agnosys 1.0.4 → 1.3.2**; sakshi held 2.2.3,
patra held 1.10.3 — already latest). Two stdlib modules added to the
manifest — **`slice`** (agnosys 1.3.2 lowers `s[i]` to `_slice_idx_get_W`
helpers) and **`thread_local`** (sigil 3.6.0 / `thread.cyr` reference
`thread_local_{init,get,set}`); without them the build fails. **`ct_eq`
compat shim RETIRED** — `src/compat.cyr` + the `[deps.argonaut_compat]`
self-dep deleted: libro 2.7.1 migrated to `ct_eq_bytes_lens`, which now
lives in the stdlib `ct` module (not sigil's mutable dist), so the
tag-mutation churn UPSTREAM-1 insulated against can no longer occur;
sigil 3.6.0 ships no `ct_eq`, and the long-standing `duplicate fn
'ct_eq'` warning is gone. **x86_64 DCE build** clean (**1,297,744
bytes**, **+253,304 / +24.3 %** from 1.8.0 — entirely upstream: sigil
3.6.0's crypto-bank static buffers (~159 KB `.bss`, new `large static
data` note) + larger sigil/agnosys code footprint; no argonaut-side
bloat, accepted as the cost of the latest crypto surface); 2,634 dead
fns NOPed (795,348 bytes reclaimed); 28 .tcyr suites / 743 assertions
green; benches neutral-to-win vs the 1.8.1 baseline. `cyrius.lock`:
**45 verified, 0 failed** (was 38). Lint / fmt / vet clean.)

**1.8.0** (shipped 2026-06-01 — toolchain pin bump to cyrius
**6.0.26** + 1.7.x closeout refactor. `cyrius.cyml` +
`qemu/helpers/cyrius.cyml` both bumped 6.0.14 → 6.0.26, clearing the
`pins 6.0.14 but cycc is 6.0.26` drift warning. **Closeout refactor**
(per CLAUDE.md Closeout Pass): removed a leftover `/child.marker`
debug write from `fork_exec_service`'s child branch — a 1.6.2 harness
artifact that wrote to the root FS on every service spawn under PID 1,
referenced nowhere (real L3 validation uses `/l3.marker` via the
helper); consolidated six open-coded `HealthCheckResult` allocations
into a `health_result_new` helper in `src/health.cyr`
(behavior-preserving); fixed a stale `cyrius.toml` → `cyrius.cyml`
comment in `src/main.cyr`. **Mandatory benchmark gate** added to
CLAUDE.md: every `VERSION` bump now runs the bench delta-check and is
release-blocked on an unexplained regression. **x86_64 DCE build**
clean (**1,044,440 bytes**, −704 from 1.7.1 — debug-block removal +
health consolidation); 28 .tcyr suites / 743 assertions green;
benches neutral vs the 1.8.0 baseline (apparent deltas < same-binary
run-to-run variance — see Bench snapshot). Dependencies unchanged:
patra 1.10.3, **libro held at 2.6.2** (2.6.3 still trips the `cycc`
unit limit under 6.0.26 — deferred). The `ct_eq` duplicate-fn warning
persists (harmless; sigil 3.0.1 dist + 1.5.5 compat shim). The broad
`0 - N` → `-N` negative-literal cleanup (74 sites, 9 files) was
deferred — cosmetic, zero perf/correctness value, would obscure the
release diff; tracked under In-flight.)

**1.7.1** (shipped 2026-05-28 — toolchain pin bump to cyrius
**6.0.14** + aarch64 cross-build restored. `cyrius.cyml` +
`qemu/helpers/cyrius.cyml` both bumped 5.10.44 → 6.0.14 (drafted
against 6.0.1; shipped on 6.0.14 once the `cycc_aarch64` fix
landed); **patra 1.9.3 → 1.10.3**. CI workflows + aarch64 dev
scripts picked up the `cc5_aarch64` → `cycc_aarch64` rename; the
`cyrius build`/`test`/`bench` driver surface is unchanged.
**aarch64 cross-build RESTORED** — the 6.0.1 `cycc_aarch64`
hang/stub on `src/main.cyr` is fixed in 6.0.14; `cyrius build
--aarch64` emits a real 1,166,336-byte ARM ELF, so the CI /
release 6.x-major gate is removed. **x86_64 DCE build** clean
(1,045,144 bytes); 28 .tcyr suites / 743 assertions green;
benches flat vs the 6.0.1 draft (±2 µs noise) — no regressions.
`cyrius.lock` now populated with per-file SHA-256s (was empty;
38 verified). **libro held at 2.6.2** — latest libro 2.6.3 makes
`cycc` 6.0.14 abort silently (no output, exit 0; over a unit
limit), deferred to a follow-up. Known warning: `ct_eq`
duplicate-fn (sigil 3.0.1 dist ships `ct_eq`, colliding with the
1.5.5 `src/compat.cyr` shim — harmless, not a cyrlint finding;
libro 2.6.3 would retire it but doesn't build). Operator gotcha
on x86_64: stale `./lib/` from pre-1.7.1 shadows the
version-pinned stdlib's `exec_*_str` family; `rm -rf lib &&
cyrius deps` clears it.)

**1.7.0** (shipped 2026-05-11 — boot-to-shell MVP path.
`default_services(BOOT_MINIMAL)` and
`build_boot_sequence(BOOT_MINIMAL)` now register/announce
agnoshi as a console shell (no `aethersafha` Wayland dep),
unblocking the AGNOS closed-beta MVP — kernel + kybernet +
agnoshi reaching a shell prompt on real iron without the
compositor stack. Breaking only for callers asserting the
1-service / 6-step BOOT_MINIMAL shape; kybernet ≤1.2.0 was
the only such consumer and bumped to 1.2.1 alongside.
BOOT_SERVER/DESKTOP/EDGE/RECOVERY unchanged.)

**1.6.3** (shipped 2026-05-11 — 1.6.x arc closeout. L3
end-to-end lands via `qemu/helpers/l3-helper.cyr` (12 KB
statically-linked cyrius helper writing `sid pid` via raw
syscalls — no shell, no dyn-loader; sidesteps the
busybox-shell blockers from 1.6.2). Full P(-1) audit per
CLAUDE.md procedure (findings in
`docs/audit/2026-05-11-audit.md`): **0 CRITICAL / 0 HIGH**;
2 MEDIUM closed with regression tests (signal-mask
inheritance into spawned services; empty envp dropping
PATH); 3 LOW (1 closed, 2 documented). Closes the
2026-04-26 audit's PID-1 graduation re-audit trigger.)

**1.6.2** (shipped 2026-05-10 — PID-1 harness extensions
(partial). New `src/pid1_harness.cyr` opt-in mode via
`/proc/cmdline argonaut.harness=1`; `pid1_harness_m3` validates
orphan reap under real-PID-1 reparenting end-to-end inside qemu.
`src/main.cyr` adds signalfd-blocked SIGTERM/SIGINT/SIGCHLD
handling — clean `sys_reboot(RB_POWER_OFF)` on shutdown signal.
Early `/proc`/`/sys`/devtmpfs `/dev` mounts under PID 1.
`fork_exec_service` rewritten to call `sys_execve` directly
(pre-1.6.2 the nested `exec_env_str` fork caused setsid to
apply to the wrong process — discovered + fixed during the
harness work). `qemu/pid1-harness-test.sh` + dyn-loader
bundling in `build-initramfs.sh`. L3 end-to-end deferred to
1.6.3.)

**1.6.1** (shipped 2026-05-10 — toolchain + cleanup combined.
Cyrius pin 5.10.34 → 5.10.44 picks up the typed Str-shape
`exec_*_str` family that argonaut's 1.5.2 upstream issue filed
against. Every `exec_vec` / `exec_env` call site in argonaut
(`run_safe_cmd`, `spawn_service`, `fork_exec_service`,
`check_command`) migrated to the `_str` variants — closes the
silent-failure-on-Str-argv class. `audit_log_new` →
`argonaut_audit_log_new` rename drops the sigil shadow warning.
`tests/tcyr/health_exec.tcyr` assertions flipped to strict
expected-result now that the silent-failure path is closed.)

**1.6.0** (shipped 2026-05-10 — PID-1 graduation. Argonaut runs
as `/sbin/init` under qemu, validated end-to-end via the new
`qemu/` harness (`build-initramfs.sh` + `boot-test.sh`, adapted
from the kybernet pattern). `src/main.cyr` adds a sleep-and-reap
supervisor loop gated on `sys_getpid() == 1` so the kernel
doesn't panic when init returns. KVM accel + `+invtsc` required
locally — TCG doesn't expose invariant TSC to sakshi's clock
init. Architecture doc 002 covers the surface. Boot wall time
~0.3 s under KVM. M3 / L3 end-to-end + signal-handled clean
shutdown deferred to 1.6.1+.)

**1.5.5** (shipped 2026-05-10 — 1.5.x arc closeout. Full
P(-1) pass per CLAUDE.md procedure; findings doc'd in
`docs/audit/2026-05-10-audit.md`. **0 CRITICAL / 0 HIGH**;
3 MEDIUM closed with regression tests (etc-hosts heap leak,
persistent log silent disk-fail, persistent log accepts
tampered chain); 4 LOW (3 closed, 1 documented); 2 UPSTREAM
(sigil dist tag instability mitigated via permanent
`src/compat.cyr` shim; sigil Ed25519-aarch64 quirk filed at
1.5.4). Orphan `src/test_*.cyr` stubs removed. kybernet
BC-clean against the 1.5.5 surface. 1.5.x arc CLOSED.)

**1.5.4** (shipped 2026-05-10 — cross-arch. Restores aarch64
builds via cyrius `cc5_aarch64` — no argonaut source changes
needed; the translator converts syscalls + ABI at codegen. CI /
release publish `argonaut-<VER>-aarch64-linux` alongside x86_64
as best-effort. `scripts/aarch64-sweep.sh` runs the full `.tcyr`
sweep under qemu-user with a documented known-failure budget
(2 suites trip qemu emulation limits + an upstream sigil
Ed25519-aarch64 verify quirk filed against sigil). Real-hardware
validation gated on aarch64 CI runner allocation, slipped to
1.6.x.)

**1.5.3** (shipped 2026-05-10 — libro extended surface. New
`src/audit_ext.cyr` adds opt-in PatraStore persistence
(record-by-record write-through, chain replayed via
`chain_from_entries` to preserve `prev_hash` linkage), snapshot
signing (Ed25519 / ML-DSA-65 / hybrid via libro
`proof_build_signed` — sign at boundaries, not per-record), and
merkle root + inclusion / consistency proof wrappers.
`ArgonautConfig` grows with `audit_persist` + `audit_path` fields
(default off); `argonaut_init_new` opens the persistent log when
configured and falls back to in-memory on open failure. New
`init_audit_record` / `init_audit_flush` dispatch helpers route
through the wrapper or chain automatically.)

**1.5.2** (shipped 2026-05-10 — HIGH-1 host resolver follow-up
patch. `src/resolver.cyr` adds a strict IPv4 dotted-quad parser
(rejects CVE-2021-29923-style leading-zero ambiguity) +
`/etc/hosts` scan; `check_tcp_connect` and HTTP_GET now route via
`resolve_host_ipv4` rather than hardcoding 127.0.0.1, with
distinct messages for resolver miss vs. connect failure vs.
unreachable. HTTP `Host:` header echoes the configured host so
virtual-host servers route correctly. `exec_env` Str/cstr quirk
filed upstream as a cyrius issue. Side benefit: sigil 3.0.1's
dist was re-published with `ct_eq` restored, retiring the 1.5.1
`src/compat.cyr` shim + `[deps.argonaut_compat]` self-dep one
minor early. DNS resolution + IPv6 transport explicitly deferred.)

**1.5.1** (shipped 2026-05-10 — toolchain + dep refresh patch. Cyrius
pin 5.7.5 → 5.10.34 (70+ upstream slots; sakshi/sigil promoted from
stdlib to external git pins; new `thread`/`random` stdlib modules);
libro 2.0.5 → 2.6.2; patra 1.1.1 → 1.9.3. `/lib/` moved out of the
tree (gitignored, repopulated by `cyrius deps`). CI / release
workflows aligned with agnosys / agnostik 5.10 pattern (versioned
toolchain layout, lockfile-gated hash verify, fmt-via-diff). New
`src/compat.cyr` shimmed `ct_eq` for libro 2.6.2's stale call
site, wired via `[deps.argonaut_compat]` self-reference;
both retired in 1.5.2 after sigil 3.0.1's upstream re-pub.)

**1.5.0** (shipped 2026-04-27 — PID-1 readiness minor: closes the
three 1.4.0 audit deferrals (M1 sd_notify SO_PEERCRED wiring, M3
generic-waitpid reaper + PR_SET_CHILD_SUBREAPER enrol, L3 setsid +
stdout/stderr dup2 in fork_exec_service) with regression coverage.
QEMU PID-1 boot harness for end-to-end M3/L3 validation slips to
1.6.0 alongside the HIGH-1 host resolver.)

**1.4.0** (shipped 2026-04-26 — P(-1) hardening minor:
docs/audit/2026-04-26-audit.md cycle, CLAUDE.md split into durable
rules + this state.md, eight audit findings landed with regression
tests, three deferred to 1.5.0 with helpers in place.)

**1.3.0** (shipped 2026-04-26 — toolchain + dep bump. Cyrius 4.5.0 → 5.7.5;
libro 1.0.3 → 2.0.5 (single-module `dist/libro.cyr`); manifest
`cyrius.toml` → `cyrius.cyml` with `version = "${file:VERSION}"`;
`cyrius.lock` introduced; CI/release workflows refactored to the
yukti 5.7-era pattern; patra `json_build/6` collision fix in
`tests/tcyr/serde.tcyr`.)

## Toolchain

- `cyrius = "6.5.35"` pinned in `cyrius.cyml [package]` (bumped from 6.4.62
  at 1.8.5). **6.5.x is a linear-scan register-allocator rewrite** — upstream
  reports the prior behaviour failing 69 of 282 corpus tests with *wrong
  answers rather than crashes*, which for PID 1 makes it the highest-risk
  bump class: a clean build proves nothing, only the full sweep + bench gate
  do. Five diagnostics became hard errors in the range (wrong argument count
  6.5.1; integer literal to a `: cstring` param 6.5.2; syntax errors in
  *uncalled* underscore-free functions 6.5.17; `cyrius lint` on an
  unparseable file 6.5.19; `#derive` placement 6.5.30/31) — argonaut trips
  none. `cyrius fmt <file>` now **rewrites in place and prints nothing**
  (see the comment on the fmt gate in `.github/workflows/ci.yml`).
  `freelist` became
  thread-safe at 6.5.19 at a **+13 % single-threaded** alloc+free cost that
  argonaut pays. Two coupled minimums: sigil ≥ 3.12.6 needs cyrius ≥ 6.5.14,
  sigil ≥ 3.12.1 needs ≥ 6.4.65 — so the toolchain pin and the libro 2.8.12
  pin move together.
- (was 6.4.62 at 1.8.4, 6.2.11 at
  1.8.3, 6.0.56 at 1.8.2, 6.0.53 at 1.8.1, 6.0.26 at 1.8.0, 6.0.14 at
  1.7.1; bumped to 6.4.62 at 1.8.4 to sit on the current toolchain. 6.4.x
  tightened the **reachability analysis** — a call to an undefined function
  that survives DCE as reachable is now a hard *"refusing to emit binary
  with N reachable undefined function(s)"* error, not a warning (surfaced
  three audit test suites missing `src/resolver.cyr`/`src/audit_ext.cyr`
  includes), and the bench harness now prints **decimal** timings in
  **mixed units** (ns/us/ms), which required the `bench-history.sh` parser
  rewrite. 6.2.x had consolidated the `json`/`bigint` stdlib modules into
  **`bayan`**, made `alloc_init()` idempotent (6.1.23 — see Bench notes),
  and tightened the manifest `stdlib` auto-resolver to skip a module
  referenced only by a transitive git dep — hence `thread_local` is an
  explicit include ahead of sigil. First 6.x adoption was 1.7.1; was
  5.10.44 at 1.7.0.)
- Compiler renamed `cc5` → `cycc` at Cyrius 6.0 (`cc5_aarch64` →
  `cycc_aarch64` follows). The `cyrius build`/`test`/`bench` driver
  is the stable surface — call sites in CI / scripts / dev loops use
  the driver, not the underlying compiler binary, except for the
  aarch64 cross-compiler check in `scripts/aarch64-{sweep,pi-smoke}.sh`
  and the cross-build guard in `.github/workflows/{ci,release}.yml`.
- No `.cyrius-toolchain` file; the manifest is the only pin source
- Versioned install layout: `~/.cyrius/versions/<V>/{bin,lib}` with
  `~/.cyrius/{bin,lib}` symlinking to the current one (required by
  the 5.10.9+ arch-peer include resolution; unchanged under 6.0)
- Some toolchain releases ship `cycc_aarch64` at the tarball
  top-level rather than under `bin/`. The install step in both
  workflows handles either layout (mirror of the
  agnosys/sankoch/yukti workaround)

## Binary

- **x86_64: ~1.12 MB** statically linked ELF (`CYRIUS_DCE=1 cyrius build --check-lib-sync src/main.cyr build/argonaut`, **1,124,776 bytes at 1.8.6** — +4,376 over 1.8.5 for the audit's new guards and the unit-name sanitizer; **1,120,400 bytes at 1.8.5** under cyrius 6.5.35; **+337,712 / +43.1 %** from 1.8.4's 782,688). Entirely upstream, and dominated by the **stdlib fold** rather than the git deps: across argonaut's declared module set the 6.4.62 → 6.5.35 snapshot grows **+660,550 bytes of source**, of which `lib/bayan.cyr` alone is **+496,834** (144,249 → 641,083; bayan 1.1.0 → 1.5.2). `lib/patra.cyr` +43,958, `freelist` +20,481, `thread` +16,077, `fs` +14,000, `alloc` +13,147, `chrono` +12,484. Git-dep side: libro's dist +90,404, sigil-mldsa +9,544. **Not a static-data regression** — `.bss` is **84,336 bytes**, no `large static data` warning. 2,260 unreachable fns NOPed (661,698 bytes reclaimed). No argonaut-side bloat.
- (1.8.4 measured 786,776 bytes as released; a rebuild of the same tree under the 6.5.35 wrapper gives 782,688 — that is the number the 1.8.5 delta is taken against, so the comparison isolates the dep/manifest change from wrapper drift.)
- **superseded** — 1.8.4 line: **~787 KB** statically linked ELF (`CYRIUS_DCE=1 cyrius build src/main.cyr build/argonaut`, **786,776 bytes at 1.8.4** under cyrius 6.4.62; **−843,104 / −51.7 %** from 1.8.3's 1,629,880). The drop is entirely upstream — **libro 2.8.0 resolves a thin sigil surface** (sha256/ed25519/ML-DSA/hex) instead of the monolithic `dist/sigil.cyr`, whose x509/RSA/authenticode bignum tables carried a ~13 MB static `.bss` footprint the audit chain never linked. No argonaut-side change. 1,683 unreachable fns NOPed (434,916 bytes reclaimed). (Was 1,629,880 at 1.8.3 under 6.2.11; 1,297,744 at 1.8.1 under 6.0.53; 1,044,440 at 1.8.0 under 6.0.26.)
- **L3 helper: 11936 bytes** static cyrius ELF (`qemu/helpers/l3-helper`); bundled into the qemu harness initramfs as `/bin/l3-helper`. **Committed binary still held at the 6.0.14 build** — a fresh `cyrius build` emits 14,592 bytes under 6.0.26 and **18,456 bytes under 6.5.35** (codegen drift; also warns `undefined function 'alloc'`, harmless given the helper's `stdlib = ["syscalls"]` and its raw-syscall-only body). The helper's syscall ABI is unchanged and the qemu harness only greps its `/l3.marker` output, so the fixture was again not re-cut at 1.8.5 — only `qemu/helpers/cyrius.cyml`'s pin moved 6.4.62 → 6.5.35. Re-cut it the next time the harness itself changes.
- **aarch64: 1,166,336 bytes** statically linked ARM ELF (`CYRIUS_DCE=1 cyrius build --aarch64 src/main.cyr`), **RESTORED under cyrius 6.0.14**. The 6.0.1 `cycc_aarch64` regression (hang > 5 min, or silent ~21 KB stub on `src/main.cyr`) is fixed; the CI / release 6.x-major gate is removed, leaving only the `cycc_aarch64`-presence check. The +121 KB delta vs x86_64 tracks aarch64's fixed-width instruction encoding. (Last green before the 6.0.1 regression: ~1.14 MB at 1.6.3 under `cc5_aarch64` 5.10.44.)
- Dead-code floor: **1,683 unreachable functions NOPed** under DCE at 1.8.4 / 6.4.62 (434,916 bytes reclaimed). −1,287 vs 1.8.3's 2,970 — the thin sigil surface links a far smaller crypto footprint, so there is less unreachable code to NOP in the first place. Not a public-surface change. (Was 2,970 at 1.8.3 / 6.2.11; 2,634 at 1.8.1 / 6.0.53; 2,090 at 1.8.0 / 6.0.26.)
- Was 378 KB at 1.2.0, 641 KB at 1.3.0, 650 KB at 1.4.0, 652 KB at
  1.5.0, ~990 KB at 1.5.1, ~995 KB at 1.5.2; +5 KB at 1.5.3 for
  the `src/audit_ext.cyr` wrapper module + new ArgonautInit slot
  + config fields. libro's patra/sign/merkle paths were already
  linked transitively; DCE now retains them since they're
  reachable from the public surface.

## Suites

- **Native x86_64: 28 .tcyr suites / 810 assertions** (0 failures on cyrius 6.5.35). **+62 at 1.8.6** — seven new `test_group`s in `tests/tcyr/audit_findings.tcyr`, one per MEDIUM finding of the 2026-08-24 P(-1) audit, each observed failing before its fix. Two assert *measurements* rather than behaviour (the notify key length, and that the PID-1 idle tick allocates exactly 0 bytes) because the defects they cover are quantitative. **1.8.5 required zero *src* changes** — the toolchain + dep bump was source-clean, which is the notable result given 6.5.x is a register-allocator rewrite. Test-tree edits: **+5 assertions** in `audit_findings.tcyr` for the new `libro-2.8.7-persist-oversize-field` group (pins the >255-byte `PATRA_ERR_ROWSZ` contract libro 2.8.7 introduced); canonical reformatting of `audit_extended.tcyr` + `modules_c.tcyr` (whitespace-only); a stale-comment fix in `serde.tcyr` (the patra `json_build/6` collision it warned about no longer exists). At 1.8.4 the toolchain/dep bump touched test *headers* — three suites (`audit_lifecycle`, `parity`, `cc3_ptr_regression`) gained `src/resolver.cyr` + `src/audit_ext.cyr` includes (6.4.62 reachability), `audit_extended` fixed three `str_from`-vs-cstr call sites, and ten files (incl. the shared `tests/test_header.cyr` and bench-gate `src/bench_main.cyr`) dropped the monolithic `lib/sigil.cyr` include — but the assertion surface is unchanged. +2 over 1.6.3 for the 1.7.0 BOOT_MINIMAL shape additions (`svcs_has_name` in `types_b.tcyr`, `steps_has_stage` in `types_a2.tcyr`).
- **qemu harness:** `qemu/pid1-harness-test.sh` covers M3 + L3 end-to-end under real PID 1 (KVM + `+invtsc`); `qemu/boot-test.sh` covers the supervisor-loop smoke. Both ~0.5 s wall time on local KVM.
- **aarch64 (qemu-user): unblocked under cyrius 6.0.14** — the `cycc_aarch64` cross-build works again, so the sweep can run. Last green sweep: **26 of 28** at 1.6.3 under `cc5_aarch64` 5.10.44 (2 suites in the documented known-failure budget — qemu emulation limits + upstream sigil Ed25519 quirk — see `docs/architecture/001-cross-arch-aarch64.md`). A fresh 6.0.14 sweep is pending a host with `qemu-aarch64` installed (absent on the current dev host); CI runs it.
- **2 .bcyr binaries** (`tests/bcyr/argonaut.bcyr`, `tests/bcyr/api.bcyr`)
- **37 benchmarks** wired into `src/bench_main.cyr`; history in `bench-history.csv`

### Bench snapshot (1.8.6-p-minus-1-audit, 2026-08-24)

**1.8.6 verdict: zero regressions.** All 29 micros sit inside the ±2 µs
noise band against the prior release label `1.8.5-cyrius-6.5.35`, and the
heavier ones are broadly faster.

**Correction on record.** An intermediate run (`1.8.6-post-audit`) showed
`generate_unit` +0.506 µs (+10.4 %) and it was first written up as the
attributable cost of audit MEDIUM-8's dependency-name sanitization.
Re-running the *same binary* gave 4.835 µs. Across four labels the micro
reads **4.911 / 4.860 / 4.835 / 5.366** — three clustered, one outlier —
and the outlier run has the **lowest `min` of all four** (3.873 µs) against
a 16.5 µs `max`. That is scheduling noise pulling up the mean, not the
sanitizer. **The claim is withdrawn**; the sanitization cost is not
measurable at this resolution.

**The real win is not a micro.** Audit MEDIUM-3 took the PID-1 idle
supervisor tick from **456 bytes/tick to 0** — ~375 MB/day of bump-arena
growth eliminated in a process that can never restart — and
`proc_table_reap` no longer materialises a key vec per call.

| Bench | 1.8.5 | 1.8.6 | Δ |
|---|---|---|---|
| build_boot_seq_desktop | 1.084 µs | 1.059 µs | −0.03 |
| init_new_desktop | 22.817 µs | 23.008 µs | +0.19 |
| init_new_minimal | 5.511 µs | 5.584 µs | +0.07 |
| resolve_order_chain_50 | 73.752 µs | 72.945 µs | −0.81 |
| resolve_order_chain_100 | 182.968 µs | 182.267 µs | −0.70 |
| **resolve_waves_chain_20** | **54.299 µs** | **52.598 µs** | **−1.70 (−3.1 %)** |
| resolve_order_desktop | 9.805 µs | 9.516 µs | −0.29 |
| plan_shutdown_reboot | 12.631 µs | 12.290 µs | −0.34 |
| generate_unit | 4.911 µs | 4.835 µs | −0.08 |
| generate_tmpfile_cmds_20 | 12.526 µs | 12.303 µs | −0.22 |
| plan_runlevel_switch | 4.833 µs | 4.597 µs | −0.24 |
| **mark_all_steps_complete** | **45.747 µs** | **44.100 µs** | **−1.65 (−3.6 %)** |
| audit_log_record | 7.113 µs | 6.949 µs | −0.16 |

⚠ **The five sub-µs micros are floor-limited.** Since the 6.4→6.5 bench
clock change they read `min = 0 ns`; their percentage swings are
meaningless at this scale. Treat 1.8.5 as their baseline and do not compare
across that boundary. (See CHANGELOG [1.8.5].)

⚠ **`bench-history.csv` records every bench twice per label** (58 rows / 29
unique). `src/bench_main.cyr` prints each micro once in its section and
again under `=== Summary ===`, and the `bench-history.sh` awk parser
matches both. Values are identical so comparisons are unaffected, but the
duplication predates 1.8.5. Fix the parser in a standalone patch.

(Full series in `bench-history.csv`; four labels were recorded this cycle —
`1.8.6-audit-baseline` per P(-1) step 3, `1.8.6-post-audit` per step 9, and
`1.8.6-p-minus-1-audit` as the release gate.)

## Dependencies

- **stdlib (26 modules)**: `string fmt alloc vec str slice syscalls io fs process hashmap tagged args bayan fnptr freelist chrono ct keccak thread thread_local random assert bench sakshi patra`. **`sakshi` + `patra` joined the array at 1.8.5**, replacing the retired git dep blocks — both are stdlib-*folded* libraries and take their version from the toolchain pin. `json` + `bigint` folded into **`bayan`** at 1.8.3/6.2.x; `slice` + `thread_local` explicit since 1.8.1 (`thread_local` must precede the transitive crypto; the auto-resolver won't pull it on its own). `atomic`, `sync` and `test` are declared leaves of `dist/libro.deps` but called zero times by `dist/libro.cyr` — deliberately **not** named, since naming a module force-includes it; cyrius resolves them transitively (clean-room `rm -rf lib && cyrius deps` → 56 deps, 0 failed).
- **libro 2.8.12 — the SOLE git dep** — single-module dist (`lib/libro.cyr`), `tag = "2.8.12" modules = ["dist/libro.cyr"]`. **Bumped 2.8.0 → 2.8.12 at 1.8.5** (latest). No public symbol renamed or removed; all 40 symbols argonaut calls from `src/audit.cyr` / `src/audit_ext.cyr` keep byte-identical signatures. Pins patra **1.13.10** + sigil **3.12.9** transitively — both matching the 6.5.35 fold, so the overlay and the fold agree and no downgrade is possible at this pin. Still resolves the **thin sigil surface**, not the monolith. ⚠ **2.8.11 + 2.8.12 are on-disk BREAKING** — see the Breaking section of CHANGELOG [1.8.5]; persisted chains from libro ≤2.8.10 will not verify.
- **patra 1.13.10 — from the FOLD, not a pin.** The explicit git block was **retired at 1.8.5**. It had pinned 1.12.9 over a 1.12.10 fold, reverting the `''` SQL-escaping fix (`patra_quote_str`) that argonaut itself filed — on every build, invisible to `deps --verify`. Now byte-identical to `~/.cyrius/versions/6.5.35/lib/patra.cyr` (sha256 `7cdc24d8…`). 1.13.0 removed all of patra's own git dep blocks for the same reason. Behaviour notes: **1.13.10** stopped `patra_init` calling `sakshi_set_level(SK_WARN)` (it was process-global and clobbered the host's level — argonaut calls `patra_init()` at `src/audit_ext.cyr:33`, so any future `SK_INFO` logging is no longer killed by store init); **1.13.6** returns `PATRA_ERR_ROWSZ` for an over-long STR instead of silently truncating to 255 B; **1.13.8** moved the WAL to format v4 (a v4 WAL is not readable by an older binary — mixed-version access to one `.patra` is the hazard).
- **sakshi 2.4.11 — from the FOLD, not a pin.** Explicit git block **retired at 1.8.5**; it had pinned 2.4.2 over a 2.4.6 fold, reinstating the `i64::MIN` formatter bug and the agnos `_sk_open` `O_RDWR` fold. Now byte-identical to the 6.5.35 fold (sha256 `585037b7…`). No breaking API change 2.4.2 → 2.4.11; argonaut's three `sakshi_span_enter` sites are balanced and it never calls `sakshi_log_kv`, so neither the 2.4.10 hook-payload nor the 2.4.11 span-refusal change reaches it.
- **sigil 3.12.9 — thin sub-bundles, transitive via libro; agnosys still out of the graph.** `lib/sigil-mldsa.cyr`, `lib/sigil_sha256.cyr`, `lib/sigil_sha_ni.cyr`, `lib/sigil_hex.cyr` (sha256 + ed25519 + ML-DSA + hex). `sha256.cyr`, `sha_ni.cyr` and `hex.cyr` are **byte-identical** 3.11.1 → 3.12.9; only `sigil-mldsa` changed (+9,544 B; sole module addition `mul64.cyr`). ⚠ Never name `"sigil"` in the stdlib array — that pulls the **monolith** (27,671 lines / 1,084,265 B at 6.5.35) with the whole x509 + RSA + authenticode surface. The `~13 MB static` figure it used to carry is **stale** post-3.12.9 de-banking (upstream now measures 785,408 B); the conclusion is not. Test/bench files must still **not** `include "lib/sigil.cyr"` — the rule now lives in `tests/test_header.cyr`, next to the include block.
- **Security posture of the sigil advance:** 3.12.9 ships two CRITICAL auth bypasses (3.12.3 PKCS#1 v1.5, 3.12.6 RSA-PSS) and a HIGH (3.12.9 Bellcore verify-after-sign). **None is on argonaut's linked path** — every fix is in `rsa.cyr` / `bignum.cyr` / `authenticode.cyr` / `sys_error.cyr`, none of which is in the `mldsa` profile, and `dist/sigil-mldsa.cyr` at 3.12.9 contains zero `bn_*` / `_rsa_*` / `x509_*` call sites. Argonaut also spawns no threads, and the whole lane-collision class needs ≥2. Recorded so it need not be re-derived.
- **`cyrius.lock`** — `cyrius deps --verify` → **56 verified, 0 failed**, **3 commit-pinned** (libro, sigil, patra); was 54 verified / 4 commit-pinned at 1.8.4. One fewer commit pin because sakshi now comes from the fold rather than a git tag. ⚠ `--verify` **cannot** detect a fold downgrade — the lock is written *from disk*. The mechanical guard is `cyrius build --check-lib-sync`, wired into CI + release at 1.8.5.

## In-flight

### Filed by the 2026-08-24 P(-1) audit (not fixed in 1.8.6)

- **Real seccomp / Landlock / capability enforcement.** `src/security.cyr`
  has **zero syscalls** — it renders description strings and one `setpriv`
  command nothing executes — and nothing in the spawn path reads the
  `seccomp` / `landlock` fields, so every service runs with PID 1's full
  root privileges. The docs claimed otherwise until 1.8.6 corrected them
  (audit DOC-1). This is the single largest gap between argonaut's stated
  surface and its behaviour. A feature, not a patch.
- **cgroup or pidfd-based PID identity.** argonaut has no cgroup usage at
  all (`grep -rn "cgroup" src/` is empty) and no `pidfd_open` /
  `pidfd_send_signal`. systemd's primary mitigation for both the PID-file
  class and the PID-reuse class is "the PID must be in the unit's cgroup".
  Audit MEDIUM-9 closed the immediate PID-file hole; the reuse window
  between reading a PID and signalling it remains open by design.
- **A real KDF for the emergency-shell password.** `password_hash` is a
  bare unsalted single-round SHA-256 — adequate against a shoulder-surfer,
  weak against an offline attacker holding the stored hash. Needs a
  dependency the thin sigil surface does not carry.
- **The unauthenticated `notify_drain` / `notify_try_recv` path** bypasses
  the M1 SO_PEERCRED check entirely. No production caller today
  (`src/init.cyr` uses the authenticated variant), so it is a footgun
  rather than a live hole — remove it or rename it so the distinction
  cannot be missed.
- **RE-AUDIT TRIGGER — wiring the supervisor loop.** Several 1.8.6
  severities are bounded by the fact that the shipped PID-1 loop never
  calls `init_start_service` / `init_poll_health` / `init_enforce_watchdog`
  / `init_notify_bind` / `init_stop_service` — that machinery is library
  surface for kybernet. **Re-rate every one of them the day the loop is
  wired.**
- **`bench-history.csv` records every bench twice per label** (58 rows / 29
  unique, since 1.8.3). `src/bench_main.cyr` prints each micro in its
  section and again under `=== Summary ===`; the `bench-history.sh` parser
  matches both. Comparisons are unaffected (values identical) but the
  parser should anchor on the pre-Summary block.


- **1.6.4 — Native aarch64 CI.** Real-arch validation in CI
  (not just qemu-user sweep). Adds an `aarch64-native` job
  to `.github/workflows/ci.yml` running the full `.tcyr`
  suite + qemu PID-1 harness natively; re-tests the
  1.5.4-filed sigil Ed25519-aarch64 quirk on real hardware.
  Real-hw smoke already validated manually via
  `scripts/aarch64-pi-smoke.sh` (added post-1.6.3, argonaut
  init in ~536 µs on RPi4); CI makes it durable. Gated on
  GitHub `ubuntu-24.04-arm` adoption or self-hosted Pi
  runner allocation.
- **Gated on external work:** WitnessAnchor publishing (AGNOS
  federation protocol); durable signing-key rotation
  (kybernet key-management surface); per-service env override
  (consumer-driven map → flat-cstrs conversion in
  `fork_exec_service`).
- **Gated on external work:** native aarch64 CI runner
  (runner allocation); WitnessAnchor publishing (AGNOS
  federation protocol); durable signing-key rotation
  (kybernet key-management surface).
- **Upstream — sigil Ed25519 aarch64 verify quirk** — filed at
  1.5.4 in sigil repo
  (`docs/development/issues/2026-05-10-ed25519-verify-aarch64-accepts-wrong-pk.md`).
  Consume via sigil bump once a fix lands.
- Release-hook gap — 1.4.0, 1.5.0, 1.7.0 all shipped without
  auto-bumping this file; 1.7.1, 1.8.0, **1.8.3, and 1.8.4** all
  hand-edited (1.8.3 also left the Dependencies section stale at
  1.8.1-era values — caught and corrected at 1.8.4). The workflow still
  does not auto-bump `state.md`; the "before 1.8.0" deadline slipped
  long ago. File against `.github/workflows/release.yml` before 1.9.0.
- **Deferred — `0 - N` → `-N` negative-literal cleanup.** 74 sites
  across 9 src files still use the pre-3.10.3 `0 - 1` form (resolver
  15, process_mgmt 20, init 19, tmpfiles 6, notify 5, others). `-N`
  has worked since cyrius 3.10.3 and CLAUDE.md discourages the old
  style, but the sweep is purely cosmetic (zero perf/correctness
  delta) and would have dominated the 1.8.0 release diff. Batch it as
  a standalone style-polish patch where it won't obscure functional
  changes.
- **RESOLVED (1.8.5) — the dep-pin downgrade class.** The `sakshi` and
  `patra` git blocks were overlaying older copies of stdlib-folded
  libraries on top of the toolchain snapshot on every build, invisible
  to `deps --verify`. Both retired; `cyrius build --check-lib-sync` is
  now wired into CI + release as the mechanical guard. The why-trail
  lives in the `cyrius.cyml` comment, next to the retired blocks. Kept
  here one release.
- **RESOLVED (1.8.4) — sigil monolith bloat in test builds.** libro
  2.8.0's thin sigil surface + dropping the explicit `include
  "lib/sigil.cyr"` from 10 test/bench files (incl. the shared header + bench entry) removes the ~13 MB static and
  the −51.7 % binary drop. (The `004` quirk doc that recorded this was
  deleted at 1.8.5 as a toolchain reference; the live rule moved into
  `tests/test_header.cyr`.) Kept here for one release as the why-trail. (The 6.0.1 `cycc_aarch64` and libro-2.6.3-unit-limit
  why-trails were pruned at 1.8.4 — long past their one-release window.)
- Stale `src/test_*.cyr` stub cleanup (predate `tests/tcyr/`).
- **RESOLVED (1.8.5) — patra `json_build/6` namespace.** Already fixed
  upstream at **patra 1.9.0**; argonaut's rule was stale by ~15 patra
  releases. `dist/patra.cyr` at 1.13.10 defines `patra_json_build` and
  `json_build_lens`, and no bare `json_build` — so the stdlib
  `json_build/1` (now in `bayan`) is unshadowed. CLAUDE.md's hard
  constraint and the `tests/tcyr/serde.tcyr` comment were both
  corrected at 1.8.5.
- **RESOLVED (1.8.5, upstream) — libro `patrastore_append` SQL
  interpolation.** Fixed in **libro 2.8.1**, whose release note credits
  **"argonaut P1"** by name: the write path is now a bound prepared
  statement (`patra_prepare` + 10x `patra_bind_text` +
  `patra_exec_prepared`), so values are stored as bytes and never
  reparsed as SQL. Two follow-ons closed at **2.8.11** — the
  `hash_algorithm` column was bound with `strlen()` on a `Str` fat
  pointer (garbage in every row written by 2.8.1–2.8.10), and the
  *read* path (`patrastore_by_source`) had never been migrated off
  interpolation. Kept here one release as the why-trail.
- **NEW (1.8.5) — three libro call sites worth migrating.** Not
  required to compile and deliberately left out of the 1.8.5 diff
  (behaviour change, not a dep bump), but all three targets exist in
  `lib/libro.cyr` today:
  - `src/audit_ext.cyr:224` `merkle_verify_proof(proof)` — libro now
    documents this as **self-referential**: it checks a proof against
    the root carried *inside that same proof*, so a proof for a
    different tree verifies. Migrate to
    `merkle_verify_inclusion(mp, expected_root, entry_h)`, passing
    argonaut's own trusted root and `merkle_leaf_hash(entry_hash(e))`.
  - `src/audit_ext.cyr:242` — same defect; migrate to
    `merkle_verify_consistency_against(cp, expected_new_root)`.
  - `src/audit_ext.cyr:103` `patrastore_load_all(store)` — now a legacy
    wrapper that **swallows I/O failure into an empty vec**, so a
    deleted or permission-stripped `.patra` is indistinguishable from
    an empty log (libro rates it High). Migrate to
    `patrastore_load_all_or_err` + `libro_is_error`. For PID 1 that
    distinction is the whole point.
- **NEW (1.8.5) — upstream request against libro.** libro 2.8.12 still
  carries its own `deps.patra` git block. It is byte-identical to the
  6.5.35 fold *at this pin*, so the hazard is closed today — but it
  reopens the moment cyrius folds a patra newer than libro's pin. Ask
  libro to drop the block (patra's own rule 2 forbids a git block for a
  folded module). Until then, re-check the `lib/patra.cyr` banner after
  a `cyrius build` — not after `cyrius deps` — on every toolchain bump.
- **NEW (1.8.5) — `bench-history.csv` records every bench twice.**
  58 rows / 29 unique per label at 1.8.3, 1.8.4 and 1.8.5 alike:
  `src/bench_main.cyr` prints each micro in its section and again under
  `=== Summary ===`, and the `bench-history.sh` awk parser matches
  both. Values are identical so comparisons are unaffected, but the
  duplication is real and predates 1.8.5. Anchor the parser on the
  pre-Summary block in a standalone patch.

## Pending release (unreleased)

- **1.8.6** (UNRELEASED — staged in the working tree, not yet tagged) — **P(-1) security / correctness / hardening pass**, the fourth and the first since 2026-05-11. `docs/audit/2026-08-24-audit.md`. **0 CRITICAL / 0 HIGH / 9 MEDIUM / 6 LOW / 1 DOC**, all closed, every MEDIUM+ with a regression test observed failing first. Six sweep findings were filed at HIGH; adversarial verification refuted four and corrected both survivors to MEDIUM. Headline: **PID-1 idle tick leaked 456 B/tick (~375 MB/day) → 0**; two **NULL derefs in the PID-1 health loop** (tests crashed the binary); **OOB read on every sd_notify datagram** (key strlen 65 vs a 64-byte buffer); **self-referential merkle verification** (BREAKING — verify wrappers now take the trusted root); **systemd unit injection + filename traversal**; **fail-open emergency auth**; **CVE-2018-16888-class PID-file check wired**; **tmpfiles device-type aliasing**. Documentation corrected: seccomp/Landlock/capabilities are **not enforced** (zero syscalls in `src/security.cyr`; services run as full root). 28 suites / **810 assertions** (+62). Bench gate **zero regressions** (`generate_unit` +0.506 µs is the deliberate cost of dep-name sanitization). Binary 1,124,776 B.
- **1.8.5** (UNRELEASED — staged, not yet tagged) — toolchain pin bump to cyrius **6.5.35** + dep refresh to latest (**libro 2.8.0 → 2.8.12**, carrying **patra 1.12.9 → 1.13.10**, **sigil 3.11.1 → 3.12.9**, **sakshi 2.4.2 → 2.4.11**). **Retired the `sakshi` and `patra` git dep blocks** — both were silently downgrading stdlib-folded libraries on every build; the patra pin was reverting argonaut's *own* filed P1 fix. `libro` is now the sole git dep. **Breaking for persisted audit chains** (libro 2.8.11 + 2.8.12 change the entry preimage and the tree-head signature). Zero `src/` changes. Fixed two latent CI defects (the fmt gate broke under 6.5.35's rewrite-in-place `cyrius fmt`; 8 files carried drift already red at 6.4.62). CI + release now pass `--check-lib-sync`. Lockfile **56 verified / 0 failed**; bench gate **net win vs 1.8.4, zero regressions**.

## Recent shipped

- **1.8.4** (2026-07-13) — toolchain pin bump to cyrius **6.4.62** + dep refresh (**patra 1.11.2 → 1.12.9**, **libro 2.7.4 → 2.8.0**). libro 2.8.0's **thin sigil surface** (sigil 3.7.14 → 3.11.1, sakshi 2.2.3 → 2.4.2, agnosys dropped) collapsed the x86_64 DCE binary 1,629,880 → 786,776 bytes (−51.7 %). 10 test/bench files dropped the monolithic `include "lib/sigil.cyr"`; 3 suites gained `src/resolver.cyr` + `src/audit_ext.cyr` includes; `bench-history.sh` parser rewritten for 6.4.x decimal/mixed-unit output. 28 / 743 green; lockfile 54 verified. (Its "lint / fmt / vet clean" claim was **wrong on fmt** — 8 files carried `--check` drift, caught and fixed at 1.8.5.)
- **1.8.3** (2026-06-15) — toolchain pin bump to cyrius **6.2.11** + dep refresh (**patra 1.10.3 → 1.11.2**, **libro 2.7.1 → 2.7.4**; transitive sigil 3.6.0 → 3.7.14). 6.2.x consolidated `json` + `bigint` stdlib modules into **`bayan`** (manifest + 9 test/bench include sites migrated); libro sub-module includes collapsed to the single `dist/libro.cyr` bundle; `thread_local` made an explicit include ahead of sigil; bench harness ported off the `alloc_reset()`+`alloc_init()` idiom to a heap high-water-mark rewind. Clean x86_64 DCE build (1,629,880 bytes; 2,970 dead-fns NOPed); 28 / 0 green; lockfile 49 verified.
- **1.8.0** (2026-06-01 — committed, untagged; superseded by 1.8.1) — toolchain pin bump to cyrius **6.0.26** + 1.7.x closeout refactor. Cleared the 6.0.14→6.0.26 pin-drift warning. Removed a leftover `/child.marker` debug write from `fork_exec_service`; consolidated six `HealthCheckResult` allocations into `health_result_new`; fixed a stale `cyrius.toml`→`cyrius.cyml` comment. Added a **mandatory benchmark gate** to CLAUDE.md (release-blocking on unexplained regression). Clean x86_64 DCE build (1,044,440 bytes, −704; 2,090 dead-fns NOPed); 28 / 743 green; benches neutral. patra 1.10.3, libro held at 2.6.2.

- **1.7.1** (2026-05-28) — toolchain pin bump to cyrius **6.0.14** + **aarch64 cross-build restored** (6.0.1 `cycc_aarch64` hang/stub fixed upstream; CI / release 6.x gate removed; real 1,166,336-byte ARM ELF). patra 1.9.3 → 1.10.3 (libro held at 2.6.2 — 2.6.3 trips a `cycc` 6.0.14 unit limit, deferred). Clean x86_64 DCE build (1,045,144 bytes, 2,086 dead-fns NOPed); 28 / 743 green; benches flat vs the 6.0.1 draft. `cyrius.lock` populated with per-file SHA-256s (was empty). Known harmless `ct_eq` duplicate-fn warning (sigil 3.0.1 dist ships `ct_eq`, collides with the live 1.5.5 compat shim).
- **1.7.0** (2026-05-11) — boot-to-shell MVP: `BOOT_MINIMAL` adds agnoshi as a console shell (no Wayland dep); service count 1 → 2, step count 6 → 7; breaking for callers asserting the pre-1.7.0 shape (kybernet ≤1.2.0); BOOT_SERVER/DESKTOP/EDGE/RECOVERY unchanged. Unblocks the AGNOS closed-beta MVP path.
- **1.6.3** (2026-05-11) — 1.6.x arc closeout: L3 end-to-end via static `qemu/helpers/l3-helper.cyr`; full P(-1) audit (0 CRITICAL / 0 HIGH, 2 MEDIUM closed with regressions — fork_exec sigmask + envp PATH); PID-1 graduation re-audit trigger CLOSED
- **1.6.2** (2026-05-10) — PID-1 harness extensions: M3 end-to-end + signalfd shutdown landed; `fork_exec_service` double-fork bug fixed (setsid wired correctly now); dyn-loader bundling in initramfs; L3 end-to-end deferred to 1.6.3
- **1.6.1** (2026-05-10) — toolchain + cleanup: cyrius 5.10.34 → 5.10.44; `exec_vec`/`exec_env` → `exec_vec_str`/`exec_env_str` migration across all argonaut call sites (closes 1.5.2 upstream issue); `audit_log_new` → `argonaut_audit_log_new` rename drops sigil shadow warning; `health_exec.tcyr` strict assertions
- **1.6.0** (2026-05-10) — PID-1 graduation: `src/main.cyr` adds sleep-and-reap supervisor loop on `getpid() == 1`; `qemu/build-initramfs.sh` + `qemu/boot-test.sh` validate boot end-to-end (three markers, ~0.3 s under KVM); `docs/architecture/002-qemu-pid1-harness.md` documents the surface (KVM + `+invtsc` requirement, future M3/L3 shape, re-audit trigger). 1.6.x arc opens.
- **1.5.5** (2026-05-10) — 1.5.x arc closeout P(-1) audit: 3 MEDIUM closed with regression tests (etc-hosts heap leak, persist silent disk-fail, persist tamper-rejected); LOW-1/2/3 closed (TCP pre-resolve split, HTTP port range, Host: header sanitize); orphan src/test_*.cyr stubs removed; sigil compat shim re-installed as permanent fixture
- **1.5.4** (2026-05-10) — cross-arch: aarch64 cross-build via `cc5_aarch64`; CI / release publish `argonaut-<VER>-aarch64-linux` best-effort; `scripts/aarch64-sweep.sh` local sweep with documented known-failure budget; `docs/architecture/001-cross-arch-aarch64.md` documents the surface; upstream sigil Ed25519 aarch64 quirk filed
- **1.5.3** (2026-05-10) — libro extended surface: `src/audit_ext.cyr` adds opt-in PatraStore persistence, Ed25519/MLDSA/hybrid snapshot signing, merkle root + inclusion / consistency proofs; `argonaut_init_new` integration via `config.audit_persist`; `init_audit_record` / `init_audit_flush` dispatch helpers
- **1.5.2** (2026-05-10) — HIGH-1 host resolver follow-up: `src/resolver.cyr` adds IPv4 dotted-quad parser + /etc/hosts scan; health checks route via `resolve_host_ipv4`; HTTP Host: header echoes configured host; `exec_env` Str/cstr quirk filed upstream; 1.5.1 compat shim retired (sigil 3.0.1 dist re-pub restored `ct_eq`)
- **1.5.1** (2026-05-10) — toolchain + dep refresh patch: cyrius 5.7.5 → 5.10.34, libro 2.0.5 → 2.6.2, patra 1.1.1 → 1.9.3; `/lib/` gitignored; CI/release workflows aligned with 5.10 pattern; `src/compat.cyr` shims `ct_eq` for libro
- **1.5.0** (2026-04-27) — PID-1 readiness minor; closes the three 1.4.0 audit deferrals (M1 sd_notify SO_PEERCRED wiring, M3 orphan reaper + subreaper enrol, L3 setsid + stdout/stderr dup2)
- **1.4.0** (2026-04-26) — P(-1) hardening minor; eight audit findings landed (2 HIGH, 1 MEDIUM, 5 LOW), three deferred to 1.5.0; CLAUDE.md durable / state.md volatile split
- **1.3.0** (2026-04-26) — toolchain + dep bump (cyrius 5.7.5, libro 2.0.5, cyrius.cyml manifest, lockfile)
- **1.2.0** (2026-04-13) — libro 1.0.2 SHA-256 audit chain integration, lifecycle audit recording, P(-1) scaffold hardening
- **1.0.0** (2026-04-12) — first 1.x release; full pre-1.0 feature set complete
- **0.96.1** (2026-04-11) — rust-old removed (Rust source deleted; Cyrius is the implementation)

## Consumers

- **AGNOS boot** — PID 1 / systemd-delegate role
- **kybernet** — uses argonaut as a library for service orchestration in the kybernet shell
- **stiva, sutra, daimon** — service definitions and lifecycle hooks consume the audit chain

(Track downstream build status against this version when bumping minors.)

## Verification

- Linux x86_64 (Arch, 6.18 LTS) — primary dev + CI host
- aarch64 — **cross-build restored under cyrius 6.0.14** (the 6.0.1 `cycc_aarch64` regression is fixed). The local 6.0.14 cross-build produces a real 1,166,336-byte ARM ELF; a full `.tcyr` sweep + smoke under `qemu-aarch64` is pending a host with qemu-user installed (absent on the current dev host — CI runs it). Last green sweep / smoke: 1.6.3 under `cc5_aarch64` 5.10.44 / qemu-user 11.0.0-1 (26 of 28). Real-hardware validation (RPi4, Apple Silicon, Graviton / Ampere) is slipped to the 1.6.4 native-aarch64-CI item, gated on runner allocation. See `docs/architecture/001-cross-arch-aarch64.md` for the surface.

## Audit cadence

- `docs/audit/` — security audit reports, dated `YYYY-MM-DD-audit.md`
- Most recent: **`2026-08-24-audit.md` (P(-1) for 1.8.6)** — 0 CRITICAL / 0 HIGH / 9 MEDIUM / 6 LOW / 1 DOC, all closed. Prior: `2026-05-11` (1.6.3 closeout), `2026-05-10` (1.5.5 closeout), `2026-04-26` (1.4.0).
- **Re-audit trigger:** several 1.8.6 severities are bounded by the fact that the shipped PID-1 loop does not call the service-lifecycle machinery (it is library surface for kybernet). **Re-rate them the day the supervisor loop is wired to `init_start_service` / `init_poll_health` / `init_notify_bind`.**
- Prior audit references retained in `CHANGELOG.md` Security sections
