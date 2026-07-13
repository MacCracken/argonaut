# Argonaut — Claude Code Instructions

> **Core rule**: this file is **preferences, process, and procedures** — durable rules that change rarely. Volatile state (current version, binary sizes, test counts, in-flight work, consumers, verification hosts) lives in [`docs/development/state.md`](docs/development/state.md), bumped every release. Do not inline state here — inlined state rots within a minor.

---

## Project Identity

**Argonaut** (Greek: sailors of the Argo — one letter off from AGNOS) — Init system and service manager for AGNOS: boot sequencing, service lifecycle (simple/forking/oneshot), dependency resolution (Kahn's algorithm), health checks (HTTP/TCP/command/process-alive), watchdog enforcement, shutdown orchestration, runlevel switching, edge boot (dm-verity/LUKS/read-only rootfs), security enforcement (seccomp/Landlock/capabilities), sd_notify protocol, systemd unit generation, tmpfiles setup, cryptographic audit chain.

- **Type**: Cyrius application (ported from a Rust crate; rust-old removed at v0.96.1)
- **License**: GPL-3.0-only
- **Language**: Cyrius (toolchain pinned in `cyrius.cyml [package].cyrius`)
- **Version**: `VERSION` at the project root is the source of truth — do not inline the number here
- **Genesis repo**: [agnosticos](https://github.com/MacCracken/agnosticos)
- **Philosophy**: [AGNOS Philosophy & Intention](https://github.com/MacCracken/agnosticos/blob/main/docs/philosophy.md)
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md) · [First-Party Documentation](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-documentation.md)
- **Shared crates**: [shared-crates.md](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/shared-crates.md)
- **Recipes**: [zugot](https://github.com/MacCracken/zugot) — takumi build recipes

## Goal

Own PID 1 for AGNOS. Boot sequencing, service lifecycle, and shutdown orchestration in pure Cyrius — no systemd dependency, no shell, no scripting language at runtime. Every operation measurable, auditable (libro chain), and traceable. The binary is the spec.

## Current State

> Volatile state lives in [`docs/development/state.md`](docs/development/state.md) —
> current version, binary size (DCE), test/assertion counts, benchmark snapshot,
> dep pins (libro / patra / sigil), consumers, verification hosts. Refreshed
> every release.
> Historical release narrative lives in [`CHANGELOG.md`](CHANGELOG.md).

This file (`CLAUDE.md`) is durable rules.

## Quick Start

```bash
cyrius build src/main.cyr build/argonaut         # build (auto-resolves deps from cyrius.cyml)
cyrius test tests/tcyr/<suite>.tcyr              # one suite
for t in tests/tcyr/*.tcyr; do cyrius test "$t"; done   # full sweep
cyrius bench tests/bcyr/argonaut.bcyr            # benchmarks
./scripts/bench-history.sh "<label>"             # bench → bench-history.csv (DCE build)
cyrius lint src/*.cyr                            # static checks
cyrius fmt <file> --check                        # format check
cyrius vet src/main.cyr                          # vet
cyrius deps --verify                             # verify cyrius.lock hashes
CYRIUS_DCE=1 cyrius build src/main.cyr build/argonaut    # release build
```

## Key Principles

- **Correctness is the optimum sovereignty** — if it's wrong, you don't own it; the bugs own you
- **Own the stack** — if an AGNOS crate wraps a primitive, depend on the AGNOS crate (libro for audit, sigil for SHA-256, patra for storage)
- **No magic** — every operation measurable, auditable, traceable. The audit chain is the truth
- **Tests + benchmarks are the way** — every behavior change earns a `.tcyr` regression test; every perf claim earns a `.bcyr` bench number
- **Vec arena over HashMap** — when indices are known, direct access beats hashing
- **str_builder over string concat** — avoid temporary allocations on the hot path
- **Packed Result** — zero-alloc on success, heap on the cold-path error
- **SafeCommand for all process execution** — never shell strings; explicit argv only
- **Build with `cyrius build`** — never raw `cat file | cc5`; the manifest auto-resolves deps and prepends includes
- **Source files only need project includes** — stdlib + external deps auto-resolve from `cyrius.cyml [deps]`
- **Cyrius language reference**: `vidya/content/cyrius/` is canonical for syntax, idioms, gotchas

## Rules (Hard Constraints)

- **Read the genesis repo's CLAUDE.md first** — [agnosticos/CLAUDE.md](https://github.com/MacCracken/agnosticos/blob/main/CLAUDE.md)
- **Do not commit or push** — the user handles all git operations (commit, tag, push)
- **NEVER use `gh` CLI** — use `curl` to the GitHub API if needed
- Do not add unnecessary dependencies — keep it lean
- Do not skip tests / benchmarks before claiming a change works
- **Do not bump `VERSION` without a benchmark delta check** — every release (patch, minor, *and* major) runs `./scripts/bench-history.sh "<version>-<label>"` against the prior label in `bench-history.csv`. Inspect the deltas, flag any regression, and record the result (win, neutral, or regression-with-justification) in the CHANGELOG entry and `state.md` bench snapshot. A regression ships only with an explicit written reason. See [Mandatory Benchmark Gate](#mandatory-benchmark-gate-every-release)
- Do not commit `build/` (compiled binaries)
- Do not add Cyrius stdlib includes in individual src files — the manifest resolves them
- Do not hardcode toolchain versions in CI YAML — `cyrius = "X.Y.Z"` in `cyrius.cyml` is the only source of truth (no separate `.cyrius-toolchain` file)
- Do not call stdlib `json_build/1` — `lib/patra.cyr` defines a 6-arg overload that silently shadows it; use `str_builder` directly
- Do not use `break` in `while` loops with `var` declarations — flag + `continue`
- Do not write negative literals as `(0 - N)` — `-N` works since cyrius 3.10.3
- Do not mix `&&` / `||` in one expression — nest `if` blocks instead
- Do not use `match` as a variable name (reserved)
- Do not use `return;` without a value — always `return 0;`
- Do not use `sys_system()` with unsanitized input — command injection
- Do not trust external data (file content, network, args) without validation

## Process

### P(-1): Scaffold / Project Hardening (before any new features)

1. **Read** roadmap, CHANGELOG, open issues — know what was intended before auditing what was built
2. **Cleanliness** — `cyrius build`, `cyrius lint`, full test sweep; all green
3. **Benchmark baseline** — `./scripts/bench-history.sh "<label>-baseline"`
4. **Internal deep review** — gaps, optimizations, correctness, edge cases, stale comments
5. **External research** — domain CVEs / 0days, best practices, prior incidents (websearch + vendor advisories)
6. **Security audit** — input handling, syscall usage, buffer sizes, pointer validation. File findings in `docs/audit/YYYY-MM-DD-audit.md` with severity tags
7. **Regression tests for findings** — every MEDIUM+ gets a failing test BEFORE the fix (per the regression-tests-over-docs rule)
8. **Additional benchmarks** from findings if perf-relevant
9. **Post-audit benchmarks** — prove the wins (or document neutrality) against step 3
10. **Documentation audit** — ADRs for non-obvious decisions, source citations for any new algorithms, update `docs/development/state.md`
11. **Repeat if heavy** — keep drilling until clean

### Work Loop (continuous)

1. **Work phase** — features, roadmap items, bug fixes
2. **Build check** — `cyrius build src/main.cyr build/argonaut`
3. **Test + benchmark additions** for new code
4. **Internal review** — performance, memory, correctness, edge cases
5. **Security check** — any new syscall usage, user input handling, buffer allocation
6. **Documentation** — update CHANGELOG, roadmap, `docs/development/state.md`, any ADR the change earned
7. **Version check** — `VERSION`, `cyrius.cyml` (via `${file:VERSION}`), CHANGELOG header in sync
8. **Benchmark gate** — on any `VERSION` bump, run the [Mandatory Benchmark Gate](#mandatory-benchmark-gate-every-release): bench delta vs the prior label, classify, record. Release-blocking on an unexplained regression
9. **Return to step 1**

### Security Hardening (before every release)

Per [first-party-standards § Security Hardening](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md#security-hardening-new--required-before-every-release):

1. **Input validation** — every fn accepting external data validates bounds, types, ranges
2. **Buffer safety** — every `var buf[N]` verified; N is **bytes**, max access < N, no adjacent-variable overflow
3. **Syscall review** — every syscall validated: args checked, returns handled, error paths complete
4. **Pointer validation** — no raw deref of untrusted input without bounds check
5. **No command injection** — `safe_cmd_*` API only; never `sys_system()` with user input
6. **No path traversal** — file paths from external input validated; no `../` escape
7. **Constant-time comparisons** for secrets — `constant_time_eq_str` (sigil), never `str_eq` for auth tokens
8. **Known-CVE sweep** — websearch for current init/service-manager/systemd CVEs and 2025-2026 0days; map to argonaut's surface
9. **Document findings** — `docs/audit/YYYY-MM-DD-audit.md`

Severity: **CRITICAL** (remote / privilege escalation), **HIGH** (moderate effort), **MEDIUM** (specific conditions), **LOW** (defense-in-depth).

### Closeout Pass (before every minor/major bump)

Run before tagging `X.Y.0` or `X.0.0`. Ship as the last patch of the prior minor where applicable.

1. **Full test suite** — all `.tcyr` pass, zero failures
2. **Benchmark snapshot** — compare against prior closeout label in `bench-history.csv`
3. **Dead code audit** — `cyrius build` "dead:" floor recorded in CHANGELOG
4. **Refactor pass** — consolidate parallel codepaths added during the minor
5. **Code review pass** — diffs end-to-end for missed guards, ABI leaks, off-by-ones, silently-ignored errors
6. **Cleanup sweep** — stale comments, dead `#ifdef` branches, unused includes, orphaned files (notably `src/test_*.cyr` stubs that predate `tests/tcyr/`)
7. **Security re-scan** — quick grep for new `sys_system`, unchecked writes, unsanitized input, buffer size mismatches
8. **Downstream check** — kybernet (and any consumer in `state.md`) builds and tests against the new version
9. **Doc sync** — CHANGELOG, roadmap, `docs/development/state.md`, CLAUDE.md (if durable content changed)
10. **Version verify** — `VERSION`, `cyrius.cyml`, CHANGELOG header, intended git tag all match
11. **Full clean build** — `rm -rf build && cyrius deps && CYRIUS_DCE=1 cyrius build` passes clean

### Mandatory Benchmark Gate (every release)

**Non-negotiable, runs for every `VERSION` bump — patch, minor, and major.** A release is not done until the bench delta is checked and recorded.

1. **Run** — `./scripts/bench-history.sh "<version>-<label>"` (DCE build) appends a fresh row to `bench-history.csv`. Use a descriptive label (e.g. `1.8.0-toolchain-6.0.26`).
2. **Compare** — diff every micro against the prior release's label. The closeout-label series is the canonical baseline; for a patch, compare against the most recent label.
3. **Classify** the result:
   - **Win / neutral** (within ±2 µs noise, or faster) — record the snapshot table in `state.md` and note it in the CHANGELOG entry.
   - **Regression** (a micro slower beyond noise) — do **not** ship silently. Either fix it, or ship with an explicit written justification in the CHANGELOG (`### Performance`) naming the bench, the delta, and why it's acceptable.
4. **Record** — update the `state.md` "Bench snapshot" table to the new label and reference `bench-history.csv` for the full series.

Rationale: argonaut is PID 1 — boot sequencing, dependency resolution, and health-check latency are user-visible. Every release proves its perf posture against the prior one; an unexplained regression is a release blocker. See the `VERSION`-bump hard constraint in [Rules](#rules-hard-constraints).

### Task Sizing

- **Low/Medium effort**: batch freely — multiple items per work loop cycle
- **Large effort**: small bites only — break into sub-tasks, verify each before moving on
- **If unsure**: treat it as large

### Refactoring Policy

- Refactor when the code tells you to — duplication, unclear boundaries, measured bottlenecks
- Never refactor speculatively. Wait for the third instance
- Every refactor passes the same test + bench gates as new code
- 3 failed attempts = defer and document — don't burn time in a rabbit hole

## Cyrius Conventions

- All struct fields are 8 bytes (`i64`), accessed via `load64` / `store64` with offset (or `#derive(accessors)` getters/setters where adopted)
- Heap allocation via `fl_alloc()` / `fl_free()` (freelist) for data with individual lifetimes
- Bump allocation via `alloc()` for long-lived data (vec, str internals)
- Enum values for constants — don't consume `gvar_toks` slots (4,096 initialized globals limit)
- Heap-allocate large buffers — `var buf[N]` inside a function is **static data** (not stack), bloats the binary, and consecutive calls clobber any returned Str/buf-borrowing values. The build's "large static data (N bytes)" warning is the upstream tell
- 5.x stdlib lookup helpers (`toml_get`, `toml_get_sections`, …) take **cstr keys**; passing `str_from("…")` silently returns 0. JSON helpers (`json_get`) still take `Str`
- `break` in while loops with `var` declarations is unreliable — flag + `continue`
- `match` is reserved; `return;` without value is invalid (use `return 0;`); all `var` declarations are function-scoped
- Per-compilation-unit limits: 4,096 variables, 1,024 functions, 4,096 initialized globals, 16,384 fixups (cc5 5.4.2+)
- Counting rule: only a top-level `var NAME = <non-literal>;` (call / identifier / expression initializer) consumes an initialized-globals slot; a bare integer-literal init (`var x = 42;`) takes the static-init fast path and enum members are const-folded, so neither counts. See the cyrius guide's **Global Initializers** section (`docs/guides/cyrius-guide.md` in the cyrius repo)
- **Patra `json_build/6` shadows stdlib `json_build/1`** — never call stdlib `json_build`; build flat JSON with `str_builder` directly

## CI / Release

- **Toolchain pin**: `cyrius = "X.Y.Z"` field in `cyrius.cyml [package]`. **No separate `.cyrius-toolchain`.** CI and release both read this; no hardcoded version strings in YAML
- **Lockfile gate**: `cyrius.lock` checked in; CI runs `cyrius deps --verify`
- **Dead code elimination**: every `cyrius build` in CI/release runs with `CYRIUS_DCE=1`. Binary size is a release metric — track in `state.md`
- **Tag filter**: release workflow accepts both `v[0-9]+.[0-9]+.[0-9]+` and `[0-9]+.[0-9]+.[0-9]+` shapes; semver shape verified before build
- **Version-verify gate**: release asserts `VERSION == ${file:VERSION}-resolved manifest version == git tag` before building. Mismatch fails the run
- **Lint / fmt sweep**: CI auto-discovers `src/*.cyr` + `tests/tcyr/*.tcyr` + `tests/bcyr/*.bcyr`; `cyrlint` warnings are blocking; `cyrfmt --check` drift is blocking
- **ELF magic check**: every CI build verifies the produced binary starts with `7f 45 4c 46`
- **Workflow layout**:
  - `.github/workflows/ci.yml` — toolchain install → deps verify → lint → fmt → vet → DCE build → ELF check → smoke → tests → bench (non-fatal artifact); reusable via `workflow_call`
  - `.github/workflows/release.yml` — version gate → CI gate → DCE build → archives (source tarball, x86_64 binary, SHA256SUMS, cyrius.lock)
- **Concurrency**: CI uses `cancel-in-progress: true` keyed on workflow + ref — only the latest push is tested
- **State sync**: every release also bumps `docs/development/state.md`. If you find yourself hand-editing state during routine work, the release hook needs fixing — file an issue against the workflow

## Docs

- [`docs/adr/`](docs/adr/) — architecture decision records. *Why X over Y?*
- [`docs/architecture/`](docs/architecture/) — non-obvious constraints and quirks
- [`docs/guides/`](docs/guides/) — task-oriented how-tos
- [`docs/examples/`](docs/examples/) — runnable examples
- [`docs/development/roadmap.md`](docs/development/roadmap.md) — completed, backlog, future, v1.0 criteria
- [`docs/development/state.md`](docs/development/state.md) — **live state snapshot, refreshed every release**
- [`docs/audit/`](docs/audit/) — security audit reports (`YYYY-MM-DD-audit.md`)
- [`CHANGELOG.md`](CHANGELOG.md) — source of truth for all changes

New quirks land in `docs/architecture/` as numbered items (`NNN-kebab-case.md`); new decisions in `docs/adr/` using the template. **Never renumber either series.**

Full doc-tree convention: [first-party-documentation.md](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-documentation.md).

## Documentation Structure

```
Root files (required):
  README.md, CHANGELOG.md, CLAUDE.md, CONTRIBUTING.md,
  SECURITY.md, CODE_OF_CONDUCT.md, LICENSE,
  VERSION, cyrius.cyml, cyrius.lock

docs/ (minimum):
  adr/ — architectural decision records (README + template.md + NNNN-*.md)
  architecture/ — non-obvious invariants (README + NNN-*.md)
  guides/ — task-oriented how-tos
  examples/ — runnable examples
  development/
    roadmap.md — completed, backlog, future
    state.md — live state snapshot (volatile; release-hook-bumped)

docs/ (when earned):
  audit/ — security audit reports (YYYY-MM-DD-audit.md)
  sources.md — algorithm/citation references
  proposals/ — pre-ADR design drafts
  api/ — curated public-surface reference
  benchmarks.md — perf history narrative
```

## CHANGELOG Format

Follow [Keep a Changelog](https://keepachangelog.com/). Performance claims **must** include benchmark numbers. Breaking changes get a **Breaking** section with migration guide. Security fixes get a **Security** section with severity (CRITICAL/HIGH/MEDIUM/LOW) and reference the audit report. See [first-party-documentation § CHANGELOG](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-documentation.md#changelog) for the full conventions.
