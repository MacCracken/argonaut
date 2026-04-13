# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.2.0] — 2026-04-13

### Added

#### Libro 1.0.2 Integration — Cryptographic Audit Chain
- **libro 1.0.2** replaces FNV-1a audit shim with SHA-256 hash-linked chain
- 8 libro modules: error, hasher, entry, verify, query, retention, chain, export
- Dependencies: sigil (SHA-256), bigint, chrono via `cyrius deps`
- `audit_log_new()` creates a libro `AuditChain` with UUID, RFC 3339 timestamps, SHA-256 hashing
- `audit_log_verify()` performs cryptographic chain integrity verification
- `audit_log_query_full(log, source, min_sev, agent, after, before)` — full QueryFilter with time range and agent_id
- `audit_log_record_with_agent(log, service, event_type, agent)` — agent-attributed events
- `audit_log_export_jsonl(log, fd)` / `audit_log_export_csv(log, fd)` — audit trail export
- `audit_entry_agent_id(e)` accessor

#### Lifecycle Audit Recording
- `ArgonautInit` carries an `audit_log` field (libro AuditChain)
- `init_audit_log(init)` accessor
- `init_start_service` records EVT_STARTING, EVT_STARTED / EVT_STOPPED_FAIL / EVT_READY_PASSED / EVT_READY_FAILED
- `init_stop_service` records EVT_STOPPING, EVT_STOPPED_OK / EVT_STOPPED_FAIL
- `init_restart_service` records EVT_RESTARTING
- `init_reap_services` records EVT_STOPPED_OK / EVT_CRASH_DETECTED
- `init_enforce_watchdog` records EVT_TIMEOUT_KILLED
- `init_poll_health` records EVT_HEALTH_PASSED / EVT_HEALTH_FAILED

#### P(-1) Scaffold Hardening
- `health_check_type_str(t)` — human-readable strings for HealthCheckType
- `ReapResult` struct for `init_reap_services` return values
- cc3 regression tests: `cc3_ptr_regression.tcyr`, `cc3_readfile_cap.tcyr`
- `audit_lifecycle.tcyr` — 17 assertions for lifecycle audit recording

### Fixed
- **Security**: non-http:// URLs rejected in HTTP health checks (was: silent port corruption)
- **Security**: `verify_emergency_auth` uses `constant_time_eq_str` (was: `str_eq` timing oracle)
- **Security**: `password_hash` upgraded from FNV-1a to SHA-256 via sigil
- **Correctness**: `execute_ready_check` initializes `timeout_ms` field (was: uninitialized heap)
- **Correctness**: zombie prevention — `sys_waitpid` after SIGKILL on ready-check failure
- **Correctness**: `generate_unit` emits correct systemd `Type=` per service type (was: always `Type=notify`)
- **Correctness**: `HealthCheckResult.check_type_str` set from actual check type (was: placeholder)
- **Correctness**: `resolve_service_order` / `resolve_service_waves` — external deps get `in_degree` entries; cycle detection uses `map_count(in_degree)`. Fixes false cycle detection when depending on unregistered services.
- **Correctness**: TCP health check verifies `SO_ERROR` via `getsockopt` (was: false positive on ECONNREFUSED)
- **Memory**: `fleet_registration_from_system` uses `str_clone` for stack-buffered values (was: dangling stack pointers)
- **Memory**: `notify_try_recv` uses static buffer (was: 1KB alloc per poll tick, never freed)

### Changed
- **audit.cyr**: 328-line FNV-1a shim → libro bridge (51% smaller)
- **Include order**: audit.cyr before init.cyr (init depends on audit)
- **Dependencies via `cyrius deps`** — `cyrius.toml` declares `[deps]` stdlib and `[deps.libro]`; no more manual vendoring
- **Include paths**: `lib/libro/X.cyr` → `lib/X.cyr` (flat layout from `cyrius deps`)
- **Binary size**: 197KB → 378KB (+181KB for libro + sigil SHA-256 + bigint)
- **Minimum toolchain**: cyrius 3.9.8 (`.cyrius-toolchain` file)
- CI/release workflows: `cyrius deps` + `cyrius build` + `cyrius test`, version from `.cyrius-toolchain`
- `cyrfmt` and `cyrlint` clean — zero warnings, zero format issues
- All documentation updated from Rust to Cyrius (README, ADRs, guides, quickstart, security, contributing)
- `scripts/bench-history.sh` rewritten for Cyrius
- `lib/fnptr.cyr` added to include chain (suppresses `fncall2` warning from hashmap)
- Test suites: 26 (607 assertions)

### Removed
- **FNV-1a hash** in audit.cyr and security.cyr — replaced by SHA-256
- **`lib/libro/`** directory — libro modules now resolved via `cyrius deps`
- **`lib/patra.cyr`** — unused vendored copy

---

## [1.0.0] — 2026-04-12

Argonaut 1.0.0 — init system and service manager for AGNOS, written in Cyrius.

All pre-1.0 features complete: boot sequencing, service lifecycle (simple/forking/oneshot), dependency resolution (Kahn's algorithm), health checks (HTTP/TCP/command/process-alive), watchdog enforcement, shutdown orchestration, runlevel switching, edge boot (dm-verity/LUKS/read-only rootfs), security enforcement (seccomp/Landlock/capabilities), sd_notify protocol, systemd unit generation, tmpfiles setup, API response builders, and cryptographic audit trail via libro.

---

## [0.96.1] — 2026-04-11

### Added
- API response builders: `init_list_services`, `init_system_status`, `init_system_metrics`, `init_boot_log`
- Boot execution plans: `init_boot_execution_plan`, `init_boot_execution_plan_waves`
- `safe_cmd_display()`, `to_prlimit_commands()`
- HTTP health check upgraded to full HTTP/1.x GET with status line parsing
- `sakshi_full.cyr` v0.7.0 — real span stack, ring buffer, UDP output
- Full sakshi tracing across all lifecycle events
- 8 new test suites (184 assertions), 8 new benchmarks (37 total)

### Changed
- Cyrius toolchain: cc2 → cc3, minimum version 3.4.0
- Binary size: 213KB → 197KB (heap buffers + sakshi_full)
- Test suites: 15 → 23 (579 assertions)

### Removed
- **`rust-old/`** — original Rust source (13,577 lines). All ported to Cyrius.

---

## [0.96.0] — 2026-04-08

### Added
- Sakshi integration — structured tracing via sakshi 0.5.0

### Changed
- Binary size: 207KB → 213KB (+6KB for sakshi)
- Test suites: 12 → 15 (395 assertions)

---

## [0.95.0] — 2026-04-08

### Added
- Full rewrite from Rust (13,577 lines) to Cyrius (6,124 lines — 2.2x compression)
- 13 source modules, 207KB statically linked ELF x86_64 binary
- `audit.cyr` — libro-compatible API shim (FNV-1a, replaced in 1.2.0)
- Edge boot: `parse_meminfo_total_mb()`, memory validation, fleet registration
- 12 test suites (395 assertions), 29 benchmarks
- CI/CD workflows for Cyrius toolchain

### Fixed
- Unknown service type transitions to STATE_FAILED (was stuck in STATE_STARTING)
- `/proc/meminfo` parsing implemented (was stub)
- `fleet_registration_from_system` reads real memory (was hardcoded 0)

---

## [0.90.0] — 2026-04-02

### Added
- Initial scaffold: types, boot sequences, service definitions, dependency resolution
- Boot modes: Server, Desktop, Minimal, Edge, Recovery
- Service management: registration, state machine, dependency-aware ordering (Kahn's)
- Shutdown planning: ordered steps with wall message, service stops, filesystem sync
- Runlevel system: Emergency, Rescue, Console, Graphical, Container, Edge
- Edge boot: read-only rootfs, dm-verity verification
- Health checks: HTTP GET, TCP connect, command, process-alive
- Emergency shell, crash action determination, safe command abstraction
- 148 tests

---

## Pre-0.90 (Rust era)

Features implemented in the original Rust codebase (v0.2.0–v0.9.0) and ported to Cyrius at v0.95.0. See `docs/benchmarks-rust-baseline.md` for Rust performance comparison. The Rust source was removed at v0.96.1.

Key milestones: v0.2.0 (hardening, `forbid(unsafe_code)`), v0.3.0 (process execution, ProcessTable), v0.4.0 (health check execution), v0.5.0 (runlevel switching), v0.6.0 (edge boot execution), v0.7.0 (API, audit, systemd integration), v0.8.0 (service types, resource limits, log rotation), v0.9.0 (seccomp, Landlock, capabilities, tmpfiles).
