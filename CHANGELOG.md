# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.5.1] — 2026-05-10

Toolchain + dep refresh. Cyrius pin 5.7.5 → 5.10.34 (70+ upstream
slots picked up: parser/codegen polish, stdlib additions
[`thread`, `random`, `result`], sakshi/sigil promoted from stdlib
to external git pins). Libro 2.0.5 → 2.6.2; patra 1.1.1 → 1.9.3.
CI / release workflows aligned with the agnosys / agnostik 5.10
pattern (versioned toolchain layout, lockfile-gated hash verify,
fmt-via-diff for the 5.9 `--check` no-op).

### Changed

- **`cyrius.cyml`** — `[package].cyrius` pinned `5.7.5` → `5.10.34`.
- **`cyrius.cyml`** — `[deps.libro]` pinned `2.0.5` → `2.6.2`,
  `[deps.patra]` pinned `1.1.1` → `1.9.3`. New `[deps.argonaut_compat]`
  self-reference auto-loads `src/compat.cyr` ahead of every
  compilation unit (build + every standalone `.tcyr` test).
- **`cyrius.cyml`** — `[deps].stdlib` trimmed: `sakshi` and `sigil`
  removed (libro 2.5+ promoted both to external git pins and patra
  1.9+ pulls sakshi as its own dep; they now land in `lib/` via
  `cyrius deps` transitive resolve and would duplicate-define
  against the version-pinned stdlib copy). `thread` and `random`
  added — libro 2.6.2's dist depends on both.
- **`/lib/`** — removed from the tree; gitignored. Now repopulated
  by `cyrius deps` from the version-pinned stdlib + git-pinned dep
  snapshots. Matches yukti / patra / agnosys / agnostik convention.
- **`.gitignore`** — adds `/lib/`, `cyrius-*.tar.gz`, `SHA256SUMS`.
- **`tests/tcyr/serde.tcyr`** — rewritten for the 5.10 `#derive(Serialize)`
  2-arg `_to_json(ptr, sb)` form (pre-5.8 was 1-arg returning Str).
  All 39 assertions pass under the new derive codegen.
- **`src/init.cyr`** — fmt pass under cyrius 5.10.34 (re-indent of
  two `str_builder` blocks; no behavioural change).

### Added

- **`src/compat.cyr`** — thin `ct_eq(a, alen, b, blen) → ct_eq_bytes_lens`
  alias. Libro 2.6.2's dist still calls bare `ct_eq` from
  `constant_time_eq_str` (5 call sites: chain verify, integrity
  walk, merkle root, anchor roots), but sigil 3.0.2 retired the
  name in favour of `ct_eq_bytes_lens`. Same signature, same
  constant-time semantics — just renamed. Wired into every
  compilation unit via `[deps.argonaut_compat]` in `cyrius.cyml`.

### Toolchain / CI

- **`.github/workflows/ci.yml`** — versioned toolchain layout
  (`~/.cyrius/versions/<V>/{bin,lib}` + symlinks) so cc5 5.10.9+
  resolves arch-peer includes (`syscalls_x86_64_linux.cyr` etc.);
  fmt check switched from `cyrius fmt --check` (no-op in 5.9.x —
  silent pass on drift) to `diff <(cyrius fmt $f) $f`; dep-hash
  verify is now lockfile-gated (skip with warning when
  `cyrius.lock` doesn't exist yet, enforce once committed).
- **`.github/workflows/release.yml`** — same toolchain + lockfile-
  gate updates as CI. Tag-style + version-verify gates unchanged.

### Stats

- **27 .tcyr suites / 649 assertions** pass under cyrius 5.10.34
  (was 27 / 649 under 5.7.5 — same coverage, regenerated against
  the new derive(Serialize) codegen).
- **Binary `652 KB` → `~990 KB`** (`CYRIUS_DCE=1`). The +338 KB
  jump tracks libro 2.6.2 bringing its full transitive surface
  (agnosys 1.0.4, sigil 3.0.1, sakshi 2.2.3) — under 2.0.5 these
  were stub-shimmed; the dist bundle now self-contains. ~2,262
  unreachable fns NOPed under DCE (was ~1,430).

### Deferred

- **Libro `ct_eq` shim** — drop `src/compat.cyr` once libro
  releases a version that calls `ct_eq_bytes_lens` directly.
  Track upstream; remove the `[deps.argonaut_compat]` self-dep at
  that time.
- **`audit_log_new` name collision** — sigil 3.0.1's dist defines
  `audit_log_new()`; argonaut's `src/audit.cyr:91` shadows it
  (last definition wins, argonaut's wrapper around `chain_new()`
  is what callers get). Benign but noisy at compile time —
  rename argonaut's wrapper (e.g. `argonaut_audit_log_new`) in a
  later minor once kybernet (the consumer) is ready to follow.

## [1.5.0] — 2026-04-27

PID-1 readiness minor — closes the three audit deferrals from
[`docs/audit/2026-04-26-audit.md`](docs/audit/2026-04-26-audit.md)
(MEDIUM-1, MEDIUM-3, LOW-3) with regression coverage. The QEMU
PID-1 boot harness that the audit gated end-to-end M3 / L3
validation on slips to 1.6.0; the syscall-level fixes themselves
ship now so consumers (kybernet, AGNOS boot) can adopt them
ahead of the harness.

### Security

- **[MEDIUM-1]** sd_notify peer-credential check now wired.
  `notify_bind` enables `SO_PASSCRED` on the AF_UNIX/SOCK_DGRAM
  socket; new `notify_try_recv_authenticated(fd, expected_pids)`
  uses `recvmsg` + `SCM_CREDENTIALS` and drops any datagram whose
  kernel-stamped sender PID isn't in the expected set. New
  `init_notify_bind(init, path)` opt-in + `init_notify_fd(init)`
  accessor; `init_poll_health` drains authenticated messages when
  a notify fd is registered. Closes the spoof primitive flagged
  against `READY=1` / `WATCHDOG=1` injection. `pid_in_vec` helper
  in `src/notify.cyr`.
- **[MEDIUM-3]** generic-`waitpid` reaper for orphans landed.
  `argonaut_init_new` now calls
  `prctl(PR_SET_CHILD_SUBREAPER, 1)` (non-fatal — kernels < 3.4
  return EINVAL and we fall back to tracked-PID-only reaping).
  `proc_table_reap_orphans()` drains `waitpid(-1, ..., WNOHANG)`
  in a bounded loop (256-iter guard) and is called from
  `init_reap_services` after the existing tracked reap. Resolves
  the zombie accumulation gap when argonaut graduates from
  systemd-delegate to true PID 1.
- **[LOW-3]** `fork_exec_service` now `setsid`s the child and
  redirects stdout / stderr to `/dev/null` after the existing
  stdin redirect. Service log output stops mixing into argonaut's
  stdio; signals on argonaut's controlling TTY (e.g. `^C` when
  foregrounded for testing) no longer reach service children.
  Production wiring should swap `/dev/null` for a journal fd.

### Added

- **`notify_try_recv_authenticated(fd, expected_pids)`** in
  `src/notify.cyr` — `recvmsg` + `SCM_CREDENTIALS` peer-cred
  variant of `notify_try_recv`.
- **`pid_in_vec(pids, pid)`** in `src/notify.cyr` — linear
  membership test for tiny pid sets.
- **`proc_table_reap_orphans()`** in `src/process_mgmt.cyr` —
  bounded `waitpid(-1, ..., WNOHANG)` drain.
- **`init_notify_bind(init, path)` + `init_notify_fd(init)`** in
  `src/init.cyr` — opt-in sd_notify socket binding for consumers
  that want per-service authenticated `READY=1` / `WATCHDOG=1`
  observation in their poll loop.
- **`notify_fd` field on `ArgonautInit`** (offset 48). The struct
  grows from 48 → 56 bytes; downstream consumers that allocate
  the struct via `argonaut_init_new` pick this up transparently.
- **3 new groups in `tests/tcyr/audit_findings.tcyr`** —
  `audit-m1-notify-cred` (7 assertions, real socketpair +
  PASSCRED + cross-PID drop test), `audit-m3-reaper-orphans` (3
  assertions, fork-and-orphan grandchild collected via subreaper +
  reap_orphans), `audit-l3-fork-setsid` (3 assertions, child
  `getsid(0)` after `syscall(112)` equals own PID — platform
  validation; end-to-end via QEMU in 1.6.0).

### Changed

- **27 test suites / 649 assertions** (was 27 / 637; +12 across
  the M1 / M3 / L3 groups).
- **Binary 650 KB → 652 KB** (`CYRIUS_DCE=1`) for the
  `notify_try_recv_authenticated` recvmsg path, the orphan reaper
  loop, the prctl + setsid + dup2 syscalls, and the
  `init_notify_bind` opt-in surface.

### Deferred to 1.6.0

- **QEMU PID-1 boot harness** — minimal initramfs + kernel boot
  + assertion harness that runs argonaut as PID 1 and validates
  the M3 / L3 integration end-to-end (orphan reap under real
  PID-1 reparenting; service detached from controlling TTY).
  Tracked in [`docs/development/roadmap.md`](docs/development/roadmap.md).
- **HIGH-1 follow-up** — real host resolver for non-loopback
  health checks. The 1.4.0 fix rejects non-loopback targets
  explicitly; the resolver restores the feature surface.

### Verified

- `cyrius lint` clean across `src/*.cyr`, `tests/tcyr/*.tcyr`,
  `tests/bcyr/*.bcyr`.
- `cyrius fmt --check` clean across all sources.
- `cyrius deps --verify` → 2 verified, 0 failed.
- Full test sweep: 27 suites, 649 assertions, 0 failures.
- `CYRIUS_DCE=1 cyrius build` produces 652584-byte ELF;
  `./build/argonaut` exits 0 with the standard sakshi span trace.

### Notes

- The audit-1.4.0 disposition table claimed M1's helper shipped
  unwired in 1.4.0. It hadn't actually landed — 1.5.0 ships both
  the helper and the `init_poll_health` wiring in one go.
- The `lib/process.cyr` `exec_env` Str/cstr quirk noted in
  `tests/tcyr/health_exec.tcyr` (Str pointers passed where the
  kernel wants `char*`) blocks unit-level shell-exec testing of
  the L3 fix path. The audit explicitly gated this on the QEMU
  harness; the 1.5.0 L3 regression test validates the platform
  setsid syscall directly. Filing the `exec_env` cleanup as a
  stdlib upstream issue is on the 1.6.0 plate.

---

## [1.4.0] — 2026-04-26

P(-1) hardening minor — full security audit cycle, CLAUDE.md split into
durable rules + a `docs/development/state.md` volatile snapshot, eight
audit findings landed with regression tests. CVE / 0day landscape
research scoped to 2025–2026 init / service-manager class
(systemd-coredump CVE-2025-4598, snap-confine × systemd-tmpfiles
CVE-2026-3888, systemd-machined CVE-2026-4105, sd_notify spoofing
class). Two HIGH, three MEDIUM (one shipped, two deferred to 1.4.x
patches with helpers in place), four LOW. Full report in
[`docs/audit/2026-04-26-audit.md`](docs/audit/2026-04-26-audit.md).

### Security

- **[HIGH-1]** `src/health.cyr` — `check_tcp_connect` and HTTP_GET
  hardcoded the destination address to `127.0.0.1` while pretending
  to consult the caller-supplied host argument. Health checks against
  any non-loopback target silently probed localhost, so the supervisor
  could mark a service "healthy" based on an unrelated localhost
  responder. Until a resolver lands, both paths now reject any host
  string that isn't `127.0.0.1`, `localhost`, or `::1` — explicit
  failure with a "non-loopback target" message instead of a
  misleading green badge. New helper: `is_localhost_target(host)` in
  `src/security.cyr`.
- **[HIGH-2]** `src/notify.cyr:92` — `sys_mkdir("/run/argonaut", 493)`
  created the notify-socket parent directory mode 0o755, leaving the
  AF_UNIX sd_notify socket reachable by every local user. Combined
  with the absence of a peer-credential check (MEDIUM-1, see below),
  any local process could inject fake `READY=1` / `MAINPID=<pid>`
  notifications. Mode tightened to **0o700** (`sys_mkdir(...,
  448)`). One-line fix; argonaut is the only consumer of
  `/run/argonaut`.
- **[MEDIUM-2]** `src/health.cyr` HTTP_GET — the request `path` was a
  raw pointer into the URL Str's data buffer with no NUL terminator,
  then passed to `str_builder_add_cstr` which reads until NUL. The
  GET line therefore contained undefined trailing bytes whenever the
  URL had a path component. Now heap-allocates a `path_len + 1`
  buffer, `memcpy`s the path bytes, NUL-terminates, and emits the
  copy.
- **[MEDIUM-4]** `src/process_mgmt.cyr` `read_pid_file` — same class
  as CVE-2025-4598's PID-recycling primitive, applied to argonaut's
  pid-file consumption: file content was trusted without verifying
  the file's owner uid or rejecting world-writable mode bits. New
  `read_pid_file_safe(path, expected_uid)` opens via `sys_open` +
  `SYS_FSTAT`, rejects mode `& 0o022` and `st_uid != expected_uid`
  before parsing the integer. The legacy `read_pid_file` now
  delegates with `expected_uid = -1` (skip-check) so existing
  callers keep working; production wiring should pass the service's
  expected runtime uid in a 1.4.x patch.
- **[LOW-1]** `src/edge_boot.cyr` `unlock_luks` — `mapped_name` was
  checked for non-empty only, allowing slashes / control bytes to
  flow through into the `/dev/mapper/<name>` path. Now validated
  against `[a-zA-Z0-9_-]{1,32}` via the new `validate_mapper_name`
  predicate (matches yukti's sysfs-basename rule).
- **[LOW-2]** `src/edge_boot.cyr` `verify_rootfs_integrity` —
  `root_hash` length was checked but its character set wasn't.
  Cryptsetup's positional argv catches this in practice; defense in
  depth says reject early. New `is_hex64(s)` predicate enforces
  exactly 64 chars in `[0-9a-fA-F]`.
- **[LOW-4]** `src/process_mgmt.cyr` `load_env_file` — env files
  larger than the 8 KB read buffer were silently truncated.
  `lseek(SEEK_END)` probe in front of the read fails with a
  distinct return code when the file exceeds 8 KB, so misconfigured
  env files don't ship as silently-half-loaded.
- **[LOW-5]** `src/health.cyr` HTTP response status parser — code
  positions were hardcoded to offsets 9–11, valid only for
  `HTTP/1.x ` prefixes. Now finds the first space and parses the
  three digits after it; any HTTP/x.y prefix length parses
  correctly.

### Deferred to 1.4.x patches (regression in audit, helper unwired)

- **[MEDIUM-1]** sd_notify peer-credential check via `recvmsg` +
  `SCM_CREDENTIALS`. Demoted from HIGH-class once HIGH-2 closed the
  socket-reachability primitive. The protocol-level
  `SO_PEERCRED` / `NotifyAccess=` model is still the right
  defense-in-depth; helper function lands in 1.4.x with the
  `init_poll_health` wiring.
- **[MEDIUM-3]** Generic-`waitpid` reaper for orphans. Required only
  when argonaut runs as actual PID 1 (the AGNOS boot path); dormant
  under the systemd-delegate consumer. Plumbing
  (`prctl(PR_SET_CHILD_SUBREAPER)` enrol + a `waitpid(-1, ...,
  WNOHANG)` sweep) lands together with the QEMU-PID-1 boot test
  harness in 1.4.x.
- **[LOW-3]** `fork_exec_service` `setsid` + stdout/stderr `dup2`.
  Same gating as MEDIUM-3 — needs the QEMU harness to validate
  that decoupling from the controlling TTY doesn't break the
  current test fixtures.

### Added

- **`docs/audit/2026-04-26-audit.md`** — full P(-1) audit report
  (external CVE landscape, 12 findings, disposition table,
  re-audit triggers).
- **`docs/development/state.md`** — live state snapshot (version,
  toolchain, binary size, suites, bench snapshot, dep pins,
  consumers, in-flight). Refreshed every release per the
  CLAUDE.md split.
- **`tests/tcyr/audit_findings.tcyr`** — 30 assertions across 3
  groups (HIGH-1 localhost gate, LOW-1 mapper-name predicate,
  LOW-2 hex64 predicate). Each finding has at least one positive
  + one negative case; HIGH-1 has end-to-end
  `execute_health_check` assertions confirming non-loopback TCP
  and HTTP targets return `passed = 0` with the explicit
  "non-loopback" message.
- **Validation helpers** in `src/security.cyr`: `is_hex64(s)`,
  `validate_mapper_name(name)`, `is_localhost_target(host)`. All
  pure / reusable.
- **`read_pid_file_safe(path, expected_uid)`** in
  `src/process_mgmt.cyr` — the M4 fix; legacy `read_pid_file`
  delegates.

### Changed

- **`CLAUDE.md` rewritten** to the agnosticos
  `example_claude.md` template — durable rules only (project
  identity, goal, key principles, hard constraints, P(-1) /
  work loop / security hardening / closeout process, cyrius
  conventions, CI/release rules, doc tree). Volatile state moved
  to `docs/development/state.md`. Adds the new
  CLAUDE-file-mandated `Rules (Hard Constraints)` and `Cyrius
  Conventions` sections; the `# DO NOT` block from prior
  versions is folded into the new Rules section.
- **`docs/development/state.md` is now the live state snapshot**
  and the source of truth for current version / binary size /
  suite count / bench snapshot / consumer status.
- **27 test suites / 637 assertions** (was 26 / 607; +1 suite
  + 30 assertions for the audit regressions).
- **Binary 641 KB → 650 KB** (CYRIUS_DCE=1) for the validation
  helpers + `read_pid_file_safe` + tightened HTTP path / status
  parsing.

### Verified

- Pre-audit baseline `bench-history.csv` label
  `v1.4.0-pre-audit`; post-audit `v1.4.0-post-audit`. All 29
  benchmarks within noise band; no regressions, small
  improvements on `init_new_desktop` (27 → 26 µs),
  `mark_all_steps_complete` (79 → 76 µs), `plan_shutdown_*`
  (22 → 20 µs).
- `cyrius lint` clean across `src/*.cyr`, `tests/tcyr/*.tcyr`,
  `tests/bcyr/*.bcyr`.
- `cyrius fmt --check` clean across `src/*.cyr`.
- `cyrius deps --verify` → "2 verified, 0 failed".
- Full test sweep: 27 suites, 637 assertions, 0 failures.
- Smoke test: "argonaut: all systems nominal" on
  `./build/argonaut`.

---

## [1.3.0] — 2026-04-26

Toolchain + dep bump release. Cyrius 4.5.0 → 5.7.5, libro 1.0.3 →
2.0.5, manifest format `cyrius.toml` → `cyrius.cyml`. CI/release
workflows refactored to the yukti 5.7-era pattern (toolchain
version derived from `cyrius.cyml`, lockfile-gated `cyrius deps
--verify`, auto-discovered `cyrius lint` / `cyrius fmt --check`,
DCE build, ELF magic check, dual `vX.Y.Z` / `X.Y.Z` release tag
shapes). All 26 test suites / 607 assertions passing on the new
toolchain.

### Changed

- **Toolchain bump 4.5.0 → 5.7.5** (`cyrius.cyml`, was
  `.cyrius-toolchain` + `cyrius.toml`). Compiler renamed cc3 → cc5
  at v5.0.0; argonaut's cc3-era regression tests
  (`cc3_ptr_regression.tcyr`, `cc3_readfile_cap.tcyr`) keep their
  filenames as historical markers.
- **Libro 1.0.3 → 2.0.5**. Argonaut's libro consumers
  (`audit_log_*` wrappers in `src/audit.cyr`) sit above the
  `chain_verify` / `chain_append` API and were API-compatible with
  the 2.0 breaking change set (`verify_chain(entries)` →
  `verify_chain(entries, base_index)` is hidden behind the
  unchanged `chain_verify(log)` wrapper; argonaut's `details`
  values are always strings, so the 2.0 nested-canonical-JSON hash
  change is a no-op for the existing chain). Libro now consumed as
  a single-file dist (`dist/libro.cyr`) instead of 8 vendored
  modules — drops `lib/libro_*.cyr`.
- **Manifest format `cyrius.toml` → `cyrius.cyml`**. Yukti-pattern
  layout: `version = "${file:VERSION}"` so VERSION is the single
  source of truth, `[deps] stdlib = […]` lists 23 stdlib modules
  (added `ct`, `keccak` for libro 2.0's signing/anchoring paths;
  swapped `sakshi_full` → `sakshi`), `[deps.libro]` and
  `[deps.patra]` git stanzas. Patra is a transitive requirement
  of libro 2.0 (PatraStore audit-entry persistence path). Deletes
  `cyrius.toml` and `.cyrius-toolchain`.
- **`cyrius.lock`** introduced — sha256 of `lib/libro.cyr` and
  `lib/patra.cyr`. Generated by `cyrius deps --lock`, enforced by
  `cyrius deps --verify` in CI.
- **CI/release workflows** rewritten on the yukti-2.1.1 pattern.
  Toolchain version pulled from `cyrius.cyml`, `cyrius deps
  --verify` gates dep hashes, auto-discovered `cyrius lint`
  (warnings = fail) and `cyrius fmt --check` (drift = fail) sweep
  `src/` + `tests/tcyr/` + `tests/bcyr/`, build uses
  `CYRIUS_DCE=1`, ELF magic verified, smoke test gate retained.
  Release accepts both `v1.3.0` and `1.3.0` tag shapes; version
  verification trusts `${file:VERSION}` instead of grepping the
  manifest. CHANGELOG slice extracted by the section anchor.
- **Binary size 378 KB → 641 KB** (CYRIUS_DCE=1 build). +263 KB
  for libro 2.0's signing / anchoring / merkle / streaming
  surface + patra dependency. The cc3-era 197 KB → 378 KB jump
  in 1.2.0 was the libro 1.0 audit-chain landing; this jump is
  the libro 2.0 cryptographic feature expansion.
- **`scripts/bench-history.sh`** — `CC=cc3` default replaced with
  `cyrius bench` (cc3 was renamed cc5 at v5.0.0; the script's
  raw-cc invocation no longer matched the 5.x toolchain layout).

### Fixed

- **`tests/tcyr/serde.tcyr` — silent SIGSEGV on `json_build`
  call**. `lib/patra.cyr:2097` defines a 6-arg `json_build(buf,
  max, keys, vals, types, n)` that shadows the stdlib's 1-arg
  `json_build(pairs)`. Cyrius doesn't have function overloading
  and emits no arity warning; the late-bound patra version reads
  garbage for the missing 5 params and returns junk (observed:
  ptr=2). Refactored the test to build the expected `{"k1":"v1",
  …}` JSON object via `str_builder` directly. Production code
  was not affected — `json_build/1` is only called from this
  test. Patra-side rename or namespacing should be filed
  upstream.
- **`tests/tcyr/modules_c.tcyr:16`** — single 121-character line
  flagged by `cyrlint`; wrapped to two lines. Lint sweep now
  clean across all `src/*.cyr` + `tests/{tcyr,bcyr}/`.

### Removed

- **`lib/libro_*.cyr`** — 8 vendored libro 1.0.3 modules
  superseded by `lib/libro.cyr` from libro 2.0.5's `dist/`.
- **`cyrius.toml`**, **`.cyrius-toolchain`** — replaced by
  `cyrius.cyml`.
- **`lib/sakshi_full.cyr`** — replaced by `lib/sakshi.cyr`
  (cyrius 5.x stdlib module name).

### Verified

- `cyrius build src/main.cyr build/argonaut` clean (CYRIUS_DCE=1
  → 641 KB ELF, smoke test "all systems nominal").
- 26 test suites / 607 assertions, 0 failed.
- `cyrius lint` clean across `src/*.cyr`, `tests/tcyr/*.tcyr`,
  `tests/bcyr/*.bcyr`.
- `cyrius fmt --check` clean across `src/*.cyr` (no drift).
- `cyrius deps --verify` → "2 verified, 0 failed" against
  `cyrius.lock`.
- One known upstream warning passes through:
  `lib/syscalls_x86_64_linux.cyr:358: syscall arity mismatch` —
  benign, documented in cyrius v5.4.20 changelog as low-priority
  cleanup; resolves automatically when upstream lands the fix.

---

## [1.2.0] — 2026-04-13

### Added

#### Libro 1.0.2 Integration — Cryptographic Audit Chain
- **libro 1.0.2** replaces FNV-1a audit shim with SHA-256 hash-linked chain
- 8 libro modules: error, hasher, entry, verify, query, retention, chain, export
- Dependencies: sigil (SHA-256), bigint, chrono via `cyrius deps`
- `audit_log_new()` creates a libro `AuditChain` with UUID, RFC 3339 timestamps, SHA-256 hashing
- `audit_log_verify()` performs cryptographic chain integrity verification
- `audit_log_query_full(log, source, min_sev, agent, after, before)` — full QueryFilter with time range and agent_id
- `audit_log_record_with_agent(log, service, event_type, agent)` — agent-attributed events
- `audit_log_export_jsonl(log, fd)` / `audit_log_export_csv(log, fd)` — audit trail export
- `audit_entry_agent_id(e)` accessor

#### Lifecycle Audit Recording
- `ArgonautInit` carries an `audit_log` field (libro AuditChain)
- `init_audit_log(init)` accessor
- `init_start_service` records EVT_STARTING, EVT_STARTED / EVT_STOPPED_FAIL / EVT_READY_PASSED / EVT_READY_FAILED
- `init_stop_service` records EVT_STOPPING, EVT_STOPPED_OK / EVT_STOPPED_FAIL
- `init_restart_service` records EVT_RESTARTING
- `init_reap_services` records EVT_STOPPED_OK / EVT_CRASH_DETECTED
- `init_enforce_watchdog` records EVT_TIMEOUT_KILLED
- `init_poll_health` records EVT_HEALTH_PASSED / EVT_HEALTH_FAILED

#### P(-1) Scaffold Hardening
- `health_check_type_str(t)` — human-readable strings for HealthCheckType
- `ReapResult` struct for `init_reap_services` return values
- cc3 regression tests: `cc3_ptr_regression.tcyr`, `cc3_readfile_cap.tcyr`
- `audit_lifecycle.tcyr` — 17 assertions for lifecycle audit recording

### Fixed
- **Security**: non-http:// URLs rejected in HTTP health checks (was: silent port corruption)
- **Security**: `verify_emergency_auth` uses `constant_time_eq_str` (was: `str_eq` timing oracle)
- **Security**: `password_hash` upgraded from FNV-1a to SHA-256 via sigil
- **Correctness**: `execute_ready_check` initializes `timeout_ms` field (was: uninitialized heap)
- **Correctness**: zombie prevention — `sys_waitpid` after SIGKILL on ready-check failure
- **Correctness**: `generate_unit` emits correct systemd `Type=` per service type (was: always `Type=notify`)
- **Correctness**: `HealthCheckResult.check_type_str` set from actual check type (was: placeholder)
- **Correctness**: `resolve_service_order` / `resolve_service_waves` — external deps get `in_degree` entries; cycle detection uses `map_count(in_degree)`. Fixes false cycle detection when depending on unregistered services.
- **Correctness**: TCP health check verifies `SO_ERROR` via `getsockopt` (was: false positive on ECONNREFUSED)
- **Memory**: `fleet_registration_from_system` uses `str_clone` for stack-buffered values (was: dangling stack pointers)
- **Memory**: `notify_try_recv` uses static buffer (was: 1KB alloc per poll tick, never freed)

### Changed
- **audit.cyr**: 328-line FNV-1a shim → libro bridge (51% smaller)
- **Include order**: audit.cyr before init.cyr (init depends on audit)
- **Dependencies via `cyrius deps`** — `cyrius.toml` declares `[deps]` stdlib and `[deps.libro]`; no more manual vendoring
- **Include paths**: `lib/libro/X.cyr` → `lib/X.cyr` (flat layout from `cyrius deps`)
- **Binary size**: 197KB → 378KB (+181KB for libro + sigil SHA-256 + bigint)
- **Minimum toolchain**: cyrius 3.9.8 (`.cyrius-toolchain` file)
- CI/release workflows: `cyrius deps` + `cyrius build` + `cyrius test`, version from `.cyrius-toolchain`
- `cyrfmt` and `cyrlint` clean — zero warnings, zero format issues
- All documentation updated from Rust to Cyrius (README, ADRs, guides, quickstart, security, contributing)
- `scripts/bench-history.sh` rewritten for Cyrius
- `lib/fnptr.cyr` added to include chain (suppresses `fncall2` warning from hashmap)
- Test suites: 26 (607 assertions)

### Removed
- **FNV-1a hash** in audit.cyr and security.cyr — replaced by SHA-256
- **`lib/libro/`** directory — libro modules now resolved via `cyrius deps`
- **`lib/patra.cyr`** — unused vendored copy

---

## [1.0.0] — 2026-04-12

Argonaut 1.0.0 — init system and service manager for AGNOS, written in Cyrius.

All pre-1.0 features complete: boot sequencing, service lifecycle (simple/forking/oneshot), dependency resolution (Kahn's algorithm), health checks (HTTP/TCP/command/process-alive), watchdog enforcement, shutdown orchestration, runlevel switching, edge boot (dm-verity/LUKS/read-only rootfs), security enforcement (seccomp/Landlock/capabilities), sd_notify protocol, systemd unit generation, tmpfiles setup, API response builders, and cryptographic audit trail via libro.

---

## [0.96.1] — 2026-04-11

### Added
- API response builders: `init_list_services`, `init_system_status`, `init_system_metrics`, `init_boot_log`
- Boot execution plans: `init_boot_execution_plan`, `init_boot_execution_plan_waves`
- `safe_cmd_display()`, `to_prlimit_commands()`
- HTTP health check upgraded to full HTTP/1.x GET with status line parsing
- `sakshi_full.cyr` v0.7.0 — real span stack, ring buffer, UDP output
- Full sakshi tracing across all lifecycle events
- 8 new test suites (184 assertions), 8 new benchmarks (37 total)

### Changed
- Cyrius toolchain: cc2 → cc3, minimum version 3.4.0
- Binary size: 213KB → 197KB (heap buffers + sakshi_full)
- Test suites: 15 → 23 (579 assertions)

### Removed
- **`rust-old/`** — original Rust source (13,577 lines). All ported to Cyrius.

---

## [0.96.0] — 2026-04-08

### Added
- Sakshi integration — structured tracing via sakshi 0.5.0

### Changed
- Binary size: 207KB → 213KB (+6KB for sakshi)
- Test suites: 12 → 15 (395 assertions)

---

## [0.95.0] — 2026-04-08

### Added
- Full rewrite from Rust (13,577 lines) to Cyrius (6,124 lines — 2.2x compression)
- 13 source modules, 207KB statically linked ELF x86_64 binary
- `audit.cyr` — libro-compatible API shim (FNV-1a, replaced in 1.2.0)
- Edge boot: `parse_meminfo_total_mb()`, memory validation, fleet registration
- 12 test suites (395 assertions), 29 benchmarks
- CI/CD workflows for Cyrius toolchain

### Fixed
- Unknown service type transitions to STATE_FAILED (was stuck in STATE_STARTING)
- `/proc/meminfo` parsing implemented (was stub)
- `fleet_registration_from_system` reads real memory (was hardcoded 0)

---

## [0.90.0] — 2026-04-02

### Added
- Initial scaffold: types, boot sequences, service definitions, dependency resolution
- Boot modes: Server, Desktop, Minimal, Edge, Recovery
- Service management: registration, state machine, dependency-aware ordering (Kahn's)
- Shutdown planning: ordered steps with wall message, service stops, filesystem sync
- Runlevel system: Emergency, Rescue, Console, Graphical, Container, Edge
- Edge boot: read-only rootfs, dm-verity verification
- Health checks: HTTP GET, TCP connect, command, process-alive
- Emergency shell, crash action determination, safe command abstraction
- 148 tests

---

## Pre-0.90 (Rust era)

Features implemented in the original Rust codebase (v0.2.0–v0.9.0) and ported to Cyrius at v0.95.0. See `docs/benchmarks-rust-baseline.md` for Rust performance comparison. The Rust source was removed at v0.96.1.

Key milestones: v0.2.0 (hardening, `forbid(unsafe_code)`), v0.3.0 (process execution, ProcessTable), v0.4.0 (health check execution), v0.5.0 (runlevel switching), v0.6.0 (edge boot execution), v0.7.0 (API, audit, systemd integration), v0.8.0 (service types, resource limits, log rotation), v0.9.0 (seccomp, Landlock, capabilities, tmpfiles).
