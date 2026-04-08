# Argonaut Roadmap

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## Current — v0.95.0

Cyrius port complete. 12 test suites (395 assertions), 29 benchmarks, 207KB binary.
All v0.7–v0.9 features ported. Audit module (libro shim) in place.

---

## v0.96.0 — Test Coverage & Hardening

### Test gaps (from audit)
- [ ] Process management tests: fork_exec_service, process_stop, process_kill, proc_table_reap
- [ ] Init service lifecycle tests: start_simple, start_oneshot, start_forking, stop, restart
- [ ] Health check execution tests: TCP connect, command check, ready check retries, HTTP parsing
- [ ] Edge boot execution tests: execute_edge_boot, close_luks full path
- [ ] Notify I/O tests: bind, try_recv, drain, send
- [ ] Security tests: landlock_description, capability_setpriv_cmd

### Code quality
- [ ] HTTP health check: validate HTTP status line (currently TCP connect only)
- [ ] API response builders: list_services, system_status, system_metrics, boot_log
- [ ] Resource limit command generation: to_prlimit_commands
- [ ] boot_execution_plan / boot_execution_plan_waves
- [ ] safe_cmd_display (SafeCommand Display)

### Missing benchmarks (3 blocked on above)
- [ ] api_responses (4 benchmarks — needs response builders)
- [ ] resource_limits_prlimit (1 benchmark — needs prlimit command gen)

---

## v0.97.0 — Sakshi Integration

- [ ] Integrate sakshi (structured tracing/logging) for service state transitions
- [ ] Trace: boot step completion/failure
- [ ] Trace: health check results
- [ ] Trace: shutdown orchestration steps
- [ ] Trace: watchdog enforcement actions
- [ ] Replace audit.cyr println-based logging with sakshi spans

---

## v0.98.0 — Libro Integration

Blocked on: majra Cyrius port -> libro Cyrius port.

- [ ] Replace audit.cyr shim with real libro includes
- [ ] Swap FNV-1a hash for libro's BLAKE3/SHA-256
- [ ] Wire AuditChain persistence (write to disk)
- [ ] QueryFilter full support (time range, action, agent_id)

---

## v1.0.0 Criteria

- [x] All P0 library gaps closed
- [x] All P1 library gaps closed
- [x] API stable — pre-1.0 with documented stability plan (ADR-012)
- [x] 12 ADRs for major design decisions
- [x] Security posture documented and reviewed
- [x] QEMU boot: minimal < 3s (2.98s)
- [x] QEMU boot: desktop < 3s (2.9s with real daimon)
- [x] Crash recovery tested (exponential backoff, restart limit, GiveUp)
- [x] Shutdown ordering tested (clean stop -> sync -> poweroff)
- [ ] Edge boot < 1s
- [ ] Real hardware testing (RPi4, NUC)
- [ ] Sakshi tracing integrated
- [ ] Libro audit chain (real, not shim)
- [ ] Kybernet using argonaut library (not hand-rolled PID 1)
- [ ] 95%+ function test coverage

---

## Kybernet Integration (separate repo)

PID 1 helmsman — https://github.com/MacCracken/kybernet

Currently uses hand-rolled init logic. Goal: replace with argonaut library calls.
Blocked on: libro Cyrius port (for audit chain).

- [ ] Wire kybernet to argonaut's init_start_service / init_stop_service
- [ ] Wire kybernet to argonaut's boot_execution_plan_waves
- [ ] Wire kybernet to argonaut's init_plan_shutdown
- [ ] Seccomp/Landlock application in pre_exec
- [ ] Control socket for agnoshi runtime commands
- [ ] Real hardware testing (RPi4, NUC)

---

## Known Compiler Issues (cc2)

| # | Issue | Impact | Workaround |
|---|-------|--------|------------|
| 16 | Adding includes shifts global addresses | Test string corruption in large compilation units | Split large .tcyr files to stay under string buffer |
| — | System stdlib cross-includes exceed token limit | Can't use `~/.cyrius/lib/` symlink | Use local `lib/` copy with updated assert.cyr |
| — | String data buffer 8192 bytes | Large test files overflow silently | Keep test files < ~500 string literals |

---

## Non-goals

- **Container orchestration** — that's stiva
- **Package installation** — that's ark
- **Agent lifecycle** — that's daimon (argonaut manages the process, daimon manages the agent)
- **Scheduling** — that's samay (argonaut starts/stops, samay decides when)
- **D-Bus interface** — only if AGNOS desktop requires it
- **Timer-based services** — that's samay's domain
