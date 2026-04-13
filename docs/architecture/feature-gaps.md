# Feature Gap Analysis

Based on research into s6, dinit, systemd, runit, OpenRC, and other production init systems.

## P0 — Required Before Production

| Feature | Why | Status |
|---------|-----|--------|
| Zombie reaping (SIGCHLD) | PID 1 MUST reap all children, not just tracked services. Orphans become zombies. | Deferred to kybernet (PID 1 binary) — **kybernet 1.0.1 depends on argonaut 1.2.0** |
| Signal forwarding | PID 1 receives all signals. Must forward SIGTERM/SIGINT/SIGHUP to services. | Deferred to kybernet (PID 1 binary) |
| Parallel service startup | Independent services should start concurrently. Toposort gives the data; need wave-based executor. | **Implemented** (v0.8.0) — `resolve_service_waves`, `boot_execution_plan_waves` |
| Cgroup-per-service | Clean process killing (kill cgroup, not PID), resource accounting, OOM priority. | Deferred to kybernet (PID 1 binary) |
| Resource limits (rlimits) | RLIMIT_NOFILE, RLIMIT_AS, RLIMIT_NPROC, RLIMIT_CORE per service. | **Implemented** (v0.8.0) — `ResourceLimits` + prlimit commands |
| Privilege drop (uid/gid) | Services must run as non-root. Library errors on uid/gid set. | Deferred to kybernet (PID 1 binary) — requires pre-exec hook at OS level |
| Forking service type | PostgreSQL and other daemons fork; parent exits. Need to track child PID via sd_notify or PID file. | **Implemented** (v0.8.0) — `ServiceType::Forking`, `read_pid_file` |

## P1 — Should Have

| Feature | Why | Status |
|---------|-----|--------|
| Socket activation | Zero-downtime restarts, on-demand start, breaks circular deps. Pass fd 3+ via LISTEN_FDS/LISTEN_PID. | **Implemented** (v0.9.0) — `SocketActivationConfig`, LISTEN_FDS env |
| sd_notify WATCHDOG=1 | Many daemons send WATCHDOG=1 keepalive. Without handling this, watchdog-reliant services fail. | **Implemented** (v0.7.0) — `NotifyMessage.watchdog` |
| sd_notify credential verification | SO_PASSCRED prevents spoofed READY=1 from arbitrary processes. | **Implemented** (v0.7.0) — `enable_credentials()` |
| Log rotation | Current append-only log files will fill disks. Need rotation or ring buffer. | **Implemented** (v0.8.0) — `LogConfig`, size-based rotation |
| Environment file loading | `/etc/argonaut/env.d/<service>` — every production init supports this. | **Implemented** (v0.8.0) — `load_environment_file`, implicit env.d |
| tmpfiles.d equivalent | Create dirs, symlinks, device nodes at boot. Critical for `/run`, `/tmp`. | **Implemented** (v0.9.0) — `TmpfileEntry`, `generate_tmpfile_commands` |
| Seccomp/Landlock per service | Boot stage exists but no implementation. Drop capabilities, apply filters. | **Implemented** (v0.9.0) — `SeccompConfig`, `LandlockConfig`, `CapabilityConfig` + agnosys integration |

## P2 — Nice to Have

| Feature | Why |
|---------|-----|
| D-Bus interface | Desktop components query service status via D-Bus. Only if AGNOS needs it. |
| Timer-based services | Cron equivalent — OnCalendar/OnBootSec triggers. |
| On-demand lazy start | Start service on first socket connection. Depends on socket activation. |
| Transient units | One-off commands in cgroups with resource limits (like `systemd-run`). |
| SBOM generation | Software Bill of Materials for supply chain compliance. |
| Reproducible builds | SLSA level 3+ provenance. |

## Research-Driven Fixes (Resolved)

### dm-verity — all resolved in v0.7.0
- ~~Remove redundant `veritysetup verify`~~ — Done, `open` already verifies on read
- ~~`--restart-on-corruption` for edge mode~~ — Done
- ~~Verity failure fatal in edge mode~~ — Done
- FEC support (`--fec-device`) — deferred (P2, no edge storage reliability issues yet)

### LUKS2 + TPM2 — all resolved in v0.7.0
- ~~`--token-id=0` and `--tries=1`~~ — Done
- ~~Default `tpm_attestation` to `true`~~ — Done
- ~~PCR binding configuration~~ — Done (`pcr_bindings` field)

### sd_notify — all resolved in v0.7.0
- ~~WATCHDOG=1 keepalive~~ — Done
- ~~SO_PASSCRED credential verification~~ — Done
- ~~RELOADING=1 and STOPPING=1~~ — Done
- Abstract socket namespace — deferred (P2, no consumer needs it yet)

## Architecture Recommendation

The research confirms the standard pattern for modern init systems:

```
kybernet (PID 1 binary — Cyrius):
  mount /proc, /sys, /dev, /run
  set up signalfd for SIGCHLD + SIGTERM + SIGPWR
  exec argonaut-manager

argonaut-manager (uses argonaut Cyrius library):
  epoll event loop: signalfd, timerfd, notify socket, control socket
  parallel service startup from dependency graph
  cgroup-per-service
  socket activation
```

This two-process split means a bug in the service manager doesn't kernel-panic the system. s6, dinit, and systemd all use this pattern. kybernet 1.0.1 boots QEMU with real AGNOS binaries using the argonaut 1.2.0 library.
