# Argonaut Roadmap

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## Current — v1.0.0

26 test suites (606 assertions), 37 benchmarks, 373KB binary (cc3 3.6.2).
Libro 1.0.2 integrated — SHA-256 audit chain with lifecycle recording.
All pre-1.0 features complete. Cyrius-native, no external runtime dependencies.

---

## Post-1.0 — Libro Extended Features

- [ ] Wire AuditChain persistence (libro FileStore — write to disk, needs patra lock fns)
- [ ] QueryFilter time range support (after/before epoch filtering)
- [ ] QueryFilter agent_id support
- [ ] Libro signing module (Ed25519 signed entries)
- [ ] Libro merkle module (Merkle tree for chain batches)
- [ ] Libro export (JSONL/CSV audit trail export)
- [ ] Include all 19 libro modules (16 compile on cc3 3.6.2, 3 need patra lock fns)

---

## Post-1.0 — Kybernet Integration (separate repo)

PID 1 helmsman — https://github.com/MacCracken/kybernet

Currently uses hand-rolled init logic. Goal: replace with argonaut library calls.
Unblocked: libro 1.0.2 integrated in v1.0.0.

- [ ] Wire kybernet to argonaut's init_start_service / init_stop_service
- [ ] Wire kybernet to argonaut's boot_execution_plan_waves
- [ ] Wire kybernet to argonaut's init_plan_shutdown
- [ ] Seccomp/Landlock application in pre_exec
- [ ] Control socket for agnoshi runtime commands
- [ ] Real hardware testing (RPi4, NUC)
- [ ] Edge boot < 1s

---

## v1.0.0 Criteria — All Met

- [x] All P0 library gaps closed
- [x] All P1 library gaps closed
- [x] API stable (ADR-012)
- [x] 12 ADRs for major design decisions
- [x] Security posture documented and reviewed
- [x] QEMU boot: minimal < 3s (2.98s)
- [x] QEMU boot: desktop < 3s (2.9s with real daimon)
- [x] Crash recovery tested (exponential backoff, restart limit, GiveUp)
- [x] Shutdown ordering tested (clean stop -> sync -> poweroff)
- [x] Sakshi tracing integrated (sakshi_full 0.7.0)
- [x] Cyrius port complete, rust-old removed (v0.96.1)
- [x] Libro audit chain (real SHA-256, not shim) — v0.97.0
- [x] Lifecycle audit recording — v0.97.0
- [x] 26 test suites, 606 assertions, 0 failures

---

## Known Compiler Issues (cc3 — see docs/issues/cc3-readfile-cap.md)

| # | Issue | Impact | Status |
|---|-------|--------|--------|
| 4 | `break` in chained if blocks inside while | json.cyr integer parsing | Fixed in stdlib 3.2.6 |
| 16 | Adding includes shifts global addresses | Test string corruption | Split large .tcyr files |
| — | String data buffer 8192 bytes | Large test files overflow | Keep < ~500 string literals |
| — | READFILE 512KB cap | Truncated includes | Fixed in cc3 3.5.1 |
| — | `ptr` variable regression | Build failure | Fixed in cc3 3.6.1+ |
| — | Codebuf/token limits | All 19 libro modules | Fixed in cc3 3.6.2 |

---

## Non-goals

- **Container orchestration** — that's stiva
- **Package installation** — that's ark
- **Agent lifecycle** — that's daimon (argonaut manages the process, daimon manages the agent)
- **Scheduling** — that's samay (argonaut starts/stops, samay decides when)
- **D-Bus interface** — only if AGNOS desktop requires it
- **Timer-based services** — that's samay's domain
