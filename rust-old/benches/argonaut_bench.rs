//! Argonaut benchmarks — boot sequence construction, service resolution,
//! shutdown planning, and health tracking.

use std::collections::HashMap;
use std::path::PathBuf;

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use argonaut::{
    ArgonautConfig, BootMode, BootStage, BootStepStatus, HealthTracker, RestartConfig,
    RestartPolicy, SafeCommand, ServiceDefinition, ServiceState, ServiceTarget, ServiceType,
    ShutdownType,
};

fn make_chain(n: usize) -> Vec<ServiceDefinition> {
    (0..n)
        .map(|i| ServiceDefinition {
            name: format!("svc-{i}"),
            description: format!("service {i}"),
            binary_path: PathBuf::from(format!("/usr/bin/svc-{i}")),
            args: vec![],
            environment: HashMap::new(),
            depends_on: if i > 0 {
                vec![format!("svc-{}", i - 1)]
            } else {
                vec![]
            },
            required_for_modes: vec![BootMode::Minimal],
            restart_policy: RestartPolicy::Never,
            restart_config: RestartConfig::default(),
            health_check: None,
            ready_check: None,
            enabled: true,
            service_type: ServiceType::Simple,
            environment_files: vec![],
            pid_file: None,
            resource_limits: None,
            log_config: None,
            socket_activation: None,
            seccomp: None,
            landlock: None,
            capabilities: None,
        })
        .collect()
}

fn boot_sequence(c: &mut Criterion) {
    c.bench_function("build_boot_sequence_desktop", |b| {
        b.iter(|| argonaut::ArgonautInit::build_boot_sequence(black_box(BootMode::Desktop)));
    });
    c.bench_function("build_boot_sequence_server", |b| {
        b.iter(|| argonaut::ArgonautInit::build_boot_sequence(black_box(BootMode::Server)));
    });
    c.bench_function("build_boot_sequence_minimal", |b| {
        b.iter(|| argonaut::ArgonautInit::build_boot_sequence(black_box(BootMode::Minimal)));
    });
    c.bench_function("build_boot_sequence_edge", |b| {
        b.iter(|| argonaut::ArgonautInit::build_boot_sequence(black_box(BootMode::Edge)));
    });
}

fn init_construction(c: &mut Criterion) {
    c.bench_function("init_new_desktop", |b| {
        b.iter(|| argonaut::ArgonautInit::new(ArgonautConfig::default()));
    });
    c.bench_function("init_new_minimal", |b| {
        let config = ArgonautConfig {
            boot_mode: BootMode::Minimal,
            ..Default::default()
        };
        b.iter(|| argonaut::ArgonautInit::new(config.clone()));
    });
    c.bench_function("init_new_edge", |b| {
        let config = ArgonautConfig {
            boot_mode: BootMode::Edge,
            ..Default::default()
        };
        b.iter(|| argonaut::ArgonautInit::new(config.clone()));
    });
}

fn service_resolution(c: &mut Criterion) {
    let desktop_svcs = argonaut::ArgonautInit::default_services(BootMode::Desktop);
    let desktop_refs: Vec<&ServiceDefinition> = desktop_svcs.iter().collect();
    c.bench_function("resolve_service_order_desktop", |b| {
        b.iter(|| argonaut::ArgonautInit::resolve_service_order(black_box(&desktop_refs)));
    });

    let chain_20 = make_chain(20);
    let chain_20_refs: Vec<&ServiceDefinition> = chain_20.iter().collect();
    c.bench_function("resolve_service_order_chain_20", |b| {
        b.iter(|| argonaut::ArgonautInit::resolve_service_order(black_box(&chain_20_refs)));
    });

    let chain_100 = make_chain(100);
    let chain_100_refs: Vec<&ServiceDefinition> = chain_100.iter().collect();
    c.bench_function("resolve_service_order_chain_100", |b| {
        b.iter(|| argonaut::ArgonautInit::resolve_service_order(black_box(&chain_100_refs)));
    });

    // Wave-based resolution benchmarks
    c.bench_function("resolve_service_waves_desktop", |b| {
        b.iter(|| argonaut::ArgonautInit::resolve_service_waves(black_box(&desktop_refs)));
    });

    c.bench_function("resolve_service_waves_chain_20", |b| {
        b.iter(|| argonaut::ArgonautInit::resolve_service_waves(black_box(&chain_20_refs)));
    });

    // Wide topology: all independent (single wave)
    let wide: Vec<ServiceDefinition> = (0..20)
        .map(|i| ServiceDefinition {
            name: format!("ind-{i}"),
            description: format!("independent {i}"),
            binary_path: PathBuf::from(format!("/usr/bin/ind-{i}")),
            args: vec![],
            environment: HashMap::new(),
            depends_on: vec![],
            required_for_modes: vec![BootMode::Minimal],
            restart_policy: RestartPolicy::Never,
            restart_config: RestartConfig::default(),
            health_check: None,
            ready_check: None,
            enabled: true,
            service_type: ServiceType::Simple,
            environment_files: vec![],
            pid_file: None,
            resource_limits: None,
            log_config: None,
            socket_activation: None,
            seccomp: None,
            landlock: None,
            capabilities: None,
        })
        .collect();
    let wide_refs: Vec<&ServiceDefinition> = wide.iter().collect();
    c.bench_function("resolve_service_waves_wide_20", |b| {
        b.iter(|| argonaut::ArgonautInit::resolve_service_waves(black_box(&wide_refs)));
    });
}

fn shutdown_planning(c: &mut Criterion) {
    let desktop_init = argonaut::ArgonautInit::new(ArgonautConfig::default());
    c.bench_function("plan_shutdown_desktop", |b| {
        b.iter(|| desktop_init.plan_shutdown(black_box(ShutdownType::Poweroff)));
    });

    let edge_init = argonaut::ArgonautInit::new(ArgonautConfig {
        boot_mode: BootMode::Edge,
        ..Default::default()
    });
    c.bench_function("plan_shutdown_edge", |b| {
        b.iter(|| edge_init.plan_shutdown(black_box(ShutdownType::Reboot)));
    });
}

fn runlevel_switching(c: &mut Criterion) {
    let init = argonaut::ArgonautInit::new(ArgonautConfig {
        boot_mode: BootMode::Server,
        ..Default::default()
    });
    let targets = ServiceTarget::defaults();
    c.bench_function("plan_runlevel_switch", |b| {
        b.iter(|| init.plan_runlevel_switch(black_box(argonaut::Runlevel::Graphical), &targets));
    });
}

fn boot_step_marking(c: &mut Criterion) {
    c.bench_function("mark_all_steps_complete", |b| {
        b.iter(|| {
            let mut init = argonaut::ArgonautInit::new(ArgonautConfig {
                boot_mode: BootMode::Desktop,
                ..Default::default()
            });
            let stages: Vec<BootStage> = init.boot_sequence.iter().map(|s| s.stage).collect();
            for stage in stages {
                init.mark_step_complete(stage);
            }
            init
        });
    });
}

fn health_tracking(c: &mut Criterion) {
    c.bench_function("health_tracker_100_checks", |b| {
        b.iter(|| {
            let mut tracker = HealthTracker::new();
            for i in 0..100 {
                let svc = format!("svc-{}", i % 10);
                let _ = tracker.record(&svc, i % 3 != 0, 3);
            }
            tracker
        });
    });
}

fn execution_plan(c: &mut Criterion) {
    let init = argonaut::ArgonautInit::new(ArgonautConfig::default());
    c.bench_function("boot_execution_plan_desktop", |b| {
        b.iter(|| init.boot_execution_plan());
    });
}

fn state_transitions(c: &mut Criterion) {
    c.bench_function("service_state_transitions", |b| {
        b.iter(|| {
            let mut init = argonaut::ArgonautInit::new(ArgonautConfig {
                boot_mode: BootMode::Minimal,
                ..Default::default()
            });
            init.set_service_state("daimon", ServiceState::Starting);
            init.set_service_state("daimon", ServiceState::Running);
            init.set_service_state("daimon", ServiceState::Stopping);
            init.set_service_state("daimon", ServiceState::Stopped);
        });
    });
}

fn safe_command(c: &mut Criterion) {
    let cmd = SafeCommand {
        binary: "veritysetup".to_string(),
        args: vec![
            "verify".to_string(),
            "/dev/sda1".to_string(),
            "/dev/sda2".to_string(),
            "a".repeat(64),
        ],
    };
    c.bench_function("safe_command_display", |b| {
        b.iter(|| black_box(&cmd).display());
    });
}

fn edge_boot(c: &mut Criterion) {
    c.bench_function("configure_readonly_rootfs", |b| {
        b.iter(argonaut::configure_readonly_rootfs);
    });

    let hash = "a".repeat(64);
    c.bench_function("verify_rootfs_integrity", |b| {
        b.iter(|| argonaut::verify_rootfs_integrity("/dev/sda1", "/dev/sda2", black_box(&hash)));
    });
}

fn stats_collection(c: &mut Criterion) {
    let mut init = argonaut::ArgonautInit::new(ArgonautConfig::default());
    for step in &mut init.boot_sequence {
        step.status = BootStepStatus::Complete;
    }
    c.bench_function("stats_desktop", |b| {
        b.iter(|| init.stats());
    });
}

fn wave_execution_plan(c: &mut Criterion) {
    let init = argonaut::ArgonautInit::new(ArgonautConfig::default());
    c.bench_function("boot_execution_plan_waves_desktop", |b| {
        b.iter(|| init.boot_execution_plan_waves());
    });
}

fn api_responses(c: &mut Criterion) {
    let mut init = argonaut::ArgonautInit::new(ArgonautConfig::default());
    // Set some services to Running for realistic output
    init.set_service_state("postgres", ServiceState::Starting);
    init.set_service_state("postgres", ServiceState::Running);

    c.bench_function("list_services_desktop", |b| {
        b.iter(|| init.list_services());
    });
    c.bench_function("system_status_desktop", |b| {
        b.iter(|| init.system_status());
    });
    c.bench_function("system_metrics_desktop", |b| {
        b.iter(|| init.system_metrics());
    });
    c.bench_function("boot_log_desktop", |b| {
        b.iter(|| init.boot_log());
    });
}

fn tmpfile_generation(c: &mut Criterion) {
    // TmpfileEntry is non_exhaustive — use serde to construct from JSON
    let entries: Vec<argonaut::TmpfileEntry> = (0..20)
        .map(|i| {
            serde_json::from_value(serde_json::json!({
                "Directory": {
                    "path": format!("/run/svc-{i}"),
                    "mode": 493,
                    "uid": 1000,
                    "gid": 1000
                }
            }))
            .unwrap()
        })
        .collect();

    c.bench_function("generate_tmpfile_commands_20", |b| {
        b.iter(|| argonaut::generate_tmpfile_commands(black_box(&entries)));
    });
}

fn resource_limit_commands(c: &mut Criterion) {
    // ResourceLimits is non_exhaustive — construct via serde
    let limits: argonaut::ResourceLimits = serde_json::from_str(
        r#"{"nofile":65536,"address_space":4294967296,"nproc":4096,"core":0}"#,
    )
    .unwrap();
    c.bench_function("resource_limits_prlimit_commands", |b| {
        b.iter(|| limits.to_prlimit_commands(black_box(12345)));
    });
}

fn systemd_unit_generation(c: &mut Criterion) {
    let services = argonaut::ArgonautInit::default_services(BootMode::Desktop);
    c.bench_function("generate_systemd_unit_desktop_all", |b| {
        b.iter(|| {
            for svc in &services {
                let _ = argonaut::generate_unit(black_box(svc));
            }
        });
    });
}

criterion_group!(
    benches,
    boot_sequence,
    init_construction,
    service_resolution,
    shutdown_planning,
    runlevel_switching,
    boot_step_marking,
    health_tracking,
    execution_plan,
    state_transitions,
    safe_command,
    edge_boot,
    stats_collection,
    wave_execution_plan,
    api_responses,
    tmpfile_generation,
    resource_limit_commands,
    systemd_unit_generation,
);
criterion_main!(benches);
