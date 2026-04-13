# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html) (pre-1.0).

---

## [1.1.0] — 2026-04-12

### Fixed
- **Correctness**: `resolve_service_order` / `resolve_service_waves` — external dependency targets now get `in_degree` entries; cycle detection uses `map_count(in_degree)` instead of `vec_len(svc_defs)`. Previously, a service depending on an unregistered service would cause a false "cycle" detection and return zero services in the startup plan.
- **Memory**: `fleet_registration_from_system` — `str_clone` on stack-buffered Str values (machine-id, hostname). Was storing pointers to stack memory that became dangling after function return.
- **Correctness**: TCP health check verifies `SO_ERROR` via `getsockopt` after non-blocking connect. Previously returned false-positive success on ECONNREFUSED (socket becomes writable on both success and refusal).
- **Memory**: `notify_try_recv` — static 1KB buffer allocated once on first call. Was allocating 1KB per poll tick with no reuse (steady leak without GC).

### Changed
- `ReapResult` struct added for `init_reap_services` return values (was reusing `sizeof(CrashAction)` coincidentally — fragile if CrashAction struct grew)

---

## [1.0.2] — 2026-04-12

### Fixed — P(-1) Audit
- **Security**: `health.cyr` — non-http:// URLs now rejected (was: silently corrupt port parsing)
- **Security**: `security.cyr` — `verify_emergency_auth` uses `constant_time_eq_str` (was: `str_eq` timing oracle)
- **Correctness**: `health.cyr` — `execute_ready_check` initializes `timeout_ms` field (was: uninitialized heap garbage)
- **Correctness**: `init.cyr` — zombie prevention: `sys_waitpid` called after SIGKILL on ready-check failure
- **Correctness**: `systemd.cyr` — `generate_unit` emits correct `Type=` per service type (was: always `Type=notify`)
- **Correctness**: `health.cyr` — `HealthCheckResult.check_type_str` set from `health_check_type_str()` (was: placeholder `"mount-filesystems"`)

### Added
- `health_check_type_str(t)` — human-readable strings for `HealthCheckType` enum

### Changed
- Minimum cc3 version: 3.8.0
- `cyrfmt` applied to all source files — zero format issues
- `cyrlint` clean — zero warnings (was 6 line-length violations)
- `src/boot.cyr`: long `boot_step_new` calls split to stay under 120 chars
- `src/types.cyr`: emergency shell banner built via `str_builder` (was single 120+ char string literal)
- `scripts/bench-history.sh` rewritten for Cyrius (was: cargo bench)
- `docs/guides/quickstart.md` rewritten for Cyrius (was: Rust)
- `SECURITY.md` updated: libro SHA-256, supported versions, http-only health checks
- `CONTRIBUTING.md`: minimum cc3 version updated to 3.6.2+

---

## [1.0.1] — 2026-04-12

### Fixed
- Added `lib/fnptr.cyr` to include chain — suppresses `undefined function 'fncall2'` warning from `hashmap.cyr`'s `map_iter`. The function was never called at runtime but produced a compiler warning on every build.

---

## [1.0.0] — 2026-04-12

### Summary

Argonaut 1.0.0 — init system and service manager for AGNOS, written in Cyrius.

- **Language**: Cyrius (compiled via cc3 3.6.2)
- **Binary**: 373KB statically linked ELF x86_64
- **Tests**: 26 suites, 606 assertions, 0 failures
- **Benchmarks**: 37
- **Audit**: libro 1.0.2 SHA-256 hash-linked chain with lifecycle recording
- **Tracing**: sakshi_full 0.7.0 structured tracing

All pre-1.0 features complete: boot sequencing, service lifecycle (simple/forking/oneshot), dependency resolution (Kahn's algorithm), health checks (HTTP/TCP/command/process-alive), watchdog enforcement, shutdown orchestration, runlevel switching, edge boot (dm-verity/LUKS/read-only rootfs), security enforcement (seccomp/Landlock/capabilities), sd_notify protocol, systemd unit generation, tmpfiles setup, API response builders, and cryptographic audit trail via libro.

---

## [0.97.0] — 2026-04-12

### Added

#### Libro 1.0.2 Integration — Real Cryptographic Audit Chain
- **libro 1.0.2** integrated: SHA-256 hash-linked audit chain replaces FNV-1a shim
- Includes libro core modules: error, hasher, entry, verify, query, retention, chain
- 16 of 19 libro modules compile-tested on cc3 3.6.2 (file_store/patra_store/streaming excluded — need patra lock fns)
- New dependencies: sigil (SHA-256, hex, constant-time comparison), bigint, chrono
- `audit_log_new()` now creates a libro `AuditChain` (was local FNV-1a vec)
- `audit_log_record()` creates real `AuditEntry` with UUID, RFC 3339 timestamps, SHA-256 hash
- `audit_log_verify()` performs cryptographic chain integrity verification
- `audit_log_by_source()`, `audit_log_by_severity()` use libro's Str comparison
- `audit_log_by_min_severity()`, `audit_log_query()` use libro's `QueryFilter` API
- Full libro `entry_verify()` available for individual entry verification
- `audit_entry_timestamp()` returns RFC 3339 Str (was epoch ms integer)
- `audit_entry_hash()` returns 64-char hex SHA-256 Str (was FNV-1a i64)
- `audit_entry_prev_hash()` returns Str (was i64, 0 for genesis)
- `audit_entry_service()` accessor (service name stored in libro details field)

#### Lifecycle Audit Recording
- `ArgonautInit` now carries an `audit_log` field (libro AuditChain)
- `init_audit_log(init)` accessor
- `init_start_service` records EVT_STARTING, EVT_STARTED / EVT_STOPPED_FAIL / EVT_READY_PASSED / EVT_READY_FAILED
- `init_stop_service` records EVT_STOPPING, EVT_STOPPED_OK / EVT_STOPPED_FAIL
- `init_restart_service` records EVT_RESTARTING
- `init_reap_services` records EVT_STOPPED_OK / EVT_CRASH_DETECTED on process exit
- `init_enforce_watchdog` records EVT_TIMEOUT_KILLED
- `init_poll_health` records EVT_HEALTH_PASSED / EVT_HEALTH_FAILED
- All events flow through libro's SHA-256 chain — tamper-proof service event history

#### Testing — 26 suites, 606 assertions
- `audit_lifecycle.tcyr` (17 assertions) — start/stop/restart produce audit entries, chain integrity, SHA-256 hashes, source attribution
- `cc3_readfile_cap.tcyr` (4) — 16 libro modules compile on cc3 3.6.2
- `cc3_ptr_regression.tcyr` (3) — full argonaut + libro build (cc3 3.5.2 ptr bug regression)
- `audit_b.tcyr` updated: SHA-256 hash length, entry_verify, RFC 3339 timestamp format

### Changed
- **audit.cyr**: 328-line FNV-1a shim → 162-line libro bridge (51% smaller)
- **Include order**: audit.cyr now included before init.cyr (init depends on audit for lifecycle recording)
- **Binary size**: 197KB → 373KB (+176KB for libro + sigil SHA-256 + bigint)
- **Minimum cc3**: 3.6.2 (3.5.2–3.6.1 had `ptr` regression; 3.5.0 had READFILE 512KB cap)
- `lib/syscalls.cyr`: added `SYS_MPROTECT`, `SYS_MUNMAP`, `MmapProt`, `MmapFlag` enums (required by freelist for libro)
- `tests/test_header.cyr`, `src/test_header.cyr`: now include libro deps + audit.cyr before init.cyr
- Libro modules in `lib/libro/` — all 19 copied, 7 core included in main build

### Removed
- **FNV-1a hash computation** in audit.cyr — replaced by libro's SHA-256 via sigil
- **AuditEntry struct** (56 bytes, FNV-1a) — replaced by libro's AuditEntry (88 bytes, SHA-256)
- **AuditLog struct** (local vec wrapper) — replaced by libro's AuditChain
- `severity_str()`, `severity_level()` — replaced by libro's `severity_as_str()`, direct enum comparison

---

## [0.96.1] — 2026-04-11

### Added

#### Code Quality
- `init_list_services()`, `init_system_status()`, `init_system_metrics()`, `init_boot_log()` — API response builders (JSON)
- `init_boot_execution_plan()`, `init_boot_execution_plan_waves()` — boot execution plans
- `safe_cmd_display()` — SafeCommand string representation
- `to_prlimit_commands()` — ResourceLimits to prlimit(1) SafeCommand generation
- HTTP health check upgraded: full HTTP/1.x GET with status line parsing (was TCP-only)

#### Sakshi Integration (v0.97.0 scope)
- `sakshi_full.cyr` v0.7.0 — real span stack, ring buffer, UDP output (from cyrius stdlib)
- State transitions traced (INFO/WARN on failure)
- Boot step failures traced (ERROR)
- Health check results traced (DEBUG pass, WARN fail)
- Watchdog enforcement traced (ERROR)
- Shutdown/runlevel switch steps traced (INFO)
- Process fork/exec traced (DEBUG), SIGKILL fallback (WARN)
- Edge boot steps traced (INFO per phase, ERROR on failure)

#### Testing — 8 new suites, 184 new assertions
- `process.tcyr` (26) — proc_table, safe_cmd, prlimit, fork_exec
- `svc_life.tcyr` (13) — start/stop/restart, oneshot, disabled, restart limit
- `health_exec.tcyr` (22) — process alive, command check, ready check, health history
- `edge_exec.tcyr` (19) — rootfs, verity, luks, device path, edge boot, profile validation
- `notify.tcyr` (16) — parse, bind, try_recv, drain, socket env
- `security.tcyr` (26) — landlock, capability, seccomp, auth, password hash
- `api_new.tcyr` (28) — API builders, boot plans, boot log with errors
- `parity.tcyr` (34) — Rust parity: boot timestamps, register overwrite, enable/disable, shruti, serde fields, audit chain

#### Benchmarks — 8 new (37 total)
- `api_list_svcs`, `api_sys_status`, `api_sys_metrics`, `api_boot_log`, `prlimit_cmds`
- `boot_exec_plan`, `boot_exec_waves`, `safe_cmd_disp`
- Separated into `tests/bcyr/api.bcyr` (string buffer limits)

### Changed
- Cyrius toolchain: cc2 → cc3, cyrb → cyrius, minimum version 3.4.0
- `sakshi_full.cyr` from cyrius stdlib (removed local copies)
- `load_env_file` buffer: 64KB static → 8KB heap (saved 56KB binary size)
- `parse_meminfo_total_mb` buffer: 4KB static → 1KB heap
- `notify_try_recv` buffer: 4KB static → 1KB heap
- Binary size: 213KB → 197KB (heap buffers + sakshi_full vs minimal)
- Test suites: 15 → 23 (579 assertions, 0 failures)
- Benchmarks: 29 → 37
- `lib/json.cyr` patched for cc3 bug #4 (break in chained if/while)
- `serde.tcyr` uses `sizeof()` instead of hardcoded struct sizes
- All docs updated: CLAUDE.md, CONTRIBUTING.md, roadmap.md (cc2→cc3, cyrb→cyrius)

### Removed
- **`rust-old/`** — original Rust source (13,577 lines) archived since v0.95.0. All functions, tests, and benchmarks ported to Cyrius. 2.3GB removed (includes `target/`).

### Fixed
- `lib/json.cyr`: `json_parse` non-string value delimiter broken on cc3 — chained `if`/`break` inside `while` doesn't exit. Replaced with flag variable + `||`. Upstream fix in cyrius stdlib 3.2.6.

## [Unreleased]

---

## [0.96.0] — 2026-04-08

### Added
- **Sakshi integration** — structured tracing via sakshi 0.5.0 (minimal profile + span stubs)
- `sakshi_span_enter`/`sakshi_span_exit` on `argonaut_init_new()` and `init_plan_shutdown()`
- `sakshi_info` on boot step completion and service stop
- `sakshi_error` on unknown service type
- `lib/sakshi.cyr` — single-file distribution (176 lines, zero heap allocation)
- Test header uses no-op sakshi stubs to avoid string buffer pressure

### Changed
- Binary size: 207KB -> 213KB (+6KB for sakshi instrumentation)
- Test suites: 12 -> 15 (types, modules, api split further for cc2 string buffer limits)
- 395 assertions across 15 .tcyr suites, 0 failures

---

## [0.95.0] — 2026-04-08

### Added

#### Cyrius Port
- Full rewrite from Rust (13,577 lines) to Cyrius (6,124 lines — 2.2x compression)
- Original Rust source preserved in `rust-old/`
- 13 source modules: types, boot, services, process_mgmt, health, edge_boot, notify, security, systemd, tmpfiles, init, audit, main
- 207KB statically linked ELF x86_64 binary (10.6x smaller than Rust musl)
- Build via `cyrb.toml` / `cc2` 2.1.0 compiler

#### Audit Module
- `audit.cyr` — self-contained audit chain (libro-compatible API shim)
- `AuditLog`, `AuditEntry` — append-only hash-linked audit entries (FNV-1a)
- `audit_event_severity()` — maps ServiceEventType to severity (Info/Warning/Error)
- `audit_log_by_source()`, `audit_log_by_severity()`, `audit_log_by_min_severity()` — query filters
- `audit_log_query()` — composite source + min_severity filter
- `audit_log_verify()` — chain integrity verification (hash + linkage)
- Designed for drop-in replacement when libro is ported to Cyrius (blocked on majra)

#### Edge Boot
- `parse_meminfo_total_mb()` — parses MemTotal from `/proc/meminfo`, returns MB
- Memory validation in `validate_edge_profile()` — checks against `max_memory_mb` budget
- `fleet_registration_from_system()` — now reads real memory via `parse_meminfo_total_mb()`

#### Cyrius 2.0 Features
- `sizeof(StructName)` in all `alloc()` calls — 25 replacements across 7 modules
- `bitget()` builtins in audit hash computation (replaces manual shift/mask)
- `lib/assert.cyr` updated with `test_group()`, `assert_streq()`, `assert_nonnull()`, `assert_lt/gte/lte()`

#### Testing
- 12 test suites (.tcyr format): types, init, lifecycle, modules_a, modules_b, display, advanced, api_a, api_b, audit_a, audit_b, serde
- 395 assertions, 0 failures on cc2 2.1.0
- Auto-discovered by `cyrb test`
- Process table tests: multi-entry insert/remove/keys/reinit

#### Benchmarks
- 29 benchmarks (.bcyr format), auto-discovered by `cyrb bench`
- Added: resolve_order_desktop, resolve_waves_chain_20, resolve_waves_wide_20, plan_shutdown_poweroff, configure_readonly_rootfs, verify_rootfs_integrity, stats_desktop, generate_tmpfile_cmds_20, plan_runlevel_switch, mark_all_steps_complete, audit_log_record
- Rust baseline comparison in `docs/benchmarks-rust-baseline.md`
- Typical 4-8x vs Rust; state transitions 0.62x (Cyrius faster)

#### CI/CD
- CI workflow rewritten for Cyrius toolchain (`cyrb build`, `cyrb test`, `cyrb bench`)
- Release workflow: static binary, source archive, SHA256SUMS, GitHub release
- Security scan: raw execve, shadow access, system() call detection
- Version consistency check: VERSION + cyrb.toml + tag

### Changed
- `build_dep_graph()` extracted from `resolve_service_order()` and `resolve_service_waves()` — eliminates 19 lines of duplicated dependency graph construction
- `execute_ready_check()` hoists temporary HealthCheck allocation outside retry loop
- `init_stop_service()` caches service lookup — removes triple `init_get_service()` call
- Environment variables in `generate_unit()` now sorted lexicographically for deterministic systemd unit output
- CLAUDE.md updated for Cyrius toolchain (was cargo/clippy/audit/deny)

### Fixed
- **init.cyr**: unknown service type now transitions to STATE_FAILED (was stuck in STATE_STARTING)
- **edge_boot.cyr**: `/proc/meminfo` parsing implemented (was dead code / TODO stub)
- **edge_boot.cyr**: `validate_edge_profile()` memory check now functional
- **fleet_registration_from_system()**: `total_mem_mb` populated from real meminfo (was hardcoded 0)

### Removed
- `programs/` directory (empty placeholder, nothing to port)

#### v0.2.0 scope — Hardening
- `#![forbid(unsafe_code)]` — no unsafe in the crate
- `BootMode::Recovery` — emergency shell only, no services, maps to `Runlevel::Emergency`
- `RestartConfig` struct — configurable `max_restarts`, `base_delay_ms`, `max_delay_ms` per service
- 100ms minimum floor on backoff delay to prevent busy-retry loops
- `Display` impl for `SafeCommand` (replaces allocation-heavy `display()` method)
- `Display` impl for `HealthCheckType` (human-readable format)
- `#[non_exhaustive]` on all public enums and output-only structs
- `#[must_use]` on ~40 pure functions across all modules
- Full tracing instrumentation across all modules
- Serde roundtrip tests for all public serializable types (24 tests)
- CI workflows: `ci.yml` (fmt, clippy, test, audit, deny, coverage), `release.yml` (tag-triggered publish)
- Criterion 0.8 benchmark harness with 21 benchmarks and `scripts/bench-history.sh` CSV tracking
- Renamed all `"agent-runtime"` service references to `"daimon"`

#### v0.3.0 scope — Process Execution
- `process.rs` module — fork/exec via `std::process::Command` from `ProcessSpec`
- `SpawnedProcess` — PID tracking, `try_wait`/`wait`, uptime, signal delivery via `nix`
- Graceful stop: SIGTERM → poll → SIGKILL with configurable timeout
- Stdout/stderr capture to log files (graceful fallback to `/dev/null` on permission error)
- `ProcessTable` — tracks all running service processes, bulk reap, bulk stop
- `ArgonautInit::start_service` / `stop_service` / `restart_service` — full service lifecycle
- `ArgonautInit::reap_services` — detects exited processes, returns `CrashAction` for each
- `execute_shutdown` — walks `ShutdownPlan` steps with real process stops, sync, signal delivery
- `run_command` / `run_command_sequence` — one-shot `SafeCommand` execution (stdout null, stderr bounded)
- Watchdog: `check_watchdog` / `enforce_watchdog` — startup + runtime timeout enforcement
- `notify.rs` module — sd_notify compatible `NotifyListener` (READY=1, STATUS, MAINPID)

#### v0.4.0 scope — Health Check Execution
- `health.rs` module — executes all `HealthCheckType` variants with zero external HTTP dependencies
- HTTP GET health check via raw TCP + HTTP/1.1 status line parsing
- TCP connect health check with timeout
- Command health check with timeout enforcement (spawn + poll + kill)
- ProcessAlive health check via `kill(pid, 0)`
- `HealthState` enum — Unknown, Healthy, Degraded, Unhealthy
- `HealthHistory` ring buffer — configurable capacity, chronological iteration, consecutive failure tracking
- Ready check execution integrated into `start_service` (poll until ready or timeout, kill if failed)
- `poll_health` — periodic health checking with `HealthTracker` integration

#### v0.5.0 scope — Live Runlevel Transitions
- `execute_runlevel_switch` — two-phase execution: drain (stop non-target) then start (dependency-ordered)
- `RunlevelSwitchResult` — structured result with stopped/started/errors/drop_to_shell
- `drop_to_emergency_shell` — spawns agnoshi from `EmergencyShellConfig`
- Emergency shortcircuit in `plan_runlevel_switch` — early return, no wasted computation

#### v0.6.0 scope — Edge Boot Execution
- `execute_edge_boot` — runs rootfs lockdown, dm-verity verification, and LUKS unlock in sequence
- `unlock_luks` / `close_luks` — LUKS command generation with input validation
- `EdgeBootConfig` wired into `ArgonautConfig` as `edge_boot` field
- `EdgeBootResult` — structured result with rootfs/verity/luks status and boot timing
- `validate_edge_profile` — validates boot time budget, rootfs lockdown, and memory usage via `/proc/meminfo`
- `FleetRegistration` — builds system identity payload from `/etc/machine-id`, `/etc/hostname`, `/proc` for fleet server registration (JSON serializable)

#### v0.7.0 scope — Research-Driven Hardening & Integration
- `RELOADING=1` and `STOPPING=1` sd_notify lifecycle field support in `NotifyMessage`
- `systemd.rs` module — `generate_unit()` / `generate_unit_filename()` for hybrid systemd installs
- `api.rs` module — shared API response types for all consumers:
  - `ServiceStatus`, `ServiceListResponse`, `SystemStatusResponse`, `BootLogResponse` (agnoshi, MCP, daimon)
  - `ServiceCreateRequest` — daimon REST API service creation with input validation
  - `ServiceMetrics`, `SystemMetrics` — nazar metrics scrape endpoint types
  - `service_status()`, `list_services()`, `system_status()`, `boot_log()`, `system_metrics()`, `create_service_from_request()` methods on `ArgonautInit`
- `audit.rs` module (feature-gated: `audit`) — libro audit chain integration:
  - `AuditLog` wrapping `libro::AuditChain` for tamper-proof service event recording
  - `event_severity()` mapping all `ServiceEventType` variants to libro severity levels
  - `AuditIntegration` trait on `ArgonautInit` for combined tracing + audit recording
- `enable_service()` / `disable_service()` — runtime service enable/disable with `Enabled`/`Disabled` event types
- `enabled` field on `ServiceDefinition` — `start_service` guards on flag, `boot_execution_plan` skips disabled
- `Default` impl for `RestartPolicy` (defaults to `OnFailure`)
- Security: systemd unit file injection prevention (newline sanitization, `$` escaping, sorted env vars)
- Security: `create_service_from_request` rejects `..` traversal names and relative `binary_path`

#### v0.8.0 scope — Production Init Features
- `ServiceType` enum — `Simple`, `Forking`, `Oneshot` with dispatch in `start_service`
- `start_forking_service` — spawns parent, waits for exit, reads child PID from PID file
- `start_oneshot_service` — spawns, waits for completion, transitions to Stopped/Failed
- `resolve_service_waves` — wave-based parallel startup grouping via modified Kahn's algorithm
- `boot_execution_plan_waves` — returns `Vec<Vec<(String, ProcessSpec)>>` for parallel boot
- `ResourceLimits` struct — `RLIMIT_NOFILE`, `RLIMIT_AS`, `RLIMIT_NPROC` via `prlimit(1)` CLI
- `LogConfig` struct — size-based log rotation with configurable max files
- `rotate_log_if_needed` — rotates `.log` → `.log.1` → `.log.N` before spawn
- `load_environment_file` / `load_environment_files` — `KEY=VALUE` file parsing with quotes, comments
- Implicit `/etc/argonaut/env.d/{service}` environment file loading
- `read_pid_file` — PID file reading with validation and liveness check
- `SpawnedProcess.child` changed to `Option<Child>` for forked process tracking
- `pid_file`, `service_type`, `environment_files`, `resource_limits`, `log_config` fields on `ServiceDefinition`

#### v0.9.0 scope — Security Enforcement
- `security.rs` module — seccomp, Landlock, capabilities, socket activation, emergency auth
- `tmpfiles.rs` module — boot-time filesystem setup (directories, symlinks, device nodes)
- `SocketActivationConfig` / `SocketSpec` / `SocketType` — LISTEN_FDS/LISTEN_PID protocol
- `SeccompConfig` — `Basic` (agnosys 20-syscall filter) or `Custom { allow, deny }` with named syscalls
- `LandlockConfig` / `LandlockRule` / `LandlockAccess` — per-service filesystem restrictions
- `CapabilityConfig` / `LinuxCapability` — capability bounding set with `capsh` command generation
- `TmpfileEntry` — `Directory`, `Symlink`, `Device` with validation and SafeCommand generation
- `verify_emergency_auth` — SHA-256 password verification for emergency shell access
- `ResourceLimits.core` — RLIMIT_CORE field + `secure_defaults()` constructor (core dumps disabled)
- `EmergencyShellConfig.auth_password_hash` — stored hash for authentication
- Feature-gated `agnosys` integration (`security` feature): `apply_seccomp`, `apply_landlock`
- `socket_activation`, `seccomp`, `landlock`, `capabilities` fields on `ServiceDefinition`
- `tmpfiles` field on `ArgonautConfig`

### Changed
- `configure_readonly_rootfs()` returns `Vec<SafeCommand>` (was `Vec<String>` — injection risk)
- `resolve_service_order` accepts `&[&ServiceDefinition]` (was `&[ServiceDefinition]` — avoids deep clone)
- `reap_services` returns `Vec<(String, i32, CrashAction)>` (was `Vec<(String, i32)>`)
- `poll_health` accepts `&mut HealthTracker` — feeds results into consecutive failure tracking
- `try_recv` on `NotifyListener` returns `Result<Option<NotifyMessage>, io::Error>` (was `Option`)
- `stop_all_services` marks non-zero exit codes as `Failed` (was unconditionally `Stopped`)
- `boot_started` set on first step start, not first step completion
- `check_watchdog` triggers on stale `last_health_check`, not just `None`
- `restart_service` checks `RestartConfig.limit_exceeded` before restarting
- `stop_all` SIGKILL wait capped at 500ms (was unbounded blocking `wait`)
- Backoff delay uses `saturating_mul` to prevent overflow

### Fixed
- **Security**: Path traversal bypass in `validate_device_path` — `..` components now rejected
- **Security**: PID `u32→i32` cast overflow — safe conversion via `i32::try_from`, no wrong-process-group signals
- **Security**: `spawn_process` errors if `uid`/`gid` are set (was silently ignored)
- **Security**: TOCTOU race on notify socket removal — atomic remove + ignore NotFound
- Removed `unwrap()` in `start_service` — returns error instead of panicking
- Removed `expect("reconnect")` in HTTP health check — returns error on stream clone failure
- Removed silent localhost fallback in TCP health check — returns error on invalid address
- Fixed `HealthHistory::iter()` — now returns chronological order after ring buffer wraps
- Fixed `execute_command_check` ignoring timeout — now spawns + polls + kills on deadline
- Fixed `ShutdownAction::StopService` ignoring `signal` field — SIGKILL (9) now force-kills
- Fixed `HealthHistory::new(0)` division-by-zero — minimum capacity enforced to 1
- Fixed duplicate `depends_on` entries inflating in-degree in Kahn's algorithm
- Fixed `${USER}` shell variable literal in shruti service env — replaced with `/var/lib/shruti`
- `should_drop_to_emergency` no longer calls `failed_steps()` twice

### Removed
- `ServiceState::Restarting` variant — was dead code, never set anywhere
- `ureq` dependency — HTTP health checks use raw TCP (leaner dep tree)
- Standalone `backoff_delay()` function — replaced by `RestartConfig::backoff_delay()`

---

## [0.90.0] — 2026-04-02

### Added
- Initial scaffold: types, boot sequences, service definitions, dependency resolution
- Boot modes: Server, Desktop, Minimal, Edge
- Service management: registration, state machine, dependency-aware ordering (Kahn's algorithm)
- Shutdown planning: ordered steps with wall message, service stops, filesystem sync, LUKS close
- Runlevel system: Emergency, Rescue, Console, Graphical, Container, Edge
- Service targets: basic, network, agnos-core, graphical, edge
- Edge boot: read-only rootfs commands, dm-verity verification with input validation
- Health check types: HTTP GET, TCP connect, command, process-alive
- Health tracker: consecutive failure counting with configurable threshold
- Emergency shell configuration with banner and env setup
- Crash action determination: restart with backoff, ignore, give up
- Safe command abstraction for shell injection prevention
- 148 tests
