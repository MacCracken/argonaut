# Argonaut

**Init system and service manager for AGNOS.**

Argonaut (Greek: sailors of the Argo — one letter off from AGNOS) is a minimal init system that boots AGNOS in under 3 seconds. It manages service startup ordering, health checks, runlevel switching, and shutdown sequences.

Written in [Cyrius](https://github.com/MacCracken/cyrius), compiled via cc3.

## Features

- **Boot modes** — Desktop, Server, Edge, Minimal, Recovery
- **Service management** — define, enable, disable, start, stop with dependency ordering
- **Health checks** — HTTP, TCP, command, process-alive per service
- **Restart policies** — always, on-failure, never, with exponential backoff
- **Runlevel switching** — transition between modes at runtime with planned service start/stop
- **Shutdown planning** — ordered shutdown with drain, stop, kill phases
- **Edge boot** — read-only rootfs, dm-verity integrity verification, LUKS unlock
- **Security** — seccomp, Landlock, capabilities, socket activation per service
- **Audit trail** — libro 1.0.2 SHA-256 hash-linked chain with lifecycle recording
- **Tracing** — sakshi_full 0.7.0 structured tracing (spans, ring buffer, UDP output)
- **Emergency shell** — configurable fallback on boot failure
- **Crash actions** — reboot, halt, or emergency shell on service crash

## Architecture

```
argonaut
├── src/
│   ├── types.cyr       — All enums, structs, config types
│   ├── boot.cyr        — Boot sequence construction per BootMode
│   ├── services.cyr    — Service registry, defaults, dependency resolution (Kahn's)
│   ├── process_mgmt.cyr — Fork/exec, PID tracking, signal delivery, ProcessTable
│   ├── health.cyr      — Health check execution, HealthState, HealthHistory
│   ├── edge_boot.cyr   — Read-only rootfs, dm-verity, LUKS, fleet registration
│   ├── notify.cyr      — sd_notify protocol listener
│   ├── security.cyr    — Seccomp, Landlock, capabilities
│   ├── systemd.cyr     — Hybrid systemd unit generation
│   ├── tmpfiles.cyr    — Boot-time filesystem setup
│   ├── audit.cyr       — Libro bridge (ServiceEventType → AuditChain)
│   ├── init.cyr        — ArgonautInit orchestrator, service lifecycle
│   └── main.cyr        — Entry point
├── lib/                — Cyrius stdlib + libro deps
│   └── libro/          — Libro 1.0.2 audit chain modules
├── tests/tcyr/         — 26 test suites (606 assertions)
└── build/              — Compiled binary (373KB ELF x86_64)
```

## Boot Modes

| Mode | Services | Use Case |
|------|----------|----------|
| Desktop | Full stack (daimon, hoosh, aethersafha, etc.) | Daily driver |
| Server | Headless (daimon, hoosh, no compositor) | Servers, CI |
| Edge | Minimal + fleet (daimon, seema) | IoT, edge nodes |
| Minimal | Core only (daimon) | Debugging, recovery |
| Recovery | Emergency shell | Broken system repair |

## Service Definition

```cyrius
var sd = svc_def_new(str_from("daimon"), str_from("Daimon agent orchestrator"), str_from("/usr/bin/daimon"));
var hc = health_check_new(HC_HTTP, str_from("http://localhost:8090/v1/health"), 10000, 5000, 3, 0);
svc_def_set_health_check(sd, hc);
store64(sd + 56, RESTART_ALWAYS);
```

## Consumers

- **AGNOS boot** — PID 1 (via kybernet) or systemd unit delegate
- **stiva** — container service lifecycle
- **sutra** — infrastructure playbook service management
- **daimon** — system_update module triggers runlevel switches

## Building

```sh
cyrius build src/main.cyr build/argonaut
cyrius test
cyrius bench
```

Requires cc3 3.6.2+.

## License

GPL-3.0-only
