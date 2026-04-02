# Feature Gap Analysis

Based on research into s6, dinit, systemd, runit, OpenRC, and other production init systems.

## P0 — Required Before Production

| Feature | Why | Status |
|---------|-----|--------|
| Zombie reaping (SIGCHLD) | PID 1 MUST reap all children, not just tracked services. Orphans become zombies. | Not implemented (library limitation) |
| Signal forwarding | PID 1 receives all signals. Must forward SIGTERM/SIGINT/SIGHUP to services. | Not implemented |
| Parallel service startup | Independent services should start concurrently. Toposort gives the data; need wave-based executor. | Not implemented (sequential only) |
| Cgroup-per-service | Clean process killing (kill cgroup, not PID), resource accounting, OOM priority. | Not implemented |
| Resource limits (rlimits) | RLIMIT_NOFILE, RLIMIT_AS, RLIMIT_NPROC per service. | Not implemented |
| Privilege drop (uid/gid) | Services must run as non-root. Currently `bail!()` on uid/gid. | Blocked by `forbid(unsafe_code)` — deferred to binary crate |
| Forking service type | PostgreSQL and other daemons fork; parent exits. Need to track child PID via sd_notify or PID file. | Not implemented |

## P1 — Should Have

| Feature | Why | Status |
|---------|-----|--------|
| Socket activation | Zero-downtime restarts, on-demand start, breaks circular deps. Pass fd 3+ via LISTEN_FDS/LISTEN_PID. | Not implemented |
| sd_notify WATCHDOG=1 | Many daemons send WATCHDOG=1 keepalive. Without handling this, watchdog-reliant services fail. | Not implemented |
| sd_notify credential verification | SO_PASSCRED prevents spoofed READY=1 from arbitrary processes. | Not implemented |
| Log rotation | Current append-only log files will fill disks. Need rotation or ring buffer. | Not implemented |
| Environment file loading | `/etc/argonaut/env.d/<service>` — every production init supports this. | Not implemented |
| tmpfiles.d equivalent | Create dirs, symlinks, device nodes at boot. Critical for `/run`, `/tmp`. | Not implemented |
| Seccomp/Landlock per service | Boot stage exists but no implementation. Drop capabilities, apply filters. | Not implemented |

## P2 — Nice to Have

| Feature | Why |
|---------|-----|
| D-Bus interface | Desktop components query service status via D-Bus. Only if AGNOS needs it. |
| Timer-based services | Cron equivalent — OnCalendar/OnBootSec triggers. |
| On-demand lazy start | Start service on first socket connection. Depends on socket activation. |
| Transient units | One-off commands in cgroups with resource limits (like `systemd-run`). |
| SBOM generation | Software Bill of Materials for supply chain compliance. |
| Reproducible builds | SLSA level 3+ provenance. |

## Research-Driven Fixes Needed

### dm-verity
- Remove redundant `veritysetup verify` from boot path (doubles I/O — `open` already verifies on read)
- Add `--restart-on-corruption` flag for edge mode
- Make verity failure fatal in edge mode (currently records error but continues)
- Consider FEC support (`--fec-device`) for unreliable edge storage

### LUKS2 + TPM2
- Add `--token-id=0` and `--tries=1` for deterministic TPM2 unlock
- Default `tpm_attestation` to `true` for edge mode
- Add PCR binding configuration to EdgeBootConfig

### sd_notify
- Implement WATCHDOG=1 keepalive handling
- Add SO_PASSCRED credential verification
- Support abstract socket namespace
- Support RELOADING=1 and STOPPING=1 lifecycle fields

## Architecture Recommendation

The research confirms the standard pattern for modern init systems:

```
PID 1 binary (tiny, may use unsafe):
  mount /proc, /sys, /dev, /run
  set up signalfd for SIGCHLD + SIGTERM + SIGPWR
  exec argonaut-manager

argonaut-manager (uses argonaut library):
  epoll event loop: signalfd, timerfd, notify socket, control socket
  parallel service startup from dependency graph
  cgroup-per-service
  socket activation
```

This two-process split means a bug in the service manager doesn't kernel-panic the system. s6, dinit, and systemd all use this pattern.
