# Argonaut

**Init system and service manager for AGNOS.**

Argonaut (Greek: sailors of the Argo — one letter off from AGNOS) is a minimal init system that boots AGNOS in under 3 seconds. It manages service startup ordering, health checks, runlevel switching, and shutdown sequences.

## Features

- **Boot modes** — Desktop, Server, Edge, Minimal, Recovery
- **Service management** — define, enable, disable, start, stop with dependency ordering
- **Health checks** — HTTP, TCP, file-exists, and custom health probes per service
- **Restart policies** — always, on-failure, never, with backoff
- **Runlevel switching** — transition between modes at runtime with planned service start/stop
- **Shutdown planning** — ordered shutdown with drain, stop, kill phases
- **Edge boot** — read-only rootfs, dm-verity integrity verification
- **Emergency shell** — configurable fallback on boot failure
- **Crash actions** — reboot, halt, or emergency shell on service crash

## Architecture

```
argonaut
├── types.rs       — All enums, structs, config types
│   ├── BootMode, BootStage, BootStep
│   ├── ServiceDefinition, ManagedService, ServiceState
│   ├── HealthCheck, HealthTracker
│   ├── RestartPolicy, CrashAction
│   ├── Runlevel, RunlevelSwitchPlan
│   ├── ShutdownPlan, ShutdownStep, ShutdownType
│   └── SafeCommand (shell injection prevention)
├── boot.rs        — Boot sequence construction per BootMode
├── services.rs    — Service registry, defaults, dependency resolution
├── runlevels.rs   — Shutdown planning and runlevel transitions
├── edge_boot.rs   — Read-only rootfs, dm-verity helpers
└── tests.rs       — 148 tests
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

```rust
use argonaut::{ServiceDefinition, HealthCheck, HealthCheckType, RestartPolicy};

let svc = ServiceDefinition {
    name: "daimon".into(),
    command: SafeCommand::new("/usr/bin/daimon"),
    depends_on: vec![],
    health_check: Some(HealthCheck {
        check_type: HealthCheckType::Http {
            url: "http://localhost:8090/v1/health".into(),
        },
        interval_secs: 10,
        timeout_secs: 5,
        retries: 3,
    }),
    restart_policy: RestartPolicy::Always,
    ..Default::default()
};
```

## Consumers

- **AGNOS boot** — PID 1 (or systemd unit delegate)
- **stiva** — container service lifecycle
- **sutra** — infrastructure playbook service management
- **daimon** — system_update module triggers runlevel switches

## Building

```bash
cargo build --release
cargo test
```

## License

GPL-3.0-only
