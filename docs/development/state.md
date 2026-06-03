# Argonaut — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Bumped at release time alongside
> `VERSION` and the CHANGELOG header.

## Version

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

- `cyrius = "6.0.53"` pinned in `cyrius.cyml [package]` (was 6.0.26 at
  1.8.0, 6.0.14 at 1.7.1; bumped to 6.0.53 at 1.8.1 to match the
  installed wrapper and clear the pin-drift warning — and, critically,
  6.0.53 raises/clears the `cycc` single-unit limit that blocked the
  libro 2.6.3+ bump under 6.0.14. First 6.x adoption was 1.7.1 — drafted
  on 6.0.1, shipped on 6.0.14 once the `cycc_aarch64` cross-build fix
  landed; was 5.10.44 at 1.7.0)
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

- **x86_64: ~1.30 MB** statically linked ELF (`CYRIUS_DCE=1 cyrius build src/main.cyr build/argonaut`, **1,297,744 bytes at 1.8.1** under cyrius 6.0.53; **+253,304 / +24.3 %** from 1.8.0's 1,044,440). The growth is entirely upstream-transitive — sigil 3.6.0's crypto-bank static buffers (`var buf[32768]` + multiple 4 KB SIGIL_CRYPTO_BANKS slots, ~159 KB of `.bss` flagged by the new `large static data (159616 bytes)` build note) plus the larger sigil 3.6.0 / agnosys 1.3.2 code footprint. No argonaut-side bloat; accepted as the cost of the latest crypto surface. 2,634 unreachable fns NOPed (795,348 bytes reclaimed). (Was 1,044,440 at 1.8.0 under 6.0.26.)
- **L3 helper: 11936 bytes** static cyrius ELF (`qemu/helpers/l3-helper`); bundled into the qemu harness initramfs as `/bin/l3-helper`. **Committed binary held at the 6.0.14 build** — under 6.0.26 a fresh `cyrius build` emits a 14,592-byte helper (codegen drift), but the helper's syscall ABI is unchanged and the qemu harness only greps its `/l3.marker` output, so the bundled fixture was not re-cut at 1.8.0 to avoid churning the vendored `qemu/helpers/lib/` snapshot. Re-cut it the next time the harness itself changes.
- **aarch64: 1,166,336 bytes** statically linked ARM ELF (`CYRIUS_DCE=1 cyrius build --aarch64 src/main.cyr`), **RESTORED under cyrius 6.0.14**. The 6.0.1 `cycc_aarch64` regression (hang > 5 min, or silent ~21 KB stub on `src/main.cyr`) is fixed; the CI / release 6.x-major gate is removed, leaving only the `cycc_aarch64`-presence check. The +121 KB delta vs x86_64 tracks aarch64's fixed-width instruction encoding. (Last green before the 6.0.1 regression: ~1.14 MB at 1.6.3 under `cc5_aarch64` 5.10.44.)
- Dead-code floor: **2,634 unreachable functions NOPed** under DCE at 1.8.1 / 6.0.53 (795,348 bytes reclaimed). +544 vs 1.8.0's 2,090 — the newer sigil 3.6.0 / agnosys 1.3.2 dists ship a much larger surface, most of which argonaut never reaches, so DCE NOPs far more. Not a public-surface change. (Was 2,090 at 1.8.0 / 6.0.26.)
- Was 378 KB at 1.2.0, 641 KB at 1.3.0, 650 KB at 1.4.0, 652 KB at
  1.5.0, ~990 KB at 1.5.1, ~995 KB at 1.5.2; +5 KB at 1.5.3 for
  the `src/audit_ext.cyr` wrapper module + new ArgonautInit slot
  + config fields. libro's patra/sign/merkle paths were already
  linked transitively; DCE now retains them since they're
  reachable from the public surface.

## Suites

- **Native x86_64: 28 .tcyr suites / 743 assertions** (0 failures on cyrius 6.0.53). +2 over 1.6.3 for the 1.7.0 BOOT_MINIMAL shape additions (`svcs_has_name` in `types_b.tcyr`, `steps_has_stage` in `types_a2.tcyr`); 1.7.1, the 1.8.0 closeout refactor, and the 1.8.1 toolchain/dep bump (incl. the `ct_eq` shim retirement) all left the test surface untouched.
- **qemu harness:** `qemu/pid1-harness-test.sh` covers M3 + L3 end-to-end under real PID 1 (KVM + `+invtsc`); `qemu/boot-test.sh` covers the supervisor-loop smoke. Both ~0.5 s wall time on local KVM.
- **aarch64 (qemu-user): unblocked under cyrius 6.0.14** — the `cycc_aarch64` cross-build works again, so the sweep can run. Last green sweep: **26 of 28** at 1.6.3 under `cc5_aarch64` 5.10.44 (2 suites in the documented known-failure budget — qemu emulation limits + upstream sigil Ed25519 quirk — see `docs/architecture/001-cross-arch-aarch64.md`). A fresh 6.0.14 sweep is pending a host with `qemu-aarch64` installed (absent on the current dev host); CI runs it.
- **2 .bcyr binaries** (`tests/bcyr/argonaut.bcyr`, `tests/bcyr/api.bcyr`)
- **37 benchmarks** wired into `src/bench_main.cyr`; history in `bench-history.csv`

### Bench snapshot (1.8.1-libro-2.7.1, 2026-06-03)

**1.8.1 verdict: neutral-to-win — no regression.** The
`1.8.1-libro-2.7.1` label (cyrius 6.0.53 + libro 2.7.1) sits at-or-below
the `1.8.1-baseline` taken before the bump on every micro — all deltas
within the ±2 µs run-to-run noise floor, with the heaviest chain micros
a touch faster: `resolve_order_chain_100` 215 → 209 µs,
`resolve_order_chain_50` 89 → 85, `resolve_waves_chain_20` 65 → 61. The
+253 KB binary growth is pure static `.bss` / code from the newer crypto
deps (see Binary) — it does not touch any benchmarked hot path. Full
series (`1.8.1-baseline`, `1.8.1-libro-2.7.1`) in `bench-history.csv`.

The 1.7.1 codegen wins below still hold (vs `1.6.3-post-audit`); the
table is retained as the standing reference micro-by-micro. The shipped
`1.7.1-patra-1.10.3` label (cyrius 6.0.14 + patra 1.10.3) landed within
±2 µs of the 6.0.1 draft on every micro.

| Bench | 1.6.3 avg | 1.7.1 avg | Δ |
|---|---|---|---|
| build_boot_seq_desktop | 5 µs | 5 µs | 0 |
| init_new_desktop | 36 µs | 35 µs | −1 |
| resolve_order_desktop | 12 µs | 11 µs | −1 |
| **resolve_order_chain_50** | **92 µs** | **84 µs** | **−8 (−8.7 %)** |
| **resolve_order_chain_100** | **217 µs** | **207 µs** | **−10 (−4.6 %)** |
| resolve_waves_chain_20 | 65 µs | 62 µs | −3 |
| resolve_waves_desktop | 15 µs | 15 µs | 0 |
| plan_shutdown_reboot | 18 µs | 17 µs | −1 |
| plan_runlevel_switch | 9 µs | 7 µs | −2 |
| mark_all_steps_complete | 70 µs | 68 µs | −2 |
| audit_log_record | 8 µs | 7 µs | −1 |
| health_tracker_record | 1 µs | 1 µs | 0 |
| state_transition_check | 1 µs | 1 µs | 0 |

(See `bench-history.csv` for the full 37-bench series + every prior label.)

## Dependencies

- **stdlib (25 modules)**: `string fmt alloc vec str slice syscalls io fs process hashmap tagged args json fnptr freelist bigint chrono ct keccak thread thread_local random assert bench` (sakshi + sigil dropped — promoted upstream from stdlib to external git pins; thread + random added at 1.5.1 for libro; **`slice` + `thread_local` added at 1.8.1** — agnosys 1.3.2 lowers `s[i]` subscripts to `_slice_idx_get_W` helpers (`lib/slice.cyr`), and sigil 3.6.0 / `thread.cyr` reference `thread_local_{init,get,set}` (`lib/thread_local.cyr`); without them the build fails)
- **libro 2.7.1** — single-module dist (`lib/libro.cyr`) via `[deps.libro] tag = "2.7.1" modules = ["dist/libro.cyr"]`. **Bumped 2.6.2 → 2.7.1 at 1.8.1**, the bump deferred since 1.7.1 (libro 2.6.3 made `cycc` 6.0.14 abort silently over a single-unit limit). Compiles clean under 6.0.53.
- **patra 1.10.3** — explicit dep (bumped from 1.9.3 at 1.7.1), was transitive of libro 2.0. Already the latest tag; held at 1.10.3. Builds clean under 6.0.53.
- **sakshi 2.2.3 + sigil 3.6.0 + agnosys 1.3.2** — transitive via libro 2.7.1 (sakshi also via patra 1.10.3); resolved into `lib/` by `cyrius deps`. sigil **3.0.1 → 3.6.0** and agnosys **1.0.4 → 1.3.2** at 1.8.1 (sakshi unchanged). **The `ct_eq` compat shim is RETIRED** (`src/compat.cyr` + `[deps.argonaut_compat]` deleted): libro 2.7.1 calls `ct_eq_bytes_lens` directly, that symbol now lives in the stdlib `ct` module (`lib/ct.cyr`) rather than sigil's mutable dist, and sigil 3.6.0 ships no `ct_eq` — so the `UPSTREAM-1` tag-mutation risk is structurally gone and the `duplicate fn 'ct_eq'` warning with it. (sigil 3.6.0's larger crypto-bank dist is the source of the +253 KB binary growth — see Binary.)
- **`cyrius.lock`** — 6.0.53's `cyrius deps` populates per-file SHA-256 hashes for all **45** resolved `lib/*.cyr` units (`cyrius deps --verify` → `45 verified, 0 failed`); was 38 at 1.7.1/1.8.0 — newer sigil/agnosys split into more files, minus the dropped `argonaut_compat_compat.cyr`.

## In-flight

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
- Release-hook gap — 1.4.0, 1.5.0, **1.7.0 all shipped without
  auto-bumping this file**; 1.7.1 *and* 1.8.0 hand-edited. The
  workflow still does not auto-bump `state.md`; the "before 1.8.0"
  deadline slipped. File against `.github/workflows/release.yml`
  before 1.9.0.
- **Deferred — `0 - N` → `-N` negative-literal cleanup.** 74 sites
  across 9 src files still use the pre-3.10.3 `0 - 1` form (resolver
  15, process_mgmt 20, init 19, tmpfiles 6, notify 5, others). `-N`
  has worked since cyrius 3.10.3 and CLAUDE.md discourages the old
  style, but the sweep is purely cosmetic (zero perf/correctness
  delta) and would have dominated the 1.8.0 release diff. Batch it as
  a standalone style-polish patch where it won't obscure functional
  changes.
- **RESOLVED — `cycc_aarch64` 6.0.1 hang/stub on `src/main.cyr`** —
  fixed in cyrius **6.0.14**: `cyrius build --aarch64` emits a real
  1,166,336-byte ARM ELF. The 6.x-major CI / release gate added at
  the 1.7.1 draft is removed. (Was: hang > 5 min or silent ~21 KB
  stub under 6.0.1.) Kept here for one release as the why-trail.
- **RESOLVED (1.8.1) — libro 2.6.3+ over `cycc` 6.0.14 unit limit.**
  The silent-abort-on-large-unit blocker (libro 2.6.3 + transitive
  sigil/agnosys made `cycc` 6.0.14 exit 0 with no output) is cleared
  under **cyrius 6.0.53**: **libro 2.7.1** (+ sigil 3.6.0 / agnosys
  1.3.2) compiles clean. The `src/compat.cyr` `ct_eq` shim rode out
  with this bump as predicted — libro 2.7.1 migrated to
  `ct_eq_bytes_lens` (now a stdlib `ct` symbol). Kept here for one
  release as the why-trail. Cost: +253 KB binary (sigil 3.6.0 crypto
  banks — see Binary), accepted.
- Stale `src/test_*.cyr` stub cleanup (predate `tests/tcyr/`).
- Patra `json_build/6` namespace upstream — file an issue against
  patra rather than continue working around it.

## Pending release (unreleased)

- **1.8.1** (UNRELEASED — staged in the working tree, not yet tagged) — toolchain pin bump to cyrius **6.0.53** + the long-deferred **libro 2.6.2 → 2.7.1** bump, now unblocked. Cleared the 6.0.26→6.0.53 pin-drift warning (`cyrius.cyml` + `qemu/helpers/cyrius.cyml`). libro 2.6.3+ no longer trips the `cycc` single-unit limit under 6.0.53; 2.7.1 pulls sigil 3.0.1→3.6.0 + agnosys 1.0.4→1.3.2 (sakshi 2.2.3, patra 1.10.3 held). Added stdlib `slice` + `thread_local` (required by agnosys 1.3.2 / sigil 3.6.0). **Retired the `ct_eq` compat shim** (`src/compat.cyr` + `[deps.argonaut_compat]` deleted — libro migrated to `ct_eq_bytes_lens`, now a stdlib `ct` symbol; sigil 3.6.0 ships no `ct_eq`; dup-fn warning gone). Clean x86_64 DCE build (**1,297,744 bytes**, +253,304/+24.3% — entirely sigil 3.6.0 crypto-bank static data + larger code, no argonaut-side bloat; 2,634 dead-fns NOPed); 28 / 743 green; benches neutral-to-win vs the 1.8.1 baseline; lockfile 45 verified / 0 failed; lint/fmt/vet clean.

## Recent shipped

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
- Most recent: `2026-04-26-audit.md` (P(-1) for 1.4.0)
- Prior audit references retained in `CHANGELOG.md` Security sections
