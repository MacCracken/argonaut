# Quickstart Guide

## Adding argonaut to your project

```toml
[dependencies]
argonaut = "0.90"
```

## Basic Usage — Boot and Manage Services

```rust
use argonaut::{ArgonautConfig, ArgonautInit, BootMode, HealthTracker, ShutdownType};
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    // Configure for server mode
    let config = ArgonautConfig {
        boot_mode: BootMode::Server,
        ..Default::default()
    };

    // Create the init system
    let mut init = ArgonautInit::new(config);

    // Start services in dependency order
    let plan = init.boot_execution_plan()?;
    for (name, _spec) in &plan {
        match init.start_service(name) {
            Ok(pid) => println!("  started {} (pid {})", name, pid),
            Err(e) => eprintln!("  FAILED {}: {}", name, e),
        }
    }

    // Runtime loop
    let mut tracker = HealthTracker::new();
    loop {
        // Reap exited processes and handle restart policy
        let reaped = init.reap_services();
        for (name, code, action) in &reaped {
            println!("  {} exited ({}): {:?}", name, code, action);
        }

        // Run health checks
        let _results = init.poll_health(&mut tracker);

        // Enforce watchdog timeouts
        let _killed = init.enforce_watchdog();

        std::thread::sleep(Duration::from_secs(1));
    }
}
```

## Defining a Custom Service

```rust
use argonaut::{
    ServiceDefinition, HealthCheck, HealthCheckType, ReadyCheck,
    RestartPolicy, RestartConfig, BootMode,
};
use std::collections::HashMap;
use std::path::PathBuf;

let my_service = ServiceDefinition {
    name: "my-api".into(),
    description: "My API server".into(),
    binary_path: PathBuf::from("/usr/bin/my-api"),
    args: vec!["--port".into(), "3000".into()],
    environment: HashMap::new(),
    depends_on: vec!["daimon".into()],
    required_for_modes: vec![BootMode::Server, BootMode::Desktop],
    restart_policy: RestartPolicy::OnFailure,
    restart_config: RestartConfig {
        max_restarts: 10,
        base_delay_ms: 500,
        max_delay_ms: 60_000,
    },
    health_check: Some(HealthCheck {
        check_type: HealthCheckType::HttpGet("http://127.0.0.1:3000/health".into()),
        interval_ms: 15_000,
        timeout_ms: 5_000,
        retries: 3,
    }),
    ready_check: Some(ReadyCheck {
        check_type: HealthCheckType::TcpConnect("127.0.0.1".into(), 3000),
        timeout_ms: 10_000,
        retries: 20,
        retry_delay_ms: 500,
    }),
};
```

## Edge Boot

```rust
use argonaut::{
    ArgonautConfig, ArgonautInit, BootMode, EdgeBootConfig,
    execute_edge_boot, validate_edge_profile, FleetRegistration,
};

let config = ArgonautConfig {
    boot_mode: BootMode::Edge,
    edge_boot: EdgeBootConfig {
        readonly_rootfs: true,
        luks_enabled: true,
        tpm_attestation: true,
        max_boot_time_ms: 3000,
    },
    ..Default::default()
};

// Execute edge boot sequence
let result = execute_edge_boot(
    &config.edge_boot,
    "/dev/mmcblk0p2",    // root device
    "/dev/mmcblk0p3",    // hash device
    "a1b2c3d4e5f6...",   // root hash (64 hex chars)
    "/dev/mmcblk0p4",    // LUKS device
);

// Validate edge profile
let violations = validate_edge_profile(&result, 128); // 128MB RAM limit
if !violations.is_empty() {
    eprintln!("Edge profile violations: {:?}", violations);
}

// Build fleet registration payload
let registration = FleetRegistration::from_system(&result);
let json = registration.to_json()?;
// POST json to fleet management server...
```

## Shutdown

```rust
use argonaut::ShutdownType;

let plan = init.plan_shutdown(ShutdownType::Poweroff)?;
let executed = init.execute_shutdown(plan);

for step in &executed.steps {
    println!("  {} — {:?}", step.description, step.status);
}
```

## Runlevel Switching

```rust
use argonaut::{Runlevel, ServiceTarget};
use std::time::Duration;

let targets = ServiceTarget::defaults();
let plan = init.plan_runlevel_switch(Runlevel::Console, &targets);
let result = init.execute_runlevel_switch(&plan, Duration::from_secs(5));

println!("Stopped: {:?}", result.stopped);
println!("Started: {:?}", result.started);

if result.drop_to_shell {
    init.drop_to_emergency_shell()?;
}
```
