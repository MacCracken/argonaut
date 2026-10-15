# Security Policy

## Scope

Argonaut is AGNOS's init-system and service-manager library, plus a minimal PID-1
binary. Its jobs are:
- service lifecycle;
- boot sequencing;
- health checks;
- shutdown;
- edge boot;
- a hash-linked audit chain.

It runs inside PID 1 (its own binary, or kybernet's) with full system privileges.

It is written in Cyrius and built with `cyrius build` into a statically linked ELF, with
no libc. At runtime it executes a few system tools: `mount`, `veritysetup` and
`cryptsetup` on the edge-boot path, plus any command health check a service configures.
Current binary size and dependency pins are in
[`docs/development/state.md`](docs/development/state.md).

**Service sandboxing is not argonaut's.** argonaut models per-service seccomp, Landlock
and capability policy, but does not apply it.
- `fork_exec_service` hands each service definition to a consumer-registered **pre-exec
  hook** in the child, just before `execve`.
- A hook that fails aborts the service with exit 126 (fail closed).
- kybernet registers that hook and enforces the policy. With no hook, a service runs with
  PID 1's privileges.

## Attack Surface

| Area | Risk | Mitigation |
|------|------|------------|
| Service execution | Command injection via service definitions | SafeCommand: explicit argv, `execve` directly, no shell |
| Service confinement | A service runs with more privilege than its policy allows | Not enforced by argonaut. The fail-closed pre-exec hook is where the consumer (kybernet) confines it |
| Process management | Signalling the wrong process | Never signals a pid it does not own as a child, or pid ≤ 1. Timed children run in their own process group. PID files are read strictly (no symlinks or FIFOs; numeric only) |
| sd_notify socket | Spoofed `READY` / `MAINPID` / `WATCHDOG` from an untrusted local sender | `SO_PASSCRED` sender check against the service's pids. Truncated datagrams dropped. Passed descriptors closed. At most 32 datagrams per tick |
| Emergency shell | Unauthorized root shell | Authentication required by default. Credential: salted Argon2id (v1 record, bounded parameters, private `mmap` arena). The legacy unsalted SHA-256 digest is still accepted. Constant-time comparison |
| systemd unit generation | Directive injection through names, arguments or environment | Escaping per setting: `%` doubled everywhere; `$` doubled on Exec lines; `"` and `\` escaped in `Environment=`; a lone `;` written as `\;`; newlines folded. Keys and dependency names validated. A non-absolute binary is refused |
| Edge boot (dm-verity) | Rootfs integrity bypass | Device-path validation; root hash must be 64 hex digits |
| Edge boot (LUKS) | Key release to the wrong system | Unlock is `--token-only --tries 1`, key handling is cryptsetup's, and mapper names are validated. The TPM key-release policy belongs to the enroller: [architecture note 005](docs/architecture/005-edge-boot-tpm-key-policy.md) |
| tmpfiles | Symlink or traversal attacks while running as root | Absolute paths only, no `..`. `tmpfiles_apply` walks every component with `O_NOFOLLOW` and creates entries with `*at` syscalls. Modes are rendered in octal |
| HTTP / TCP health checks | Request splitting; hangs | `http://` only; control bytes in the path refused; ports range-checked; every check has a deadline |
| Audit chain | Tampering | [libro](https://github.com/MacCracken/libro) SHA-256 hash-linked entries. Optional persistence, signed snapshots and merkle proofs |

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.16.x  | Yes |
| 1.15.x  | Security fixes only (kybernet still pins 1.15.4) |
| < 1.15  | No |

## Reporting a Vulnerability

Please report security issues to **security@agnos.dev**.

- You will receive acknowledgement within 48 hours.
- We follow a 90-day coordinated disclosure timeline.
- Please do not open public issues for security vulnerabilities.

Audit reports are in [`docs/audit/`](docs/audit/). Each release's fixes are in the
Security section of [CHANGELOG.md](CHANGELOG.md).

## Design Principles

- SafeCommand for all process execution: no shell interpolation.
- Constant-time comparisons for secrets (emergency auth).
- No secret material in log output.
- Validation before use for every external input: device paths, mapper names, PID files,
  notify datagrams, ports, URLs.
- Fail closed: a failed pre-exec hook, an unusable credential or a failed Argon2 mapping
  denies.
- PID 1 reaps every child, tracked or orphaned.
- Every lifecycle decision is audited, including a restart refused at the limit.
