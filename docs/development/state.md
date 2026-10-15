# Argonaut — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Bumped at release time alongside
> `VERSION` and the CHANGELOG header.

## Version

**1.16.1** (the 2026-10-14 hardening and optimization pass: 13 MEDIUM and 18 LOW
fixed, each MEDIUM behind a test that failed first. The fixes include a bounded
HTTP check, process-group kills, an allocation-free hosts lookup, a strict PID-file
reader, closed unit-file injection, octal tmpfiles modes, private edge tmpfs, a
stoppable STARTING state, a restart window, monotonic deadlines, a bounded start
and a persisted audit chain. `ag_sys_poll` issues `sys_ppoll`. The PID-1 loop never
wakes while idle, syncs before poweroff, and survives a `signalfd` failure.
Resolution is one level-synchronous pass: waves up to 2× faster, `stats_desktop`
−79 %. 38 suites / 1,336 assertions on x86_64 and under qemu-aarch64. Bench gate:
a win. Several consumer-visible behaviour changes; see CHANGELOG [1.16.1].)

**1.16.0** (toolchain cyrius **6.6.20 → 6.7.9**; libro **2.10.8 → 2.11.0**, which
carries sigil 3.13.12 and patra 1.16.0; sakshi 2.5.8 and patra 1.16.0 from the 6.7.9
fold. The toolchain bump needed no source change, and none of 6.7.x's silent-semantics
shapes occur in the tree. The four `fcntl` calls moved to the stdlib's `sys_fcntl`, and
`ag_sys_fcntl` / `ag_sys_sendto` were removed. **Breaking:** a consumer compiling
argonaut's `src/` needs cyrius ≥ 6.6.8. 34 suites / 1,017 assertions on x86_64 and under
qemu-aarch64. Bench gate win / neutral. Closeout pass run for the minor.)

**1.15.6** (the startup order is deterministic. Each wave is sorted bytewise by
name, and `resolve_service_order` is the waves read in sequence, as ADR 008
always said. `send_notify` uses the stdlib's `sys_sendto`. `ag_sys_sendto` is a
deprecated forwarder, and `ag_sys_fcntl` stays until 1.16.0, because kybernet is
still on cyrius 6.6.6. 34 suites / 1,016 assertions on x86_64 and under
qemu-aarch64. Bench: `resolve_waves_wide_20` +3–4 µs for the sort, shipped with a
written justification. kybernet takes it with a tag bump.)

**1.15.5** (toolchain cyrius **6.6.6 → 6.6.20**; libro **2.10.3 → 2.10.8**, which
carries sigil 3.13.10 and patra 1.15.2; sakshi 2.5.7 and patra 1.15.2 from the 6.6.20
fold. `src/` changes are comments only. With 6.6.8's `MAP_NORESERVE` heap, argonaut
boots as PID 1 at `-m 256M` and down to `-m 128M`, which closes the 1.15.2
In-flight item. 17 test comparisons moved off `streq(str_data(..))`. 33 suites / 984
assertions on x86_64 and under qemu-aarch64. Bench gate neutral, with one micro
outside ±2 µs explained as noise. kybernet takes it with a tag bump.)

**1.15.4** (`RestartPolicy` and `SeccompAction` made value-identical to agnostik's,
NEVER = 0 / ALWAYS = 1 / ON_FAILURE = 2 and KILL = 1 / TRAP = 3 / LOG = 4, because the
two same-named enums meet in kybernet's unit. `restart_policy_parse` renamed
`argonaut_restart_policy_parse` (Breaking: one fn), so agnostik can add its own
F-018 pair. 33 suites / 984 assertions on x86_64 and under qemu-aarch64. Bench gate
neutral: the bench binaries are byte-identical to 1.15.3's. kybernet takes it with a
tag bump plus one renamed call.)

**1.15.3** (`default_services(BOOT_DESKTOP)` adds an `agnos-init` oneshot and
aethersafha `depends_on` it, so the compositor's `/run/agnos/{agents,plugins}`
socket directories exist before it starts. The program ships in the kybernet package
(kybernet 1.7.6). 33 suites / 976 assertions on x86_64 and under qemu-aarch64. Bench
gate neutral.)

**1.15.2** (toolchain cyrius **6.6.2 → 6.6.6**; libro **2.9.0 → 2.10.3**, which
carries sigil 3.12.18 and patra 1.14.3; sakshi 2.5.2 and patra 1.14.3 from the
6.6.6 fold — every library at its latest tag. `cyrius.cyml` cut back to a
manifest, 96 → 56 lines. No code change in `src/`. `audit_log_record` drops to
**16 B/record and 4.38 µs** (was 176 B and 7.21 µs) on libro 2.10's
allocation-free append. 33 suites / 971 assertions on x86_64 and under
qemu-aarch64. Bench gate win/neutral, no regression.)

1.13.3 – 1.15.1: see CHANGELOG — this section was not refreshed for them.

**1.13.2** (the edge-boot execution path could never succeed. `run_safe_cmd`
passed `SafeCommand`'s bare binary name — `"mount"`, `"veritysetup"`,
`"cryptsetup"` — to `exec_vec_str`, which calls `sys_execve` directly, and
**execve does not search `$PATH`**. Every command failed ENOENT and the child
exited 127, so `configure_readonly_rootfs` / `verify_rootfs_integrity` /
`unlock_luks` / `close_luks` were all inert. Unnoticed because no consumer
imports `src/edge_boot.cyr`. `_resolve_safe_binary` now resolves against
/usr/sbin, /sbin, /usr/bin, /bin with access(X_OK); explicit paths pass
through; nothing found returns 127 without forking. Found by kybernet's
v1.5.7 work, before it wired the path up — consuming it as-is with
readonly_rootfs required would have refused boot on every edge device.
872 → **886 assertions**; new tests observed failing on the unfixed tree.
Bench gate flagged three sub-µs benches; all three land within 1-2 ns of
1.12.0 and none reaches run_safe_cmd — 1.13.1 was the fast outlier, not
1.13.2 a regression. Also fixed: run_safe_cmd no longer delegates to the
stdlib's exec_vec_str, which waits UNBOUNDED and FAILS OPEN — it discards
waitpid's return and reads a STATIC status buffer (verified on 6.5.35), so a
failed wait reports the previous command's exit code, or 0 = success on the
first call. run_safe_cmd_timeout does a bounded WNOHANG wait with a checked
status, SIGKILL+reap on timeout, reset_child_signal_mask in the child, and a
real PATH in envp.)

**1.13.1** (aarch64 syscall + `struct stat` portability. Every literal
x86_64 `syscall(N, ...)` in argonaut is gone; the survivors live in one
audited file, `src/syscall_compat.cyr`. Nine numbers were landing on the
wrong kernel entry in the cross-built aarch64 binary kybernet ships:
`nanosleep`→`unlinkat`, `sendto`→`fstatfs`, `recvmsg`→`fallocate`,
`socketpair`→`fchmodat`, `unlink`→`timerfd_gettime`, `chmod`→`capget`,
`setsid`→`clock_settime`, `getsid`→`sched_yield`, and — worst —
`prctl(PR_SET_CHILD_SUBREAPER)`→`setsid`, so orphan reparenting was never
enrolled on ARM. Separately, `read_pid_file_safe` read `st_mode`/`st_uid`
at the x86_64 `struct stat` offsets; on the aarch64 asm-generic layout
those hold `st_uid`/`st_gid`, so for a root-owned file both read 0 and the
CVE-2025-4598-class world-writable check **failed open**. Now uses the
arch-dispatched `STAT_MODE`/`STAT_UID`. The aarch64 sweep goes 29 pass /
1 known-fail / 711 assertions → **30/30, 872 assertions**, identical to
native x86_64; arch doc 001's "qemu-user emulation limits" known-failure
allowance was a misdiagnosis of these bugs and has been removed from both
the doc and `scripts/aarch64-sweep.sh`. No API change; x86_64 behaviour
byte-equivalent. Bench: 0 regressions ≥15%, max delta +5.5%.)

**1.13.0** (`ServiceDefinition` carries cgroup limits. New field at +168 with
`svc_def_cgroup_limits` / `svc_def_set_cgroup_limits`, opaque to argonaut for
the same reason `seccomp`/`landlock`/`capabilities` are. Deliberately NOT a
reuse of `resource_limits` (+120): argonaut's `ResourceLimits` is
`{nofile, address_space, nproc, core}` while **agnostik independently defines
a different 48-byte struct with the same name** and ships the only `rlim_*`
accessors — and duplicate *structs* emit no compiler warning, so reading one
through the other's accessors is silent. `rlim_max_memory` on an
argonaut-built value returns `nofile`, which would have written "1024 open
files" to `memory.max` as 1024 bytes. Additive; every existing offset
unchanged. 868 → **872 assertions** across 30 suites. Bench not re-run — no
code path changed.)

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
- **Every call to a redefined fn binds its last definition** (6.6.20+). The main
  build defines no fn twice. The test and bench builds redefine eight `sakshi_*`
  fns as stubs, which prints eight `duplicate fn` warnings per build, as on 6.6.6.
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

- **x86_64: 443,848 B**, a statically linked ELF, at 1.16.1
  (`CYRIUS_DCE=1 cyrius build --check-lib-sync src/main.cyr build/argonaut`).
  2,544 fns are unreachable and 845,564 B of dead code is eliminated. That is
  +9,248 over 1.16.0's 434,600 B, for the hardening code (guards, the start-timeout
  loop, notify promotion, octal formatting, the strict parsers). 1.16.0 was
  +3,568 over 1.15.6, all toolchain.
- **aarch64: 1,569,992 B** at 1.16.1 (`CYRIUS_DCE=1 cyrius build --aarch64 src/main.cyr`).
  2,546 fns are unreachable and 1,002,288 B is NOPed. That is +66,568 over
  1.16.0's 1,503,424 B. aarch64 DCE fills dead code with NOPs instead of
  compacting, so new code shows in full. 1.15.5's −193,872 came from 6.6.18's
  ESYSXLAT compile-time fold.
- **L3 helper: 15,080 B**, a static ELF (`qemu/helpers/l3-helper`), re-cut under
  6.7.9 at 1.16.0 (15,064 B under 6.6.20). It is bundled into the qemu harness
  initramfs as `/bin/l3-helper`. Its build is warning-free.

## Suites

- **x86_64: 38 `.tcyr` suites / 1,336 assertions**, 0 failures (cyrius 6.7.9).
  +319 at 1.16.1: `harden_net` (98), `harden_proc` (106), `harden_core` (99),
  `poll_ms` (15) and one `health_exec` case.
- **aarch64 (qemu-user, `scripts/aarch64-sweep.sh`): 38 / 38, 1,336 assertions**,
  identical to x86_64. Run on the dev host at 1.16.1. CI still runs only the
  aarch64 smoke.
- **PID 1 in a pid namespace** (`unshare -Urpf --mount-proc ./build/argonaut`, no
  root needed): the quick check for the supervisor loop. At 1.16.1 it shows 0 idle
  wakeups in 3 s, reaps 300 simultaneous orphans with 0 zombies left, and shuts down
  cleanly on SIGTERM.
- **qemu PID-1 harness** (`qemu/pid1-harness-test.sh`, `qemu/boot-test.sh`,
  KVM + `+invtsc`): at 1.16.1, M3 + L3 end-to-end and the supervisor-loop smoke
  pass at the scripts' committed `-m 256M`. They have passed there since 1.15.5;
  the 1.15.4 binary, built on 6.6.6, panics under the same harness.
- **Benches**: `src/bench_main.cyr` — 29 micros, the release gate
  (`scripts/bench-history.sh`); `tests/bcyr/argonaut.bcyr` carries the same 29,
  `tests/bcyr/api.bcyr` 8 more. History in `bench-history.csv`.

### Bench snapshot (1.16.1-hardening, 2026-10-14)

**Verdict: win, no regression** against `1.16.0-cyrius-6.7.9`, on a quiet host
(load 0.4), plus a 24-round interleaved A/B pinned to one core:

| Bench | 1.16.0 | 1.16.1 | A/B Δ |
|---|---:|---:|---:|
| resolve_waves_chain_20 | 63.931 | 32.732 | −48.8 % |
| resolve_waves_desktop | 15.148 | 12.006 | −20.7 % |
| resolve_waves_wide_20 | 14.972 | 12.913 | −13.8 % |
| stats_desktop | 1.375 | 0.287 | −79.1 % |
| plan_runlevel_switch | 5.499 | 4.026 | −26.8 % |
| plan_shutdown_reboot | 15.206 | 14.041 | −7.7 % |
| generate_unit | 4.892 | 4.162 | −14.9 % |
| generate_tmpfile_cmds_20 | 12.417 | 13.468 | +1.051 µs (in band) |
| mark_all_steps_complete | 46.356 | 47.880 | +1.524 µs (in band) |

- `scripts/bench-history.sh` records one row per micro from this release on
  (29 per label, not 58).
- The `audit_log_record` bench measures the streaming chain from
  `1.16.1-audit-bench-streaming` on (4.098 µs). **Use that label as the next gate's
  baseline.** Its other 28 rows are the same code as `1.16.1-hardening`.

### Bench snapshot (1.16.0-cyrius-6.7.9, 2026-10-14)

**Verdict: win / neutral, no regression** against `1.15.6-baseline`: the 1.15.6 tag
re-run on a quiet host at this cycle, because 1.15.6's release-day row was
load-inflated. Both rows were recorded at load ~2.3, plus a 24-round interleaved A/B.

| Bench | 1.15.6-baseline | 1.16.0 | A/B Δ |
|---|---:|---:|---:|
| resolve_waves_desktop | 15.597 | 14.587 | −8 % |
| resolve_order_desktop | 12.206 | 11.646 | −9 % |
| plan_shutdown_reboot | 15.709 | 14.334 | −6 % |
| resolve_waves_wide_20 | 18.551 | 18.456 | −3 % |
| init_new_desktop | 24.652 | 24.681 | −1 % |
| audit_log_record | 4.338 | 4.462 | −1 % |
| resolve_order_chain_100 ⚠ | 229.428 | 192.313 | +2.5 % |

- ⚠ The chain micros remain noise-bound; no claim either way.
- Three sub-µs micros read `0.000` in the 1.16.0 row (`SUB-FLOOR`, below one
  clock read). No claim. Full table in CHANGELOG [1.16.0].
- `1.16.0-cyrius-6.7.9` is a clean row; it was the 1.16.1 gate's baseline.

### Bench snapshot (1.15.6-deterministic-order, 2026-10-14)

**Verdict: one regression, with a written justification**, against the 1.15.5 tag.
The gate row was recorded at load 16.7, with another job on every core, so it is
inflated about 1.5×. The verdict rests on interleaved A/B runs pinned to one core.

| Bench | 1.15.5 → 1.15.6 | read |
|---|---|---|
| resolve_waves_wide_20 ⚠ | +27 % to +34 % (≈ +3.3 to +4.3 µs) | the sort of a 20-name wave |
| resolve_waves_desktop, resolve_order_desktop, plan_shutdown_* | +8 % to +16 % (≈ +1.2 to +2.2 µs) | band edge |
| chain micros and the other 20 | the sign flips between runs | neutral |

- ⚠ **`resolve_waves_wide_20` is the cost of the fix**: sorting each wave is what
  makes the boot order reproducible, and a boot resolves a handful of times. A
  direct-call insertion sort was measured slower and dropped. Full tables are in
  CHANGELOG [1.15.6] Performance.
- ⚠ The next gate should compare against this label only with a quiet host; its row
  is load-inflated.

### Bench snapshot (1.15.5-cyrius-6.6.20, 2026-10-14)

**Verdict: neutral** against `1.15.4-agnostik-lockstep`. The gate row was recorded at
load 6.7 and reads the chain micros 7 to 57 µs slow. So the verdict rests on 24
interleaved rounds, pinned to one core, of three bench binaries: 1.15.4 on 6.6.6, the
1.15.4 tree on 6.6.20, and 1.15.5. Medians, µs:

| Bench | 1.15.4 (6.6.6) | toolchain only | 1.15.5 | Δ |
|---|---:|---:|---:|---:|
| resolve_order_chain_50 ⚠ | 102.662 | 107.101 | 106.840 | +4.178 |
| resolve_order_chain_100 | 222.250 | 225.080 | 221.983 | −0.267 |
| resolve_waves_chain_20 | 62.408 | 61.602 | 62.898 | +0.489 |
| resolve_order_desktop | 11.549 | 12.155 | 12.255 | +0.706 |
| init_new_desktop | 24.657 | 25.131 | 25.064 | +0.407 |
| plan_shutdown_reboot | 14.875 | 15.601 | 15.794 | +0.919 |
| mark_all_steps_complete | 46.876 | 47.581 | 47.760 | +0.884 |
| audit_log_record | 4.347 | 4.392 | 4.454 | +0.107 |

- ⚠ **`resolve_order_chain_50` is the one micro outside ±2 µs, and it is noise.** Its
  ranges overlap (91.4–111.2 µs against 93.5–138.4 µs). `chain_100`, which does the
  same per-node work on twice the nodes, reads −0.267 µs.
- The micros above 10 µs drift up 0.4–0.9 µs. The toolchain-only build drifts the same
  way, so the drift is 6.6.20's. Full table in CHANGELOG [1.15.5].

### Bench snapshot (1.15.3-agnos-init, 2026-09-23)

**Verdict: neutral** against `1.15.2-cyrius-6.6.6`. The desktop micros grow about
1.2 to 1.6 µs for one more service and edge (`init_new_desktop` 22.737 → 24.301,
`resolve_order_desktop` 9.681 → 10.888). `resolve_waves_chain_20` read +3.9 µs, and
three unrecorded reruns (53.9 / 64.4 / 50.4 µs) show that is its own noise. Full
table in CHANGELOG [1.15.3]. The 1.15.2 snapshot below is kept for the long view.

### Bench snapshot (1.15.2-cyrius-6.6.6, 2026-09-22)

**Verdict: win / neutral, no regression** against `1.15.1-baseline` — the
untouched 1.15.1 tree on 6.6.2, recorded this cycle because the prior label,
`1.13.2-exec-bounded`, was four releases old. The figures below are medians of
8 interleaved runs of each bench binary pinned to one core; the two CSV labels
are single runs.

| Bench | 1.15.1 (6.6.2) | 1.15.2 (6.6.6) | Δ µs |
|---|---:|---:|---:|
| **audit_log_record** | **7.213** | **4.383** | **−2.830 (−39 %)** |
| init_new_desktop | 23.244 | 23.170 | −0.074 |
| init_new_minimal | 5.725 | 5.699 | −0.026 |
| resolve_order_chain_50 ⚠ | 110.316 | 90.952 | −19.364 |
| resolve_order_chain_100 ⚠ | 232.963 | 213.245 | −19.718 |
| resolve_order_desktop | 10.240 | 9.902 | −0.338 |
| resolve_waves_chain_20 | 64.392 | 59.004 | −5.389 |
| resolve_waves_wide_20 | 13.150 | 13.611 | +0.461 |
| plan_shutdown_reboot | 13.017 | 12.772 | −0.245 |
| generate_unit | 5.120 | 5.139 | +0.019 |
| generate_tmpfile_cmds_20 | 12.729 | 12.853 | +0.123 |
| plan_runlevel_switch | 4.723 | 4.718 | −0.005 |
| mark_all_steps_complete | 45.648 | 45.578 | −0.070 |

- **`audit_log_record` is the one real move**, and it is libro 2.10.x's: the
  ranges do not overlap (6.81–7.43 µs against 4.19–4.41 µs).
- ⚠ **`resolve_order_chain_*` swings tens of µs run to run on this host** — the
  6.6.2 binary alone spans 88.7–158.2 µs on `chain_50`, and an unpinned A/B
  read it 13.7 µs *slower*. Neither direction is claimed. The stdlib it runs on
  did not change behaviour on x86_64: `hashmap` and `str` are byte-identical,
  and the `vec` / `alloc` diffs are include lines and comments.
- ⚠ **Sub-µs micros do not compare across 6.6.5's `lib/bench.cyr` rework**,
  which removed a per-window clamp that biased averages upward. No win is
  claimed for them.
- ⚠ `bench-history.csv` still records every bench twice per label (58 rows /
  29 unique) — see In-flight.

## Dependencies

- **stdlib (26 modules)**: `string fmt alloc vec str slice syscalls io fs
  process hashmap tagged args bayan fnptr freelist chrono ct keccak thread
  thread_local random assert bench sakshi patra`, resolved by `cyrius deps` into
  the gitignored `lib/`.
- **libro 2.11.0 — the sole git dep** (`modules = ["dist/libro.cyr"]`). It pins
  sigil **3.13.12** and patra **1.16.0**; with libro these are the three commit
  pins in `cyrius.lock`. No digest, proof or signature changed from 2.9.0 on.
  2.10.0 made a streaming append allocation-free, which is what takes
  `audit_log_record` to 16 B / ~4.4 µs.
  ⚠ On a streaming chain the head hash `chain_append_nokeep` returns — and
  `audit_log_record` passes through — is valid only until the second subsequent
  append.
  From 2.10.7, libro's `dist/libro.deps` sidecar names only `hashmap`, so
  argonaut's own `[deps] stdlib` must list every leaf argonaut calls. It does.
  From 2.10.8, `patrastore_open` returns 0, instead of crashing, if
  `patra_init()` has not run. argonaut runs `patra_init()` in `audit_ext_init`
  before it opens a store.
- **patra 1.16.0 and sakshi 2.5.8 come from the 6.7.9 fold**, byte-identical to
  their upstream `dist/` files. libro also resolves patra 1.16.0 as a package,
  and cyrius keeps the fold's copy (see Toolchain). On argonaut's `audit_persist`
  path (default off), patra 1.16.0 runs WAL recovery before every statement:
  correct after a crashed writer, and libro measured +5–7 % on appends. patra
  1.14.3's aarch64 `O_NOFOLLOW` fix still covers that path on aarch64
  (CHANGELOG [1.15.2] Security).
- **sigil 3.13.12 — thin sub-bundles only, transitive via libro**:
  `lib/sigil-mldsa.cyr` (ed25519 + ML-DSA + hybrid + sha512),
  `lib/sigil_sha256.cyr`, `lib/sigil_sha_ni.cyr` and `lib/sigil_hex.cyr` — 6,361
  lines / 264,388 B of source. 6.7.9 folds sigil 3.13.11, and libro's notes record
  that this thin set differs from 3.13.11 only in the version header.
  ⚠ **Never name `"sigil"` in the stdlib array.**
  At 6.6.6 that pulls the 28,084-line / 1,106,698 B monolith (x509, RSA,
  authenticode), and the x86_64 binary grows 403,672 → 900,472 B with a
  444,496 B static-data warning and 233 duplicate-fn warnings. Test and bench
  files must not `include "lib/sigil.cyr"` either; that rule lives in
  `tests/test_header.cyr`.
- **Security posture of the sigil 3.12.18 → 3.13.12 advance**: the ECDSA
  P-256 / P-384 timing fix (CYRIUS-2026-0023) is outside the four thin bundles.
  Inside them: the SHA-NI register scrub (3.13.8) and `sha256()` zeroing its
  context (3.13.11) matter for hashing secrets, and argonaut hashes audit
  entries. The thread-safe first-use init (3.13.6) matters only with threads,
  and argonaut starts none. Nothing on argonaut's path.
- **`cyrius.lock`** — `cyrius deps --verify` → **60 verified, 0 failed**,
  3 commit-pinned (libro, sigil, patra), sorted, ending with `cyrius 6.7.9`.
  It equals the lock a fresh copy of the tree resolves from nothing.

## In-flight

### Open (each is on the roadmap)

- **Real seccomp / Landlock / capability enforcement.** `src/security.cyr` has
  zero syscalls and nothing in the spawn path reads those fields, so every
  service runs with PID 1's root privileges. The largest gap between argonaut's
  stated surface and its behaviour. A feature, not a patch.
- **cgroup or pidfd-based PID identity.** The 2026-10-14 audit's B-1 closed the
  PID-file hole (symlinks, FIFOs, loose parsing); the reuse window between
  reading a PID and signalling it remains open by design.
- **Emergency shell:** `password_hash` is unsalted single-round SHA-256, and the
  default config has `require_auth = 0` (2026-10-14 audit, deferred).
- **Ordered shutdown (2026-10-14 audit C-8).** `init_stop_all` SIGTERMs
  everything at once and leaves services RUNNING with stale pids; kybernet
  builds `init_plan_shutdown` and then calls only this.
- **The unauthenticated `notify_try_recv` / `notify_drain` path** skips the
  SO_PEERCRED check and cannot detect a truncated datagram. No production
  caller; remove or rename it.
- **Other deferred LOWs from 2026-10-14:**
  - `WatchdogSec=` is emitted under `Type=simple`.
  - An ExecStart binary starting with `-@:+!` is read as a systemd prefix.
  - tmpfiles still runs external commands rather than a native `*at` path.
  - The CVE-2026-32606 class (TPM PCR policy) depends on kybernet's enrolment.
  - Every passing probe writes an audit record (PERF).
- **RE-AUDIT TRIGGER — wiring the supervisor loop.** Several severities stay
  bounded only because the shipped PID-1 loop never calls the lifecycle
  machinery. Re-rate them the day it does.
- **Hygiene:** native aarch64 CI; the release workflow still does not bump this
  file; the 80 `0 - N` literal sites in `src/` (cosmetic, batch alone).

### Closed recently

- **1.16.1:** the 2026-10-14 audit's 13 MEDIUM and 18 LOW
  (`docs/audit/2026-10-14-audit.md`). Also `bench-history.csv` recording every
  bench twice (open since 1.8.3) and the quickstart's raw x86 syscall number.
- **1.16.0:** `fcntl` and `sendto` on the stdlib; the `ag_` wrappers removed.
- **1.15.6:** the boot order of unordered services changed on every boot.
- **1.15.5:** PID 1 could not boot below ~256 MB (cyrius 6.6.8's
  `MAP_NORESERVE`).
- **Upstream sigil Ed25519 aarch64 verify quirk** (filed at 1.5.4): resolved
  upstream on 2026-05-21 under cyrius 6.0.1 / sigil 3.1.2 (sigil's archived
  issue file).

## Pending release (unreleased)

- **1.16.1** (cut in the working tree, not yet tagged): the 2026-10-14
  hardening and optimization pass, `ag_sys_poll` on `sys_ppoll`, and the
  idle-free PID-1 loop. See Version and CHANGELOG [1.16.1].

## Recent shipped

- **1.16.0 – 1.16.1** (2026-10-14) — toolchain 6.6.20 → 6.7.9 and libro 2.11.0;
  `fcntl` / `sendto` on the stdlib; the 2026-10-14 hardening and optimization
  pass (13 MEDIUM, 18 LOW). See CHANGELOG.
- **1.15.2 – 1.15.6** (2026-09-22 – 2026-10-14) — toolchain 6.6.6 → 6.6.20,
  libro 2.10.3 → 2.10.8, the agnostik enum lockstep, `agnos-init` before the
  compositor, PID 1 bootable at 256 MB, and a deterministic startup order. See
  CHANGELOG.
- **1.9.0 – 1.15.1** (2026-08-24 – 2026-09-11) — see CHANGELOG; this list was
  not refreshed for them.
- **1.8.6** (2026-08-24) — **P(-1) security / correctness / hardening pass**, the fourth and the first since 2026-05-11. `docs/audit/2026-08-24-audit.md`. **0 CRITICAL / 0 HIGH / 9 MEDIUM / 6 LOW / 1 DOC**, all closed, every MEDIUM+ with a regression test observed failing first. Six sweep findings were filed at HIGH; adversarial verification refuted four and corrected both survivors to MEDIUM. Headline: **PID-1 idle tick leaked 456 B/tick (~375 MB/day) → 0**; two **NULL derefs in the PID-1 health loop** (tests crashed the binary); **OOB read on every sd_notify datagram** (key strlen 65 vs a 64-byte buffer); **self-referential merkle verification** (BREAKING — verify wrappers now take the trusted root); **systemd unit injection + filename traversal**; **fail-open emergency auth**; **CVE-2018-16888-class PID-file check wired**; **tmpfiles device-type aliasing**. Documentation corrected: seccomp/Landlock/capabilities are **not enforced** (zero syscalls in `src/security.cyr`; services run as full root). 28 suites / **810 assertions** (+62). Bench gate **zero regressions** (`generate_unit` +0.506 µs is the deliberate cost of dep-name sanitization). Binary 1,124,776 B.
- **1.8.5** (2026-08-24) — toolchain pin bump to cyrius **6.5.35** + dep refresh to latest (**libro 2.8.0 → 2.8.12**, carrying **patra 1.12.9 → 1.13.10**, **sigil 3.11.1 → 3.12.9**, **sakshi 2.4.2 → 2.4.11**). **Retired the `sakshi` and `patra` git dep blocks** — both were silently downgrading stdlib-folded libraries on every build; the patra pin was reverting argonaut's *own* filed P1 fix. `libro` is now the sole git dep. **Breaking for persisted audit chains** (libro 2.8.11 + 2.8.12 change the entry preimage and the tree-head signature). Zero `src/` changes. Fixed two latent CI defects (the fmt gate broke under 6.5.35's rewrite-in-place `cyrius fmt`; 8 files carried drift already red at 6.4.62). CI + release now pass `--check-lib-sync`. Lockfile **56 verified / 0 failed**; bench gate **net win vs 1.8.4, zero regressions**.
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
- **kybernet** — uses argonaut as a library for service orchestration in the kybernet shell. Vendors argonaut's `src/` modules directly; pins argonaut **1.15.4** on cyrius 6.6.6 with libro 2.10.3. Against 1.16.0 a scratch kybernet fails on its 6.6.6 pin (`undefined function 'sys_fcntl'`). On 6.6.20 it builds at 746,680 B and on 6.7.9 at 746,136 B, passing 882 / 882 both times. So it takes 1.16.0 by raising its own cyrius pin to ≥ 6.6.8 in the same change. On 6.7.9 its own code warns: four `streq(str_data(..))` calls and 21 deprecated `json_v_obj_get` calls (CHANGELOG [1.16.0] Downstream). 1.16.1 changes behaviour on several paths kybernet drives: the start refusal, the 90 s start timeout, the restart window, the reused `init_poll_health` result, the bounded HTTP check and the strict PID-file reader (CHANGELOG [1.16.1] Breaking (behaviour) and Downstream). Not yet rebuilt against kybernet; that is kybernet's own update.
- **stiva, sutra, daimon** — service definitions and lifecycle hooks consume the audit chain

(Track downstream build status against this version when bumping minors.)

## Verification

- Linux x86_64 (Arch, kernel 7.2, AMD Ryzen 7 5800H, HPET clocksource) — primary dev host; CI on `ubuntu-latest`
- aarch64 — cross-build + full `.tcyr` sweep under `qemu-aarch64` on the dev host: **38 / 38, 1,336 assertions at 1.16.1** (cyrius 6.7.9), identical to x86_64. CI runs the aarch64 smoke only. Real-hardware validation (RPi4, Apple Silicon, Graviton / Ampere) is still the native-aarch64-CI item, gated on runner allocation. See `docs/architecture/001-cross-arch-aarch64.md` for the surface.
- qemu PID-1 harness under KVM — passes at 1.16.1 at the scripts' committed `-m 256M`; at 1.16.0 the boot test also booted at `-m 128M`.
- PID 1 of an unprivileged pid namespace (`unshare -Urpf`) — idle wakeups, orphan reaping and SIGTERM shutdown, at 1.16.1.

## Audit cadence

- `docs/audit/` — security audit reports, dated `YYYY-MM-DD-audit.md`
- Most recent: **`2026-10-14-audit.md` (hardening and optimization pass for 1.16.1)** — 0 CRITICAL / 0 HIGH / 13 MEDIUM / 18 LOW fixed; 6 LOW and 1 PERF deferred to the roadmap. Prior: `2026-08-24` (P(-1) for 1.8.6; 9 MEDIUM / 6 LOW / 1 DOC, all closed), `2026-05-11` (1.6.3 closeout), `2026-05-10` (1.5.5 closeout), `2026-04-26` (1.4.0).
- The 2026-10-14 pass rated reachability three ways (shipped binary, kybernet config, library-only), since kybernet drives most of the lifecycle machinery from PID 1.
- **Re-audit trigger:** several severities (1.8.6 and 2026-10-14) are bounded by the fact that the shipped PID-1 loop does not call the service-lifecycle machinery (it is library surface for kybernet). **Re-rate them the day the supervisor loop is wired to `init_start_service` / `init_poll_health` / `init_notify_bind`.**
- Prior audit references retained in `CHANGELOG.md` Security sections
