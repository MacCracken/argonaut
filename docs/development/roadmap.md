# Argonaut Roadmap

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## v0.6.0 — Edge Boot Execution (complete)

- [x] `execute_edge_boot` — runs rootfs lockdown, dm-verity, LUKS unlock in sequence
- [x] `unlock_luks` / `close_luks` — LUKS command generation with input validation
- [x] `EdgeBootConfig` wired into `ArgonautConfig`
- [x] Boot time budget enforcement — reports `within_budget` in `EdgeBootResult`
- [x] `validate_edge_profile` — memory + boot time + rootfs validation
- [x] `FleetRegistration` — system payload builder from `/proc` + `/etc` (JSON serializable)
- [x] 256 tests, 0 benchmark regressions

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
