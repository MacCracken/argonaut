# Argonaut Security Posture

## Overview

Argonaut runs inside PID 1 (its own minimal binary, or kybernet) with root privileges.
Compromising the init process compromises the whole system. This document describes the
security measures argonaut actually implements, what it leaves to its consumer, and the
known gaps.

Findings and fixes are in [`docs/audit/`](../audit/) and in the Security sections of
the CHANGELOG.

**Two facts frame everything below:**

- **argonaut does not confine services.** It models seccomp / Landlock / capability
  policy but does not apply it. `fork_exec_service` passes each service definition to a
  consumer-registered pre-exec hook, in the child just before `execve`. A hook that fails
  aborts the service with exit 126 (fail closed). kybernet's hook applies the policy.
  **With no hook, a service runs with PID 1's full root privileges**
  ([ADR-013](../adrs/013-policy-enforcement-in-the-consumer.md)).
- **argonaut makes syscalls itself:** process, socket, signalfd, mount and `*at` calls.
  Each goes through a stdlib `sys_*` wrapper or an arch-dispatched `ag_sys_*` in
  `src/syscall_compat.cyr`, never a literal number.

## Defense Layers

### 1. Execution and injection

| Measure | Detail |
|---------|--------|
| SafeCommand | Every external command is an explicit `{binary, args}` executed by `run_safe_cmd_timeout`. It resolves the binary against a fixed PATH, puts the child in its own process group, calls `execve` directly with no shell, and waits against a deadline, killing the whole group on timeout |
| systemd unit generation | Escaping per setting: `%` doubled everywhere; `$` doubled on Exec lines; `"` and `\` escaped inside `Environment=`; a lone `;` written as `\;`; newlines folded. Keys and dependency names are validated, not escaped. A non-absolute binary is refused |
| Device paths | `validate_device_path` requires `/dev/`, has no `..`, and allows a character whitelist only |
| dm-verity / LUKS parameters | The root hash must be 64 hex digits (`is_hex64`). Mapper names are validated (`validate_mapper_name`; no leading `-`), and `--` precedes positional arguments |
| tmpfiles | Entries must be absolute with no `..` (`validate_tmpfile_entries`). `tmpfiles_apply` walks every path component with `openat(O_PATH|O_DIRECTORY|O_NOFOLLOW)` and creates with `mkdirat` / `symlinkat` / `mknodat`. It sets owner and mode on an `O_NOFOLLOW` fd, and never follows a symlink. Modes are rendered in octal |
| HTTP health checks | `http://` only; control bytes in the path refused; ports range-checked and digits-only; the `Host:` header sanitised |

### 2. Boot integrity (edge mode)

| Measure | Detail |
|---------|--------|
| dm-verity | `veritysetup open` with `--restart-on-corruption`. A verity failure stops the edge boot before LUKS |
| Read-only rootfs | The root is remounted read-only, and writable state goes on tmpfs overlays with size limits and explicit modes. `/tmp` and `/var/tmp` are `noexec,nosuid,nodev` (sticky); `/var/run` and `/var/log` are `nosuid,nodev,mode=0755` |
| LUKS2 | `cryptsetup open --type luks2 --token-only --token-id 0 --tries 1 --`: the key comes only from the volume's token, with one attempt and no passphrase fallback |
| TPM key-release policy | **Not argonaut's.** Which boot states the TPM releases the key to is set at enrolment, which kybernet owns. `pcr_bindings` is a read-out set, not a sealing policy. See [architecture note 005](../architecture/005-edge-boot-tpm-key-policy.md) |
| Boot budget | `EdgeBootConfig.max_boot_ms`; the result reports `within_budget` |

### 3. Process supervision

| Measure | Detail |
|---------|--------|
| Signalling the right process | argonaut never signals a pid that is not its child, or any pid ≤ 1. Timed children run in their own process group, and a timeout kills the group. The window between reading a pid and signalling it is still open; pidfd identity is roadmap 1.18.0 |
| PID files | Read strictly: no symlinks, FIFOs or loose parsing; numeric only (`read_pid_file_safe`) |
| Start and stop bounds | Oneshot and forking starts have a 90 s default timeout. Stops escalate from SIGTERM to a process-group SIGKILL |
| Restarts | Exponential backoff with a 100 ms floor, inside a restart window. `max_restarts` gives up, and the give-up is audited (`EVT_RESTART_LIMIT`) |
| Watchdog | An authenticated `WATCHDOG=1` refreshes the deadline from `watchdog_ms`; a service that misses it is killed and audited |
| Ordered shutdown | `init_execute_shutdown` stops services in reverse dependency waves within one budget. Every service ends STOPPED and audited |
| Orphans | PR_SET_CHILD_SUBREAPER plus an orphan sweep; PID 1 reaps every child |

### 4. sd_notify

| Measure | Detail |
|---------|--------|
| Socket | `SOCK_CLOEXEC`, with `SO_PASSCRED` required (`notify_bind`) |
| Sender authentication | `notify_try_recv_authenticated` accepts a datagram only from one of the service's pids |
| Malformed input | Truncated datagrams (`MSG_TRUNC` / `MSG_CTRUNC`) are dropped. `MAINPID` is strictly parsed |
| Passed descriptors | Any `SCM_RIGHTS` descriptor is closed before the verdict (1.16.2) |
| Flooding | At most 32 datagrams per supervisor tick |
| Deprecated paths | `notify_try_recv` / `notify_drain` skip authentication. They are deprecated, with removal at 1.17.0 |

### 5. Emergency shell

| Measure | Detail |
|---------|--------|
| Default | `emergency_shell_default()` requires authentication and has no credential, so it denies until one is configured |
| Credential | A v1 Argon2id record: `v1$t$m$p$salt$tag`, with kybernet's bounds rejected rather than clamped. The Argon2 arena is a private `mmap`, unmapped before the verdict |
| Legacy | A 64-lowercase-hex unsalted SHA-256 digest is still accepted, for kybernet's migration. Retiring it is gated on kybernet (roadmap) |
| Comparison | Constant time. A null config or password denies |

### 6. Audit trail

| Measure | Detail |
|---------|--------|
| Hash-linked chain | Every lifecycle event goes into a [libro](https://github.com/MacCracken/libro) SHA-256 chain. It streams by default (hashed, not retained). Retention, persistence (patra), signed snapshots and merkle proofs are opt-in |
| Event types | `ServiceEventType` in `src/audit.cyr`: enable / disable, start / started, stop OK / fail, restart, crash, timeout kill, health pass / fail (on change plus an hourly heartbeat), restart limit, and others |
| Tracing | sakshi structured tracing |

### 7. Supply chain

| Measure | Detail |
|---------|--------|
| Dependencies | libro and sigil are git dependencies pinned by tag; patra and sakshi come from the stdlib fold. `cyrius.lock` records a SHA-256 for every resolved file, and CI runs `cyrius deps --verify` |
| No package registry | All dependencies are AGNOS-maintained |
| Release artifacts | Source tarball, binaries, `SHA256SUMS` and `cyrius.lock`. No provenance attestation (see gaps) |

## CWE Coverage

| CWE | Name | Mitigation |
|-----|------|------------|
| CWE-78 | OS Command Injection | SafeCommand: explicit argv, `execve`, no shell |
| CWE-93 / CWE-74 | Injection into systemd units | Per-setting escaping; validated keys and dependency names |
| CWE-22 | Path Traversal | `validate_device_path`, `validate_tmpfile_entries`, absolute-binary check in `generate_unit` |
| CWE-59 | Link Following | `tmpfiles_apply` (`O_NOFOLLOW` walk, `*at` creation); strict PID-file reader |
| CWE-20 | Improper Input Validation | Hash, device-path and mapper-name validation; strict PID-file and `MAINPID` parsing; port and URL checks; credential-record bounds |
| CWE-190 | Integer Overflow | Saturating backoff; Argon2 `m` bounded during digit accumulation |
| CWE-400 | Uncontrolled Resource Consumption | Restart window and limits, backoff floor, health deadlines, watchdog kills, notify drain cap, streaming audit chain |
| CWE-362 | TOCTOU | `*at` / `O_NOFOLLOW` tmpfiles. The pid-recycling window is open (pidfd is roadmap 1.18.0) |
| CWE-269 / CWE-250 | Privilege management | **Consumer**: the fail-closed pre-exec hook (ADR-013). argonaut drops nothing itself |
| CWE-284 | Access control | Emergency-shell authentication (Argon2id, required by default); notify sender authentication |
| CWE-835 | Infinite Loop / hang | Deadline-bounded health checks, starts and stops; SIGKILL escalation |
| CWE-696 | Incorrect Behavior Order | Kahn's algorithm with cycle detection; ordered shutdown |
| CWE-829 | Untrusted Control Sphere | Tag-pinned, hash-locked AGNOS dependencies |

## Known Gaps

| Gap | Severity | Status |
|-----|----------|--------|
| No confinement without a hook | High (by design) | A consumer that does not register the pre-exec hook runs services as root. kybernet registers it |
| Health-check commands bypass the hook | Medium | A confined service's command check runs unconfined (roadmap 1.18.0) |
| PID reuse between read and signal | Low | pidfd identity is roadmap 1.18.0 |
| Resource limits and log rotation | Low | Modelled only; not applied |
| Legacy emergency credential | Low | Unsalted SHA-256 is still accepted beside Argon2id; retirement is gated on kybernet |
| SBOM / provenance | Low | No SBOM or SLSA provenance attestation |

## Module Map

| Capability | Module |
|------------|--------|
| Service lifecycle, health, watchdog, shutdown | `src/init.cyr` |
| Health checks | `src/health.cyr`, `src/resolver.cyr` |
| Boot sequencing | `src/boot.cyr`, `src/services.cyr` |
| Process execution, SafeCommand runner, pre-exec hook | `src/process_mgmt.cyr` (SafeCommand type in `src/types.cyr`) |
| Edge boot | `src/edge_boot.cyr` |
| sd_notify | `src/notify.cyr` |
| Policy modelling, emergency credential, validators | `src/security.cyr` |
| systemd units | `src/systemd.cyr` |
| tmpfiles | `src/tmpfiles.cyr` |
| Audit chain, persistence, signing | `src/audit.cyr`, `src/audit_ext.cyr` |
| Arch-dispatched syscalls | `src/syscall_compat.cyr` |
