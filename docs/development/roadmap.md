# Argonaut Roadmap

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## Current — v0.97.0

23 test suites (582 assertions), 37 benchmarks, 373KB binary (cc3 3.5.0).
Libro 1.0.2 integrated — real SHA-256 audit chain replaces FNV-1a shim.

---

## v0.98.0 — Libro Extended Features

- [ ] `record_audited_event()` bridge (ArgonautInit + AuditLog wired into service lifecycle)
- [ ] Wire AuditChain persistence (libro FileStore — write to disk)
- [ ] QueryFilter time range support (after/before epoch filtering)
- [ ] QueryFilter agent_id support
- [ ] Libro signing module (Ed25519 signed entries)
- [ ] Libro merkle module (Merkle tree for chain batches)
- [ ] Libro export (JSONL/CSV audit trail export)

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
- [x] Sakshi tracing integrated (sakshi_full 0.7.0, v0.96.1)
- [x] Rust port complete, rust-old removed (v0.96.1)
- [ ] Edge boot < 1s
- [ ] Real hardware testing (RPi4, NUC)
- [x] Libro audit chain (real, not shim) — v0.97.0, SHA-256 via libro 1.0.2
- [ ] Kybernet using argonaut library (not hand-rolled PID 1)
- [ ] 95%+ function test coverage

---

## Kybernet Integration (separate repo)

PID 1 helmsman — https://github.com/MacCracken/kybernet

Currently uses hand-rolled init logic. Goal: replace with argonaut library calls.
Unblocked: libro 1.0.2 integrated in v0.97.0.

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
