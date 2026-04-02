# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html) (pre-1.0).

---

## [Unreleased]

### Added

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

#### v0.5.0 scope — Live Runlevel Transitions
- `execute_runlevel_switch` — two-phase execution: drain (stop non-target) then start (dependency-ordered)
- `RunlevelSwitchResult` — structured result with stopped/started/errors/drop_to_shell
- `drop_to_emergency_shell` — spawns agnoshi from `EmergencyShellConfig`
- Emergency shortcircuit in `plan_runlevel_switch` — early return, no wasted computation

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

## [0.1.0] — 2026-04-02

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
