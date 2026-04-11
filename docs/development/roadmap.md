# Argonaut Roadmap

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## Current — v0.96.1

22 test suites (545 assertions), 34 benchmarks, 197KB binary (cc3 3.2.5+).
sakshi_full.cyr from stdlib. All v0.96/v0.97 items complete.

---

## v0.98.0 — Libro Integration

Blocked on: majra Cyrius port -> libro Cyrius port (nearly ready).

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
- [x] Sakshi tracing integrated (sakshi_full 0.7.0, v0.96.1)
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

## Known Compiler Issues (cc3)

| # | Issue | Impact | Workaround |
|---|-------|--------|------------|
| 4 | `break` in chained if blocks inside while | json.cyr integer parsing broken | Flag variable + `||` (fixed in stdlib 3.2.6) |
| 16 | Adding includes shifts global addresses | Test string corruption in large compilation units | Split large .tcyr files to stay under string buffer |
| — | String data buffer 8192 bytes | Large test files overflow silently | Keep test files < ~500 string literals |

---

## Non-goals

- **Container orchestration** — that's stiva
- **Package installation** — that's ark
- **Agent lifecycle** — that's daimon (argonaut manages the process, daimon manages the agent)
- **Scheduling** — that's samay (argonaut starts/stops, samay decides when)
- **D-Bus interface** — only if AGNOS desktop requires it
- **Timer-based services** — that's samay's domain
