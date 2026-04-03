//! Argonaut unit tests.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::Utc;

use super::ArgonautInit;
use super::edge_boot::{configure_readonly_rootfs, verify_rootfs_integrity};
use super::types::{
    ArgonautConfig, BootMode, BootStage, BootStepStatus, CrashAction, EdgeBootConfig,
    EmergencyShellConfig, ExitStatus, HealthCheckType, ManagedService, ProcessSpec, RestartConfig,
    RestartPolicy, Runlevel, SafeCommand, ServiceDefinition, ServiceEventType, ServiceState,
    ServiceTarget, ServiceType, ShutdownAction, ShutdownStepStatus, ShutdownType,
};

// --- helpers ---

fn minimal_config() -> ArgonautConfig {
    ArgonautConfig {
        boot_mode: BootMode::Minimal,
        ..Default::default()
    }
}

fn server_config() -> ArgonautConfig {
    ArgonautConfig {
        boot_mode: BootMode::Server,
        ..Default::default()
    }
}

fn desktop_config() -> ArgonautConfig {
    ArgonautConfig {
        boot_mode: BootMode::Desktop,
        ..Default::default()
    }
}

fn dummy_service(name: &str, deps: Vec<&str>) -> ServiceDefinition {
    ServiceDefinition {
        name: name.into(),
        description: format!("test service {name}"),
        binary_path: PathBuf::from(format!("/usr/bin/{name}")),
        args: vec![],
        environment: HashMap::new(),
        depends_on: deps.into_iter().map(String::from).collect(),
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
    }
}

// --- BootMode ---

#[test]
fn boot_mode_display_server() {
    assert_eq!(BootMode::Server.to_string(), "server");
}

#[test]
fn boot_mode_display_desktop() {
    assert_eq!(BootMode::Desktop.to_string(), "desktop");
}

#[test]
fn boot_mode_display_minimal() {
    assert_eq!(BootMode::Minimal.to_string(), "minimal");
}

#[test]
fn boot_mode_display_edge() {
    assert_eq!(BootMode::Edge.to_string(), "edge");
}

// --- BootStage ordering ---

#[test]
fn boot_stage_ordering() {
    assert!(BootStage::MountFilesystems < BootStage::StartDeviceManager);
    assert!(BootStage::StartDeviceManager < BootStage::VerifyRootfs);
    assert!(BootStage::VerifyRootfs < BootStage::StartSecurity);
    assert!(BootStage::StartSecurity < BootStage::StartDatabaseServices);
    assert!(BootStage::StartDatabaseServices < BootStage::StartAgentRuntime);
    assert!(BootStage::StartAgentRuntime < BootStage::StartLlmGateway);
    assert!(BootStage::StartLlmGateway < BootStage::StartModelServices);
    assert!(BootStage::StartModelServices < BootStage::StartCompositor);
    assert!(BootStage::StartCompositor < BootStage::StartShell);
    assert!(BootStage::StartShell < BootStage::BootComplete);
}

#[test]
fn boot_stage_display() {
    assert_eq!(BootStage::MountFilesystems.to_string(), "mount-filesystems");
    assert_eq!(BootStage::BootComplete.to_string(), "boot-complete");
}

// --- BootStepStatus ---

#[test]
fn boot_step_status_variants() {
    assert_eq!(BootStepStatus::Pending.to_string(), "pending");
    assert_eq!(BootStepStatus::Running.to_string(), "running");
    assert_eq!(BootStepStatus::Complete.to_string(), "complete");
    assert_eq!(BootStepStatus::Failed.to_string(), "failed");
    assert_eq!(BootStepStatus::Skipped.to_string(), "skipped");
}

// --- RestartPolicy ---

#[test]
fn restart_policy_display() {
    assert_eq!(RestartPolicy::Always.to_string(), "always");
    assert_eq!(RestartPolicy::OnFailure.to_string(), "on-failure");
    assert_eq!(RestartPolicy::Never.to_string(), "never");
}

// --- HealthCheckType ---

#[test]
fn health_check_type_variants() {
    let http = HealthCheckType::HttpGet("http://localhost/health".into());
    let tcp = HealthCheckType::TcpConnect("127.0.0.1".into(), 8080);
    let cmd = HealthCheckType::Command("systemctl is-active foo".into());
    let alive = HealthCheckType::ProcessAlive;

    assert!(matches!(http, HealthCheckType::HttpGet(_)));
    assert!(matches!(tcp, HealthCheckType::TcpConnect(_, 8080)));
    assert!(matches!(cmd, HealthCheckType::Command(_)));
    assert!(matches!(alive, HealthCheckType::ProcessAlive));
}

// --- Boot sequence per mode ---

#[test]
fn boot_sequence_minimal_no_compositor() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Minimal);
    let stages: Vec<BootStage> = steps.iter().map(|s| s.stage).collect();
    assert!(!stages.contains(&BootStage::StartCompositor));
    assert!(!stages.contains(&BootStage::StartShell));
    assert!(!stages.contains(&BootStage::StartLlmGateway));
    assert!(stages.contains(&BootStage::StartAgentRuntime));
    assert!(stages.contains(&BootStage::BootComplete));
}

#[test]
fn boot_sequence_server_no_compositor() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Server);
    let stages: Vec<BootStage> = steps.iter().map(|s| s.stage).collect();
    assert!(!stages.contains(&BootStage::StartCompositor));
    assert!(!stages.contains(&BootStage::StartShell));
    assert!(stages.contains(&BootStage::StartLlmGateway));
    assert!(stages.contains(&BootStage::StartAgentRuntime));
}

#[test]
fn boot_sequence_desktop_all_stages() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Desktop);
    let stages: Vec<BootStage> = steps.iter().map(|s| s.stage).collect();
    assert!(stages.contains(&BootStage::StartCompositor));
    assert!(stages.contains(&BootStage::StartShell));
    assert!(stages.contains(&BootStage::StartLlmGateway));
    assert!(stages.contains(&BootStage::StartAgentRuntime));
    assert!(stages.contains(&BootStage::BootComplete));
}

#[test]
fn boot_sequence_step_count_minimal() {
    // MountFS, DevMgr, Verify, Security, AgentRuntime, BootComplete = 6
    let steps = ArgonautInit::build_boot_sequence(BootMode::Minimal);
    assert_eq!(steps.len(), 6);
}

#[test]
fn boot_sequence_step_count_server() {
    // 6 (minimal) + DatabaseServices + LlmGateway + ModelServices = 9
    let steps = ArgonautInit::build_boot_sequence(BootMode::Server);
    assert_eq!(steps.len(), 9);
}

#[test]
fn boot_sequence_step_count_desktop() {
    // 9 (server) + Compositor + Shell = 11
    let steps = ArgonautInit::build_boot_sequence(BootMode::Desktop);
    assert_eq!(steps.len(), 11);
}

// --- Default services ---

#[test]
fn default_services_minimal() {
    let svcs = ArgonautInit::default_services(BootMode::Minimal);
    assert_eq!(svcs.len(), 1);
    assert_eq!(svcs[0].name, "daimon");
}

#[test]
fn default_services_server() {
    let svcs = ArgonautInit::default_services(BootMode::Server);
    assert_eq!(svcs.len(), 5);
    let names: Vec<&str> = svcs.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"daimon"));
    assert!(names.contains(&"llm-gateway"));
}

#[test]
fn default_services_desktop() {
    let svcs = ArgonautInit::default_services(BootMode::Desktop);
    assert_eq!(svcs.len(), 7);
    let names: Vec<&str> = svcs.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"daimon"));
    assert!(names.contains(&"llm-gateway"));
    assert!(names.contains(&"synapse"));
    assert!(names.contains(&"aethersafha"));
    assert!(names.contains(&"agnoshi"));
}

// --- Service order resolution ---

#[test]
fn resolve_service_order_simple_chain() {
    let services = [
        dummy_service("c", vec!["b"]),
        dummy_service("b", vec!["a"]),
        dummy_service("a", vec![]),
    ];
    let refs: Vec<&ServiceDefinition> = services.iter().collect();
    let order = ArgonautInit::resolve_service_order(&refs).unwrap();
    assert_eq!(order, vec!["a", "b", "c"]);
}

#[test]
fn resolve_service_order_independent() {
    let services = [
        dummy_service("alpha", vec![]),
        dummy_service("beta", vec![]),
        dummy_service("gamma", vec![]),
    ];
    let refs: Vec<&ServiceDefinition> = services.iter().collect();
    let order = ArgonautInit::resolve_service_order(&refs).unwrap();
    assert_eq!(order.len(), 3);
    // All independent — any valid topological order contains all three.
    assert!(order.contains(&"alpha".to_string()));
    assert!(order.contains(&"beta".to_string()));
    assert!(order.contains(&"gamma".to_string()));
}

#[test]
fn resolve_service_order_cycle_detection() {
    let services = [dummy_service("a", vec!["b"]), dummy_service("b", vec!["a"])];
    let refs: Vec<&ServiceDefinition> = services.iter().collect();
    let result = ArgonautInit::resolve_service_order(&refs);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("cycle detected"));
}

// --- Register and get service ---

#[test]
fn register_and_get_service() {
    let mut init = ArgonautInit::new(minimal_config());
    let svc = dummy_service("my-service", vec![]);
    init.register_service(svc);
    let got = init.get_service("my-service");
    assert!(got.is_some());
    assert_eq!(got.unwrap().definition.name, "my-service");
    assert_eq!(got.unwrap().state, ServiceState::Stopped);
}

#[test]
fn get_service_not_found() {
    let init = ArgonautInit::new(minimal_config());
    assert!(init.get_service("nonexistent").is_none());
}

// --- Service state transitions ---

#[test]
fn set_service_state_valid_transitions() {
    let mut init = ArgonautInit::new(minimal_config());
    // Stopped → Starting (agent-runtime has no deps)
    assert!(init.set_service_state("daimon", ServiceState::Starting));
    assert_eq!(
        init.get_service_state("daimon"),
        Some(&ServiceState::Starting)
    );
    // Starting → Running
    assert!(init.set_service_state("daimon", ServiceState::Running));
    assert_eq!(
        init.get_service_state("daimon"),
        Some(&ServiceState::Running)
    );
}

#[test]
fn set_service_state_unknown_service() {
    let mut init = ArgonautInit::new(minimal_config());
    assert!(!init.set_service_state("nonexistent", ServiceState::Running));
}

// --- Boot step marking ---

#[test]
fn mark_step_complete() {
    let mut init = ArgonautInit::new(minimal_config());
    assert!(init.mark_step_complete(BootStage::MountFilesystems));
    let step = init
        .boot_sequence
        .iter()
        .find(|s| s.stage == BootStage::MountFilesystems)
        .unwrap();
    assert_eq!(step.status, BootStepStatus::Complete);
    assert!(step.completed_at.is_some());
}

#[test]
fn mark_step_failed() {
    let mut init = ArgonautInit::new(minimal_config());
    assert!(init.mark_step_failed(BootStage::VerifyRootfs, "dm-verity mismatch".into()));
    let step = init
        .boot_sequence
        .iter()
        .find(|s| s.stage == BootStage::VerifyRootfs)
        .unwrap();
    assert_eq!(step.status, BootStepStatus::Failed);
    assert_eq!(step.error.as_deref(), Some("dm-verity mismatch"));
}

#[test]
fn mark_step_nonexistent() {
    let mut init = ArgonautInit::new(minimal_config());
    // Minimal mode has no compositor stage.
    assert!(!init.mark_step_complete(BootStage::StartCompositor));
}

// --- Current stage ---

#[test]
fn current_stage_returns_first_pending() {
    let init = ArgonautInit::new(minimal_config());
    let current = init.current_stage().unwrap();
    assert_eq!(current.stage, BootStage::MountFilesystems);
}

#[test]
fn current_stage_skips_complete() {
    let mut init = ArgonautInit::new(minimal_config());
    init.mark_step_complete(BootStage::MountFilesystems);
    let current = init.current_stage().unwrap();
    assert_eq!(current.stage, BootStage::StartDeviceManager);
}

// --- Boot complete ---

#[test]
fn is_boot_complete_all_complete() {
    let mut init = ArgonautInit::new(minimal_config());
    for step in &mut init.boot_sequence {
        step.status = BootStepStatus::Complete;
    }
    assert!(init.is_boot_complete());
}

#[test]
fn is_boot_complete_required_failed() {
    let mut init = ArgonautInit::new(minimal_config());
    for step in &mut init.boot_sequence {
        if step.required {
            step.status = BootStepStatus::Failed;
        } else {
            step.status = BootStepStatus::Complete;
        }
    }
    // Required steps failed — boot is NOT complete.
    assert!(!init.is_boot_complete());
}

#[test]
fn is_boot_complete_optional_failed_ok() {
    let mut init = ArgonautInit::new(desktop_config());
    for step in &mut init.boot_sequence {
        if step.required {
            step.status = BootStepStatus::Complete;
        } else {
            step.status = BootStepStatus::Failed;
        }
    }
    // Non-required failures are tolerated.
    assert!(init.is_boot_complete());
}

// --- Boot duration ---

#[test]
fn boot_duration_ms_calculation() {
    let mut init = ArgonautInit::new(minimal_config());
    let start = Utc::now();
    init.boot_started = Some(start);
    init.boot_completed = Some(start + chrono::Duration::milliseconds(1234));
    assert_eq!(init.boot_duration_ms(), Some(1234));
}

#[test]
fn boot_duration_ms_not_complete() {
    let init = ArgonautInit::new(minimal_config());
    assert_eq!(init.boot_duration_ms(), None);
}

// --- Failed steps ---

#[test]
fn failed_steps_list() {
    let mut init = ArgonautInit::new(minimal_config());
    init.mark_step_failed(BootStage::VerifyRootfs, "bad hash".into());
    init.mark_step_failed(BootStage::StartSecurity, "seccomp err".into());
    let failed = init.failed_steps();
    assert_eq!(failed.len(), 2);
    let stages: Vec<BootStage> = failed.iter().map(|s| s.stage).collect();
    assert!(stages.contains(&BootStage::VerifyRootfs));
    assert!(stages.contains(&BootStage::StartSecurity));
}

// --- Shutdown order ---

#[test]
fn shutdown_order_is_reverse_of_startup() {
    let init = ArgonautInit::new(desktop_config());
    let definitions: Vec<&ServiceDefinition> =
        init.services.values().map(|s| &s.definition).collect();
    let startup = ArgonautInit::resolve_service_order(&definitions).unwrap();
    let shutdown = init.shutdown_order().unwrap();
    let reversed_startup: Vec<String> = startup.into_iter().rev().collect();
    assert_eq!(shutdown, reversed_startup);
}

// --- ArgonautConfig defaults ---

#[test]
fn config_defaults() {
    let cfg = ArgonautConfig::default();
    assert_eq!(cfg.boot_mode, BootMode::Desktop);
    assert_eq!(cfg.boot_timeout_ms, 30_000);
    assert_eq!(cfg.shutdown_timeout_ms, 10_000);
    assert!(cfg.log_to_console);
    assert!(cfg.verify_on_boot);
    assert!(cfg.services.is_empty());
}

// --- ServiceDefinition with checks ---

#[test]
fn service_definition_with_health_check() {
    let svcs = ArgonautInit::default_services(BootMode::Minimal);
    let agent_rt = &svcs[0];
    assert!(agent_rt.health_check.is_some());
    let hc = agent_rt.health_check.as_ref().unwrap();
    assert!(matches!(hc.check_type, HealthCheckType::HttpGet(_)));
    assert_eq!(hc.retries, 3);
}

#[test]
fn service_definition_with_ready_check() {
    let svcs = ArgonautInit::default_services(BootMode::Minimal);
    let agent_rt = &svcs[0];
    assert!(agent_rt.ready_check.is_some());
    let rc = agent_rt.ready_check.as_ref().unwrap();
    assert!(matches!(
        rc.check_type,
        HealthCheckType::TcpConnect(_, 8090)
    ));
    assert_eq!(rc.retry_delay_ms, 200);
}

// --- ManagedService initial state ---

#[test]
fn managed_service_initial_state() {
    let init = ArgonautInit::new(minimal_config());
    let svc = init.get_service("daimon").unwrap();
    assert_eq!(svc.state, ServiceState::Stopped);
    assert!(svc.pid.is_none());
    assert!(svc.started_at.is_none());
    assert_eq!(svc.restart_count, 0);
    assert!(svc.last_health_check.is_none());
}

// --- services_for_mode ---

#[test]
fn services_for_mode_filtering() {
    let init = ArgonautInit::new(desktop_config());
    let minimal_svcs = init.services_for_mode(&BootMode::Minimal);
    // Only agent-runtime is required for Minimal.
    assert_eq!(minimal_svcs.len(), 1);
    assert_eq!(minimal_svcs[0].name, "daimon");
}

#[test]
fn services_for_mode_desktop() {
    let init = ArgonautInit::new(desktop_config());
    let desktop_svcs = init.services_for_mode(&BootMode::Desktop);
    assert_eq!(desktop_svcs.len(), 7);
}

// --- Stats ---

#[test]
fn stats_accuracy() {
    let mut init = ArgonautInit::new(server_config());
    // Start database services first (agent-runtime depends on them in server mode)
    assert!(init.set_service_state("postgres", ServiceState::Starting));
    assert!(init.set_service_state("postgres", ServiceState::Running));
    assert!(init.set_service_state("redis", ServiceState::Starting));
    assert!(init.set_service_state("redis", ServiceState::Running));
    // Valid transition path: Stopped → Starting → Running
    assert!(init.set_service_state("daimon", ServiceState::Starting));
    assert!(init.set_service_state("daimon", ServiceState::Running));
    // llm-gateway depends on agent-runtime which is now Running
    assert!(init.set_service_state("llm-gateway", ServiceState::Starting));
    assert!(init.set_service_state("llm-gateway", ServiceState::Failed("crash".into()),));
    if let Some(svc) = init.services.get_mut("daimon") {
        svc.restart_count = 3;
    }
    let s = init.stats();
    assert_eq!(s.boot_mode, BootMode::Server);
    assert_eq!(s.services_running, 3);
    assert_eq!(s.services_failed, 1);
    assert_eq!(s.total_restarts, 3);
    assert!(!s.boot_complete);
}

#[test]
fn stats_empty_init() {
    let init = ArgonautInit::new(minimal_config());
    let s = init.stats();
    assert_eq!(s.boot_mode, BootMode::Minimal);
    assert_eq!(s.services_running, 0);
    assert_eq!(s.services_failed, 0);
    assert_eq!(s.total_restarts, 0);
    assert!(!s.boot_complete);
    assert!(s.boot_duration_ms.is_none());
}

// --- Boot step timeout values ---

#[test]
fn boot_step_timeout_values() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Desktop);
    let fs_step = steps
        .iter()
        .find(|s| s.stage == BootStage::MountFilesystems)
        .unwrap();
    assert_eq!(fs_step.timeout_ms, 2000);
    let verify_step = steps
        .iter()
        .find(|s| s.stage == BootStage::VerifyRootfs)
        .unwrap();
    assert_eq!(verify_step.timeout_ms, 5000);
    let complete_step = steps
        .iter()
        .find(|s| s.stage == BootStage::BootComplete)
        .unwrap();
    assert_eq!(complete_step.timeout_ms, 1000);
}

// --- Depends-on resolution for default desktop ---

#[test]
fn service_depends_on_resolution_desktop() {
    let svcs = ArgonautInit::default_services(BootMode::Desktop);
    let refs: Vec<&ServiceDefinition> = svcs.iter().collect();
    let order = ArgonautInit::resolve_service_order(&refs).unwrap();
    let rt_pos = order.iter().position(|n| n == "daimon").unwrap();
    let gw_pos = order.iter().position(|n| n == "llm-gateway").unwrap();
    let comp_pos = order.iter().position(|n| n == "aethersafha").unwrap();
    let shell_pos = order.iter().position(|n| n == "agnoshi").unwrap();
    // agent-runtime before everything.
    assert!(rt_pos < gw_pos);
    assert!(rt_pos < comp_pos);
    assert!(rt_pos < shell_pos);
    // aethersafha before agnoshi (shell depends on compositor).
    assert!(comp_pos < shell_pos);
}

// --- Audit fix tests ---

#[test]
fn invalid_state_transition_stopped_to_running() {
    let mut init = ArgonautInit::new(minimal_config());
    // Stopped → Running is not valid (must go through Starting)
    assert!(!init.set_service_state("daimon", ServiceState::Running));
    // State should remain Stopped
    assert_eq!(
        init.get_service_state("daimon"),
        Some(&ServiceState::Stopped)
    );
}

#[test]
fn valid_state_transition_full_lifecycle() {
    let mut init = ArgonautInit::new(minimal_config());
    // Stopped → Starting → Running → Stopping → Stopped
    assert!(init.set_service_state("daimon", ServiceState::Starting));
    assert!(init.set_service_state("daimon", ServiceState::Running));
    assert!(init.set_service_state("daimon", ServiceState::Stopping));
    assert!(init.set_service_state("daimon", ServiceState::Stopped));
    // Failed → Starting (restart), Failed → Stopped
    assert!(init.set_service_state("daimon", ServiceState::Starting));
    assert!(init.set_service_state("daimon", ServiceState::Failed("err".into())));
    assert!(init.set_service_state("daimon", ServiceState::Starting));
    assert!(init.set_service_state("daimon", ServiceState::Failed("err2".into())));
    assert!(init.set_service_state("daimon", ServiceState::Stopped));
}

#[test]
fn starting_blocked_when_dependency_not_running() {
    let mut init = ArgonautInit::new(server_config());
    // llm-gateway depends on agent-runtime. agent-runtime is Stopped.
    assert!(!init.set_service_state("llm-gateway", ServiceState::Starting));
    // Start database services first (agent-runtime depends on them in server mode)
    assert!(init.set_service_state("postgres", ServiceState::Starting));
    assert!(init.set_service_state("postgres", ServiceState::Running));
    assert!(init.set_service_state("redis", ServiceState::Starting));
    assert!(init.set_service_state("redis", ServiceState::Running));
    // Start agent-runtime but leave it in Starting (not Running).
    assert!(init.set_service_state("daimon", ServiceState::Starting));
    assert!(!init.set_service_state("llm-gateway", ServiceState::Starting));
    // Now make agent-runtime Running.
    assert!(init.set_service_state("daimon", ServiceState::Running));
    assert!(init.set_service_state("llm-gateway", ServiceState::Starting));
}

#[test]
fn register_service_overwrites_definition_preserves_state() {
    let mut init = ArgonautInit::new(minimal_config());
    let svc = dummy_service("my-svc", vec![]);
    init.register_service(svc);
    // Transition to Starting
    assert!(init.set_service_state("my-svc", ServiceState::Starting));
    assert!(init.set_service_state("my-svc", ServiceState::Running));
    if let Some(s) = init.services.get_mut("my-svc") {
        s.restart_count = 5;
    }
    // Re-register with updated description
    let mut svc2 = dummy_service("my-svc", vec![]);
    svc2.description = "updated description".into();
    init.register_service(svc2);
    let got = init.get_service("my-svc").unwrap();
    // Definition updated
    assert_eq!(got.definition.description, "updated description");
    // State preserved
    assert_eq!(got.state, ServiceState::Running);
    assert_eq!(got.restart_count, 5);
}

#[test]
fn boot_started_set_after_first_step_completes() {
    let mut init = ArgonautInit::new(minimal_config());
    assert!(init.boot_started.is_none());
    init.mark_step_complete(BootStage::MountFilesystems);
    assert!(init.boot_started.is_some());
}

#[test]
fn boot_started_set_after_first_step_fails() {
    let mut init = ArgonautInit::new(minimal_config());
    assert!(init.boot_started.is_none());
    init.mark_step_failed(BootStage::MountFilesystems, "fail".into());
    assert!(init.boot_started.is_some());
}

#[test]
fn started_at_populated_on_mark_step_complete() {
    let mut init = ArgonautInit::new(minimal_config());
    init.mark_step_complete(BootStage::MountFilesystems);
    let step = init
        .boot_sequence
        .iter()
        .find(|s| s.stage == BootStage::MountFilesystems)
        .unwrap();
    assert!(step.started_at.is_some());
    assert!(step.completed_at.is_some());
    // started_at should be <= completed_at
    assert!(step.started_at.unwrap() <= step.completed_at.unwrap());
}

#[test]
fn started_at_populated_on_mark_step_failed() {
    let mut init = ArgonautInit::new(minimal_config());
    init.mark_step_failed(BootStage::MountFilesystems, "oops".into());
    let step = init
        .boot_sequence
        .iter()
        .find(|s| s.stage == BootStage::MountFilesystems)
        .unwrap();
    assert!(step.started_at.is_some());
}

#[test]
fn missing_dependency_returns_error() {
    let services = [dummy_service("a", vec!["nonexistent"])];
    let refs: Vec<&ServiceDefinition> = services.iter().collect();
    let result = ArgonautInit::resolve_service_order(&refs);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("depends on"));
    assert!(err.contains("nonexistent"));
    assert!(err.contains("not defined"));
}

#[test]
fn shutdown_order_returns_error_on_cycle() {
    let mut init = ArgonautInit::new(minimal_config());
    // Create a cycle: x depends on y, y depends on x
    let svc_x = dummy_service("x", vec!["y"]);
    let svc_y = dummy_service("y", vec!["x"]);
    init.services.clear();
    init.services.insert(
        "x".into(),
        ManagedService {
            definition: svc_x,
            state: ServiceState::Stopped,
            pid: None,
            started_at: None,
            restart_count: 0,
            last_health_check: None,
        },
    );
    init.services.insert(
        "y".into(),
        ManagedService {
            definition: svc_y,
            state: ServiceState::Stopped,
            pid: None,
            started_at: None,
            restart_count: 0,
            last_health_check: None,
        },
    );
    let result = init.shutdown_order();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("cycle detected"));
}

// --- Database services ---

#[test]
fn database_services_returns_postgres_and_redis() {
    let svcs = ArgonautInit::database_services();
    assert_eq!(svcs.len(), 2);
    let names: Vec<&str> = svcs.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"postgres"));
    assert!(names.contains(&"redis"));
}

#[test]
fn database_services_health_checks() {
    let svcs = ArgonautInit::database_services();
    for svc in &svcs {
        assert!(svc.health_check.is_some());
        assert!(svc.ready_check.is_some());
    }
}

#[test]
fn database_services_restart_policies() {
    let svcs = ArgonautInit::database_services();
    let pg = svcs.iter().find(|s| s.name == "postgres").unwrap();
    let redis = svcs.iter().find(|s| s.name == "redis").unwrap();
    assert_eq!(pg.restart_policy, RestartPolicy::OnFailure);
    assert_eq!(redis.restart_policy, RestartPolicy::Always);
}

#[test]
fn database_services_modes() {
    let svcs = ArgonautInit::database_services();
    for svc in &svcs {
        assert!(svc.required_for_modes.contains(&BootMode::Server));
        assert!(svc.required_for_modes.contains(&BootMode::Desktop));
        assert!(!svc.required_for_modes.contains(&BootMode::Minimal));
    }
}

#[test]
fn default_services_server_includes_databases() {
    let svcs = ArgonautInit::default_services(BootMode::Server);
    let names: Vec<&str> = svcs.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"postgres"));
    assert!(names.contains(&"redis"));
}

#[test]
fn default_services_minimal_excludes_databases() {
    let svcs = ArgonautInit::default_services(BootMode::Minimal);
    let names: Vec<&str> = svcs.iter().map(|s| s.name.as_str()).collect();
    assert!(!names.contains(&"postgres"));
    assert!(!names.contains(&"redis"));
}

#[test]
fn agent_runtime_depends_on_databases_in_server_mode() {
    let svcs = ArgonautInit::default_services(BootMode::Server);
    let rt = svcs.iter().find(|s| s.name == "daimon").unwrap();
    assert!(rt.depends_on.contains(&"postgres".to_string()));
    assert!(rt.depends_on.contains(&"redis".to_string()));
}

#[test]
fn agent_runtime_no_db_deps_in_minimal_mode() {
    let svcs = ArgonautInit::default_services(BootMode::Minimal);
    let rt = svcs.iter().find(|s| s.name == "daimon").unwrap();
    assert!(rt.depends_on.is_empty());
}

#[test]
fn boot_stage_database_ordering() {
    assert!(BootStage::StartSecurity < BootStage::StartDatabaseServices);
    assert!(BootStage::StartDatabaseServices < BootStage::StartAgentRuntime);
}

#[test]
fn boot_stage_database_display() {
    assert_eq!(
        BootStage::StartDatabaseServices.to_string(),
        "start-database-services"
    );
}

#[test]
fn boot_sequence_server_includes_database_stage() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Server);
    let stages: Vec<BootStage> = steps.iter().map(|s| s.stage).collect();
    assert!(stages.contains(&BootStage::StartDatabaseServices));
}

#[test]
fn boot_sequence_minimal_excludes_database_stage() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Minimal);
    let stages: Vec<BootStage> = steps.iter().map(|s| s.stage).collect();
    assert!(!stages.contains(&BootStage::StartDatabaseServices));
}

// --- Synapse service ---

#[test]
fn synapse_service_definition() {
    let svc = ArgonautInit::synapse_service();
    assert_eq!(svc.name, "synapse");
    assert!(svc.depends_on.contains(&"daimon".to_string()));
    assert!(svc.depends_on.contains(&"llm-gateway".to_string()));
    assert!(svc.health_check.is_some());
    assert!(svc.ready_check.is_some());
}

#[test]
fn server_mode_includes_synapse() {
    let services = ArgonautInit::default_services(BootMode::Server);
    assert!(services.iter().any(|s| s.name == "synapse"));
}

#[test]
fn desktop_mode_includes_synapse() {
    let services = ArgonautInit::default_services(BootMode::Desktop);
    assert!(services.iter().any(|s| s.name == "synapse"));
}

#[test]
fn minimal_mode_excludes_synapse() {
    let services = ArgonautInit::default_services(BootMode::Minimal);
    assert!(!services.iter().any(|s| s.name == "synapse"));
}

#[test]
fn synapse_starts_after_llm_gateway() {
    let services = ArgonautInit::default_services(BootMode::Server);
    let refs: Vec<&ServiceDefinition> = services.iter().collect();
    let order = ArgonautInit::resolve_service_order(&refs).unwrap();
    let gw_pos = order.iter().position(|s| s == "llm-gateway").unwrap();
    let syn_pos = order.iter().position(|s| s == "synapse").unwrap();
    assert!(syn_pos > gw_pos);
}

#[test]
fn boot_sequence_includes_model_services_for_server() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Server);
    assert!(
        steps
            .iter()
            .any(|s| s.stage == BootStage::StartModelServices)
    );
}

#[test]
fn boot_sequence_excludes_model_services_for_minimal() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Minimal);
    assert!(
        !steps
            .iter()
            .any(|s| s.stage == BootStage::StartModelServices)
    );
}

#[test]
fn model_services_stage_after_llm_gateway() {
    assert!(BootStage::StartModelServices > BootStage::StartLlmGateway);
    assert!(BootStage::StartModelServices < BootStage::StartCompositor);
}

#[test]
fn server_service_count_with_synapse() {
    let services = ArgonautInit::default_services(BootMode::Server);
    // postgres, redis, agent-runtime, llm-gateway, synapse = 5
    assert_eq!(services.len(), 5);
}

#[test]
fn desktop_service_count_with_synapse() {
    let services = ArgonautInit::default_services(BootMode::Desktop);
    // postgres, redis, agent-runtime, llm-gateway, synapse, aethersafha, agnoshi = 7
    assert_eq!(services.len(), 7);
}

// -----------------------------------------------------------------------
// Phase 12A: Runlevel tests
// -----------------------------------------------------------------------

#[test]
fn runlevel_to_boot_mode() {
    assert_eq!(Runlevel::Console.to_boot_mode(), Some(BootMode::Server));
    assert_eq!(Runlevel::Graphical.to_boot_mode(), Some(BootMode::Desktop));
    assert_eq!(Runlevel::Container.to_boot_mode(), Some(BootMode::Minimal));
    assert_eq!(Runlevel::Emergency.to_boot_mode(), None);
    assert_eq!(Runlevel::Rescue.to_boot_mode(), None);
}

#[test]
fn runlevel_from_boot_mode() {
    assert_eq!(
        Runlevel::from_boot_mode(BootMode::Server),
        Runlevel::Console
    );
    assert_eq!(
        Runlevel::from_boot_mode(BootMode::Desktop),
        Runlevel::Graphical
    );
    assert_eq!(
        Runlevel::from_boot_mode(BootMode::Minimal),
        Runlevel::Container
    );
}

#[test]
fn runlevel_levels() {
    assert_eq!(Runlevel::Emergency.level(), 0);
    assert_eq!(Runlevel::Rescue.level(), 1);
    assert_eq!(Runlevel::Console.level(), 3);
    assert_eq!(Runlevel::Graphical.level(), 5);
    assert_eq!(Runlevel::Container.level(), 7);
}

#[test]
fn runlevel_display() {
    assert_eq!(format!("{}", Runlevel::Emergency), "emergency");
    assert_eq!(format!("{}", Runlevel::Rescue), "rescue");
    assert_eq!(format!("{}", Runlevel::Console), "console");
    assert_eq!(format!("{}", Runlevel::Graphical), "graphical");
    assert_eq!(format!("{}", Runlevel::Container), "container");
}

// -----------------------------------------------------------------------
// Service target tests
// -----------------------------------------------------------------------

#[test]
fn default_targets_exist() {
    let targets = ServiceTarget::defaults();
    assert_eq!(targets.len(), 5);
    let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"basic"));
    assert!(names.contains(&"network"));
    assert!(names.contains(&"agnos-core"));
    assert!(names.contains(&"graphical"));
    assert!(names.contains(&"edge"));
}

#[test]
fn target_active_in_runlevel() {
    let targets = ServiceTarget::defaults();
    let basic = targets.iter().find(|t| t.name == "basic").unwrap();
    assert!(basic.is_active_in(Runlevel::Console));
    assert!(basic.is_active_in(Runlevel::Graphical));
    assert!(basic.is_active_in(Runlevel::Rescue));
    assert!(!basic.is_active_in(Runlevel::Emergency));

    let graphical = targets.iter().find(|t| t.name == "graphical").unwrap();
    assert!(graphical.is_active_in(Runlevel::Graphical));
    assert!(!graphical.is_active_in(Runlevel::Console));
}

#[test]
fn target_all_services() {
    let targets = ServiceTarget::defaults();
    let network = targets.iter().find(|t| t.name == "network").unwrap();
    let svcs = network.all_services();
    assert!(svcs.contains(&"networkmanager"));
    assert!(svcs.contains(&"nftables"));
    assert!(svcs.contains(&"openssh"));
}

// -----------------------------------------------------------------------
// Shutdown plan tests
// -----------------------------------------------------------------------

#[test]
fn shutdown_plan_poweroff() {
    let config = ArgonautConfig::default();
    let init = ArgonautInit::new(config);
    let plan = init.plan_shutdown(ShutdownType::Poweroff).unwrap();
    assert_eq!(plan.shutdown_type, ShutdownType::Poweroff);
    assert!(!plan.steps.is_empty());
    // Last step should be KernelAction
    let last = plan.steps.last().unwrap();
    assert_eq!(
        last.action,
        ShutdownAction::KernelAction(ShutdownType::Poweroff)
    );
}

#[test]
fn shutdown_plan_reboot() {
    let config = ArgonautConfig::default();
    let init = ArgonautInit::new(config);
    let plan = init.plan_shutdown(ShutdownType::Reboot).unwrap();
    assert_eq!(plan.shutdown_type, ShutdownType::Reboot);
    let last = plan.steps.last().unwrap();
    assert_eq!(
        last.action,
        ShutdownAction::KernelAction(ShutdownType::Reboot)
    );
}

#[test]
fn shutdown_plan_includes_sync() {
    let config = ArgonautConfig::default();
    let init = ArgonautInit::new(config);
    let plan = init.plan_shutdown(ShutdownType::Poweroff).unwrap();
    assert!(
        plan.steps
            .iter()
            .any(|s| s.action == ShutdownAction::SyncFilesystems)
    );
}

#[test]
fn shutdown_plan_includes_unmount() {
    let config = ArgonautConfig::default();
    let init = ArgonautInit::new(config);
    let plan = init.plan_shutdown(ShutdownType::Halt).unwrap();
    assert!(
        plan.steps
            .iter()
            .any(|s| s.action == ShutdownAction::UnmountFilesystems)
    );
}

#[test]
fn shutdown_plan_has_wall_message() {
    let config = ArgonautConfig::default();
    let init = ArgonautInit::new(config);
    let plan = init.plan_shutdown(ShutdownType::Reboot).unwrap();
    assert!(plan.wall_message.is_some());
    assert!(plan.wall_message.unwrap().contains("reboot"));
}

#[test]
fn shutdown_plan_stops_running_services() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Minimal,
        ..ArgonautConfig::default()
    };
    let mut init = ArgonautInit::new(config);
    // Minimal has only agent-runtime with no deps
    init.set_service_state("daimon", ServiceState::Starting);
    init.set_service_state("daimon", ServiceState::Running);

    let plan = init.plan_shutdown(ShutdownType::Poweroff).unwrap();
    let stop_steps: Vec<_> = plan
        .steps
        .iter()
        .filter(|s| matches!(s.action, ShutdownAction::StopService { .. }))
        .collect();
    assert!(!stop_steps.is_empty());
}

#[test]
fn shutdown_plan_includes_luks_close() {
    let config = ArgonautConfig::default();
    let init = ArgonautInit::new(config);
    let plan = init.plan_shutdown(ShutdownType::Poweroff).unwrap();
    assert!(
        plan.steps
            .iter()
            .any(|s| s.action == ShutdownAction::CloseLuks)
    );
}

#[test]
fn shutdown_type_display() {
    assert_eq!(format!("{}", ShutdownType::Poweroff), "poweroff");
    assert_eq!(format!("{}", ShutdownType::Reboot), "reboot");
    assert_eq!(format!("{}", ShutdownType::Halt), "halt");
    assert_eq!(format!("{}", ShutdownType::Kexec), "kexec");
}

#[test]
fn shutdown_step_status_display() {
    assert_eq!(format!("{}", ShutdownStepStatus::Pending), "pending");
    assert_eq!(
        format!("{}", ShutdownStepStatus::Failed("disk busy".into())),
        "failed: disk busy"
    );
}

// -----------------------------------------------------------------------
// Runlevel switch plan tests
// -----------------------------------------------------------------------

#[test]
fn runlevel_switch_console_to_graphical() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Server,
        ..ArgonautConfig::default()
    };
    let init = ArgonautInit::new(config);
    let targets = ServiceTarget::defaults();
    let plan = init.plan_runlevel_switch(Runlevel::Graphical, &targets);
    assert_eq!(plan.from, Runlevel::Console);
    assert_eq!(plan.to, Runlevel::Graphical);
    // Should want to start graphical services
    assert!(plan.services_to_start.contains(&"aethersafha".to_string()));
    assert!(!plan.drop_to_shell);
}

#[test]
fn runlevel_switch_to_emergency_stops_all() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Minimal,
        ..ArgonautConfig::default()
    };
    let mut init = ArgonautInit::new(config);
    // Minimal has agent-runtime with no deps, so state transition works
    init.set_service_state("daimon", ServiceState::Starting);
    init.set_service_state("daimon", ServiceState::Running);

    let targets = ServiceTarget::defaults();
    let plan = init.plan_runlevel_switch(Runlevel::Emergency, &targets);
    assert!(plan.drop_to_shell);
    assert!(plan.services_to_start.is_empty());
    // Should stop running services
    assert!(plan.services_to_stop.contains(&"daimon".to_string()));
}

#[test]
fn runlevel_switch_rescue_drops_to_shell() {
    let config = ArgonautConfig::default();
    let init = ArgonautInit::new(config);
    let targets = ServiceTarget::defaults();
    let plan = init.plan_runlevel_switch(Runlevel::Rescue, &targets);
    assert!(plan.drop_to_shell);
}

// -----------------------------------------------------------------------
// Health tracker tests
// -----------------------------------------------------------------------

#[test]
fn health_tracker_records_pass() {
    use super::types::HealthTracker;
    let mut tracker = HealthTracker::new();
    let should_restart = tracker.record("svc1", true, 3);
    assert!(!should_restart);
    assert_eq!(tracker.failure_count("svc1"), 0);
}

#[test]
fn health_tracker_records_failures() {
    use super::types::HealthTracker;
    let mut tracker = HealthTracker::new();
    assert!(!tracker.record("svc1", false, 3));
    assert_eq!(tracker.failure_count("svc1"), 1);
    assert!(!tracker.record("svc1", false, 3));
    assert_eq!(tracker.failure_count("svc1"), 2);
    // Third failure triggers restart
    assert!(tracker.record("svc1", false, 3));
    assert_eq!(tracker.failure_count("svc1"), 3);
}

#[test]
fn health_tracker_resets_on_pass() {
    use super::types::HealthTracker;
    let mut tracker = HealthTracker::new();
    let _ = tracker.record("svc1", false, 3);
    let _ = tracker.record("svc1", false, 3);
    // Pass resets counter
    let _ = tracker.record("svc1", true, 3);
    assert_eq!(tracker.failure_count("svc1"), 0);
}

#[test]
fn health_tracker_reset_manual() {
    use super::types::HealthTracker;
    let mut tracker = HealthTracker::new();
    let _ = tracker.record("svc1", false, 3);
    let _ = tracker.record("svc1", false, 3);
    tracker.reset("svc1");
    assert_eq!(tracker.failure_count("svc1"), 0);
}

#[test]
fn health_tracker_independent_services() {
    use super::types::HealthTracker;
    let mut tracker = HealthTracker::new();
    let _ = tracker.record("svc1", false, 2);
    let _ = tracker.record("svc2", false, 2);
    assert_eq!(tracker.failure_count("svc1"), 1);
    assert_eq!(tracker.failure_count("svc2"), 1);
    // Only svc1 reaches threshold
    assert!(tracker.record("svc1", false, 2));
    assert!(!tracker.record("svc2", true, 2));
}

// -----------------------------------------------------------------------
// Exit status and event tests
// -----------------------------------------------------------------------

#[test]
fn exit_status_display() {
    assert_eq!(format!("{}", ExitStatus::Code(0)), "exit(0)");
    assert_eq!(format!("{}", ExitStatus::Code(1)), "exit(1)");
    assert_eq!(format!("{}", ExitStatus::Signal(9)), "signal(9)");
    assert_eq!(format!("{}", ExitStatus::Signal(15)), "signal(15)");
    assert_eq!(format!("{}", ExitStatus::Running), "running");
    assert_eq!(format!("{}", ExitStatus::NotStarted), "not-started");
}

#[test]
fn service_event_type_display() {
    assert_eq!(format!("{}", ServiceEventType::Starting), "starting");
    assert_eq!(
        format!("{}", ServiceEventType::Started { pid: 42 }),
        "started(pid=42)"
    );
    assert_eq!(
        format!("{}", ServiceEventType::HealthCheckFailed { consecutive: 3 }),
        "health-fail(3x)"
    );
    assert_eq!(
        format!("{}", ServiceEventType::TimeoutKilled),
        "timeout-killed"
    );
    assert_eq!(
        format!(
            "{}",
            ServiceEventType::CrashDetected {
                exit_status: ExitStatus::Signal(11)
            }
        ),
        "crash(signal(11))"
    );
}

#[test]
fn record_event_creates_event() {
    let config = ArgonautConfig::default();
    let init = ArgonautInit::new(config);
    let event = init.record_event("daimon", ServiceEventType::Starting);
    assert_eq!(event.service, "daimon");
    assert_eq!(event.event_type, ServiceEventType::Starting);
}

// -----------------------------------------------------------------------
// Process spec tests
// -----------------------------------------------------------------------

#[test]
fn process_spec_from_service() {
    use super::types::ProcessSpec;
    let def = ServiceDefinition {
        name: "test-svc".into(),
        description: "test".into(),
        binary_path: PathBuf::from("/usr/bin/test"),
        args: vec!["--flag".into()],
        environment: HashMap::new(),
        depends_on: vec![],
        required_for_modes: vec![BootMode::Server],
        restart_policy: RestartPolicy::Always,
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
    };
    let spec = ProcessSpec::from_service(&def);
    assert_eq!(spec.binary, PathBuf::from("/usr/bin/test"));
    assert_eq!(spec.args, vec!["--flag"]);
    assert!(
        spec.stdout_log
            .unwrap()
            .to_str()
            .unwrap()
            .contains("test-svc")
    );
    assert!(
        spec.stderr_log
            .unwrap()
            .to_str()
            .unwrap()
            .contains("test-svc")
    );
}

// -----------------------------------------------------------------------
// Emergency shell tests
// -----------------------------------------------------------------------

#[test]
fn emergency_shell_default_config() {
    let config = EmergencyShellConfig::default();
    assert_eq!(config.shell_path, PathBuf::from("/usr/bin/agnoshi"));
    assert!(!config.require_auth);
    assert!(config.banner.contains("Emergency"));
    assert_eq!(config.environment.get("SHELL").unwrap(), "/usr/bin/agnoshi");
}

#[test]
fn should_drop_to_emergency_on_required_failure() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Server,
        ..ArgonautConfig::default()
    };
    let mut init = ArgonautInit::new(config);
    // MountFilesystems is required — fail it
    init.mark_step_failed(BootStage::MountFilesystems, "disk error".into());
    assert!(init.should_drop_to_emergency());
}

#[test]
fn no_emergency_without_required_failure() {
    let config = ArgonautConfig::default();
    let init = ArgonautInit::new(config);
    assert!(!init.should_drop_to_emergency());
}

// -----------------------------------------------------------------------
// Boot execution plan tests
// -----------------------------------------------------------------------

#[test]
fn boot_execution_plan_ordered() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Minimal,
        ..ArgonautConfig::default()
    };
    let init = ArgonautInit::new(config);
    let plan = init.boot_execution_plan().unwrap();
    assert!(!plan.is_empty());
    // First service should be agent-runtime (only service in minimal)
    assert_eq!(plan[0].0, "daimon");
}

#[test]
fn boot_execution_plan_server() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Server,
        ..ArgonautConfig::default()
    };
    let init = ArgonautInit::new(config);
    let plan = init.boot_execution_plan().unwrap();
    let names: Vec<&str> = plan.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"daimon"));
    assert!(names.contains(&"llm-gateway"));
    // agent-runtime should come after postgres/redis (dependencies)
    let pg_idx = names.iter().position(|n| *n == "postgres");
    let ar_idx = names.iter().position(|n| *n == "daimon");
    if let (Some(pg), Some(ar)) = (pg_idx, ar_idx) {
        assert!(pg < ar, "postgres should start before agent-runtime");
    }
}

// -----------------------------------------------------------------------
// Crash action tests
// -----------------------------------------------------------------------

#[test]
fn crash_action_always_restarts() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Minimal,
        ..ArgonautConfig::default()
    };
    let init = ArgonautInit::new(config);
    let action = init.on_service_crash("daimon", &ExitStatus::Code(1));
    assert!(matches!(action, CrashAction::Restart { .. }));
}

#[test]
fn crash_action_on_failure_ignores_clean_exit() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Server,
        ..ArgonautConfig::default()
    };
    let init = ArgonautInit::new(config);
    // postgres has OnFailure restart policy
    let action = init.on_service_crash("postgres", &ExitStatus::Code(0));
    assert_eq!(action, CrashAction::Ignore);
}

#[test]
fn crash_action_on_failure_restarts_on_error() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Server,
        ..ArgonautConfig::default()
    };
    let init = ArgonautInit::new(config);
    let action = init.on_service_crash("postgres", &ExitStatus::Code(1));
    assert!(matches!(action, CrashAction::Restart { .. }));
}

#[test]
fn crash_action_unknown_service() {
    let config = ArgonautConfig::default();
    let init = ArgonautInit::new(config);
    let action = init.on_service_crash("nonexistent", &ExitStatus::Code(1));
    assert_eq!(action, CrashAction::Ignore);
}

#[test]
fn crash_action_gives_up_after_limit() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Minimal,
        ..ArgonautConfig::default()
    };
    let mut init = ArgonautInit::new(config);
    // Simulate 5 restarts
    if let Some(svc) = init.services.get_mut("daimon") {
        svc.restart_count = 5;
    }
    let action = init.on_service_crash("daimon", &ExitStatus::Signal(11));
    assert!(matches!(action, CrashAction::GiveUp { .. }));
}

#[test]
fn backoff_delay_exponential() {
    let cfg = RestartConfig::default();
    assert_eq!(cfg.backoff_delay(0), 1000);
    assert_eq!(cfg.backoff_delay(1), 2000);
    assert_eq!(cfg.backoff_delay(2), 4000);
    assert_eq!(cfg.backoff_delay(3), 8000);
    assert_eq!(cfg.backoff_delay(4), 16000);
    // Capped at 30s
    assert_eq!(cfg.backoff_delay(5), 30000);
    assert_eq!(cfg.backoff_delay(10), 30000);
}

#[test]
fn backoff_delay_custom_config() {
    let cfg = RestartConfig {
        max_restarts: 3,
        base_delay_ms: 500,
        max_delay_ms: 10_000,
    };
    assert_eq!(cfg.backoff_delay(0), 500);
    assert_eq!(cfg.backoff_delay(1), 1000);
    assert_eq!(cfg.backoff_delay(2), 2000);
    assert_eq!(cfg.backoff_delay(3), 4000);
    assert_eq!(cfg.backoff_delay(4), 8000);
    assert_eq!(cfg.backoff_delay(5), 10000); // capped
}

#[test]
fn restart_config_limit_exceeded() {
    let cfg = RestartConfig::default(); // max_restarts = 5
    assert!(!cfg.limit_exceeded(0));
    assert!(!cfg.limit_exceeded(4));
    assert!(cfg.limit_exceeded(5));
    assert!(cfg.limit_exceeded(10));
}

#[test]
fn restart_config_zero_means_unlimited() {
    let cfg = RestartConfig {
        max_restarts: 0,
        base_delay_ms: 1000,
        max_delay_ms: 30_000,
    };
    assert!(!cfg.limit_exceeded(0));
    assert!(!cfg.limit_exceeded(100));
    assert!(!cfg.limit_exceeded(u32::MAX));
}

// -----------------------------------------------------------------------
// Phase 14A: Edge boot mode tests
// -----------------------------------------------------------------------

fn edge_config() -> ArgonautConfig {
    ArgonautConfig {
        boot_mode: BootMode::Edge,
        ..Default::default()
    }
}

#[test]
fn boot_sequence_edge_minimal_stages() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Edge);
    let stages: Vec<BootStage> = steps.iter().map(|s| s.stage).collect();
    // Edge gets: MountFS, DevMgr, Verify, Security, AgentRuntime, BootComplete = 6
    assert_eq!(steps.len(), 6);
    assert!(stages.contains(&BootStage::MountFilesystems));
    assert!(stages.contains(&BootStage::VerifyRootfs));
    assert!(stages.contains(&BootStage::StartAgentRuntime));
    assert!(stages.contains(&BootStage::BootComplete));
    // Must NOT have these:
    assert!(!stages.contains(&BootStage::StartDatabaseServices));
    assert!(!stages.contains(&BootStage::StartLlmGateway));
    assert!(!stages.contains(&BootStage::StartModelServices));
    assert!(!stages.contains(&BootStage::StartCompositor));
    assert!(!stages.contains(&BootStage::StartShell));
}

#[test]
fn boot_sequence_edge_fast_timeouts() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Edge);
    let rt_step = steps
        .iter()
        .find(|s| s.stage == BootStage::StartAgentRuntime)
        .unwrap();
    // Edge agent-runtime timeout should be 3s (tight for fast boot)
    assert_eq!(rt_step.timeout_ms, 3000);
    let complete = steps
        .iter()
        .find(|s| s.stage == BootStage::BootComplete)
        .unwrap();
    assert_eq!(complete.timeout_ms, 500);
}

#[test]
fn default_services_edge() {
    let svcs = ArgonautInit::default_services(BootMode::Edge);
    assert_eq!(svcs.len(), 1);
    assert_eq!(svcs[0].name, "daimon");
    assert!(svcs[0].depends_on.is_empty());
    assert!(svcs[0].required_for_modes.contains(&BootMode::Edge));
    // Edge mode env vars
    assert_eq!(svcs[0].environment.get("AGNOS_EDGE_MODE").unwrap(), "1");
    assert_eq!(
        svcs[0].environment.get("AGNOS_READONLY_ROOTFS").unwrap(),
        "1"
    );
}

#[test]
fn edge_services_no_databases() {
    let svcs = ArgonautInit::default_services(BootMode::Edge);
    let names: Vec<&str> = svcs.iter().map(|s| s.name.as_str()).collect();
    assert!(!names.contains(&"postgres"));
    assert!(!names.contains(&"redis"));
    assert!(!names.contains(&"llm-gateway"));
    assert!(!names.contains(&"synapse"));
    assert!(!names.contains(&"aethersafha"));
    assert!(!names.contains(&"agnoshi"));
}

#[test]
fn edge_init_creates_single_service() {
    let init = ArgonautInit::new(edge_config());
    assert_eq!(init.services.len(), 1);
    assert!(init.services.contains_key("daimon"));
}

#[test]
fn edge_boot_can_complete() {
    let mut init = ArgonautInit::new(edge_config());
    for step in &mut init.boot_sequence {
        step.status = BootStepStatus::Complete;
    }
    assert!(init.is_boot_complete());
}

#[test]
fn edge_shutdown_plan() {
    let init = ArgonautInit::new(edge_config());
    let plan = init.plan_shutdown(ShutdownType::Reboot).unwrap();
    assert_eq!(plan.shutdown_type, ShutdownType::Reboot);
    assert!(
        plan.steps
            .iter()
            .any(|s| s.action == ShutdownAction::CloseLuks)
    );
}

#[test]
fn runlevel_edge_mapping() {
    assert_eq!(Runlevel::Edge.to_boot_mode(), Some(BootMode::Edge));
    assert_eq!(Runlevel::from_boot_mode(BootMode::Edge), Runlevel::Edge);
    assert_eq!(Runlevel::Edge.level(), 8);
    assert_eq!(format!("{}", Runlevel::Edge), "edge");
}

#[test]
fn edge_target_active_in_edge_runlevel() {
    let targets = ServiceTarget::defaults();
    let edge = targets.iter().find(|t| t.name == "edge").unwrap();
    assert!(edge.is_active_in(Runlevel::Edge));
    assert!(!edge.is_active_in(Runlevel::Console));
    assert!(!edge.is_active_in(Runlevel::Graphical));
    assert!(edge.requires.contains(&"daimon".to_string()));
    assert!(edge.wants.contains(&"aegis".to_string()));
}

#[test]
fn edge_service_state_lifecycle() {
    let mut init = ArgonautInit::new(edge_config());
    assert!(init.set_service_state("daimon", ServiceState::Starting));
    assert!(init.set_service_state("daimon", ServiceState::Running));
    let stats = init.stats();
    assert_eq!(stats.boot_mode, BootMode::Edge);
    assert_eq!(stats.services_running, 1);
}

// --- Read-only rootfs / dm-verity (Phase 14A-3/4) ---

#[test]
fn readonly_rootfs_returns_five_commands() {
    let cmds = configure_readonly_rootfs();
    assert_eq!(cmds.len(), 5);
}

#[test]
fn readonly_rootfs_remounts_root_ro() {
    let cmds = configure_readonly_rootfs();
    assert_eq!(cmds[0].display(), "mount -o remount,ro /");
}

#[test]
fn readonly_rootfs_tmpfs_noexec() {
    let cmds = configure_readonly_rootfs();
    // /tmp and /var/tmp should have noexec
    assert!(cmds[1].display().contains("noexec"));
    assert!(cmds[4].display().contains("noexec"));
    // /var/run and /var/log should NOT have noexec
    assert!(!cmds[2].display().contains("noexec"));
    assert!(!cmds[3].display().contains("noexec"));
}

#[test]
fn verify_rootfs_integrity_success() {
    let hash = "a".repeat(64);
    let result = verify_rootfs_integrity("/dev/sda1", "/dev/sda2", &hash);
    assert!(result.is_ok());
    let cmds = result.unwrap();
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0].binary, "veritysetup");
    assert_eq!(cmds[0].args[0], "open");
    assert!(
        cmds[0]
            .args
            .contains(&"--restart-on-corruption".to_string())
    );
    assert_eq!(cmds[1].binary, "mount");
    assert!(
        cmds[1]
            .args
            .contains(&"/dev/mapper/verified-root".to_string())
    );
}

#[test]
fn verify_rootfs_integrity_empty_params() {
    let hash = "a".repeat(64);
    assert!(verify_rootfs_integrity("", "/dev/sda2", &hash).is_err());
    assert!(verify_rootfs_integrity("/dev/sda1", "", &hash).is_err());
    assert!(verify_rootfs_integrity("/dev/sda1", "/dev/sda2", "").is_err());
}

#[test]
fn verify_rootfs_integrity_bad_hash_length() {
    let result = verify_rootfs_integrity("/dev/sda1", "/dev/sda2", "abcdef");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("64 hex characters"));

    let long_hash = "a".repeat(128);
    let result = verify_rootfs_integrity("/dev/sda1", "/dev/sda2", &long_hash);
    assert!(result.is_err());
}

#[test]
fn verify_rootfs_integrity_bad_hash_chars() {
    // 64 chars but non-hex
    let hash = "g".repeat(64);
    let result = verify_rootfs_integrity("/dev/sda1", "/dev/sda2", &hash);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("hex characters"));
}

#[test]
fn verify_rootfs_integrity_bad_device_path() {
    let hash = "a".repeat(64);
    // Path without /dev/ prefix
    let result = verify_rootfs_integrity("/tmp/sda1", "/dev/sda2", &hash);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("/dev/"));

    // Path with shell metacharacters
    let result = verify_rootfs_integrity("/dev/sda1; rm -rf /", "/dev/sda2", &hash);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid characters"));
}

#[test]
fn verify_rootfs_integrity_commands_contain_devices() {
    let hash = "b".repeat(64);
    let cmds = verify_rootfs_integrity("/dev/vda1", "/dev/vda2", &hash).unwrap();
    assert!(cmds[0].args.contains(&"/dev/vda1".to_string()));
    assert!(cmds[0].args.contains(&"/dev/vda2".to_string()));
    assert!(cmds[0].args.contains(&hash));
    assert!(cmds[0].args.contains(&"verified-root".to_string()));
}

#[test]
fn verify_rootfs_integrity_mount_is_readonly() {
    let hash = "c".repeat(64);
    let cmds = verify_rootfs_integrity("/dev/sda1", "/dev/sda2", &hash).unwrap();
    assert!(cmds[1].args.contains(&"ro".to_string()));
}

#[test]
fn safe_command_display() {
    let cmd = SafeCommand {
        binary: "mount".to_string(),
        args: vec!["-o".to_string(), "ro".to_string(), "/dev/sda1".to_string()],
    };
    assert_eq!(cmd.display(), "mount -o ro /dev/sda1");
    assert_eq!(cmd.to_string(), "mount -o ro /dev/sda1");
}

// -----------------------------------------------------------------------
// Phase 14D: Edge Security tests
// -----------------------------------------------------------------------

#[test]
fn edge_boot_config_defaults() {
    let cfg = EdgeBootConfig::default();
    assert!(cfg.readonly_rootfs);
    assert!(cfg.luks_enabled);
    assert!(cfg.tpm_attestation);
    assert_eq!(cfg.max_boot_time_ms, 3000);
    assert_eq!(cfg.pcr_bindings, "7+14");
}

#[test]
fn edge_boot_config_custom() {
    let cfg = EdgeBootConfig {
        readonly_rootfs: false,
        luks_enabled: false,
        tpm_attestation: true,
        max_boot_time_ms: 5000,
        pcr_bindings: "7".to_string(),
    };
    assert!(!cfg.readonly_rootfs);
    assert!(!cfg.luks_enabled);
    assert!(cfg.tpm_attestation);
    assert_eq!(cfg.max_boot_time_ms, 5000);
    assert_eq!(cfg.pcr_bindings, "7");
}

#[test]
fn edge_boot_config_serde_roundtrip() {
    let cfg = EdgeBootConfig::default();
    let json = serde_json::to_string(&cfg).unwrap();
    let deserialized: EdgeBootConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.luks_enabled, cfg.luks_enabled);
    assert_eq!(deserialized.readonly_rootfs, cfg.readonly_rootfs);
    assert_eq!(deserialized.tpm_attestation, cfg.tpm_attestation);
    assert_eq!(deserialized.max_boot_time_ms, cfg.max_boot_time_ms);
    assert_eq!(deserialized.pcr_bindings, cfg.pcr_bindings);
}

#[test]
fn default_services_edge_has_luks_env() {
    let svcs = ArgonautInit::default_services(BootMode::Edge);
    assert_eq!(svcs.len(), 1);
    assert_eq!(svcs[0].environment.get("AGNOS_EDGE_LUKS").unwrap(), "1");
}

// -----------------------------------------------------------------------
// Shruti optional service
// -----------------------------------------------------------------------

#[test]
fn shruti_service_definition() {
    let svc = ArgonautInit::shruti_service();
    assert_eq!(svc.name, "shruti");
    assert_eq!(svc.binary_path, PathBuf::from("/usr/local/bin/shruti"));
    assert!(svc.depends_on.contains(&"daimon".into()));
    assert!(svc.depends_on.contains(&"aethersafha".into()));
    assert!(
        svc.required_for_modes.is_empty(),
        "shruti must not auto-start"
    );
    assert_eq!(svc.restart_policy, RestartPolicy::OnFailure);
    assert!(svc.health_check.is_some());
    assert!(svc.ready_check.is_none());
}

#[test]
fn shruti_not_in_default_services() {
    for mode in [
        BootMode::Desktop,
        BootMode::Server,
        BootMode::Minimal,
        BootMode::Edge,
        BootMode::Recovery,
    ] {
        let svcs = ArgonautInit::default_services(mode);
        assert!(
            !svcs.iter().any(|s| s.name == "shruti"),
            "shruti should not appear in default services for {:?}",
            mode,
        );
    }
}

#[test]
fn shruti_optional_service_lookup() {
    assert!(ArgonautInit::optional_service("shruti").is_some());
    assert!(ArgonautInit::optional_service("nonexistent").is_none());
}

#[test]
fn enable_optional_shruti_service() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Desktop,
        ..Default::default()
    };
    let mut init = ArgonautInit::new(config);
    assert!(!init.services.contains_key("shruti"));

    let added = init.enable_optional_service("shruti");
    assert!(added);
    assert!(init.services.contains_key("shruti"));
    assert_eq!(init.services["shruti"].state, ServiceState::Stopped);

    // Second call is a no-op
    let added_again = init.enable_optional_service("shruti");
    assert!(!added_again);
}

#[test]
fn shruti_user_config_service() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Desktop,
        services: vec![ArgonautInit::shruti_service()],
        ..Default::default()
    };
    let init = ArgonautInit::new(config);
    assert!(init.services.contains_key("shruti"));
}

// -----------------------------------------------------------------------
// Recovery boot mode tests
// -----------------------------------------------------------------------

fn recovery_config() -> ArgonautConfig {
    ArgonautConfig {
        boot_mode: BootMode::Recovery,
        ..Default::default()
    }
}

#[test]
fn boot_mode_display_recovery() {
    assert_eq!(BootMode::Recovery.to_string(), "recovery");
}

#[test]
fn boot_sequence_recovery_minimal_stages() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Recovery);
    let stages: Vec<BootStage> = steps.iter().map(|s| s.stage).collect();
    // Recovery: MountFS, DevMgr, Verify, Security, BootComplete = 5
    assert_eq!(steps.len(), 5);
    assert!(stages.contains(&BootStage::MountFilesystems));
    assert!(stages.contains(&BootStage::VerifyRootfs));
    assert!(stages.contains(&BootStage::BootComplete));
    // Must NOT have any service stages
    assert!(!stages.contains(&BootStage::StartAgentRuntime));
    assert!(!stages.contains(&BootStage::StartDatabaseServices));
    assert!(!stages.contains(&BootStage::StartLlmGateway));
    assert!(!stages.contains(&BootStage::StartCompositor));
    assert!(!stages.contains(&BootStage::StartShell));
}

#[test]
fn default_services_recovery_is_empty() {
    let svcs = ArgonautInit::default_services(BootMode::Recovery);
    assert!(svcs.is_empty());
}

#[test]
fn recovery_init_has_no_services() {
    let init = ArgonautInit::new(recovery_config());
    assert!(init.services.is_empty());
}

#[test]
fn recovery_boot_can_complete() {
    let mut init = ArgonautInit::new(recovery_config());
    for step in &mut init.boot_sequence {
        step.status = BootStepStatus::Complete;
    }
    assert!(init.is_boot_complete());
}

#[test]
fn recovery_maps_to_emergency_runlevel() {
    assert_eq!(
        Runlevel::from_boot_mode(BootMode::Recovery),
        Runlevel::Emergency
    );
}

#[test]
fn recovery_shutdown_plan_has_no_service_stops() {
    let init = ArgonautInit::new(recovery_config());
    let plan = init.plan_shutdown(ShutdownType::Reboot).unwrap();
    let stop_steps: Vec<_> = plan
        .steps
        .iter()
        .filter(|s| matches!(s.action, ShutdownAction::StopService { .. }))
        .collect();
    assert!(stop_steps.is_empty());
}

#[test]
fn shruti_not_in_recovery_default_services() {
    let svcs = ArgonautInit::default_services(BootMode::Recovery);
    assert!(!svcs.iter().any(|s| s.name == "shruti"));
}

// -----------------------------------------------------------------------
// Serde roundtrip tests
// -----------------------------------------------------------------------

/// Helper: serialize to JSON and back, assert the JSON values are equal.
/// Uses serde_json::Value comparison to avoid HashMap ordering issues.
fn serde_roundtrip<T>(val: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(val).expect("serialize");
    let _back: T = serde_json::from_str(&json).expect("deserialize");
    let original: serde_json::Value = serde_json::to_value(val).expect("to_value original");
    let roundtrip: serde_json::Value = serde_json::to_value(&_back).expect("to_value roundtrip");
    assert_eq!(original, roundtrip);
}

#[test]
fn serde_boot_mode() {
    for mode in [
        BootMode::Server,
        BootMode::Desktop,
        BootMode::Minimal,
        BootMode::Edge,
        BootMode::Recovery,
    ] {
        serde_roundtrip(&mode);
    }
}

#[test]
fn serde_boot_stage() {
    for stage in [
        BootStage::MountFilesystems,
        BootStage::StartDeviceManager,
        BootStage::VerifyRootfs,
        BootStage::StartSecurity,
        BootStage::StartDatabaseServices,
        BootStage::StartAgentRuntime,
        BootStage::StartLlmGateway,
        BootStage::StartModelServices,
        BootStage::StartCompositor,
        BootStage::StartShell,
        BootStage::BootComplete,
    ] {
        serde_roundtrip(&stage);
    }
}

#[test]
fn serde_boot_step_status() {
    for status in [
        BootStepStatus::Pending,
        BootStepStatus::Running,
        BootStepStatus::Complete,
        BootStepStatus::Failed,
        BootStepStatus::Skipped,
    ] {
        serde_roundtrip(&status);
    }
}

#[test]
fn serde_restart_policy() {
    for policy in [
        RestartPolicy::Always,
        RestartPolicy::OnFailure,
        RestartPolicy::Never,
    ] {
        serde_roundtrip(&policy);
    }
}

#[test]
fn serde_restart_config() {
    serde_roundtrip(&RestartConfig::default());
    serde_roundtrip(&RestartConfig {
        max_restarts: 0,
        base_delay_ms: 500,
        max_delay_ms: 10_000,
    });
}

#[test]
fn serde_service_state() {
    use super::types::ServiceState;
    for state in [
        ServiceState::Stopped,
        ServiceState::Starting,
        ServiceState::Running,
        ServiceState::Stopping,
        ServiceState::Failed("crash".into()),
    ] {
        serde_roundtrip(&state);
    }
}

#[test]
fn serde_exit_status() {
    for status in [
        ExitStatus::Code(0),
        ExitStatus::Code(1),
        ExitStatus::Signal(9),
        ExitStatus::Running,
        ExitStatus::NotStarted,
    ] {
        serde_roundtrip(&status);
    }
}

#[test]
fn serde_shutdown_type() {
    for st in [
        ShutdownType::Poweroff,
        ShutdownType::Reboot,
        ShutdownType::Halt,
        ShutdownType::Kexec,
    ] {
        serde_roundtrip(&st);
    }
}

#[test]
fn serde_runlevel() {
    for rl in [
        Runlevel::Emergency,
        Runlevel::Rescue,
        Runlevel::Console,
        Runlevel::Graphical,
        Runlevel::Container,
        Runlevel::Edge,
    ] {
        serde_roundtrip(&rl);
    }
}

#[test]
fn serde_crash_action() {
    for action in [
        CrashAction::Restart { delay_ms: 1000 },
        CrashAction::Ignore,
        CrashAction::GiveUp {
            reason: "too many".into(),
        },
    ] {
        serde_roundtrip(&action);
    }
}

#[test]
fn serde_health_check_type() {
    for hct in [
        HealthCheckType::HttpGet("http://localhost/health".into()),
        HealthCheckType::TcpConnect("127.0.0.1".into(), 8080),
        HealthCheckType::Command("true".into()),
        HealthCheckType::ProcessAlive,
    ] {
        serde_roundtrip(&hct);
    }
}

#[test]
fn serde_shutdown_step_status() {
    for status in [
        ShutdownStepStatus::Pending,
        ShutdownStepStatus::InProgress,
        ShutdownStepStatus::Complete,
        ShutdownStepStatus::Failed("err".into()),
        ShutdownStepStatus::Skipped,
    ] {
        serde_roundtrip(&status);
    }
}

#[test]
fn serde_shutdown_action() {
    for action in [
        ShutdownAction::WallMessage("shutting down".into()),
        ShutdownAction::NotifyAgents,
        ShutdownAction::StopService {
            name: "daimon".into(),
            signal: 15,
        },
        ShutdownAction::ForceKillService {
            name: "daimon".into(),
        },
        ShutdownAction::SyncFilesystems,
        ShutdownAction::UnmountFilesystems,
        ShutdownAction::SwapOff,
        ShutdownAction::CloseLuks,
        ShutdownAction::KernelAction(ShutdownType::Reboot),
    ] {
        serde_roundtrip(&action);
    }
}

#[test]
fn serde_service_event_type() {
    for evt in [
        ServiceEventType::Starting,
        ServiceEventType::Started { pid: 42 },
        ServiceEventType::HealthCheckPassed,
        ServiceEventType::HealthCheckFailed { consecutive: 3 },
        ServiceEventType::ReadyCheckPassed,
        ServiceEventType::ReadyCheckFailed,
        ServiceEventType::Stopping,
        ServiceEventType::Stopped {
            exit_status: ExitStatus::Code(0),
        },
        ServiceEventType::Restarting { restart_count: 2 },
        ServiceEventType::DependencyWaiting {
            dependency: "pg".into(),
        },
        ServiceEventType::DependencyMet {
            dependency: "pg".into(),
        },
        ServiceEventType::TimeoutKilled,
        ServiceEventType::CrashDetected {
            exit_status: ExitStatus::Signal(11),
        },
    ] {
        serde_roundtrip(&evt);
    }
}

#[test]
fn serde_argonaut_config() {
    serde_roundtrip(&ArgonautConfig::default());
}

#[test]
fn serde_argonaut_stats() {
    let init = ArgonautInit::new(ArgonautConfig::default());
    serde_roundtrip(&init.stats());
}

#[test]
fn serde_edge_boot_config() {
    serde_roundtrip(&EdgeBootConfig::default());
}

// -----------------------------------------------------------------------
// L-9: Path traversal rejection tests
// -----------------------------------------------------------------------

#[test]
fn verify_rootfs_rejects_path_traversal() {
    let hash = "a".repeat(64);
    let result = verify_rootfs_integrity("/dev/../etc/shadow", "/dev/sda2", &hash);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains(".."));
}

#[test]
fn verify_rootfs_rejects_hash_device_traversal() {
    let hash = "a".repeat(64);
    let result = verify_rootfs_integrity("/dev/sda1", "/dev/../etc/crypttab", &hash);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains(".."));
}

#[test]
fn serde_emergency_shell_config() {
    serde_roundtrip(&EmergencyShellConfig::default());
}

#[test]
fn serde_service_definition() {
    let svcs = ArgonautInit::default_services(BootMode::Desktop);
    for svc in &svcs {
        serde_roundtrip(svc);
    }
}

#[test]
fn serde_managed_service() {
    let init = ArgonautInit::new(ArgonautConfig::default());
    for svc in init.services.values() {
        serde_roundtrip(svc);
    }
}

#[test]
fn serde_boot_step() {
    let steps = ArgonautInit::build_boot_sequence(BootMode::Desktop);
    for step in &steps {
        serde_roundtrip(step);
    }
}

#[test]
fn serde_shutdown_plan() {
    let init = ArgonautInit::new(ArgonautConfig::default());
    let plan = init.plan_shutdown(ShutdownType::Poweroff).unwrap();
    serde_roundtrip(&plan);
}

#[test]
fn serde_runlevel_switch_plan() {
    let init = ArgonautInit::new(ArgonautConfig::default());
    let targets = ServiceTarget::defaults();
    let plan = init.plan_runlevel_switch(Runlevel::Console, &targets);
    serde_roundtrip(&plan);
}

#[test]
fn serde_service_target() {
    let targets = ServiceTarget::defaults();
    for target in &targets {
        serde_roundtrip(target);
    }
}

// -----------------------------------------------------------------------
// Process execution integration tests
// -----------------------------------------------------------------------

use std::time::Duration;

/// Create a minimal config with a single service pointing to a real binary.
fn config_with_service(name: &str, binary: &str, args: Vec<&str>) -> ArgonautConfig {
    ArgonautConfig {
        boot_mode: BootMode::Minimal,
        services: vec![ServiceDefinition {
            name: name.into(),
            description: format!("test service {name}"),
            binary_path: PathBuf::from(binary),
            args: args.into_iter().map(String::from).collect(),
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
        }],
        ..Default::default()
    }
}

#[test]
fn start_service_spawns_process() {
    let config = config_with_service("sleeper", "/usr/bin/sleep", vec!["60"]);
    let mut init = ArgonautInit::new(config);
    let pid = init.start_service("sleeper").unwrap();
    assert!(pid > 0);
    assert_eq!(
        init.get_service_state("sleeper"),
        Some(&ServiceState::Running)
    );
    assert_eq!(init.services.get("sleeper").unwrap().pid, Some(pid));

    // Cleanup
    init.stop_service("sleeper", Duration::from_secs(2))
        .unwrap();
}

#[test]
fn stop_service_terminates_process() {
    let config = config_with_service("sleeper", "/usr/bin/sleep", vec!["60"]);
    let mut init = ArgonautInit::new(config);
    init.start_service("sleeper").unwrap();

    let code = init
        .stop_service("sleeper", Duration::from_secs(2))
        .unwrap();
    assert_ne!(code, 0); // killed by SIGTERM
    assert_eq!(
        init.get_service_state("sleeper"),
        Some(&ServiceState::Stopped)
    );
    assert_eq!(init.services.get("sleeper").unwrap().pid, None);
    assert!(!init.processes.contains("sleeper"));
}

#[test]
fn start_unknown_service_fails() {
    let mut init = ArgonautInit::new(ArgonautConfig {
        boot_mode: BootMode::Minimal,
        ..Default::default()
    });
    let result = init.start_service("nonexistent");
    assert!(result.is_err());
}

#[test]
fn start_service_with_bad_binary_transitions_to_failed() {
    let config = config_with_service("bad", "/nonexistent/binary", vec![]);
    let mut init = ArgonautInit::new(config);
    let result = init.start_service("bad");
    assert!(result.is_err());
    assert!(matches!(
        init.get_service_state("bad"),
        Some(ServiceState::Failed(_))
    ));
}

#[test]
fn restart_service_increments_count() {
    let config = config_with_service("sleeper", "/usr/bin/sleep", vec!["60"]);
    let mut init = ArgonautInit::new(config);
    init.start_service("sleeper").unwrap();

    init.restart_service("sleeper", Duration::from_secs(2))
        .unwrap();
    assert_eq!(init.services.get("sleeper").unwrap().restart_count, 1);
    assert_eq!(
        init.get_service_state("sleeper"),
        Some(&ServiceState::Running)
    );

    // Cleanup
    init.stop_service("sleeper", Duration::from_secs(2))
        .unwrap();
}

#[test]
fn reap_services_detects_exited() {
    let config = config_with_service("fast", "/usr/bin/true", vec![]);
    let mut init = ArgonautInit::new(config);
    init.start_service("fast").unwrap();

    // Wait for it to exit
    std::thread::sleep(Duration::from_millis(200));

    let reaped = init.reap_services();
    assert_eq!(reaped.len(), 1);
    let (ref name, code, ref _action) = reaped[0];
    assert_eq!(name, "fast");
    assert_eq!(code, 0);
    assert_eq!(init.get_service_state("fast"), Some(&ServiceState::Stopped));
}

#[test]
fn reap_services_marks_nonzero_as_failed() {
    let config = config_with_service("fail", "/usr/bin/false", vec![]);
    let mut init = ArgonautInit::new(config);
    init.start_service("fail").unwrap();

    std::thread::sleep(Duration::from_millis(200));

    let reaped = init.reap_services();
    assert_eq!(reaped.len(), 1);
    assert!(matches!(
        init.get_service_state("fail"),
        Some(ServiceState::Failed(_))
    ));
}

#[test]
fn stop_all_services_stops_everything() {
    let mut config = ArgonautConfig {
        boot_mode: BootMode::Minimal,
        services: vec![],
        ..Default::default()
    };
    for i in 0..3 {
        config.services.push(ServiceDefinition {
            name: format!("svc-{i}"),
            description: format!("test {i}"),
            binary_path: PathBuf::from("/usr/bin/sleep"),
            args: vec!["60".into()],
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
        });
    }

    let mut init = ArgonautInit::new(config);
    for i in 0..3 {
        init.start_service(&format!("svc-{i}")).unwrap();
    }

    let results = init.stop_all_services(Duration::from_secs(3));
    assert_eq!(results.len(), 3);
    assert!(init.processes.is_empty());
}

// -----------------------------------------------------------------------
// Shutdown execution tests
// -----------------------------------------------------------------------

#[test]
fn execute_shutdown_stops_running_services() {
    let config = config_with_service("sleeper", "/usr/bin/sleep", vec!["60"]);
    let mut init = ArgonautInit::new(config);
    init.start_service("sleeper").unwrap();

    let plan = init.plan_shutdown(ShutdownType::Poweroff).unwrap();
    let executed = init.execute_shutdown(plan);

    // All steps should be Complete (or non-fatal)
    for step in &executed.steps {
        assert!(
            matches!(
                step.status,
                ShutdownStepStatus::Complete | ShutdownStepStatus::Failed(_)
            ),
            "step '{}' has unexpected status: {:?}",
            step.description,
            step.status,
        );
    }

    // Service should be stopped
    assert_eq!(
        init.get_service_state("sleeper"),
        Some(&ServiceState::Stopped)
    );
    assert!(init.processes.is_empty());
}

#[test]
fn execute_shutdown_completes_with_no_services() {
    let mut init = ArgonautInit::new(recovery_config());
    let plan = init.plan_shutdown(ShutdownType::Halt).unwrap();
    let executed = init.execute_shutdown(plan);

    // All steps should complete (no services to stop)
    let completed = executed
        .steps
        .iter()
        .filter(|s| s.status == ShutdownStepStatus::Complete)
        .count();
    assert!(completed > 0);
}

// -----------------------------------------------------------------------
// Runlevel switch execution tests
// -----------------------------------------------------------------------

#[test]
fn execute_runlevel_switch_stops_and_starts() {
    // Start in server mode with services running
    let config = config_with_service("sleeper", "/usr/bin/sleep", vec!["60"]);
    let mut init = ArgonautInit::new(config);
    init.start_service("sleeper").unwrap();

    // Plan a switch to emergency — should stop everything
    let targets = ServiceTarget::defaults();
    let plan = init.plan_runlevel_switch(Runlevel::Emergency, &targets);
    let result = init.execute_runlevel_switch(&plan, Duration::from_secs(2));

    assert_eq!(result.to, Runlevel::Emergency);
    assert!(result.drop_to_shell);
    assert!(!result.stopped.is_empty());
    assert!(result.started.is_empty());
    assert!(result.errors.is_empty());
    assert!(init.processes.is_empty());
}

#[test]
fn execute_runlevel_switch_emergency_with_no_services() {
    let mut init = ArgonautInit::new(recovery_config());
    let targets = ServiceTarget::defaults();
    let plan = init.plan_runlevel_switch(Runlevel::Emergency, &targets);
    let result = init.execute_runlevel_switch(&plan, Duration::from_secs(1));

    assert!(result.drop_to_shell);
    assert!(result.stopped.is_empty());
    assert!(result.started.is_empty());
    assert!(result.errors.is_empty());
}

#[test]
fn execute_runlevel_switch_rescue_drops_to_shell() {
    let mut init = ArgonautInit::new(ArgonautConfig::default());
    let targets = ServiceTarget::defaults();
    let plan = init.plan_runlevel_switch(Runlevel::Rescue, &targets);
    let result = init.execute_runlevel_switch(&plan, Duration::from_secs(1));

    assert!(result.drop_to_shell);
}

#[test]
fn serde_runlevel_switch_result() {
    use super::types::RunlevelSwitchResult;
    let result = RunlevelSwitchResult {
        from: Runlevel::Console,
        to: Runlevel::Graphical,
        stopped: vec!["svc-a".into()],
        started: vec!["svc-b".into()],
        errors: vec![],
        drop_to_shell: false,
    };
    serde_roundtrip(&result);
}

// -----------------------------------------------------------------------
// Edge boot execution tests
// -----------------------------------------------------------------------

use super::edge_boot::{
    EdgeBootResult, FleetRegistration, close_luks, unlock_luks, validate_edge_profile,
};

#[test]
fn unlock_luks_generates_commands() {
    let cmds = unlock_luks("/dev/sda3", "agnos-data").unwrap();
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].binary, "cryptsetup");
    assert!(cmds[0].args.contains(&"/dev/sda3".to_string()));
    assert!(cmds[0].args.contains(&"agnos-data".to_string()));
    assert!(cmds[0].args.contains(&"--token-id".to_string()));
    assert!(cmds[0].args.contains(&"0".to_string()));
    assert!(cmds[0].args.contains(&"--tries".to_string()));
    assert!(cmds[0].args.contains(&"1".to_string()));
}

#[test]
fn unlock_luks_rejects_bad_device() {
    assert!(unlock_luks("/tmp/fake", "data").is_err());
    assert!(unlock_luks("/dev/../etc/shadow", "data").is_err());
}

#[test]
fn unlock_luks_rejects_bad_mapped_name() {
    assert!(unlock_luks("/dev/sda3", "").is_err());
    assert!(unlock_luks("/dev/sda3", "name with spaces").is_err());
    assert!(unlock_luks("/dev/sda3", "name;inject").is_err());
}

#[test]
fn close_luks_generates_command() {
    let cmds = close_luks("agnos-data");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].binary, "cryptsetup");
    assert!(cmds[0].args.contains(&"close".to_string()));
    assert!(cmds[0].args.contains(&"agnos-data".to_string()));
}

#[test]
fn validate_edge_profile_passes_good_result() {
    let result = EdgeBootResult {
        rootfs_locked: true,
        verity_verified: true,
        luks_unlocked: true,
        boot_time_ms: 1500,
        within_budget: true,
        errors: vec![],
    };
    // Use a very high memory limit so the test passes on any host
    let violations = validate_edge_profile(&result, 1_000_000);
    assert!(violations.is_empty());
}

#[test]
fn validate_edge_profile_detects_budget_exceeded() {
    let result = EdgeBootResult {
        rootfs_locked: true,
        verity_verified: true,
        luks_unlocked: true,
        boot_time_ms: 5000,
        within_budget: false,
        errors: vec![],
    };
    let violations = validate_edge_profile(&result, 256);
    assert!(violations.iter().any(|v| v.contains("boot time")));
}

#[test]
fn validate_edge_profile_detects_rootfs_unlocked() {
    let result = EdgeBootResult {
        rootfs_locked: false,
        verity_verified: true,
        luks_unlocked: true,
        boot_time_ms: 1000,
        within_budget: true,
        errors: vec![],
    };
    let violations = validate_edge_profile(&result, 256);
    assert!(violations.iter().any(|v| v.contains("rootfs")));
}

#[test]
fn validate_edge_profile_detects_errors() {
    let result = EdgeBootResult {
        rootfs_locked: true,
        verity_verified: false,
        luks_unlocked: true,
        boot_time_ms: 1000,
        within_budget: true,
        errors: vec!["dm-verity failed".into()],
    };
    let violations = validate_edge_profile(&result, 256);
    assert!(violations.iter().any(|v| v.contains("error")));
}

#[test]
fn fleet_registration_to_json() {
    let result = EdgeBootResult {
        rootfs_locked: true,
        verity_verified: true,
        luks_unlocked: true,
        boot_time_ms: 800,
        within_budget: true,
        errors: vec![],
    };
    let reg = FleetRegistration {
        machine_id: "abc123".into(),
        hostname: "edge-node-1".into(),
        boot_mode: "edge".into(),
        verity_active: result.verity_verified,
        luks_active: result.luks_unlocked,
        kernel_version: "6.18.0".into(),
        total_memory_mb: 128,
    };
    let json = reg.to_json().unwrap();
    assert!(json.contains("edge-node-1"));
    assert!(json.contains("abc123"));
    serde_roundtrip(&reg);
}

#[test]
fn edge_config_in_argonaut_config() {
    let config = ArgonautConfig {
        boot_mode: BootMode::Edge,
        edge_boot: EdgeBootConfig {
            readonly_rootfs: true,
            luks_enabled: true,
            tpm_attestation: true,
            max_boot_time_ms: 2000,
            pcr_bindings: "7+14".to_string(),
        },
        ..Default::default()
    };
    assert!(config.edge_boot.tpm_attestation);
    assert_eq!(config.edge_boot.max_boot_time_ms, 2000);
}

// --- API response types ---

#[test]
fn list_services_returns_all() {
    let init = ArgonautInit::new(ArgonautConfig {
        boot_mode: BootMode::Server,
        ..Default::default()
    });
    let list = init.list_services();
    assert!(list.total > 0);
    assert_eq!(list.total, list.services.len());
}

#[test]
fn list_services_counts_correct() {
    let init = ArgonautInit::new(minimal_config());
    let list = init.list_services();
    // All services start as Stopped
    assert_eq!(list.stopped, list.total);
    assert_eq!(list.running, 0);
    assert_eq!(list.failed, 0);
}

#[test]
fn list_services_sorted_by_name() {
    let init = ArgonautInit::new(ArgonautConfig {
        boot_mode: BootMode::Server,
        ..Default::default()
    });
    let list = init.list_services();
    let names: Vec<&str> = list.services.iter().map(|s| s.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
}

#[test]
fn service_status_unknown_returns_none() {
    let init = ArgonautInit::new(minimal_config());
    assert!(init.service_status("nonexistent").is_none());
}

#[test]
fn service_status_known_service() {
    let init = ArgonautInit::new(minimal_config());
    let status = init.service_status("daimon");
    assert!(status.is_some());
    let s = status.unwrap();
    assert_eq!(s.name, "daimon");
    assert_eq!(s.state, ServiceState::Stopped);
    assert!(s.enabled);
}

#[test]
fn system_status_includes_boot_info() {
    let init = ArgonautInit::new(minimal_config());
    let status = init.system_status();
    assert_eq!(status.boot_mode, BootMode::Minimal);
    assert!(!status.boot_complete);
}

#[test]
fn boot_log_returns_all_steps() {
    let init = ArgonautInit::new(minimal_config());
    let log = init.boot_log();
    assert_eq!(log.boot_mode, BootMode::Minimal);
    assert_eq!(log.steps.len(), init.boot_sequence.len());
}

#[test]
fn system_metrics_per_service() {
    let init = ArgonautInit::new(minimal_config());
    let metrics = init.system_metrics();
    assert_eq!(metrics.service_metrics.len(), init.services.len());
    assert_eq!(metrics.boot_mode, BootMode::Minimal);
}

#[test]
fn create_service_from_request_valid() {
    use super::api::ServiceCreateRequest;
    let mut init = ArgonautInit::new(minimal_config());
    let req = ServiceCreateRequest {
        name: "test-svc".into(),
        description: "A test service".into(),
        binary_path: PathBuf::from("/usr/bin/test"),
        args: vec![],
        environment: HashMap::new(),
        depends_on: vec![],
        restart_policy: RestartPolicy::Never,
        restart_config: None,
        health_check: None,
        ready_check: None,
        enabled: true,
        resource_limits: None,
        log_config: None,
    };
    let status = init.create_service_from_request(req).unwrap();
    assert_eq!(status.name, "test-svc");
    assert_eq!(status.state, ServiceState::Stopped);
    assert!(init.service_status("test-svc").is_some());
}

#[test]
fn create_service_from_request_duplicate_fails() {
    use super::api::ServiceCreateRequest;
    let mut init = ArgonautInit::new(minimal_config());
    let req = ServiceCreateRequest {
        name: "daimon".into(),
        description: "Duplicate".into(),
        binary_path: PathBuf::from("/usr/bin/daimon"),
        args: vec![],
        environment: HashMap::new(),
        depends_on: vec![],
        restart_policy: RestartPolicy::Never,
        restart_config: None,
        health_check: None,
        ready_check: None,
        enabled: true,
        resource_limits: None,
        log_config: None,
    };
    assert!(init.create_service_from_request(req).is_err());
}

// --- Enable/Disable ---

#[test]
fn enable_service_sets_flag() {
    let mut init = ArgonautInit::new(minimal_config());
    init.disable_service("daimon").unwrap();
    assert!(!init.get_service("daimon").unwrap().definition.enabled);
    init.enable_service("daimon").unwrap();
    assert!(init.get_service("daimon").unwrap().definition.enabled);
}

#[test]
fn disable_service_sets_flag() {
    let mut init = ArgonautInit::new(minimal_config());
    init.disable_service("daimon").unwrap();
    assert!(!init.get_service("daimon").unwrap().definition.enabled);
}

#[test]
fn disable_unknown_service_returns_error() {
    let mut init = ArgonautInit::new(minimal_config());
    assert!(init.disable_service("nonexistent").is_err());
}

#[test]
fn boot_execution_plan_skips_disabled() {
    let mut init = ArgonautInit::new(minimal_config());
    let plan_before = init.boot_execution_plan().unwrap();
    let count_before = plan_before.len();
    assert!(count_before > 0);

    init.disable_service("daimon").unwrap();
    let plan_after = init.boot_execution_plan().unwrap();
    assert_eq!(plan_after.len(), count_before - 1);
    assert!(!plan_after.iter().any(|(name, _)| name == "daimon"));
}

// --- systemd unit generation ---

#[test]
fn systemd_generate_unit_roundtrip() {
    let svc = dummy_service("test-app", vec![]);
    let unit = super::systemd::generate_unit(&svc);
    assert!(unit.contains("[Unit]"));
    assert!(unit.contains("[Service]"));
    assert!(unit.contains("[Install]"));
    assert!(unit.contains("ExecStart=/usr/bin/test-app"));
}

#[test]
fn systemd_unit_filename() {
    let svc = dummy_service("my-service", vec![]);
    assert_eq!(
        super::systemd::generate_unit_filename(&svc),
        "my-service.service"
    );
}

// --- Security: audit-driven edge case tests ---

#[test]
fn create_service_path_traversal_name_rejected() {
    use super::api::ServiceCreateRequest;
    let mut init = ArgonautInit::new(minimal_config());
    let req = ServiceCreateRequest {
        name: "a..b".into(),
        description: "traversal".into(),
        binary_path: PathBuf::from("/usr/bin/test"),
        args: vec![],
        environment: HashMap::new(),
        depends_on: vec![],
        restart_policy: RestartPolicy::Never,
        restart_config: None,
        health_check: None,
        ready_check: None,
        enabled: true,
        resource_limits: None,
        log_config: None,
    };
    let err = init.create_service_from_request(req).unwrap_err();
    assert!(err.to_string().contains("traversal"));
}

#[test]
fn create_service_relative_binary_path_rejected() {
    use super::api::ServiceCreateRequest;
    let mut init = ArgonautInit::new(minimal_config());
    let req = ServiceCreateRequest {
        name: "test-svc".into(),
        description: "relative path".into(),
        binary_path: PathBuf::from("../evil/binary"),
        args: vec![],
        environment: HashMap::new(),
        depends_on: vec![],
        restart_policy: RestartPolicy::Never,
        restart_config: None,
        health_check: None,
        ready_check: None,
        enabled: true,
        resource_limits: None,
        log_config: None,
    };
    let err = init.create_service_from_request(req).unwrap_err();
    assert!(err.to_string().contains("absolute"));
}

#[test]
fn create_service_empty_name_rejected() {
    use super::api::ServiceCreateRequest;
    let mut init = ArgonautInit::new(minimal_config());
    let req = ServiceCreateRequest {
        name: "".into(),
        description: "empty".into(),
        binary_path: PathBuf::from("/usr/bin/test"),
        args: vec![],
        environment: HashMap::new(),
        depends_on: vec![],
        restart_policy: RestartPolicy::Never,
        restart_config: None,
        health_check: None,
        ready_check: None,
        enabled: true,
        resource_limits: None,
        log_config: None,
    };
    assert!(init.create_service_from_request(req).is_err());
}

#[test]
fn systemd_description_injection_sanitized() {
    let mut svc = dummy_service("test", vec![]);
    svc.description = "legit\nExecStartPre=/bin/evil".into();
    let unit = super::systemd::generate_unit(&svc);
    // Newline must be replaced — ExecStartPre must NOT appear as its own line
    for line in unit.lines() {
        assert!(
            !line.starts_with("ExecStartPre"),
            "injected line found: {line}"
        );
    }
    // The description should be on a single line with the injection collapsed
    assert!(unit.contains("Description=legit ExecStartPre=/bin/evil"));
}

#[test]
fn systemd_env_dollar_escaped() {
    let mut svc = dummy_service("test", vec![]);
    svc.environment
        .insert("PATH".into(), "$HOME/bin:/usr/bin".into());
    let unit = super::systemd::generate_unit(&svc);
    // $ must be escaped to $$ for systemd
    assert!(unit.contains("$$HOME"));
}

#[test]
fn systemd_env_deterministic_ordering() {
    let mut svc = dummy_service("test", vec![]);
    svc.environment.insert("ZZZ".into(), "last".into());
    svc.environment.insert("AAA".into(), "first".into());
    svc.environment.insert("MMM".into(), "middle".into());
    let unit = super::systemd::generate_unit(&svc);
    let aaa_pos = unit.find("AAA").unwrap();
    let mmm_pos = unit.find("MMM").unwrap();
    let zzz_pos = unit.find("ZZZ").unwrap();
    assert!(aaa_pos < mmm_pos);
    assert!(mmm_pos < zzz_pos);
}

#[test]
fn list_services_starting_stopping_counts() {
    let init = ArgonautInit::new(minimal_config());
    let list = init.list_services();
    assert_eq!(list.starting, 0);
    assert_eq!(list.stopping, 0);
    // total should equal sum of all state counts
    assert_eq!(
        list.total,
        list.running + list.starting + list.stopping + list.failed + list.stopped
    );
}

#[test]
fn enable_disable_records_events() {
    let init = ArgonautInit::new(minimal_config());
    // Verify the new event types exist and display correctly
    let event = init.record_event("daimon", ServiceEventType::Enabled);
    assert_eq!(event.event_type, ServiceEventType::Enabled);
    let event = init.record_event("daimon", ServiceEventType::Disabled);
    assert_eq!(event.event_type, ServiceEventType::Disabled);
}

// --- v0.8.0: ServiceType ---

#[test]
fn service_type_display() {
    assert_eq!(ServiceType::Simple.to_string(), "simple");
    assert_eq!(ServiceType::Forking.to_string(), "forking");
    assert_eq!(ServiceType::Oneshot.to_string(), "oneshot");
}

#[test]
fn service_type_default_is_simple() {
    assert_eq!(ServiceType::default(), ServiceType::Simple);
}

#[test]
fn service_type_serde_roundtrip() {
    let types = [
        ServiceType::Simple,
        ServiceType::Forking,
        ServiceType::Oneshot,
    ];
    for st in types {
        let json = serde_json::to_string(&st).unwrap();
        let back: ServiceType = serde_json::from_str(&json).unwrap();
        assert_eq!(st, back);
    }
}

// --- v0.8.0: ResourceLimits ---

#[test]
fn resource_limits_prlimit_commands_all() {
    use super::types::ResourceLimits;
    let limits = ResourceLimits {
        nofile: Some(65536),
        address_space: Some(4_294_967_296),
        nproc: Some(1024),
        core: None,
    };
    let cmds = limits.to_prlimit_commands(42);
    assert_eq!(cmds.len(), 3);
    assert_eq!(cmds[0].binary, "prlimit");
    assert!(cmds[0].args.iter().any(|a| a.contains("--nofile=")));
    assert!(cmds[1].args.iter().any(|a| a.contains("--as=")));
    assert!(cmds[2].args.iter().any(|a| a.contains("--nproc=")));
}

#[test]
fn resource_limits_prlimit_commands_partial() {
    use super::types::ResourceLimits;
    let limits = ResourceLimits {
        nofile: Some(1024),
        address_space: None,
        nproc: None,
        core: None,
    };
    let cmds = limits.to_prlimit_commands(1);
    assert_eq!(cmds.len(), 1);
}

#[test]
fn resource_limits_empty() {
    use super::types::ResourceLimits;
    let limits = ResourceLimits {
        nofile: None,
        address_space: None,
        nproc: None,
        core: None,
    };
    assert!(limits.is_empty());
    assert!(limits.to_prlimit_commands(1).is_empty());
}

// --- v0.8.0: LogConfig ---

#[test]
fn log_config_default() {
    use super::types::LogConfig;
    let cfg = LogConfig::default();
    assert_eq!(cfg.max_size_bytes, 10 * 1024 * 1024);
    assert_eq!(cfg.max_files, 5);
}

// --- v0.8.0: Environment file loading ---

#[test]
fn load_env_file_basic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("env");
    std::fs::write(&path, "KEY1=value1\nKEY2=value2\n").unwrap();
    let env = super::process::load_environment_file(&path).unwrap();
    assert_eq!(env.get("KEY1").unwrap(), "value1");
    assert_eq!(env.get("KEY2").unwrap(), "value2");
}

#[test]
fn load_env_file_comments_and_blanks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("env");
    std::fs::write(&path, "# comment\n\nKEY=val\n  # indented comment\n").unwrap();
    let env = super::process::load_environment_file(&path).unwrap();
    assert_eq!(env.len(), 1);
    assert_eq!(env.get("KEY").unwrap(), "val");
}

#[test]
fn load_env_file_quoted_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("env");
    std::fs::write(&path, "A=\"hello world\"\nB='single quoted'\n").unwrap();
    let env = super::process::load_environment_file(&path).unwrap();
    assert_eq!(env.get("A").unwrap(), "hello world");
    assert_eq!(env.get("B").unwrap(), "single quoted");
}

#[test]
fn load_env_file_value_with_equals() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("env");
    std::fs::write(&path, "CONN=host=localhost port=5432\n").unwrap();
    let env = super::process::load_environment_file(&path).unwrap();
    assert_eq!(env.get("CONN").unwrap(), "host=localhost port=5432");
}

#[test]
fn load_env_file_missing_returns_error() {
    let result = super::process::load_environment_file(Path::new("/nonexistent/env"));
    assert!(result.is_err());
}

#[test]
fn load_env_files_merge_order() {
    let dir = tempfile::tempdir().unwrap();
    let p1 = dir.path().join("env1");
    let p2 = dir.path().join("env2");
    std::fs::write(&p1, "A=first\nB=only1\n").unwrap();
    std::fs::write(&p2, "A=second\nC=only2\n").unwrap();
    let env = super::process::load_environment_files(&[p1, p2]);
    assert_eq!(env.get("A").unwrap(), "second"); // p2 overrides p1
    assert_eq!(env.get("B").unwrap(), "only1");
    assert_eq!(env.get("C").unwrap(), "only2");
}

// --- v0.8.0: Log rotation ---

#[test]
fn log_config_in_process_spec() {
    use super::types::LogConfig;
    // Default service has no log config
    let svc = dummy_service("test", vec![]);
    let spec = ProcessSpec::from_service(&svc);
    assert!(spec.log_config.is_none());

    // With log config set
    let mut svc = dummy_service("test", vec![]);
    svc.log_config = Some(LogConfig {
        max_size_bytes: 5_000_000,
        max_files: 3,
    });
    let spec = ProcessSpec::from_service(&svc);
    let lc = spec.log_config.unwrap();
    assert_eq!(lc.max_size_bytes, 5_000_000);
    assert_eq!(lc.max_files, 3);
}

// --- v0.8.0: Wave-based startup ---

#[test]
fn resolve_waves_no_deps() {
    let services = [
        dummy_service("a", vec![]),
        dummy_service("b", vec![]),
        dummy_service("c", vec![]),
    ];
    let refs: Vec<&ServiceDefinition> = services.iter().collect();
    let waves = ArgonautInit::resolve_service_waves(&refs).unwrap();
    assert_eq!(waves.len(), 1); // All in one wave
    assert_eq!(waves[0].len(), 3);
    // Sorted alphabetically
    assert_eq!(waves[0], vec!["a", "b", "c"]);
}

#[test]
fn resolve_waves_linear_chain() {
    let services = [
        dummy_service("c", vec!["b"]),
        dummy_service("b", vec!["a"]),
        dummy_service("a", vec![]),
    ];
    let refs: Vec<&ServiceDefinition> = services.iter().collect();
    let waves = ArgonautInit::resolve_service_waves(&refs).unwrap();
    assert_eq!(waves.len(), 3); // One per level
    assert_eq!(waves[0], vec!["a"]);
    assert_eq!(waves[1], vec!["b"]);
    assert_eq!(waves[2], vec!["c"]);
}

#[test]
fn resolve_waves_diamond() {
    // a → b, a → c, b → d, c → d
    let services = [
        dummy_service("a", vec![]),
        dummy_service("b", vec!["a"]),
        dummy_service("c", vec!["a"]),
        dummy_service("d", vec!["b", "c"]),
    ];
    let refs: Vec<&ServiceDefinition> = services.iter().collect();
    let waves = ArgonautInit::resolve_service_waves(&refs).unwrap();
    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0], vec!["a"]);
    assert_eq!(waves[1], vec!["b", "c"]); // parallel
    assert_eq!(waves[2], vec!["d"]);
}

#[test]
fn resolve_waves_cycle_detected() {
    let services = [dummy_service("a", vec!["b"]), dummy_service("b", vec!["a"])];
    let refs: Vec<&ServiceDefinition> = services.iter().collect();
    let result = ArgonautInit::resolve_service_waves(&refs);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cycle"));
}

#[test]
fn boot_execution_plan_waves_desktop() {
    let init = ArgonautInit::new(ArgonautConfig {
        boot_mode: BootMode::Desktop,
        ..Default::default()
    });
    let waves = init.boot_execution_plan_waves().unwrap();
    assert!(waves.len() > 1); // Desktop should have multiple waves
    // First wave should be services with no deps (postgres, redis)
    let first_wave_names: Vec<&str> = waves[0].iter().map(|(n, _)| n.as_str()).collect();
    assert!(first_wave_names.contains(&"postgres"));
    assert!(first_wave_names.contains(&"redis"));
}

// --- v0.8.0: PID file reading ---

#[test]
fn read_pid_file_valid() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.pid");
    // Use PID 1 (init, always alive)
    std::fs::write(&path, "1\n").unwrap();
    let pid = super::process::read_pid_file(&path).unwrap();
    assert_eq!(pid, 1);
}

#[test]
fn read_pid_file_invalid_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.pid");
    std::fs::write(&path, "not-a-number\n").unwrap();
    assert!(super::process::read_pid_file(&path).is_err());
}

#[test]
fn read_pid_file_zero_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.pid");
    std::fs::write(&path, "0\n").unwrap();
    assert!(super::process::read_pid_file(&path).is_err());
}

#[test]
fn read_pid_file_missing() {
    assert!(super::process::read_pid_file(Path::new("/nonexistent/pid")).is_err());
}

// --- v0.8.0: service_status includes service_type ---

#[test]
fn service_status_includes_service_type() {
    let init = ArgonautInit::new(minimal_config());
    let status = init.service_status("daimon").unwrap();
    assert_eq!(status.service_type, ServiceType::Simple);
}

#[test]
fn system_metrics_includes_service_type() {
    let init = ArgonautInit::new(minimal_config());
    let metrics = init.system_metrics();
    for svc in &metrics.service_metrics {
        assert_eq!(svc.service_type, ServiceType::Simple);
    }
}

// --- Audit-driven edge case tests ---

#[test]
fn env_file_single_quote_value() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("env");
    // Single character that looks like a quote but isn't paired
    std::fs::write(&path, "A=\"\n").unwrap();
    let env = super::process::load_environment_file(&path).unwrap();
    // Single " is not a pair — value should be the raw quote
    assert_eq!(env.get("A").unwrap(), "\"");
}

#[test]
fn env_file_empty_value() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("env");
    std::fs::write(&path, "EMPTY=\n").unwrap();
    let env = super::process::load_environment_file(&path).unwrap();
    assert_eq!(env.get("EMPTY").unwrap(), "");
}

#[test]
fn env_file_empty_key_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("env");
    std::fs::write(&path, "=value\nGOOD=ok\n").unwrap();
    let env = super::process::load_environment_file(&path).unwrap();
    assert_eq!(env.len(), 1);
    assert_eq!(env.get("GOOD").unwrap(), "ok");
}

#[test]
fn log_config_new_enforces_min_files() {
    use super::types::LogConfig;
    let cfg = LogConfig::new(1024, 0);
    assert_eq!(cfg.max_files, 1); // 0 clamped to 1
}

#[test]
fn resource_limits_prlimit_pid_in_args() {
    use super::types::ResourceLimits;
    let limits = ResourceLimits {
        nofile: Some(1024),
        address_space: None,
        nproc: None,
        core: None,
    };
    let cmds = limits.to_prlimit_commands(12345);
    assert_eq!(cmds.len(), 1);
    assert!(cmds[0].args[0].contains("12345"));
}

#[test]
fn resolve_waves_single_service() {
    let services = [dummy_service("solo", vec![])];
    let refs: Vec<&ServiceDefinition> = services.iter().collect();
    let waves = ArgonautInit::resolve_service_waves(&refs).unwrap();
    assert_eq!(waves.len(), 1);
    assert_eq!(waves[0], vec!["solo"]);
}

#[test]
fn resolve_waves_missing_dependency() {
    let services = [dummy_service("a", vec!["missing"])];
    let refs: Vec<&ServiceDefinition> = services.iter().collect();
    let result = ArgonautInit::resolve_service_waves(&refs);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not defined"));
}

// --- v0.9.0 audit-driven tests ---

#[test]
fn resource_limits_secure_defaults() {
    use super::types::ResourceLimits;
    let limits = ResourceLimits::secure_defaults();
    assert_eq!(limits.core, Some(0));
    assert!(limits.nofile.is_none());
    let cmds = limits.to_prlimit_commands(1);
    assert_eq!(cmds.len(), 1);
    assert!(cmds[0].args.iter().any(|a| a.contains("--core=")));
}

#[test]
fn resource_limits_core_in_prlimit() {
    use super::types::ResourceLimits;
    let limits = ResourceLimits {
        nofile: None,
        address_space: None,
        nproc: None,
        core: Some(0),
    };
    let cmds = limits.to_prlimit_commands(42);
    assert_eq!(cmds.len(), 1);
    assert!(cmds[0].args.iter().any(|a| a.contains("--core=0:0")));
}

#[test]
fn capability_setpriv_no_shell_injection() {
    use super::types::{CapabilityConfig, LinuxCapability};
    let config = CapabilityConfig {
        drop: vec![LinuxCapability::SysAdmin],
    };
    // Binary with spaces — should be a separate arg, not shell-interpreted
    let cmd = config.to_setpriv_command(
        "/usr/bin/my app",
        &["--flag".into(), "arg with space".into()],
    );
    assert_eq!(cmd.binary, "setpriv");
    // Binary and args are separate elements, not concatenated into a shell string
    assert!(cmd.args.contains(&"/usr/bin/my app".to_string()));
    assert!(cmd.args.contains(&"arg with space".to_string()));
}

#[test]
fn socket_activation_listen_fds_only() {
    use super::types::{SocketActivationConfig, SocketSpec, SocketType};
    let config = SocketActivationConfig {
        sockets: vec![SocketSpec {
            address: "127.0.0.1".into(),
            port: 80,
            socket_type: SocketType::Stream,
        }],
    };
    let (key, value) = config.listen_fds_env();
    assert_eq!(key, "LISTEN_FDS");
    assert_eq!(value, "1");
}

#[test]
fn tmpfile_validate_symlink_target_traversal() {
    use super::types::TmpfileEntry;
    let entries = vec![TmpfileEntry::Symlink {
        path: PathBuf::from("/run/link"),
        target: PathBuf::from("/var/../etc/shadow"),
    }];
    let err = super::tmpfiles::validate_tmpfile_entries(&entries).unwrap_err();
    assert!(err.to_string().contains("traversal"));
}

// =======================================================================
// Coverage improvement tests
// =======================================================================

// --- BootStage Display (all variants) ---

#[test]
fn boot_stage_display_all_variants() {
    assert_eq!(BootStage::MountFilesystems.to_string(), "mount-filesystems");
    assert_eq!(
        BootStage::StartDeviceManager.to_string(),
        "start-device-manager"
    );
    assert_eq!(BootStage::VerifyRootfs.to_string(), "verify-rootfs");
    assert_eq!(BootStage::StartSecurity.to_string(), "start-security");
    assert_eq!(
        BootStage::StartDatabaseServices.to_string(),
        "start-database-services"
    );
    assert_eq!(
        BootStage::StartAgentRuntime.to_string(),
        "start-agent-runtime"
    );
    assert_eq!(BootStage::StartLlmGateway.to_string(), "start-llm-gateway");
    assert_eq!(
        BootStage::StartModelServices.to_string(),
        "start-model-services"
    );
    assert_eq!(BootStage::StartCompositor.to_string(), "start-compositor");
    assert_eq!(BootStage::StartShell.to_string(), "start-shell");
    assert_eq!(BootStage::BootComplete.to_string(), "boot-complete");
}

// --- BootStage ordering ---

#[test]
fn boot_stage_ord_all() {
    assert!(BootStage::MountFilesystems < BootStage::StartDeviceManager);
    assert!(BootStage::StartDeviceManager < BootStage::VerifyRootfs);
    assert!(BootStage::VerifyRootfs < BootStage::StartSecurity);
    assert!(BootStage::StartSecurity < BootStage::StartDatabaseServices);
    assert!(BootStage::StartDatabaseServices < BootStage::StartAgentRuntime);
    assert!(BootStage::StartAgentRuntime < BootStage::StartLlmGateway);
    assert!(BootStage::StartLlmGateway < BootStage::StartModelServices);
    assert!(BootStage::StartModelServices < BootStage::StartCompositor);
    assert!(BootStage::StartCompositor < BootStage::StartShell);
    assert!(BootStage::StartShell < BootStage::BootComplete);
    // PartialOrd consistent with Ord
    assert_eq!(
        BootStage::MountFilesystems.partial_cmp(&BootStage::BootComplete),
        Some(std::cmp::Ordering::Less)
    );
}

// --- Runlevel Display (all variants including Edge) ---

#[test]
fn runlevel_display_edge() {
    assert_eq!(Runlevel::Edge.to_string(), "edge");
}

#[test]
fn runlevel_level_edge() {
    assert_eq!(Runlevel::Edge.level(), 8);
}

// --- Runlevel <-> BootMode ---

#[test]
fn runlevel_to_boot_mode_all() {
    assert_eq!(Runlevel::Emergency.to_boot_mode(), None);
    assert_eq!(Runlevel::Rescue.to_boot_mode(), None);
    assert_eq!(Runlevel::Console.to_boot_mode(), Some(BootMode::Server));
    assert_eq!(Runlevel::Graphical.to_boot_mode(), Some(BootMode::Desktop));
    assert_eq!(Runlevel::Container.to_boot_mode(), Some(BootMode::Minimal));
    assert_eq!(Runlevel::Edge.to_boot_mode(), Some(BootMode::Edge));
}

#[test]
fn runlevel_from_boot_mode_all() {
    assert_eq!(
        Runlevel::from_boot_mode(BootMode::Server),
        Runlevel::Console
    );
    assert_eq!(
        Runlevel::from_boot_mode(BootMode::Desktop),
        Runlevel::Graphical
    );
    assert_eq!(
        Runlevel::from_boot_mode(BootMode::Minimal),
        Runlevel::Container
    );
    assert_eq!(Runlevel::from_boot_mode(BootMode::Edge), Runlevel::Edge);
    assert_eq!(
        Runlevel::from_boot_mode(BootMode::Recovery),
        Runlevel::Emergency
    );
}

// --- ShutdownType Display ---

#[test]
fn shutdown_type_display_all() {
    assert_eq!(ShutdownType::Poweroff.to_string(), "poweroff");
    assert_eq!(ShutdownType::Reboot.to_string(), "reboot");
    assert_eq!(ShutdownType::Halt.to_string(), "halt");
    assert_eq!(ShutdownType::Kexec.to_string(), "kexec");
}

// --- ShutdownAction Display ---

#[test]
fn shutdown_action_variants_constructable() {
    // Ensure all variants can be constructed and compared
    let wall = ShutdownAction::WallMessage("test".into());
    let notify = ShutdownAction::NotifyAgents;
    let stop = ShutdownAction::StopService {
        name: "foo".into(),
        signal: 15,
    };
    let force = ShutdownAction::ForceKillService { name: "bar".into() };
    let sync_fs = ShutdownAction::SyncFilesystems;
    let umount = ShutdownAction::UnmountFilesystems;
    let swap = ShutdownAction::SwapOff;
    let luks = ShutdownAction::CloseLuks;
    let kernel = ShutdownAction::KernelAction(ShutdownType::Reboot);

    assert_ne!(wall, notify);
    assert_ne!(stop, force);
    assert_ne!(sync_fs, umount);
    assert_ne!(swap, luks);
    assert_eq!(kernel, ShutdownAction::KernelAction(ShutdownType::Reboot));
}

// --- ShutdownStepStatus Display (all variants) ---

#[test]
fn shutdown_step_status_display_all() {
    assert_eq!(ShutdownStepStatus::Pending.to_string(), "pending");
    assert_eq!(ShutdownStepStatus::InProgress.to_string(), "in-progress");
    assert_eq!(ShutdownStepStatus::Complete.to_string(), "complete");
    assert_eq!(
        ShutdownStepStatus::Failed("oops".into()).to_string(),
        "failed: oops"
    );
    assert_eq!(ShutdownStepStatus::Skipped.to_string(), "skipped");
}

// --- ServiceType Display ---

#[test]
fn service_type_display_all() {
    assert_eq!(ServiceType::Simple.to_string(), "simple");
    assert_eq!(ServiceType::Forking.to_string(), "forking");
    assert_eq!(ServiceType::Oneshot.to_string(), "oneshot");
}

// --- ExitStatus Display ---

#[test]
fn exit_status_display_all() {
    assert_eq!(ExitStatus::Code(0).to_string(), "exit(0)");
    assert_eq!(ExitStatus::Code(1).to_string(), "exit(1)");
    assert_eq!(ExitStatus::Signal(9).to_string(), "signal(9)");
    assert_eq!(ExitStatus::Signal(15).to_string(), "signal(15)");
    assert_eq!(ExitStatus::Running.to_string(), "running");
    assert_eq!(ExitStatus::NotStarted.to_string(), "not-started");
}

// --- ServiceState Display ---

#[test]
fn service_state_display_all() {
    assert_eq!(ServiceState::Stopped.to_string(), "stopped");
    assert_eq!(ServiceState::Starting.to_string(), "starting");
    assert_eq!(ServiceState::Running.to_string(), "running");
    assert_eq!(ServiceState::Stopping.to_string(), "stopping");
    assert_eq!(
        ServiceState::Failed("boom".into()).to_string(),
        "failed: boom"
    );
}

// --- ServiceState valid_transition comprehensive ---

#[test]
fn service_state_valid_transition_comprehensive() {
    // Same state is always OK
    assert!(ServiceState::Stopped.valid_transition(&ServiceState::Stopped));
    assert!(ServiceState::Running.valid_transition(&ServiceState::Running));

    // Valid transitions
    assert!(ServiceState::Stopped.valid_transition(&ServiceState::Starting));
    assert!(ServiceState::Starting.valid_transition(&ServiceState::Running));
    assert!(ServiceState::Starting.valid_transition(&ServiceState::Failed("x".into())));
    assert!(ServiceState::Running.valid_transition(&ServiceState::Stopping));
    assert!(ServiceState::Running.valid_transition(&ServiceState::Failed("x".into())));
    assert!(ServiceState::Stopping.valid_transition(&ServiceState::Stopped));
    assert!(ServiceState::Stopping.valid_transition(&ServiceState::Failed("x".into())));
    assert!(ServiceState::Failed("x".into()).valid_transition(&ServiceState::Starting));
    assert!(ServiceState::Failed("x".into()).valid_transition(&ServiceState::Stopped));

    // Invalid transitions
    assert!(!ServiceState::Stopped.valid_transition(&ServiceState::Running));
    assert!(!ServiceState::Stopped.valid_transition(&ServiceState::Stopping));
    assert!(!ServiceState::Starting.valid_transition(&ServiceState::Stopped));
    assert!(!ServiceState::Starting.valid_transition(&ServiceState::Stopping));
    assert!(!ServiceState::Running.valid_transition(&ServiceState::Starting));
    assert!(!ServiceState::Running.valid_transition(&ServiceState::Stopped));
    assert!(!ServiceState::Stopping.valid_transition(&ServiceState::Starting));
    assert!(!ServiceState::Stopping.valid_transition(&ServiceState::Running));
    assert!(!ServiceState::Failed("x".into()).valid_transition(&ServiceState::Running));
    assert!(!ServiceState::Failed("x".into()).valid_transition(&ServiceState::Stopping));
}

// --- ServiceEventType Display ---

#[test]
fn service_event_type_display_all() {
    assert_eq!(ServiceEventType::Starting.to_string(), "starting");
    assert_eq!(
        ServiceEventType::Started { pid: 42 }.to_string(),
        "started(pid=42)"
    );
    assert_eq!(ServiceEventType::HealthCheckPassed.to_string(), "health-ok");
    assert_eq!(
        ServiceEventType::HealthCheckFailed { consecutive: 3 }.to_string(),
        "health-fail(3x)"
    );
    assert_eq!(ServiceEventType::ReadyCheckPassed.to_string(), "ready");
    assert_eq!(ServiceEventType::ReadyCheckFailed.to_string(), "not-ready");
    assert_eq!(ServiceEventType::Stopping.to_string(), "stopping");
    assert_eq!(
        ServiceEventType::Stopped {
            exit_status: ExitStatus::Code(0)
        }
        .to_string(),
        "stopped(exit(0))"
    );
    assert_eq!(
        ServiceEventType::Restarting { restart_count: 2 }.to_string(),
        "restarting(#2)"
    );
    assert_eq!(
        ServiceEventType::DependencyWaiting {
            dependency: "db".into()
        }
        .to_string(),
        "waiting(db)"
    );
    assert_eq!(
        ServiceEventType::DependencyMet {
            dependency: "db".into()
        }
        .to_string(),
        "dep-met(db)"
    );
    assert_eq!(
        ServiceEventType::TimeoutKilled.to_string(),
        "timeout-killed"
    );
    assert_eq!(
        ServiceEventType::CrashDetected {
            exit_status: ExitStatus::Signal(11)
        }
        .to_string(),
        "crash(signal(11))"
    );
    assert_eq!(ServiceEventType::Enabled.to_string(), "enabled");
    assert_eq!(ServiceEventType::Disabled.to_string(), "disabled");
}

// --- HealthCheckType Display ---

#[test]
fn health_check_type_display_all() {
    assert_eq!(
        HealthCheckType::HttpGet("http://localhost/health".into()).to_string(),
        "http-get(http://localhost/health)"
    );
    assert_eq!(
        HealthCheckType::TcpConnect("127.0.0.1".into(), 8080).to_string(),
        "tcp-connect(127.0.0.1:8080)"
    );
    assert_eq!(
        HealthCheckType::Command("/bin/true".into()).to_string(),
        "command(/bin/true)"
    );
    assert_eq!(HealthCheckType::ProcessAlive.to_string(), "process-alive");
}

// --- SeccompAction Display ---

#[test]
fn seccomp_action_display_all() {
    use super::types::SeccompAction;
    assert_eq!(SeccompAction::Kill.to_string(), "kill");
    assert_eq!(SeccompAction::Trap.to_string(), "trap");
    assert_eq!(SeccompAction::Log.to_string(), "log");
}

// --- SocketType Display ---

#[test]
fn socket_type_display_all() {
    use super::types::SocketType;
    assert_eq!(SocketType::Stream.to_string(), "stream");
    assert_eq!(SocketType::Datagram.to_string(), "dgram");
    assert_eq!(SocketType::SeqPacket.to_string(), "seqpacket");
}

// --- LandlockAccess Display ---

#[test]
fn landlock_access_display_all() {
    use super::types::LandlockAccess;
    assert_eq!(LandlockAccess::NoAccess.to_string(), "none");
    assert_eq!(LandlockAccess::ReadOnly.to_string(), "ro");
    assert_eq!(LandlockAccess::ReadWrite.to_string(), "rw");
}

// --- LinuxCapability Display and as_str ---

#[test]
fn linux_capability_display_all() {
    use super::types::LinuxCapability;
    assert_eq!(
        LinuxCapability::NetBindService.to_string(),
        "cap_net_bind_service"
    );
    assert_eq!(LinuxCapability::SysAdmin.to_string(), "cap_sys_admin");
    assert_eq!(LinuxCapability::DacOverride.to_string(), "cap_dac_override");
    assert_eq!(LinuxCapability::NetRaw.to_string(), "cap_net_raw");
    assert_eq!(LinuxCapability::SysChroot.to_string(), "cap_sys_chroot");
    assert_eq!(LinuxCapability::Setuid.to_string(), "cap_setuid");
    assert_eq!(LinuxCapability::Setgid.to_string(), "cap_setgid");
    assert_eq!(LinuxCapability::Kill.to_string(), "cap_kill");
    assert_eq!(LinuxCapability::SysPtrace.to_string(), "cap_sys_ptrace");
    assert_eq!(LinuxCapability::SysTime.to_string(), "cap_sys_time");
    assert_eq!(LinuxCapability::NetAdmin.to_string(), "cap_net_admin");
    assert_eq!(LinuxCapability::Fowner.to_string(), "cap_fowner");
    assert_eq!(LinuxCapability::Fsetid.to_string(), "cap_fsetid");
}

// --- CapabilityConfig with empty drop list ---

#[test]
fn capability_config_empty_drop_list() {
    use super::types::CapabilityConfig;
    let config = CapabilityConfig { drop: vec![] };
    let cmd = config.to_setpriv_command("/usr/bin/test", &["--arg".into()]);
    assert_eq!(cmd.binary, "setpriv");
    // Should have --no-new-privs, binary, arg but no --bounding-set
    assert!(cmd.args.contains(&"--no-new-privs".to_string()));
    assert!(cmd.args.contains(&"/usr/bin/test".to_string()));
    assert!(cmd.args.contains(&"--arg".to_string()));
    assert!(!cmd.args.iter().any(|a| a.starts_with("--bounding-set")));
}

// --- CapabilityConfig with multiple capabilities ---

#[test]
fn capability_config_multiple_caps() {
    use super::types::{CapabilityConfig, LinuxCapability};
    let config = CapabilityConfig {
        drop: vec![LinuxCapability::SysAdmin, LinuxCapability::NetRaw],
    };
    let cmd = config.to_setpriv_command("/usr/bin/foo", &[]);
    assert!(
        cmd.args
            .iter()
            .any(|a| a.contains("cap_sys_admin") && a.contains("cap_net_raw"))
    );
}

// --- SafeCommand Display ---

#[test]
fn safe_command_display_with_args() {
    let cmd = SafeCommand {
        binary: "echo".into(),
        args: vec!["hello".into(), "world".into()],
    };
    assert_eq!(cmd.to_string(), "echo hello world");
    assert_eq!(cmd.display(), "echo hello world");
}

#[test]
fn safe_command_display_bare() {
    let cmd = SafeCommand {
        binary: "sync".into(),
        args: vec![],
    };
    assert_eq!(cmd.to_string(), "sync");
}

// --- BootMode Display (recovery and edge) ---

#[test]
fn boot_mode_display_edge_variant() {
    assert_eq!(BootMode::Edge.to_string(), "edge");
}

#[test]
fn boot_mode_display_recovery_variant() {
    assert_eq!(BootMode::Recovery.to_string(), "recovery");
}

// --- ServiceTarget::all_services ---

#[test]
fn service_target_all_services() {
    let target = ServiceTarget {
        name: "test".into(),
        description: "test target".into(),
        requires: vec!["a".into(), "b".into()],
        wants: vec!["c".into()],
        active_in: vec![Runlevel::Console],
    };
    let all = target.all_services();
    assert_eq!(all.len(), 3);
    assert!(all.contains(&"a"));
    assert!(all.contains(&"b"));
    assert!(all.contains(&"c"));
}

#[test]
fn service_target_is_active_in_edge() {
    let targets = ServiceTarget::defaults();
    let edge = targets.iter().find(|t| t.name == "edge").unwrap();
    assert!(edge.is_active_in(Runlevel::Edge));
    assert!(!edge.is_active_in(Runlevel::Console));
    assert!(!edge.is_active_in(Runlevel::Emergency));
}

// --- ProcessSpec::from_service ---

#[test]
fn process_spec_from_service_fields() {
    let def = dummy_service("test-svc", vec![]);
    let spec = ProcessSpec::from_service(&def);
    assert_eq!(spec.binary, PathBuf::from("/usr/bin/test-svc"));
    assert!(spec.args.is_empty());
    assert!(
        spec.stdout_log
            .unwrap()
            .to_string_lossy()
            .contains("test-svc.log")
    );
    assert!(
        spec.stderr_log
            .unwrap()
            .to_string_lossy()
            .contains("test-svc.err")
    );
    assert!(spec.uid.is_none());
    assert!(spec.gid.is_none());
    assert!(spec.working_dir.is_none());
}

// --- LogConfig ---

#[test]
fn log_config_new_min_files() {
    use super::types::LogConfig;
    // max_files of 0 should be clamped to 1
    let config = LogConfig::new(1024, 0);
    assert_eq!(config.max_files, 1);
    assert_eq!(config.max_size_bytes, 1024);
}

#[test]
fn log_config_default_values() {
    use super::types::LogConfig;
    let config = LogConfig::default();
    assert_eq!(config.max_size_bytes, 10 * 1024 * 1024);
    assert_eq!(config.max_files, 5);
}

// --- EdgeBootConfig default ---

#[test]
fn edge_boot_config_default() {
    let config = EdgeBootConfig::default();
    assert!(config.readonly_rootfs);
    assert!(config.luks_enabled);
    assert!(config.tpm_attestation);
    assert_eq!(config.max_boot_time_ms, 3000);
    assert_eq!(config.pcr_bindings, "7+14");
}

// --- EmergencyShellConfig default ---

#[test]
fn emergency_shell_config_default() {
    let config = EmergencyShellConfig::default();
    assert_eq!(config.shell_path, PathBuf::from("/usr/bin/agnoshi"));
    assert!(config.environment.contains_key("HOME"));
    assert!(config.environment.contains_key("TERM"));
    assert!(config.environment.contains_key("PATH"));
    assert!(config.environment.contains_key("SHELL"));
    assert!(config.banner.contains("Emergency Shell"));
    assert!(!config.require_auth);
    assert!(config.auth_password_hash.is_none());
}

// --- RestartConfig ---

#[test]
fn restart_config_backoff_with_zero_base() {
    let config = RestartConfig {
        max_restarts: 5,
        base_delay_ms: 0,
        max_delay_ms: 0,
    };
    // Both zero should be clamped to 100
    let delay = config.backoff_delay(0);
    assert_eq!(delay, 100);
}

#[test]
fn restart_config_limit_exceeded_zero_means_infinite() {
    let config = RestartConfig {
        max_restarts: 0,
        base_delay_ms: 1000,
        max_delay_ms: 30_000,
    };
    // max_restarts=0 means never give up
    assert!(!config.limit_exceeded(100));
    assert!(!config.limit_exceeded(u32::MAX));
}

// --- ResourceLimits ---

#[test]
fn resource_limits_is_empty() {
    use super::types::ResourceLimits;
    let limits = ResourceLimits {
        nofile: None,
        address_space: None,
        nproc: None,
        core: None,
    };
    assert!(limits.is_empty());

    let limits2 = ResourceLimits {
        nofile: Some(1024),
        address_space: None,
        nproc: None,
        core: None,
    };
    assert!(!limits2.is_empty());
}

#[test]
fn resource_limits_to_prlimit_all_fields() {
    use super::types::ResourceLimits;
    let limits = ResourceLimits {
        nofile: Some(1024),
        address_space: Some(4096),
        nproc: Some(64),
        core: Some(0),
    };
    let cmds = limits.to_prlimit_commands(100);
    assert_eq!(cmds.len(), 4);
    assert!(cmds[0].args.iter().any(|a| a.contains("--nofile=")));
    assert!(cmds[1].args.iter().any(|a| a.contains("--as=")));
    assert!(cmds[2].args.iter().any(|a| a.contains("--nproc=")));
    assert!(cmds[3].args.iter().any(|a| a.contains("--core=")));
}

#[test]
fn resource_limits_to_prlimit_empty() {
    use super::types::ResourceLimits;
    let limits = ResourceLimits {
        nofile: None,
        address_space: None,
        nproc: None,
        core: None,
    };
    assert!(limits.to_prlimit_commands(1).is_empty());
}

// --- CrashAction ---

#[test]
fn crash_action_constructable() {
    let restart = CrashAction::Restart { delay_ms: 1000 };
    let ignore = CrashAction::Ignore;
    let give_up = CrashAction::GiveUp {
        reason: "too many".into(),
    };
    assert_ne!(restart, ignore);
    assert_ne!(ignore, give_up);
}

// --- HealthTracker ---

#[test]
fn health_tracker_record_and_reset() {
    use super::types::HealthTracker;
    let mut tracker = HealthTracker::new();

    // Passing check resets
    assert!(!tracker.record("svc", true, 3));
    assert_eq!(tracker.failure_count("svc"), 0);

    // Failing checks accumulate
    assert!(!tracker.record("svc", false, 3));
    assert_eq!(tracker.failure_count("svc"), 1);
    assert!(!tracker.record("svc", false, 3));
    assert_eq!(tracker.failure_count("svc"), 2);
    assert!(tracker.record("svc", false, 3)); // hits threshold
    assert_eq!(tracker.failure_count("svc"), 3);

    // Reset clears
    tracker.reset("svc");
    assert_eq!(tracker.failure_count("svc"), 0);
}

#[test]
fn health_tracker_pass_resets_failures() {
    use super::types::HealthTracker;
    let mut tracker = HealthTracker::new();
    let _ = tracker.record("svc", false, 3);
    let _ = tracker.record("svc", false, 3);
    // Now pass — should reset
    let _ = tracker.record("svc", true, 3);
    assert_eq!(tracker.failure_count("svc"), 0);
    // Next failure starts from 0 again
    assert!(!tracker.record("svc", false, 3));
    assert_eq!(tracker.failure_count("svc"), 1);
}

#[test]
fn health_tracker_unknown_service() {
    use super::types::HealthTracker;
    let tracker = HealthTracker::new();
    assert_eq!(tracker.failure_count("nonexistent"), 0);
}

// --- HealthHistory iter with wrapping ---

#[test]
fn health_history_iter_chronological_after_wrap() {
    use super::health::HealthHistory;
    use super::types::HealthCheckResult;

    let mut h = HealthHistory::new(3);
    // Push 5 items into a capacity-3 buffer
    for i in 0..5u64 {
        h.record(
            HealthCheckResult {
                service: "svc".into(),
                check_type: "test".into(),
                passed: true,
                latency_ms: i,
                message: None,
                checked_at: Utc::now(),
            },
            3,
        );
    }
    assert_eq!(h.len(), 3);
    assert_eq!(h.total_checks(), 5);

    // Iter should return items in chronological order (latency 2, 3, 4)
    let latencies: Vec<u64> = h.iter().map(|r| r.latency_ms).collect();
    assert_eq!(latencies, vec![2, 3, 4]);
}

#[test]
fn health_history_latest_after_wrap() {
    use super::health::HealthHistory;
    use super::types::HealthCheckResult;

    let mut h = HealthHistory::new(2);
    for i in 0..4u64 {
        h.record(
            HealthCheckResult {
                service: "svc".into(),
                check_type: "test".into(),
                passed: i % 2 == 0,
                latency_ms: i,
                message: None,
                checked_at: Utc::now(),
            },
            3,
        );
    }
    // Latest should be the last pushed (latency 3)
    let latest = h.latest().unwrap();
    assert_eq!(latest.latency_ms, 3);
    assert!(!latest.passed);
}

#[test]
fn health_history_latest_empty() {
    use super::health::HealthHistory;
    let h = HealthHistory::new(5);
    assert!(h.latest().is_none());
}

#[test]
fn health_history_iter_not_full() {
    use super::health::HealthHistory;
    use super::types::HealthCheckResult;

    let mut h = HealthHistory::new(10);
    for i in 0..3u64 {
        h.record(
            HealthCheckResult {
                service: "svc".into(),
                check_type: "test".into(),
                passed: true,
                latency_ms: i,
                message: None,
                checked_at: Utc::now(),
            },
            3,
        );
    }
    let latencies: Vec<u64> = h.iter().map(|r| r.latency_ms).collect();
    assert_eq!(latencies, vec![0, 1, 2]);
}

// --- HealthState Display ---

#[test]
fn health_state_display_all() {
    use super::health::HealthState;
    assert_eq!(HealthState::Unknown.to_string(), "unknown");
    assert_eq!(HealthState::Healthy.to_string(), "healthy");
    assert_eq!(HealthState::Degraded.to_string(), "degraded");
    assert_eq!(HealthState::Unhealthy.to_string(), "unhealthy");
}

// --- edge_boot: verify_rootfs_integrity validation ---

#[test]
fn verify_rootfs_empty_params() {
    let err = super::edge_boot::verify_rootfs_integrity("", "/dev/sda2", "a".repeat(64).as_str())
        .unwrap_err();
    assert!(err.contains("cannot be empty"));
}

#[test]
fn verify_rootfs_bad_hash_length() {
    let err =
        super::edge_boot::verify_rootfs_integrity("/dev/sda1", "/dev/sda2", "abc").unwrap_err();
    assert!(err.contains("64 hex"));
}

#[test]
fn verify_rootfs_non_hex_hash() {
    let hash = "g".repeat(64);
    let err =
        super::edge_boot::verify_rootfs_integrity("/dev/sda1", "/dev/sda2", &hash).unwrap_err();
    assert!(err.contains("hex characters"));
}

#[test]
fn verify_rootfs_path_traversal() {
    let hash = "a".repeat(64);
    let err = super::edge_boot::verify_rootfs_integrity("/dev/../etc/passwd", "/dev/sda2", &hash)
        .unwrap_err();
    assert!(err.contains(".."));
}

#[test]
fn verify_rootfs_non_dev_path() {
    let hash = "a".repeat(64);
    let err =
        super::edge_boot::verify_rootfs_integrity("/tmp/sda1", "/dev/sda2", &hash).unwrap_err();
    assert!(err.contains("/dev/"));
}

#[test]
fn verify_rootfs_valid() {
    let hash = "a".repeat(64);
    let cmds = super::edge_boot::verify_rootfs_integrity("/dev/sda1", "/dev/sda2", &hash).unwrap();
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0].binary, "veritysetup");
    assert_eq!(cmds[1].binary, "mount");
}

// --- edge_boot: unlock_luks validation ---

#[test]
fn unlock_luks_empty_mapped_name() {
    let err = super::edge_boot::unlock_luks("/dev/sda3", "").unwrap_err();
    assert!(err.contains("mapped name cannot be empty"));
}

#[test]
fn unlock_luks_invalid_mapped_name() {
    let err = super::edge_boot::unlock_luks("/dev/sda3", "foo bar").unwrap_err();
    assert!(err.contains("invalid characters"));
}

#[test]
fn unlock_luks_invalid_device() {
    let err = super::edge_boot::unlock_luks("/tmp/foo", "data").unwrap_err();
    assert!(err.contains("/dev/"));
}

#[test]
fn unlock_luks_valid() {
    let cmds = super::edge_boot::unlock_luks("/dev/sda3", "agnos-data").unwrap();
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].binary, "cryptsetup");
    assert!(cmds[0].args.contains(&"agnos-data".to_string()));
}

// --- edge_boot: close_luks ---

#[test]
fn close_luks_command_structure() {
    let cmds = super::edge_boot::close_luks("agnos-data");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0].binary, "cryptsetup");
    assert!(cmds[0].args.contains(&"close".to_string()));
    assert!(cmds[0].args.contains(&"agnos-data".to_string()));
}

// --- edge_boot: configure_readonly_rootfs ---

#[test]
fn configure_readonly_rootfs_has_mount_commands() {
    let cmds = configure_readonly_rootfs();
    assert!(cmds.len() >= 5);
    // First should remount root as ro
    assert!(cmds[0].args.contains(&"remount,ro".to_string()));
    // Others should be tmpfs mounts
    for cmd in &cmds[1..] {
        assert!(cmd.args.contains(&"tmpfs".to_string()));
    }
}

// --- edge_boot: validate_edge_profile ---

#[test]
fn validate_edge_profile_all_good() {
    use super::edge_boot::{EdgeBootResult, validate_edge_profile};
    let result = EdgeBootResult {
        rootfs_locked: true,
        verity_verified: true,
        luks_unlocked: true,
        boot_time_ms: 500,
        within_budget: true,
        errors: vec![],
    };
    // Use a very large memory limit so the real system passes
    let violations = validate_edge_profile(&result, 1_000_000);
    // Should have no violations (or possibly just a memory one on large machines)
    assert!(violations.is_empty() || violations.iter().all(|v| v.contains("memory")));
}

#[test]
fn validate_edge_profile_multiple_violations() {
    use super::edge_boot::{EdgeBootResult, validate_edge_profile};
    let result = EdgeBootResult {
        rootfs_locked: false,
        verity_verified: false,
        luks_unlocked: false,
        boot_time_ms: 5000,
        within_budget: false,
        errors: vec!["err1".into(), "err2".into()],
    };
    let violations = validate_edge_profile(&result, 1_000_000);
    assert!(violations.iter().any(|v| v.contains("boot time")));
    assert!(violations.iter().any(|v| v.contains("rootfs")));
    assert!(violations.iter().any(|v| v.contains("errors")));
}

// --- edge_boot: FleetRegistration ---

#[test]
fn fleet_registration_from_system() {
    use super::edge_boot::{EdgeBootResult, FleetRegistration};
    let result = EdgeBootResult {
        rootfs_locked: true,
        verity_verified: true,
        luks_unlocked: false,
        boot_time_ms: 100,
        within_budget: true,
        errors: vec![],
    };
    let reg = FleetRegistration::from_system(&result);
    assert_eq!(reg.boot_mode, "edge");
    assert!(reg.verity_active);
    assert!(!reg.luks_active);
    // machine_id and hostname come from /etc, may be empty in test env
    // kernel_version comes from /proc/version
    let json = reg.to_json().unwrap();
    assert!(json.contains("boot_mode"));
    assert!(json.contains("edge"));
}

// --- ArgonautInit: stats with mixed states ---

#[test]
fn stats_with_mixed_service_states() {
    let config = minimal_config();
    let mut init = ArgonautInit::new(config);

    // Set some services to various states
    let names: Vec<String> = init.services.keys().cloned().collect();
    if names.len() >= 2 {
        // Start and run first service
        init.set_service_state(&names[0], ServiceState::Starting);
        init.set_service_state(&names[0], ServiceState::Running);
        init.services.get_mut(&names[0]).unwrap().restart_count = 3;

        // Fail second service
        init.set_service_state(&names[1], ServiceState::Starting);
        init.set_service_state(&names[1], ServiceState::Failed("test".into()));

        let stats = init.stats();
        assert_eq!(stats.services_running, 1);
        assert_eq!(stats.services_failed, 1);
        assert!(stats.total_restarts >= 3);
    }
}

// --- ArgonautInit: boot_duration_ms ---

#[test]
fn boot_duration_ms_with_completed_boot() {
    let config = minimal_config();
    let mut init = ArgonautInit::new(config);

    // Mark first and last stages complete
    init.mark_step_complete(BootStage::MountFilesystems);
    // Skip to BootComplete
    init.mark_step_complete(BootStage::BootComplete);

    // boot_started and boot_completed should both be set
    assert!(init.boot_started.is_some());
    assert!(init.boot_completed.is_some());
    let dur = init.boot_duration_ms();
    assert!(dur.is_some());
    // Duration should be very small since we completed immediately
    assert!(dur.unwrap() < 5000);
}

// --- ArgonautInit: should_drop_to_emergency ---

#[test]
fn should_drop_to_emergency_no_failures() {
    let config = minimal_config();
    let init = ArgonautInit::new(config);
    assert!(!init.should_drop_to_emergency());
}

#[test]
fn should_drop_to_emergency_required_failure() {
    let config = minimal_config();
    let mut init = ArgonautInit::new(config);
    // Fail a required step
    init.mark_step_failed(BootStage::MountFilesystems, "disk failure".into());
    assert!(init.should_drop_to_emergency());
}

#[test]
fn should_drop_to_emergency_optional_failure_only() {
    let config = minimal_config();
    let mut init = ArgonautInit::new(config);
    // Mark a step as not-required and failed
    if let Some(step) = init.boot_sequence.iter_mut().find(|s| !s.required) {
        let stage = step.stage;
        step.required = false;
        init.mark_step_failed(stage, "non-critical".into());
        // Should NOT drop for optional failures
        // (only if no required steps have failed)
        let has_required_failure = init.failed_steps().iter().any(|s| s.required);
        if !has_required_failure {
            assert!(!init.should_drop_to_emergency());
        }
    }
}

// --- ArgonautInit: mark_step_complete / mark_step_failed ---

#[test]
fn mark_step_complete_unknown_stage() {
    let mut init = ArgonautInit::new(ArgonautConfig {
        boot_mode: BootMode::Minimal,
        ..Default::default()
    });
    // StartShell is not in Minimal boot sequence
    let result = init.mark_step_complete(BootStage::StartShell);
    assert!(!result);
}

#[test]
fn mark_step_failed_unknown_stage() {
    let mut init = ArgonautInit::new(ArgonautConfig {
        boot_mode: BootMode::Minimal,
        ..Default::default()
    });
    let result = init.mark_step_failed(BootStage::StartShell, "err".into());
    assert!(!result);
}

// --- ArgonautInit: is_boot_complete ---

#[test]
fn is_boot_complete_fresh() {
    let init = ArgonautInit::new(minimal_config());
    assert!(!init.is_boot_complete());
}

#[test]
fn is_boot_complete_all_done() {
    let mut init = ArgonautInit::new(minimal_config());
    let stages: Vec<BootStage> = init.boot_sequence.iter().map(|s| s.stage).collect();
    for stage in stages {
        init.mark_step_complete(stage);
    }
    assert!(init.is_boot_complete());
}

// --- ArgonautInit: current_stage ---

#[test]
fn current_stage_is_first_pending() {
    let init = ArgonautInit::new(minimal_config());
    let current = init.current_stage().unwrap();
    assert_eq!(current.stage, BootStage::MountFilesystems);
}

#[test]
fn current_stage_advances_after_complete() {
    let mut init = ArgonautInit::new(minimal_config());
    init.mark_step_complete(BootStage::MountFilesystems);
    let current = init.current_stage().unwrap();
    assert_ne!(current.stage, BootStage::MountFilesystems);
}

// --- ArgonautInit: failed_steps ---

#[test]
fn failed_steps_empty_initially() {
    let init = ArgonautInit::new(minimal_config());
    assert!(init.failed_steps().is_empty());
}

// --- ArgonautInit: emergency_shell_config ---

#[test]
fn emergency_shell_config_accessible() {
    let init = ArgonautInit::new(minimal_config());
    let config = init.emergency_shell_config();
    assert!(config.banner.contains("Emergency"));
}

// --- ArgonautInit: runlevel_boot_mode ---

#[test]
fn runlevel_boot_mode_mapping() {
    assert_eq!(
        ArgonautInit::runlevel_boot_mode(Runlevel::Console),
        Some(BootMode::Server)
    );
    assert_eq!(ArgonautInit::runlevel_boot_mode(Runlevel::Emergency), None);
    assert_eq!(
        ArgonautInit::runlevel_boot_mode(Runlevel::Edge),
        Some(BootMode::Edge)
    );
}

// --- ArgonautInit: plan_runlevel_switch ---

#[test]
fn plan_runlevel_switch_to_rescue() {
    let config = server_config();
    let init = ArgonautInit::new(config);
    let targets = ServiceTarget::defaults();
    let plan = init.plan_runlevel_switch(Runlevel::Rescue, &targets);
    assert_eq!(plan.to, Runlevel::Rescue);
    assert!(plan.drop_to_shell);
    // Should start basic services (eudev, dbus, syslogd)
    let basic_target = targets.iter().find(|t| t.name == "basic").unwrap();
    for svc in &basic_target.requires {
        assert!(
            plan.services_to_start.contains(svc),
            "should start {} for rescue",
            svc
        );
    }
}

#[test]
fn plan_runlevel_switch_to_edge() {
    let config = server_config();
    let init = ArgonautInit::new(config);
    let targets = ServiceTarget::defaults();
    let plan = init.plan_runlevel_switch(Runlevel::Edge, &targets);
    assert_eq!(plan.to, Runlevel::Edge);
    assert!(!plan.drop_to_shell);
    // Edge target requires daimon
    assert!(plan.services_to_start.contains(&"daimon".to_string()));
}

#[test]
fn plan_runlevel_switch_to_container() {
    let config = desktop_config();
    let init = ArgonautInit::new(config);
    let targets = ServiceTarget::defaults();
    let plan = init.plan_runlevel_switch(Runlevel::Container, &targets);
    assert_eq!(plan.to, Runlevel::Container);
    assert!(!plan.drop_to_shell);
}

// --- ArgonautInit: shutdown plan ---

#[test]
fn shutdown_plan_poweroff_has_expected_steps() {
    let config = minimal_config();
    let init = ArgonautInit::new(config);
    let plan = init.plan_shutdown(ShutdownType::Poweroff).unwrap();
    assert_eq!(plan.shutdown_type, ShutdownType::Poweroff);
    assert!(plan.wall_message.is_some());
    // Should have at least wall, notify, sync, umount, swapoff, luks, kernel
    assert!(plan.steps.len() >= 7);
    // First step should be wall message
    assert!(matches!(
        plan.steps[0].action,
        ShutdownAction::WallMessage(_)
    ));
    // Last step should be kernel action
    assert!(matches!(
        plan.steps.last().unwrap().action,
        ShutdownAction::KernelAction(ShutdownType::Poweroff)
    ));
}

#[test]
fn shutdown_plan_reboot_variant() {
    let config = minimal_config();
    let init = ArgonautInit::new(config);
    let plan = init.plan_shutdown(ShutdownType::Reboot).unwrap();
    assert_eq!(plan.shutdown_type, ShutdownType::Reboot);
    assert!(matches!(
        plan.steps.last().unwrap().action,
        ShutdownAction::KernelAction(ShutdownType::Reboot)
    ));
}

// --- ArgonautInit: execute_shutdown with no running services ---

#[test]
fn execute_shutdown_no_services() {
    let config = minimal_config();
    let mut init = ArgonautInit::new(config);
    let plan = init.plan_shutdown(ShutdownType::Poweroff).unwrap();
    let result = init.execute_shutdown(plan);
    // All non-service steps should complete
    for step in &result.steps {
        match &step.action {
            ShutdownAction::SyncFilesystems => {
                // sync should succeed
                assert!(
                    matches!(step.status, ShutdownStepStatus::Complete),
                    "sync step should complete, got: {:?}",
                    step.status
                );
            }
            ShutdownAction::StopService { .. } => {
                // No services running, so no stop steps expected
            }
            _ => {
                // Wall, notify, umount, luks, kernel should all complete
                assert!(
                    matches!(
                        step.status,
                        ShutdownStepStatus::Complete | ShutdownStepStatus::Failed(_)
                    ),
                    "step '{}' unexpected status: {:?}",
                    step.description,
                    step.status
                );
            }
        }
    }
}

// --- Process: SpawnedProcess::from_forked_pid ---

#[test]
fn spawned_process_from_forked_pid() {
    let proc = super::process::SpawnedProcess::from_forked_pid("test-svc", 12345);
    assert_eq!(proc.service_name, "test-svc");
    assert_eq!(proc.pid, 12345);
    assert!(proc.forked);
    assert!(proc.stdout_log.is_none());
    assert!(proc.stderr_log.is_none());
}

#[test]
fn spawned_process_from_forked_pid_uptime() {
    let proc = super::process::SpawnedProcess::from_forked_pid("svc", 1);
    let uptime = proc.uptime();
    // Just created, should be very small
    assert!(uptime.as_secs() < 5);
}

// --- ProcessTable ---

#[test]
fn process_table_basic_operations() {
    use super::process::{ProcessTable, SpawnedProcess};
    let mut table = ProcessTable::new();
    assert!(table.is_empty());
    assert_eq!(table.len(), 0);
    assert!(!table.contains("foo"));

    let proc = SpawnedProcess::from_forked_pid("foo", 999);
    table.insert(proc);
    assert!(!table.is_empty());
    assert_eq!(table.len(), 1);
    assert!(table.contains("foo"));
    assert!(table.get("foo").is_some());
    assert!(table.get("bar").is_none());

    let removed = table.remove("foo");
    assert!(removed.is_some());
    assert!(table.is_empty());
}

#[test]
fn process_table_iter() {
    use super::process::{ProcessTable, SpawnedProcess};
    let mut table = ProcessTable::new();
    table.insert(SpawnedProcess::from_forked_pid("a", 1));
    table.insert(SpawnedProcess::from_forked_pid("b", 2));

    let names: Vec<&str> = table.iter().map(|(name, _)| name).collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"a"));
    assert!(names.contains(&"b"));
}

#[test]
fn process_table_get_mut() {
    use super::process::{ProcessTable, SpawnedProcess};
    let mut table = ProcessTable::new();
    table.insert(SpawnedProcess::from_forked_pid("svc", 100));
    let proc = table.get_mut("svc").unwrap();
    assert_eq!(proc.pid, 100);
}

// --- SocketActivationConfig ---

#[test]
fn socket_activation_env_vars() {
    use super::types::{SocketActivationConfig, SocketSpec, SocketType};
    let config = SocketActivationConfig {
        sockets: vec![
            SocketSpec {
                address: "0.0.0.0".into(),
                port: 80,
                socket_type: SocketType::Stream,
            },
            SocketSpec {
                address: "0.0.0.0".into(),
                port: 443,
                socket_type: SocketType::Stream,
            },
        ],
    };
    let vars = config.env_vars(1234);
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0], ("LISTEN_FDS".into(), "2".into()));
    assert_eq!(vars[1], ("LISTEN_PID".into(), "1234".into()));
}

// --- Security module ---

#[test]
fn seccomp_description_basic() {
    use super::security::seccomp_description;
    use super::types::SeccompConfig;
    let desc = seccomp_description(&SeccompConfig::Basic);
    assert!(desc.contains("basic") || desc.contains("Basic"));
}

#[test]
fn seccomp_description_custom() {
    use super::security::seccomp_description;
    use super::types::{SeccompAction, SeccompConfig};
    let desc = seccomp_description(&SeccompConfig::Custom {
        allow: vec!["read".into(), "write".into()],
        deny: vec![("mount".into(), SeccompAction::Kill)],
    });
    assert!(desc.contains("allow") || desc.contains("2"));
}

#[test]
fn landlock_description_test() {
    use super::security::landlock_description;
    use super::types::{LandlockAccess, LandlockConfig, LandlockRule};
    let config = LandlockConfig {
        rules: vec![
            LandlockRule {
                path: PathBuf::from("/tmp"),
                access: LandlockAccess::ReadWrite,
            },
            LandlockRule {
                path: PathBuf::from("/etc"),
                access: LandlockAccess::ReadOnly,
            },
        ],
    };
    let desc = landlock_description(&config);
    assert!(desc.contains("/tmp") || desc.contains("2 rule"));
}

#[test]
fn verify_emergency_auth_no_auth_required() {
    use super::security::verify_emergency_auth;
    let config = EmergencyShellConfig::default();
    // require_auth is false, should always pass
    assert!(verify_emergency_auth(&config, "anything"));
}

#[test]
fn verify_emergency_auth_with_no_hash_configured() {
    use super::security::verify_emergency_auth;
    // require_auth=true but no hash configured => should grant access
    let config = EmergencyShellConfig {
        require_auth: true,
        auth_password_hash: None,
        ..EmergencyShellConfig::default()
    };
    assert!(verify_emergency_auth(&config, "anything"));
}

#[test]
fn generate_socket_env_test() {
    use super::security::generate_socket_env;
    use super::types::{SocketActivationConfig, SocketSpec, SocketType};
    let config = SocketActivationConfig {
        sockets: vec![SocketSpec {
            address: "127.0.0.1".into(),
            port: 8080,
            socket_type: SocketType::Datagram,
        }],
    };
    let vars = generate_socket_env(&config, 42);
    assert!(vars.iter().any(|(k, _)| k == "LISTEN_FDS"));
    assert!(vars.iter().any(|(k, _)| k == "LISTEN_PID"));
}

// --- Health check command execution (safe tests using /usr/bin/true and /usr/bin/false) ---

#[test]
fn execute_health_check_command_true() {
    use super::health::execute_health_check;
    use super::types::{HealthCheck, HealthCheckType};
    let check = HealthCheck {
        check_type: HealthCheckType::Command("/usr/bin/true".into()),
        interval_ms: 1000,
        timeout_ms: 5000,
        retries: 1,
    };
    let result = execute_health_check("test-svc", &check, None);
    assert!(result.passed);
    assert_eq!(result.service, "test-svc");
}

#[test]
fn execute_health_check_command_false() {
    use super::health::execute_health_check;
    use super::types::{HealthCheck, HealthCheckType};
    let check = HealthCheck {
        check_type: HealthCheckType::Command("/usr/bin/false".into()),
        interval_ms: 1000,
        timeout_ms: 5000,
        retries: 1,
    };
    let result = execute_health_check("test-svc", &check, None);
    assert!(!result.passed);
}

#[test]
fn execute_health_check_process_alive_self() {
    use super::health::execute_health_check;
    use super::types::{HealthCheck, HealthCheckType};
    let check = HealthCheck {
        check_type: HealthCheckType::ProcessAlive,
        interval_ms: 1000,
        timeout_ms: 5000,
        retries: 1,
    };
    let result = execute_health_check("test-svc", &check, Some(std::process::id()));
    assert!(result.passed);
}

#[test]
fn execute_health_check_process_alive_no_pid() {
    use super::health::execute_health_check;
    use super::types::{HealthCheck, HealthCheckType};
    let check = HealthCheck {
        check_type: HealthCheckType::ProcessAlive,
        interval_ms: 1000,
        timeout_ms: 5000,
        retries: 1,
    };
    let result = execute_health_check("test-svc", &check, None);
    assert!(!result.passed);
}

#[test]
fn execute_health_check_https_rejected() {
    use super::health::execute_health_check;
    use super::types::{HealthCheck, HealthCheckType};
    let check = HealthCheck {
        check_type: HealthCheckType::HttpGet("https://localhost/health".into()),
        interval_ms: 1000,
        timeout_ms: 500,
        retries: 1,
    };
    let result = execute_health_check("test-svc", &check, None);
    assert!(!result.passed);
    assert!(result.message.as_ref().unwrap().contains("HTTPS"));
}

#[test]
fn execute_health_check_tcp_refused() {
    use super::health::execute_health_check;
    use super::types::{HealthCheck, HealthCheckType};
    let check = HealthCheck {
        check_type: HealthCheckType::TcpConnect("127.0.0.1".into(), 1),
        interval_ms: 1000,
        timeout_ms: 500,
        retries: 1,
    };
    let result = execute_health_check("test-svc", &check, None);
    assert!(!result.passed);
}

// --- Ready check (using /usr/bin/true) ---

#[test]
fn execute_ready_check_passes() {
    use super::health::execute_ready_check;
    use super::types::{HealthCheckType, ReadyCheck};
    let check = ReadyCheck {
        check_type: HealthCheckType::Command("/usr/bin/true".into()),
        timeout_ms: 5000,
        retries: 2,
        retry_delay_ms: 10,
    };
    let result = execute_ready_check("test-svc", &check, None);
    assert!(result.passed);
}

#[test]
fn execute_ready_check_fails_after_retries() {
    use super::health::execute_ready_check;
    use super::types::{HealthCheckType, ReadyCheck};
    let check = ReadyCheck {
        check_type: HealthCheckType::Command("/usr/bin/false".into()),
        timeout_ms: 5000,
        retries: 1,
        retry_delay_ms: 10,
    };
    let result = execute_ready_check("test-svc", &check, None);
    assert!(!result.passed);
    assert!(result.message.as_ref().unwrap().contains("retries"));
}

// --- Serde round-trips ---

#[test]
fn serde_roundtrip_boot_mode() {
    for mode in [
        BootMode::Server,
        BootMode::Desktop,
        BootMode::Minimal,
        BootMode::Edge,
        BootMode::Recovery,
    ] {
        let json = serde_json::to_string(&mode).unwrap();
        let back: BootMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, back);
    }
}

#[test]
fn serde_roundtrip_runlevel() {
    for rl in [
        Runlevel::Emergency,
        Runlevel::Rescue,
        Runlevel::Console,
        Runlevel::Graphical,
        Runlevel::Container,
        Runlevel::Edge,
    ] {
        let json = serde_json::to_string(&rl).unwrap();
        let back: Runlevel = serde_json::from_str(&json).unwrap();
        assert_eq!(rl, back);
    }
}

#[test]
fn serde_roundtrip_shutdown_type() {
    for st in [
        ShutdownType::Poweroff,
        ShutdownType::Reboot,
        ShutdownType::Halt,
        ShutdownType::Kexec,
    ] {
        let json = serde_json::to_string(&st).unwrap();
        let back: ShutdownType = serde_json::from_str(&json).unwrap();
        assert_eq!(st, back);
    }
}

#[test]
fn serde_roundtrip_service_state() {
    for state in [
        ServiceState::Stopped,
        ServiceState::Starting,
        ServiceState::Running,
        ServiceState::Stopping,
        ServiceState::Failed("test err".into()),
    ] {
        let json = serde_json::to_string(&state).unwrap();
        let back: ServiceState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, back);
    }
}

#[test]
fn serde_roundtrip_argonaut_config() {
    let config = ArgonautConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let back: ArgonautConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.boot_mode, config.boot_mode);
    assert_eq!(back.boot_timeout_ms, config.boot_timeout_ms);
}

// --- to_capability_commands ---

#[test]
fn to_capability_commands_generates_setpriv() {
    use super::security::to_capability_commands;
    use super::types::{CapabilityConfig, LinuxCapability};
    let config = CapabilityConfig {
        drop: vec![LinuxCapability::SysAdmin, LinuxCapability::NetRaw],
    };
    let cmds = to_capability_commands(&config, "/usr/bin/test", &["--flag".into()]);
    assert_eq!(cmds.binary, "setpriv");
    assert!(cmds.args.contains(&"--no-new-privs".to_string()));
}
