# Argonaut — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures (durable);
> this file is **state** (volatile). Bumped at release time alongside
> `VERSION` and the CHANGELOG header.

## Version

**1.4.0** (in flight — P(-1) hardening pass: docs/audit/2026-04-26-audit.md
cycle, CLAUDE.md split into durable rules + this state.md, regression
tests added for audit findings).

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

- **641 KB** statically linked ELF x86_64 (`CYRIUS_DCE=1 cyrius build src/main.cyr build/argonaut`)
- Was 378 KB at 1.2.0; +263 KB at 1.3.0 for libro 2.0's signing /
  anchoring / merkle / streaming surface + the patra dep
- Dead-code floor: ~1430 unreachable functions NOPed under DCE

## Suites

- **26 .tcyr suites / 607 assertions** (0 failures on cyrius 5.7.6)
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

- 1.4.0 P(-1) hardening pass — see `docs/audit/2026-04-26-audit.md`
- Stale `src/test_*.cyr` stub cleanup (predate `tests/tcyr/`)
- Patra `json_build/6` namespace upstream — file an issue against patra rather than continue working around it

## Recent shipped

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
