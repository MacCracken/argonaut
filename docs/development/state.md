# Argonaut — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Bumped at release time alongside
> `VERSION` and the CHANGELOG header.

## Version

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

- `cyrius = "5.10.34"` pinned in `cyrius.cyml [package]`
- Local toolchain `cc5` 5.10.39+ (newer than pin — CI installs the pin)
- No `.cyrius-toolchain` file; the manifest is the only pin source
- Versioned install layout: `~/.cyrius/versions/<V>/{bin,lib}` with
  `~/.cyrius/{bin,lib}` symlinking to the current one (required by
  cc5 5.10.9+'s arch-peer include resolution)

## Binary

- **x86_64: ~1.00 MB** statically linked ELF (`CYRIUS_DCE=1 cyrius build src/main.cyr build/argonaut`, 1023544 bytes)
- **aarch64: ~1.14 MB** statically linked ELF (`CYRIUS_DCE=1 cyrius build --aarch64 src/main.cyr build/argonaut-aarch64`, 1141376 bytes; cross-build via `cc5_aarch64` translator since 1.5.4). +140 KB delta tracks aarch64's fixed-width instruction encoding.
- Was 378 KB at 1.2.0, 641 KB at 1.3.0, 650 KB at 1.4.0, 652 KB at
  1.5.0, ~990 KB at 1.5.1, ~995 KB at 1.5.2; +5 KB at 1.5.3 for
  the `src/audit_ext.cyr` wrapper module + new ArgonautInit slot
  + config fields. libro's patra/sign/merkle paths were already
  linked transitively; DCE now retains them since they're
  reachable from the public surface.
- Dead-code floor: ~2,140 unreachable functions NOPed under DCE
  (was ~2,268 at 1.5.2 — audit_ext brings ~128 previously-dead
  libro fns into the reachable set)

## Suites

- **Native x86_64: 28 .tcyr suites / 720 assertions** (0 failures on cyrius 5.10.34). Unchanged from 1.5.3.
- **aarch64 (qemu-user): 26 of 28 / 605 assertions** pass via `scripts/aarch64-sweep.sh`. 2 suites in the documented known-failure budget (qemu emulation limits + upstream sigil Ed25519 quirk — see `docs/architecture/001-cross-arch-aarch64.md`).
- **2 .bcyr binaries** (`tests/bcyr/argonaut.bcyr`, `tests/bcyr/api.bcyr`)
- **37 benchmarks** wired into `src/bench_main.cyr`; history in `bench-history.csv`

### Bench snapshot (1.3.0-post, 2026-04-26)

| Bench | avg | range |
|---|---|---|
| build_boot_seq_desktop | 5 µs | 3–217 |
| init_new_desktop | 26 µs | 23–126 |
| resolve_order_desktop | 11 µs | 8–95 |
| resolve_order_chain_100 | 208 µs | 190–350 |
| resolve_waves_desktop | 14 µs | 11–128 |
| plan_shutdown_reboot | 21 µs | 17–172 |
| audit_log_record | 7 µs | 5–46 |
| health_tracker_record | 1 µs | 488 ns–12 µs |
| state_transition_check | 1 µs | 838 ns–9 µs |

(See `bench-history.csv` for the full series and any post-audit comparison.)

## Dependencies

- **stdlib (23 modules)**: `string fmt alloc vec str syscalls io fs process hashmap tagged args json fnptr freelist bigint chrono ct keccak thread random assert bench` (sakshi + sigil dropped — promoted upstream from stdlib to external git pins; thread + random added — libro 2.6.2's dist depends on both)
- **libro 2.6.2** — single-module dist (`lib/libro.cyr`) via `[deps.libro] tag = "2.6.2" modules = ["dist/libro.cyr"]`. SHA pinned in `cyrius.lock`.
- **patra 1.9.3** — explicit dep, was transitive of libro 2.0. SHA pinned in `cyrius.lock`.
- **sakshi 2.2.3 + sigil 3.0.1 + agnosys 1.0.4** — transitive via libro 2.6.2 (sakshi also via patra 1.9.3). All SHA-pinned in `cyrius.lock`; resolved into `lib/` by `cyrius deps`. Sigil 3.0.1's dist re-published 2026-05-10 with `ct_eq` restored; argonaut's 1.5.1 `src/compat.cyr` shim retired at 1.5.2.
- **`cyrius.lock`** — 5 deps locked (down from 6 at 1.5.1 — `[deps.argonaut_compat]` self-reference removed).

## In-flight

- **1.5.5 — Closeout P(-1) audit.** Arc-closing security re-pass
  before 1.6.0 tagging. Covers the libro extended surface
  (persistence, signing, merkle) added in 1.5.3 + the aarch64
  cross-arch syscall surface added in 1.5.4.
- **1.6.x — QEMU PID-1 harness + carry-forwards.** Validates M3
  (orphan reap under real PID-1 reparenting) and L3
  (controlling-TTY decoupling) on a minimal initramfs harness;
  native aarch64 CI runner allocation (closes qemu-user
  emulation gap); also lands the deferred `audit_log_new`
  rename, WitnessAnchor publishing (gated on consumer demand +
  AGNOS federation protocol), and durable signing-key rotation
  (gated on kybernet's key-management surface).
- **Upstream — sigil Ed25519 aarch64 verify quirk** — filed at
  1.5.4 in sigil repo
  (`docs/development/issues/2026-05-10-ed25519-verify-aarch64-accepts-wrong-pk.md`).
  Consume via sigil bump once a fix lands.
- **Upstream — `lib/process.cyr` exec_env Str/cstr quirk** —
  filed at 1.5.2 in cyrius repo
  (`docs/development/issues/2026-05-10-process-exec-str-cstr-ambiguity.md`).
  Blocks unit-level shell-exec testing in `health_exec.tcyr` /
  `audit-l3-fork-setsid`; consume via toolchain bump once
  cyrius lands the fix.
- Release-hook gap — 1.4.0 shipped without auto-bumping this file;
  same again for 1.5.0 if the workflow isn't fixed. File against
  the release workflow before 1.6.0.
- Stale `src/test_*.cyr` stub cleanup (predate `tests/tcyr/`).
- Patra `json_build/6` namespace upstream — file an issue against
  patra rather than continue working around it.

## Recent shipped

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
- aarch64 — cross-built via `cc5_aarch64` since 1.5.4; smoked + swept under `qemu-aarch64` (qemu-user 11.0.0-1). Real-hardware (RPi4, Apple Silicon, Graviton / Ampere) validation slipped to 1.6.x — gated on aarch64 CI runner allocation. See `docs/architecture/001-cross-arch-aarch64.md`.

## Audit cadence

- `docs/audit/` — security audit reports, dated `YYYY-MM-DD-audit.md`
- Most recent: `2026-04-26-audit.md` (P(-1) for 1.4.0)
- Prior audit references retained in `CHANGELOG.md` Security sections
