# Argonaut

**Init system and service manager for AGNOS.**

Argonaut (Greek: sailors of the Argo — one letter off from AGNOS) handles:
- service startup ordering;
- health checks;
- restarts;
- runlevel switching;
- shutdown;
- edge boot (read-only rootfs, dm-verity, LUKS).

Every lifecycle event goes into a hash-linked audit chain.

It is written in [Cyrius](https://github.com/MacCracken/cyrius) and built with
`cyrius build`. Most of it is a library. [kybernet](https://github.com/MacCracken/kybernet),
AGNOS's PID 1, drives that library from its own supervisor. argonaut's own binary also
runs a minimal PID-1 loop: it mounts, takes signals on a signalfd, reaps, syncs and powers
off.

## Features

- **Boot modes:** Desktop, Server, Edge, Minimal, Recovery.
- **Service lifecycle:**
  - define, enable, disable, start and stop services, in dependency order;
  - simple, forking (PID file), oneshot and notify (sd_notify `READY=1`) service types;
  - startup in waves, by Kahn's algorithm, with a deterministic order inside each wave.
- **Health checks:** HTTP, TCP, command and process-alive, all with bounded deadlines.
- **Restarts:** always / on-failure / never, with exponential backoff inside a restart
  window. A service gives up at `max_restarts`, and the give-up is audited.
- **Watchdog:** `WATCHDOG=1` keepalives over an authenticated sd_notify socket.
- **Shutdown:** `init_execute_shutdown` stops services in reverse dependency waves within
  one budget. Each wave gets SIGTERM, then a process-group SIGKILL.
- **Runlevel switching:** a planned start / stop set for moving between modes.
- **Edge boot:** a read-only rootfs with tmpfs overlays, dm-verity verification, and a
  LUKS2 unlock from a TPM token (key-release policy: [architecture note 005](docs/architecture/005-edge-boot-tpm-key-policy.md)).
- **Emergency shell:** authentication is required by default. It verifies a salted
  Argon2id credential.
- **Audit trail:** [libro](https://github.com/MacCracken/libro) SHA-256 hash-linked chain.
  Optional persistence, signed snapshots and merkle proofs.
- **systemd interop:** unit-file generation with injection-safe escaping, and tmpfiles
  entries applied natively without following symlinks.
- **Security policy:** per-service seccomp / Landlock / capability fields, plus a
  fail-closed **pre-exec hook** between fork and exec. **argonaut does not enforce the
  policy itself.**
  - The consumer registers the hook and applies the policy; kybernet does.
  - With no hook registered, a service runs with PID 1's privileges.

## Layout

```
argonaut
├── src/
│   ├── types.cyr          — enums, structs, config types and accessors
│   ├── boot.cyr           — boot sequence per BootMode
│   ├── services.cyr       — default services, dependency resolution (Kahn's, waves)
│   ├── process_mgmt.cyr   — fork/exec, the pre-exec hook, process table, signals
│   ├── resolver.cyr       — IPv4 literals and /etc/hosts for health checks
│   ├── health.cyr         — health checks, state and history
│   ├── edge_boot.cyr      — read-only rootfs, dm-verity, LUKS, fleet registration
│   ├── notify.cyr         — sd_notify server and client
│   ├── security.cyr       — policy modelling; the emergency-shell credential
│   ├── systemd.cyr        — systemd unit generation
│   ├── tmpfiles.cyr       — tmpfiles entries: native apply and command rendering
│   ├── audit.cyr          — the libro bridge (ServiceEventType → audit chain)
│   ├── audit_ext.cyr      — persistence, signing, merkle proofs
│   ├── init.cyr           — the ArgonautInit orchestrator and service lifecycle
│   ├── syscall_compat.cyr — per-arch wrappers for syscalls the stdlib lacks
│   ├── pid1_harness.cyr   — the qemu PID-1 harness modes
│   ├── bench_main.cyr     — the release benchmark gate
│   └── main.cyr           — entry point and the minimal PID-1 loop
├── tests/tcyr/            — test suites
├── tests/bcyr/            — benchmarks
├── qemu/                  — PID-1 boot harness
└── scripts/               — bench history, aarch64 sweep, Pi smoke test
```

Suite counts, binary sizes and dependency pins change every release; they are in
[`docs/development/state.md`](docs/development/state.md).

## Boot Modes

| Mode | Default services | Use case |
|------|------------------|----------|
| Desktop | postgres, redis, daimon, llm-gateway (hoosh), agnos-init, aethersafha, agnoshi | Daily driver |
| Server | postgres, redis, daimon, llm-gateway (hoosh) | Servers, CI |
| Edge | daimon (edge mode) | IoT, edge nodes |
| Minimal | daimon, console agnoshi | Debugging |
| Recovery | none; emergency shell | Broken-system repair |

## Service Definition

```cyrius
var sd = svc_def_new(str_from("daimon"), str_from("Daimon agent orchestrator"),
                     str_from("/usr/bin/daimon"));
# type, target, port, interval_ms, timeout_ms, retries
var hc = health_check_new(HC_HTTP_GET, str_from("http://127.0.0.1:8090/v1/health"),
                          0, 10000, 5000, 3);
svc_def_set_health_check(sd, hc);
svc_def_set_restart_policy(sd, RESTART_ALWAYS);
```

More in [`docs/guides/quickstart.md`](docs/guides/quickstart.md).

## Consumers

- **kybernet** (AGNOS PID 1) consumes argonaut's `src/` through `[deps.argonaut]`, and
  enforces service policy through the pre-exec hook. Its pin is listed in
  [`state.md`](docs/development/state.md#consumers).

## Building

```sh
cyrius deps                                    # resolve lib/ from cyrius.cyml
cyrius build src/main.cyr build/argonaut       # build
for t in tests/tcyr/*.tcyr; do cyrius test "$t"; done
CYRIUS_DCE=1 cyrius build --check-lib-sync src/main.cyr build/argonaut   # release build
```

The toolchain version is pinned in `cyrius.cyml` (`[package] cyrius`). The full set of
gates is in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

GPL-3.0-only
