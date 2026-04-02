# Argonaut Roadmap

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).
Feature gap analysis: [docs/architecture/feature-gaps.md](../architecture/feature-gaps.md).

---

## v0.7.0 — Research-Driven Hardening

Fixes identified by external research into init system best practices, protocol specs, and security standards.

### dm-verity / LUKS corrections
- [ ] Remove redundant `veritysetup verify` from boot path (doubles I/O — `open` verifies on read)
- [ ] Add `--restart-on-corruption` to `veritysetup open` for edge mode
- [ ] Make dm-verity failure fatal in edge mode (currently continues with errors)
- [ ] Add `--token-id=0` and `--tries=1` to LUKS unlock for deterministic TPM2
- [ ] Default `tpm_attestation` to `true` for edge mode
- [ ] Add PCR binding configuration to `EdgeBootConfig`

### sd_notify protocol completion
- [ ] Handle `WATCHDOG=1` keepalive messages (reset service watchdog timer)
- [ ] `SO_PASSCRED` / `SCM_CREDENTIALS` for sender verification
- [ ] Support `RELOADING=1` and `STOPPING=1` lifecycle fields
- [ ] Drain limit on `NotifyListener::drain()` to prevent DoS

### Integration items
- [ ] systemd unit generation (for hybrid installs)
- [ ] agnoshi commands: `service start/stop/restart/status/enable/disable`
- [ ] MCP tools: `argonaut_services`, `argonaut_status`, `argonaut_boot_log`
- [ ] Audit logging via libro (service state transitions)
- [ ] daimon API: `/v1/services` CRUD endpoints backed by argonaut
- [ ] Integration with nazar (expose `/v1/services` metrics endpoint)

---

## v0.8.0 — Production Init Features (P0 gaps)

Items required before argonaut can be trusted as PID 1. These are implemented in the binary crate but the library must provide the primitives.

- [ ] Parallel service startup (wave-based executor from toposort)
- [ ] Forking service type support (track child PID via sd_notify MAINPID or PID file)
- [ ] Resource limits per service (RLIMIT_NOFILE, RLIMIT_AS, RLIMIT_NPROC fields on ProcessSpec)
- [ ] Environment file loading (`/etc/argonaut/env.d/<service>`)
- [ ] Log rotation (size-capped or time-rotated service logs)
- [ ] Oneshot service type (run-to-completion, no supervision)

---

## v0.9.0 — Security Enforcement (P1 gaps)

- [ ] Socket activation (LISTEN_FDS / LISTEN_PID protocol)
- [ ] Seccomp filter generation and application per service
- [ ] Landlock filesystem restrictions per service
- [ ] Capability bounding set management
- [ ] tmpfiles.d equivalent (directory/symlink/device creation at boot)
- [ ] Emergency shell authentication (`require_auth` enforcement)
- [ ] Core dump restriction (RLIMIT_CORE = 0 by default)

---

## v1.0.0 Criteria

- [ ] All boot modes tested on real hardware (QEMU, RPi4, NUC)
- [ ] Boot time < 3s (Desktop), < 1s (Edge)
- [ ] Crash recovery tested: kill every service, verify auto-restart
- [ ] Shutdown ordering tested: no orphan processes after halt
- [ ] API stable — no breaking changes to public types
- [ ] 80%+ code coverage
- [ ] Benchmark history proves no regressions
- [ ] All P0 gaps closed
- [ ] Security posture documented and reviewed
- [ ] ADRs for all major design decisions

---

## Binary Crate (argonaut-init, separate repo)

Items that require `unsafe` and run as actual PID 1:

- [ ] Zombie reaping via `signalfd` + `waitpid(-1, WNOHANG)` loop
- [ ] Signal forwarding (SIGTERM, SIGINT, SIGHUP, SIGPWR)
- [ ] Cgroup v2 per-service setup (`/sys/fs/cgroup/argonaut.slice/<service>/`)
- [ ] Privilege drop (`setuid`/`setgid`/`setgroups` in `Command::pre_exec`)
- [ ] Essential filesystem mounting (`/proc`, `/sys`, `/dev`, `/run`, cgroups)
- [ ] `epoll` event loop (signalfd, timerfd, notify socket, control socket)
- [ ] Console I/O setup (`/dev/console`, `/dev/null`)

---

## Non-goals

- **Container orchestration** — that's stiva
- **Package installation** — that's ark
- **Agent lifecycle** — that's daimon (argonaut manages the process, daimon manages the agent)
- **Scheduling** — that's samay (argonaut starts/stops, samay decides when)
- **D-Bus interface** — only if AGNOS desktop requires it (P2)
- **Timer-based services** — that's samay's domain
