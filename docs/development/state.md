# Argonaut — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Bumped at release time alongside
> `VERSION` and the CHANGELOG header.

## Version

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

- `cyrius = "5.7.5"` pinned in `cyrius.cyml [package]`
- Local toolchain `cc5` 5.7.6 (newer than pin — CI installs the pin)
- No `.cyrius-toolchain` file; the manifest is the only pin source

## Binary

- **652 KB** statically linked ELF x86_64 (`CYRIUS_DCE=1 cyrius build src/main.cyr build/argonaut`)
- Was 378 KB at 1.2.0, 641 KB at 1.3.0, 650 KB at 1.4.0; +2 KB at
  1.5.0 for `notify_try_recv_authenticated` (recvmsg + SCM_CREDENTIALS),
  `proc_table_reap_orphans`, `prctl(PR_SET_CHILD_SUBREAPER)` enrol,
  `setsid` + stdout/stderr `dup2` in `fork_exec_service`, and the
  `init_notify_bind` / `init_notify_fd` opt-in surface
- Dead-code floor: ~1430 unreachable functions NOPed under DCE

## Suites

- **27 .tcyr suites / 649 assertions** (0 failures on cyrius 5.7.6).
  +12 assertions over 1.4.0 for the M1 / M3 / L3 audit-deferral
  regressions in `tests/tcyr/audit_findings.tcyr` (groups
  `audit-m1-notify-cred`, `audit-m3-reaper-orphans`,
  `audit-l3-fork-setsid`)
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

- **stdlib (23 modules)**: `string fmt alloc vec str syscalls io fs process hashmap tagged args json fnptr sakshi freelist bigint chrono sigil ct keccak assert bench`
- **libro 2.0.5** — single-module dist (`lib/libro.cyr`) via `[deps.libro] tag = "2.0.5" modules = ["dist/libro.cyr"]`. SHA pinned in `cyrius.lock`.
- **patra 1.1.1** — transitive dep of libro 2.0 (PatraStore audit-entry persistence). SHA pinned in `cyrius.lock`.

## In-flight

- **1.6.0 — QEMU PID-1 harness + HIGH-1 resolver.** The
  audit gated end-to-end M3 (orphan reap under real PID-1
  reparenting) and L3 (controlling-TTY decoupling) on a QEMU
  PID-1 boot test harness — minimal initramfs + kernel + assertion
  output. Sibling repo (`kybernet/qemu/`) has the pattern. Same
  minor lands the HIGH-1 follow-up: replace the 1.4.0
  reject-non-loopback gate in health checks with a real host
  resolver (dotted-quad parser + `gethostbyname` or
  `getaddrinfo`).
- **`lib/process.cyr` exec_env Str/cstr quirk** — file an upstream
  stdlib issue. Blocks unit-level shell-exec testing across
  `health_exec.tcyr`, the new `audit-l3-fork-setsid` group, and
  any future shell-driven test.
- Release-hook gap — 1.4.0 shipped without auto-bumping this file;
  same again for 1.5.0 if the workflow isn't fixed. File against
  the release workflow before 1.6.0.
- Stale `src/test_*.cyr` stub cleanup (predate `tests/tcyr/`).
- Patra `json_build/6` namespace upstream — file an issue against
  patra rather than continue working around it.

## Recent shipped

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
- aarch64 / Apple Silicon — not yet covered (the toolchain ships `cc5_aarch64` from 5.5.x; argonaut hasn't been cross-built since the cc3 era; lift planned for a future minor)

## Audit cadence

- `docs/audit/` — security audit reports, dated `YYYY-MM-DD-audit.md`
- Most recent: `2026-04-26-audit.md` (P(-1) for 1.4.0)
- Prior audit references retained in `CHANGELOG.md` Security sections
