# Argonaut Roadmap

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## v0.2.0 — Hardening (complete)

- [x] Serde roundtrip tests for all public types (24 tests)
- [x] CI workflows (ci.yml, release.yml)
- [x] Rename "agent-runtime" to "daimon" in all service definitions
- [x] Add `BootMode::Recovery` with boot sequence, tests, runlevel mapping
- [x] `#![forbid(unsafe_code)]`
- [x] `RestartConfig` struct — configurable backoff curve and restart limit per-service
- [x] Minimum 100ms floor on backoff delay to prevent busy-retry loops

---

## v0.3.0 — Process Execution (complete)

- [x] `process.rs` — fork/exec via `std::process::Command` from `ProcessSpec`
- [x] `SpawnedProcess` — PID tracking, `try_wait`/`wait`, uptime
- [x] Signal delivery via `nix` — SIGTERM, SIGKILL, arbitrary signals
- [x] Graceful stop: SIGTERM → poll → SIGKILL with configurable timeout
- [x] Stdout/stderr capture to log files (graceful fallback to /dev/null)
- [x] `ProcessTable` — tracks all running processes, bulk reap, bulk stop
- [x] `ArgonautInit::start_service/stop_service/restart_service` — full lifecycle
- [x] `ArgonautInit::reap_services` — detect and handle exited processes
- [x] `execute_shutdown` — walks `ShutdownPlan` steps with real process stops
- [x] `run_command`/`run_command_sequence` — one-shot `SafeCommand` execution
- [x] Watchdog timer — `check_watchdog`/`enforce_watchdog` for startup + runtime timeouts
- [x] `notify.rs` — sd_notify compatible `NotifyListener` (READY=1, STATUS, MAINPID)
- [x] 223 tests, 0 benchmark regressions

---

## v0.4.0 — Health Check Execution (complete)

- [x] `health.rs` — execute all `HealthCheckType` variants (zero external HTTP deps)
- [x] HTTP GET via raw TCP + HTTP/1.1 status parsing
- [x] TCP connect check with timeout
- [x] Command check (exit code 0 = healthy)
- [x] ProcessAlive check via `kill(pid, 0)`
- [x] `HealthState` enum (Unknown → Healthy → Degraded → Unhealthy)
- [x] `HealthHistory` ring buffer with configurable capacity
- [x] Ready check execution in `start_service` (poll until ready or timeout, kill if failed)
- [x] `poll_health` method for periodic health checking of all running services
- [x] Removed `ureq` dependency — raw TCP keeps dep tree lean
- [x] 240 tests, 0 benchmark regressions

---

## v0.5.0 — Live Runlevel Transitions

Planning logic exists (`plan_runlevel_switch`, `plan_shutdown`, `shutdown_order`). This milestone executes those plans.

- [ ] Execute runlevel switch plans (stop/start services per plan)
- [ ] Graceful drain: stop non-target services before starting target services
- [ ] Execute shutdown plans (walk `ShutdownPlan.steps` in order)
- [ ] Emergency shell: actually exec agnoshi on required boot step failure

---

## v0.6.0 — Edge Boot Execution

Command generation exists (`configure_readonly_rootfs`, `verify_rootfs_integrity`). This milestone runs them.

- [ ] Execute rootfs/verity `SafeCommand` sequences
- [ ] dm-verity integration via agnosys crate
- [ ] LUKS unlock during boot (TPM or passphrase)
- [ ] Boot time budget enforcement (watchdog at `EdgeBootConfig.max_boot_time_ms`)
- [ ] Minimal boot profile validation (< 2s target, < 50MB RAM)
- [ ] Fleet auto-registration on first boot (daimon edge handshake)

---

## v0.7.0 — Integration

- [ ] systemd unit generation (for hybrid installs)
- [ ] agnoshi commands: `service start/stop/restart/status/enable/disable`
- [ ] MCP tools: `argonaut_services`, `argonaut_status`, `argonaut_boot_log`
- [ ] Audit logging via libro (service state transitions)
- [ ] daimon API: `/v1/services` CRUD endpoints backed by argonaut
- [ ] Integration with nazar (expose `/v1/services` metrics endpoint)

---

## v1.0.0 Criteria

- [ ] All boot modes tested on real hardware (QEMU, RPi4, NUC)
- [ ] Boot time < 3s (Desktop), < 1s (Edge)
- [ ] Crash recovery tested: kill every service, verify auto-restart
- [ ] Shutdown ordering tested: no orphan processes after halt
- [ ] API stable — no breaking changes to public types
- [ ] 80%+ code coverage
- [ ] Benchmark history proves no regressions

---

## Non-goals

- **Container orchestration** — that's stiva
- **Package installation** — that's ark
- **Agent lifecycle** — that's daimon (argonaut manages the process, daimon manages the agent)
- **Scheduling** — that's samay (argonaut starts/stops, samay decides when)
