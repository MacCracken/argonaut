# Argonaut Roadmap

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## v0.2.0 — Hardening (current)

- [ ] Serde roundtrip tests for all public types
- [ ] CI workflows (ci.yml, release.yml)
- [ ] Rename "agent-runtime" references in service definitions to "daimon"
- [ ] Add `BootMode::Recovery` (README documents it, enum doesn't have it)
- [ ] `#![forbid(unsafe_code)]` (no unsafe exists — just add the annotation)
- [ ] Make backoff delay curve configurable via `RestartPolicy` (currently hardcoded 1s/2s/4s/8s/16s cap 30s)
- [ ] Make restart limit configurable per-service (currently hardcoded to 5)

---

## v0.3.0 — Process Execution

Fork/exec is the critical gap — everything before this is types and planning only.

- [ ] Fork/exec service processes via `SafeCommand` / `ProcessSpec`
- [ ] PID tracking and reaping (waitpid)
- [ ] Signal delivery (SIGTERM for graceful, SIGKILL after timeout)
- [ ] Stdout/stderr capture to log files (`/var/log/agnos/services/{name}.log`)
- [ ] Shutdown timeout enforcement (SIGTERM → wait → SIGKILL)
- [ ] Watchdog timer (kill unresponsive services after configured timeout)
- [ ] Service readiness notification (sd_notify compatible)

---

## v0.4.0 — Health Check Execution

Types exist (`HealthCheck`, `HealthCheckType`, `HealthTracker`, `ReadyCheck`). This milestone wires them to real I/O.

- [ ] HTTP health check execution (GET, check 2xx)
- [ ] TCP health check (connect and close)
- [ ] Command health check (run command, check exit code)
- [ ] ProcessAlive health check (kill(pid, 0))
- [ ] Health state machine (healthy → degraded → unhealthy)
- [ ] Health history ring buffer per service
- [ ] Ready check execution at startup (block until ready or timeout)

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
