# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html) (pre-1.0).

---

## [Unreleased]

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
