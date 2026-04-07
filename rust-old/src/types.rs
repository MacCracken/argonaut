//! Argonaut type definitions — enums, structs, and configuration types.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Boot mode
// ---------------------------------------------------------------------------

/// Which mode to boot into. Determines which services and boot stages
/// are executed.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BootMode {
    /// Headless server: agent-runtime + llm-gateway, no compositor.
    Server,
    /// Full desktop: agent-runtime + llm-gateway + compositor + shell.
    Desktop,
    /// Bare minimum: agent-runtime only (container/embedded use).
    Minimal,
    /// Edge: constrained hardware (RPi, NUC, IoT). Boots daimon + edge agent
    /// only. No compositor, no shell, no LLM gateway. Read-only rootfs,
    /// dm-verity enforced, LUKS enabled, minimal seccomp profile.
    /// Target: <256 MB disk, <128 MB RAM, <3s boot.
    Edge,
    /// Recovery: emergency shell only. No services started. Used for
    /// broken system repair when the normal boot path is unusable.
    Recovery,
}

impl fmt::Display for BootMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Server => write!(f, "server"),
            Self::Desktop => write!(f, "desktop"),
            Self::Minimal => write!(f, "minimal"),
            Self::Edge => write!(f, "edge"),
            Self::Recovery => write!(f, "recovery"),
        }
    }
}

// ---------------------------------------------------------------------------
// Edge boot configuration (Phase 14D)
// ---------------------------------------------------------------------------

/// Security and performance configuration for Edge boot mode.
///
/// Controls LUKS full-disk encryption, TPM attestation requirements,
/// read-only rootfs enforcement, and maximum allowed boot time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeBootConfig {
    /// Whether the rootfs should be mounted read-only (dm-verity enforced).
    pub readonly_rootfs: bool,
    /// Whether LUKS full-disk encryption is enabled for the data partition.
    pub luks_enabled: bool,
    /// Whether TPM 2.0 attestation is required during boot.
    pub tpm_attestation: bool,
    /// Maximum boot time in milliseconds before the watchdog triggers.
    pub max_boot_time_ms: u64,
    /// TPM2 PCR bindings for LUKS unlock (e.g., "7+14" for Secure Boot policy + MOK).
    /// Empty string means use default token without PCR policy.
    pub pcr_bindings: String,
}

impl Default for EdgeBootConfig {
    fn default() -> Self {
        Self {
            readonly_rootfs: true,
            luks_enabled: true,
            tpm_attestation: true,
            max_boot_time_ms: 3000,
            pcr_bindings: "7+14".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Boot stages
// ---------------------------------------------------------------------------

/// Ordered boot stages. The init system walks through these in order,
/// skipping stages that are not relevant to the current [`BootMode`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BootStage {
    MountFilesystems,
    StartDeviceManager,
    VerifyRootfs,
    StartSecurity,
    StartDatabaseServices,
    StartAgentRuntime,
    StartLlmGateway,
    StartModelServices,
    StartCompositor,
    StartShell,
    BootComplete,
}

impl BootStage {
    /// Numeric order for sorting (lower = earlier).
    #[must_use]
    pub(crate) fn order(self) -> u8 {
        match self {
            Self::MountFilesystems => 0,
            Self::StartDeviceManager => 1,
            Self::VerifyRootfs => 2,
            Self::StartSecurity => 3,
            Self::StartDatabaseServices => 4,
            Self::StartAgentRuntime => 5,
            Self::StartLlmGateway => 6,
            Self::StartModelServices => 7,
            Self::StartCompositor => 8,
            Self::StartShell => 9,
            Self::BootComplete => 10,
        }
    }
}

impl fmt::Display for BootStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MountFilesystems => write!(f, "mount-filesystems"),
            Self::StartDeviceManager => write!(f, "start-device-manager"),
            Self::VerifyRootfs => write!(f, "verify-rootfs"),
            Self::StartSecurity => write!(f, "start-security"),
            Self::StartDatabaseServices => write!(f, "start-database-services"),
            Self::StartAgentRuntime => write!(f, "start-agent-runtime"),
            Self::StartLlmGateway => write!(f, "start-llm-gateway"),
            Self::StartModelServices => write!(f, "start-model-services"),
            Self::StartCompositor => write!(f, "start-compositor"),
            Self::StartShell => write!(f, "start-shell"),
            Self::BootComplete => write!(f, "boot-complete"),
        }
    }
}

impl PartialOrd for BootStage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BootStage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order().cmp(&other.order())
    }
}

// ---------------------------------------------------------------------------
// Boot step status
// ---------------------------------------------------------------------------

/// Status of an individual boot step.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootStepStatus {
    Pending,
    Running,
    Complete,
    Failed,
    Skipped,
}

impl fmt::Display for BootStepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Complete => write!(f, "complete"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

// ---------------------------------------------------------------------------
// Boot step
// ---------------------------------------------------------------------------

/// A single step in the boot sequence.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootStep {
    /// Which stage this step represents.
    pub stage: BootStage,
    /// Human-readable description of the step.
    pub description: String,
    /// If true, failure aborts the entire boot.
    pub required: bool,
    /// Maximum time this step may take (milliseconds).
    pub timeout_ms: u64,
    /// Current status.
    pub status: BootStepStatus,
    /// When the step started executing.
    pub started_at: Option<DateTime<Utc>>,
    /// When the step finished (success or failure).
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message, if any.
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Restart policy
// ---------------------------------------------------------------------------

/// How the init system should handle a service that exits.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RestartPolicy {
    /// Always restart, regardless of exit code.
    Always,
    /// Restart only on non-zero exit (default).
    #[default]
    OnFailure,
    /// Never restart; the service is one-shot.
    Never,
}

impl fmt::Display for RestartPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Always => write!(f, "always"),
            Self::OnFailure => write!(f, "on-failure"),
            Self::Never => write!(f, "never"),
        }
    }
}

/// Configuration for restart backoff and limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartConfig {
    /// Maximum number of restarts before giving up. 0 = never give up.
    pub max_restarts: u32,
    /// Base delay in milliseconds for the first restart.
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds (cap for exponential backoff).
    pub max_delay_ms: u64,
}

impl Default for RestartConfig {
    fn default() -> Self {
        Self {
            max_restarts: 5,
            base_delay_ms: 1000,
            max_delay_ms: 30_000,
        }
    }
}

impl RestartConfig {
    /// Compute exponential backoff delay for the given restart count.
    /// Returns at least 100ms to prevent busy-retry loops even if
    /// `base_delay_ms` or `max_delay_ms` are misconfigured to 0.
    #[must_use]
    pub fn backoff_delay(&self, restart_count: u32) -> u64 {
        let base = self.base_delay_ms.max(100);
        let cap = self.max_delay_ms.max(100);
        let delay = base.saturating_mul(2u64.saturating_pow(restart_count));
        delay.min(cap)
    }

    /// Whether the restart limit has been exceeded.
    #[must_use]
    pub fn limit_exceeded(&self, restart_count: u32) -> bool {
        self.max_restarts > 0 && restart_count >= self.max_restarts
    }
}

// ---------------------------------------------------------------------------
// Health / ready checks
// ---------------------------------------------------------------------------

/// Method used to check health or readiness.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthCheckType {
    /// HTTP GET against a URL — expects 2xx.
    HttpGet(String),
    /// TCP connect to host:port.
    TcpConnect(String, u16),
    /// Run a shell command — 0 exit = healthy.
    /// The command string is split on whitespace. For arguments containing
    /// spaces, use a wrapper script instead.
    Command(String),
    /// Simply check if the PID is still alive.
    ProcessAlive,
}

impl fmt::Display for HealthCheckType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpGet(url) => write!(f, "http-get({})", url),
            Self::TcpConnect(host, port) => write!(f, "tcp-connect({}:{})", host, port),
            Self::Command(cmd) => write!(f, "command({})", cmd),
            Self::ProcessAlive => write!(f, "process-alive"),
        }
    }
}

/// Periodic health check for a running service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub check_type: HealthCheckType,
    /// Milliseconds between checks.
    pub interval_ms: u64,
    /// Per-check timeout (milliseconds).
    pub timeout_ms: u64,
    /// How many consecutive failures before declaring unhealthy.
    pub retries: u32,
}

/// One-shot readiness check executed at startup. The service is not
/// considered "running" until the ready check passes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyCheck {
    pub check_type: HealthCheckType,
    /// Maximum time to wait for readiness (milliseconds).
    pub timeout_ms: u64,
    /// Number of retries before giving up.
    pub retries: u32,
    /// Delay between retry attempts (milliseconds).
    pub retry_delay_ms: u64,
}

// ---------------------------------------------------------------------------
// Service types and configuration
// ---------------------------------------------------------------------------

/// How a service process behaves after being spawned.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    /// Long-running daemon. Argonaut supervises the process, restarts
    /// on crash, and runs periodic health checks.
    #[default]
    Simple,
    /// Forking daemon (e.g. PostgreSQL). The spawned parent exits and
    /// the real service runs as a child. Argonaut reads the child PID
    /// from a PID file or sd_notify `MAINPID`.
    Forking,
    /// Run-to-completion task. Argonaut spawns the process, waits for
    /// it to exit, and marks the service as Stopped (exit 0) or Failed.
    /// No health checks, no supervision, no restart by default.
    Oneshot,
}

impl fmt::Display for ServiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simple => write!(f, "simple"),
            Self::Forking => write!(f, "forking"),
            Self::Oneshot => write!(f, "oneshot"),
        }
    }
}

/// Per-service resource limits applied via `prlimit(1)` after spawn.
///
/// Values are in the unit expected by the kernel (bytes for memory,
/// count for file descriptors and processes). Each limit sets both
/// soft and hard to the same value.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// `RLIMIT_NOFILE` — maximum open file descriptors.
    pub nofile: Option<u64>,
    /// `RLIMIT_AS` — maximum virtual memory (bytes).
    pub address_space: Option<u64>,
    /// `RLIMIT_NPROC` — maximum number of processes.
    pub nproc: Option<u64>,
    /// `RLIMIT_CORE` — maximum core dump size (bytes). Set to `0` to disable core dumps.
    pub core: Option<u64>,
}

impl ResourceLimits {
    /// Generate `prlimit` commands to apply these limits to a running process.
    ///
    /// Uses the `prlimit` CLI tool to set limits without requiring
    /// `unsafe` code (no `Command::pre_exec` needed).
    #[must_use]
    pub fn to_prlimit_commands(&self, pid: u32) -> Vec<SafeCommand> {
        let mut cmds = Vec::new();
        let pid_str = pid.to_string();

        if let Some(nofile) = self.nofile {
            let val = format!("{nofile}:{nofile}");
            cmds.push(SafeCommand {
                binary: "prlimit".to_string(),
                args: vec![format!("--pid={pid_str}"), format!("--nofile={val}")],
            });
        }
        if let Some(addr_space) = self.address_space {
            let val = format!("{addr_space}:{addr_space}");
            cmds.push(SafeCommand {
                binary: "prlimit".to_string(),
                args: vec![format!("--pid={pid_str}"), format!("--as={val}")],
            });
        }
        if let Some(nproc) = self.nproc {
            let val = format!("{nproc}:{nproc}");
            cmds.push(SafeCommand {
                binary: "prlimit".to_string(),
                args: vec![format!("--pid={pid_str}"), format!("--nproc={val}")],
            });
        }
        if let Some(core) = self.core {
            let val = format!("{core}:{core}");
            cmds.push(SafeCommand {
                binary: "prlimit".to_string(),
                args: vec![format!("--pid={pid_str}"), format!("--core={val}")],
            });
        }
        cmds
    }

    /// Whether any limits are configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nofile.is_none()
            && self.address_space.is_none()
            && self.nproc.is_none()
            && self.core.is_none()
    }

    /// Return secure defaults: core dumps disabled, no other limits.
    #[must_use]
    pub fn secure_defaults() -> Self {
        Self {
            nofile: None,
            address_space: None,
            nproc: None,
            core: Some(0),
        }
    }
}

/// Log rotation configuration for a service's stdout/stderr files.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Rotate when the log file exceeds this size in bytes.
    pub max_size_bytes: u64,
    /// Number of rotated files to keep (e.g. 5 → `.log.1` through `.log.5`).
    /// Minimum 1.
    pub max_files: u32,
}

impl LogConfig {
    /// Create a new `LogConfig`, enforcing minimum 1 for `max_files`.
    #[must_use]
    pub fn new(max_size_bytes: u64, max_files: u32) -> Self {
        Self {
            max_size_bytes,
            max_files: max_files.max(1),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 10 * 1024 * 1024, // 10 MB
            max_files: 5,
        }
    }
}

// ---------------------------------------------------------------------------
// Security configuration
// ---------------------------------------------------------------------------

/// Socket type for socket activation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SocketType {
    /// TCP stream socket.
    Stream,
    /// UDP datagram socket.
    Datagram,
    /// Sequential packet socket.
    SeqPacket,
}

impl fmt::Display for SocketType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stream => write!(f, "stream"),
            Self::Datagram => write!(f, "dgram"),
            Self::SeqPacket => write!(f, "seqpacket"),
        }
    }
}

/// A socket to be pre-opened for socket activation.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketSpec {
    /// Listen address (e.g. `"0.0.0.0"`, `"127.0.0.1"`, `"::"`).
    pub address: String,
    /// Listen port.
    pub port: u16,
    /// Socket type.
    pub socket_type: SocketType,
}

/// Socket activation configuration for a service.
///
/// The binary crate creates the actual sockets and passes file
/// descriptors. The library generates the `LISTEN_FDS` and
/// `LISTEN_PID` environment variables.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketActivationConfig {
    /// Sockets to pre-open for this service.
    pub sockets: Vec<SocketSpec>,
}

impl SocketActivationConfig {
    /// Generate environment variables for sd_listen_fds protocol.
    ///
    /// Returns `LISTEN_FDS=N` and `LISTEN_PID=<pid>`.
    /// Pass `pid = 0` during planning — the binary crate must update
    /// `LISTEN_PID` to the actual child PID after fork.
    #[must_use]
    pub fn env_vars(&self, pid: u32) -> Vec<(String, String)> {
        vec![
            ("LISTEN_FDS".to_string(), self.sockets.len().to_string()),
            ("LISTEN_PID".to_string(), pid.to_string()),
        ]
    }

    /// Generate only `LISTEN_FDS=N` (without LISTEN_PID).
    ///
    /// Use this when the PID is not yet known (pre-spawn). The binary
    /// crate sets `LISTEN_PID` after fork when the child PID is available.
    #[must_use]
    pub fn listen_fds_env(&self) -> (String, String) {
        ("LISTEN_FDS".to_string(), self.sockets.len().to_string())
    }
}

/// Action to take when a seccomp filter blocks a syscall.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeccompAction {
    /// Kill the process immediately.
    Kill,
    /// Send SIGSYS to the process.
    Trap,
    /// Log the violation but allow the syscall.
    Log,
}

impl fmt::Display for SeccompAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kill => write!(f, "kill"),
            Self::Trap => write!(f, "trap"),
            Self::Log => write!(f, "log"),
        }
    }
}

/// Seccomp BPF filter configuration for a service.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeccompConfig {
    /// Use the agnosys basic filter (20 safe syscalls, kill on everything else).
    Basic,
    /// Custom filter with explicit allow/deny lists using syscall names.
    Custom {
        /// Syscall names to allow (e.g. `["read", "write", "socket"]`).
        allow: Vec<String>,
        /// Syscall names to deny with specific actions.
        deny: Vec<(String, SeccompAction)>,
    },
}

/// Filesystem access level for Landlock rules.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LandlockAccess {
    /// No access (default deny).
    #[default]
    NoAccess,
    /// Read-only access.
    ReadOnly,
    /// Read and write access.
    ReadWrite,
}

impl fmt::Display for LandlockAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAccess => write!(f, "none"),
            Self::ReadOnly => write!(f, "ro"),
            Self::ReadWrite => write!(f, "rw"),
        }
    }
}

/// A Landlock filesystem restriction rule.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandlockRule {
    /// Filesystem path to restrict.
    pub path: PathBuf,
    /// Access level granted.
    pub access: LandlockAccess,
}

/// Landlock filesystem restriction configuration for a service.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandlockConfig {
    /// Filesystem rules to apply.
    pub rules: Vec<LandlockRule>,
}

/// Linux capability for bounding set management.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LinuxCapability {
    NetBindService,
    SysAdmin,
    DacOverride,
    NetRaw,
    SysChroot,
    Setuid,
    Setgid,
    Kill,
    SysPtrace,
    SysTime,
    NetAdmin,
    Fowner,
    Fsetid,
}

impl LinuxCapability {
    /// Return the kernel capability name (e.g. `"cap_sys_admin"`).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NetBindService => "cap_net_bind_service",
            Self::SysAdmin => "cap_sys_admin",
            Self::DacOverride => "cap_dac_override",
            Self::NetRaw => "cap_net_raw",
            Self::SysChroot => "cap_sys_chroot",
            Self::Setuid => "cap_setuid",
            Self::Setgid => "cap_setgid",
            Self::Kill => "cap_kill",
            Self::SysPtrace => "cap_sys_ptrace",
            Self::SysTime => "cap_sys_time",
            Self::NetAdmin => "cap_net_admin",
            Self::Fowner => "cap_fowner",
            Self::Fsetid => "cap_fsetid",
        }
    }
}

impl fmt::Display for LinuxCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Capability bounding set configuration for a service.
///
/// Specifies which capabilities to DROP from the service process.
/// The process starts with the full bounding set; listed capabilities
/// are explicitly removed.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityConfig {
    /// Capabilities to drop from the bounding set.
    pub drop: Vec<LinuxCapability>,
}

impl CapabilityConfig {
    /// Generate a `setpriv` command that drops capabilities and execs a binary.
    ///
    /// Uses `setpriv --no-new-privs --bounding-set=-<cap>,...` which is
    /// safer than `capsh` — no shell interpretation of arguments.
    #[must_use]
    pub fn to_setpriv_command(&self, binary: &str, args: &[String]) -> SafeCommand {
        let mut setpriv_args: Vec<String> = Vec::new();
        setpriv_args.push("--no-new-privs".to_string());
        if !self.drop.is_empty() {
            let caps: Vec<&str> = self.drop.iter().map(|c| c.as_str()).collect();
            setpriv_args.push(format!("--bounding-set=-{}", caps.join(",-")));
        }
        setpriv_args.push(binary.to_string());
        setpriv_args.extend_from_slice(args);
        SafeCommand {
            binary: "setpriv".to_string(),
            args: setpriv_args,
        }
    }
}

/// Entry for boot-time filesystem setup (tmpfiles.d equivalent).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TmpfileEntry {
    /// Create a directory with the given permissions.
    Directory {
        path: PathBuf,
        /// Octal mode (e.g. `0o755`).
        mode: u32,
        uid: Option<u32>,
        gid: Option<u32>,
    },
    /// Create a symbolic link.
    Symlink { path: PathBuf, target: PathBuf },
    /// Create a device node.
    Device {
        path: PathBuf,
        /// Device type: `'b'` for block, `'c'` for character.
        dev_type: char,
        major: u32,
        minor: u32,
        /// Octal mode (e.g. `0o660`).
        mode: u32,
    },
}

/// Static definition of a service managed by argonaut.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    /// Unique service name (e.g. "daimon").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Path to the executable binary.
    pub binary_path: PathBuf,
    /// Command-line arguments.
    pub args: Vec<String>,
    /// Environment variables.
    pub environment: HashMap<String, String>,
    /// Names of services that must be running before this one starts.
    pub depends_on: Vec<String>,
    /// Boot modes that require this service.
    pub required_for_modes: Vec<BootMode>,
    /// What to do when the service exits.
    pub restart_policy: RestartPolicy,
    /// Backoff and restart limit configuration.
    pub restart_config: RestartConfig,
    /// Optional periodic health check.
    pub health_check: Option<HealthCheck>,
    /// Optional one-shot startup readiness check.
    pub ready_check: Option<ReadyCheck>,
    /// Whether the service is enabled for automatic startup.
    /// Disabled services can still be started manually but are
    /// excluded from boot execution plans.
    pub enabled: bool,
    /// How the service process behaves (simple daemon, forking, oneshot).
    pub service_type: ServiceType,
    /// Paths to environment files loaded before spawning.
    /// Files are loaded in order; later files override earlier ones.
    /// Format: one `KEY=VALUE` per line, `#` comments, empty lines ignored.
    pub environment_files: Vec<PathBuf>,
    /// PID file path for forking services. After the parent exits,
    /// argonaut reads the child PID from this file.
    pub pid_file: Option<PathBuf>,
    /// Resource limits applied post-spawn via `prlimit(1)`.
    pub resource_limits: Option<ResourceLimits>,
    /// Log rotation configuration for stdout/stderr files.
    pub log_config: Option<LogConfig>,
    /// Socket activation configuration (LISTEN_FDS/LISTEN_PID protocol).
    pub socket_activation: Option<SocketActivationConfig>,
    /// Seccomp BPF filter configuration.
    pub seccomp: Option<SeccompConfig>,
    /// Landlock filesystem restriction rules.
    pub landlock: Option<LandlockConfig>,
    /// Linux capability bounding set (capabilities to drop).
    pub capabilities: Option<CapabilityConfig>,
}

/// Runtime state of a service.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed(String),
}

impl ServiceState {
    /// Check whether a transition from `self` to `to` is valid.
    #[must_use]
    pub fn valid_transition(&self, to: &ServiceState) -> bool {
        // Same state is always a no-op.
        if self == to {
            return true;
        }
        match self {
            ServiceState::Stopped => matches!(to, ServiceState::Starting),
            ServiceState::Starting => {
                matches!(to, ServiceState::Running | ServiceState::Failed(_))
            }
            ServiceState::Running => {
                matches!(to, ServiceState::Stopping | ServiceState::Failed(_))
            }
            ServiceState::Stopping => {
                matches!(to, ServiceState::Stopped | ServiceState::Failed(_))
            }
            ServiceState::Failed(_) => {
                matches!(to, ServiceState::Starting | ServiceState::Stopped)
            }
        }
    }
}

impl fmt::Display for ServiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stopped => write!(f, "stopped"),
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::Stopping => write!(f, "stopping"),
            Self::Failed(msg) => write!(f, "failed: {msg}"),
        }
    }
}

/// A service with its runtime state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedService {
    pub definition: ServiceDefinition,
    pub state: ServiceState,
    pub pid: Option<u32>,
    pub started_at: Option<DateTime<Utc>>,
    pub restart_count: u32,
    pub last_health_check: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Top-level configuration for the argonaut init system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgonautConfig {
    /// Which boot mode to use.
    pub boot_mode: BootMode,
    /// Service definitions to manage.
    pub services: Vec<ServiceDefinition>,
    /// Total boot timeout in milliseconds (default 30 000).
    pub boot_timeout_ms: u64,
    /// Graceful shutdown timeout in milliseconds (default 10 000).
    pub shutdown_timeout_ms: u64,
    /// Whether to log to the console (useful for early boot).
    pub log_to_console: bool,
    /// Whether to run dm-verity rootfs verification at boot.
    pub verify_on_boot: bool,
    /// Edge boot configuration (used when `boot_mode` is `Edge`).
    pub edge_boot: EdgeBootConfig,
    /// Boot-time filesystem entries (directories, symlinks, device nodes)
    /// created before services start. Equivalent to systemd tmpfiles.d.
    pub tmpfiles: Vec<TmpfileEntry>,
}

impl Default for ArgonautConfig {
    fn default() -> Self {
        Self {
            boot_mode: BootMode::Desktop,
            services: Vec::new(),
            boot_timeout_ms: 30_000,
            shutdown_timeout_ms: 10_000,
            log_to_console: true,
            verify_on_boot: true,
            edge_boot: EdgeBootConfig::default(),
            tmpfiles: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// Snapshot of init-system statistics.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgonautStats {
    pub boot_mode: BootMode,
    pub boot_duration_ms: Option<u64>,
    pub services_running: usize,
    pub services_failed: usize,
    pub total_restarts: u32,
    pub boot_complete: bool,
}

// ---------------------------------------------------------------------------
// Runlevel — runtime boot mode switching
// ---------------------------------------------------------------------------

/// Runlevel represents a system operational state, analogous to SysV runlevels
/// but mapped to AGNOS boot modes. Supports runtime switching.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Runlevel {
    /// Emergency: single-user, no services, drop to agnoshi shell.
    Emergency,
    /// Rescue: basic services only, network down, for recovery.
    Rescue,
    /// Console: multi-user text mode (equivalent to BootMode::Server).
    Console,
    /// Graphical: full desktop (equivalent to BootMode::Desktop).
    Graphical,
    /// Container: minimal services for container/embedded use.
    Container,
    /// Edge: constrained IoT/edge device, agent-runtime only.
    Edge,
}

impl Runlevel {
    /// Map a runlevel to the services that should be running.
    #[must_use]
    pub fn to_boot_mode(self) -> Option<BootMode> {
        match self {
            Self::Emergency | Self::Rescue => None,
            Self::Console => Some(BootMode::Server),
            Self::Graphical => Some(BootMode::Desktop),
            Self::Container => Some(BootMode::Minimal),
            Self::Edge => Some(BootMode::Edge),
        }
    }

    /// Map a boot mode to the corresponding runlevel.
    #[must_use]
    pub fn from_boot_mode(mode: BootMode) -> Self {
        match mode {
            BootMode::Server => Self::Console,
            BootMode::Desktop => Self::Graphical,
            BootMode::Minimal => Self::Container,
            BootMode::Edge => Self::Edge,
            BootMode::Recovery => Self::Emergency,
        }
    }

    /// Numeric level for display (compatible with SysV conventions).
    #[must_use]
    pub fn level(self) -> u8 {
        match self {
            Self::Emergency => 0,
            Self::Rescue => 1,
            Self::Console => 3,
            Self::Graphical => 5,
            Self::Container => 7,
            Self::Edge => 8,
        }
    }
}

impl fmt::Display for Runlevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Emergency => write!(f, "emergency"),
            Self::Rescue => write!(f, "rescue"),
            Self::Console => write!(f, "console"),
            Self::Graphical => write!(f, "graphical"),
            Self::Container => write!(f, "container"),
            Self::Edge => write!(f, "edge"),
        }
    }
}

// ---------------------------------------------------------------------------
// Service target — grouping services for coordinated lifecycle
// ---------------------------------------------------------------------------

/// A target groups related services that should start/stop together.
/// Analogous to systemd targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTarget {
    pub name: String,
    pub description: String,
    /// Services that MUST be running for this target to be met.
    pub requires: Vec<String>,
    /// Services that SHOULD be running but aren't fatal if missing.
    pub wants: Vec<String>,
    /// Runlevels where this target is active.
    pub active_in: Vec<Runlevel>,
}

impl ServiceTarget {
    /// Predefined targets for AGNOS.
    #[must_use]
    pub fn defaults() -> Vec<Self> {
        vec![
            Self {
                name: "basic".into(),
                description: "Basic system services".into(),
                requires: vec!["eudev".into(), "dbus".into(), "syslogd".into()],
                wants: vec![],
                active_in: vec![
                    Runlevel::Rescue,
                    Runlevel::Console,
                    Runlevel::Graphical,
                    Runlevel::Container,
                ],
            },
            Self {
                name: "network".into(),
                description: "Network connectivity".into(),
                requires: vec!["networkmanager".into()],
                wants: vec!["nftables".into(), "openssh".into()],
                active_in: vec![Runlevel::Console, Runlevel::Graphical],
            },
            Self {
                name: "agnos-core".into(),
                description: "AGNOS agent runtime and LLM gateway".into(),
                requires: vec!["daimon".into()],
                wants: vec!["hoosh".into(), "aegis".into()],
                active_in: vec![Runlevel::Console, Runlevel::Graphical, Runlevel::Container],
            },
            Self {
                name: "graphical".into(),
                description: "Desktop environment".into(),
                requires: vec!["aethersafha".into()],
                wants: vec!["pipewire".into(), "agnoshi".into()],
                active_in: vec![Runlevel::Graphical],
            },
            Self {
                name: "edge".into(),
                description: "Edge device — minimal agent runtime".into(),
                requires: vec!["daimon".into()],
                wants: vec!["aegis".into()],
                active_in: vec![Runlevel::Edge],
            },
        ]
    }

    /// Check if this target is active in the given runlevel.
    #[must_use]
    pub fn is_active_in(&self, runlevel: Runlevel) -> bool {
        self.active_in.contains(&runlevel)
    }

    /// All services needed for this target (requires + wants).
    #[must_use]
    pub fn all_services(&self) -> Vec<&str> {
        self.requires
            .iter()
            .chain(self.wants.iter())
            .map(|s| s.as_str())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Process execution types
// ---------------------------------------------------------------------------

/// Describes how a service process exited.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExitStatus {
    /// Exited normally with the given code (0 = success).
    Code(i32),
    /// Killed by a signal (e.g. SIGTERM=15, SIGKILL=9).
    Signal(i32),
    /// Process hasn't exited yet.
    Running,
    /// Never started.
    NotStarted,
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Code(c) => write!(f, "exit({})", c),
            Self::Signal(s) => write!(f, "signal({})", s),
            Self::Running => write!(f, "running"),
            Self::NotStarted => write!(f, "not-started"),
        }
    }
}

/// A service lifecycle event recorded in the audit log.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEvent {
    pub timestamp: DateTime<Utc>,
    pub service: String,
    pub event_type: ServiceEventType,
    pub details: Option<String>,
}

/// Types of service lifecycle events.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServiceEventType {
    Starting,
    Started { pid: u32 },
    HealthCheckPassed,
    HealthCheckFailed { consecutive: u32 },
    ReadyCheckPassed,
    ReadyCheckFailed,
    Stopping,
    Stopped { exit_status: ExitStatus },
    Restarting { restart_count: u32 },
    DependencyWaiting { dependency: String },
    DependencyMet { dependency: String },
    TimeoutKilled,
    CrashDetected { exit_status: ExitStatus },
    Enabled,
    Disabled,
}

impl fmt::Display for ServiceEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Started { pid } => write!(f, "started(pid={})", pid),
            Self::HealthCheckPassed => write!(f, "health-ok"),
            Self::HealthCheckFailed { consecutive } => {
                write!(f, "health-fail({}x)", consecutive)
            }
            Self::ReadyCheckPassed => write!(f, "ready"),
            Self::ReadyCheckFailed => write!(f, "not-ready"),
            Self::Stopping => write!(f, "stopping"),
            Self::Stopped { exit_status } => write!(f, "stopped({})", exit_status),
            Self::Restarting { restart_count } => write!(f, "restarting(#{})", restart_count),
            Self::DependencyWaiting { dependency } => write!(f, "waiting({})", dependency),
            Self::DependencyMet { dependency } => write!(f, "dep-met({})", dependency),
            Self::TimeoutKilled => write!(f, "timeout-killed"),
            Self::CrashDetected { exit_status } => write!(f, "crash({})", exit_status),
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
        }
    }
}

// ---------------------------------------------------------------------------
// Shutdown orchestration
// ---------------------------------------------------------------------------

/// Shutdown type selection.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShutdownType {
    /// Clean shutdown and power off.
    Poweroff,
    /// Clean shutdown and reboot.
    Reboot,
    /// Halt the CPU without powering off.
    Halt,
    /// Kexec into a new kernel (fast reboot).
    Kexec,
}

impl fmt::Display for ShutdownType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Poweroff => write!(f, "poweroff"),
            Self::Reboot => write!(f, "reboot"),
            Self::Halt => write!(f, "halt"),
            Self::Kexec => write!(f, "kexec"),
        }
    }
}

/// A shutdown plan describing the ordered steps to cleanly shut down.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownPlan {
    pub shutdown_type: ShutdownType,
    pub steps: Vec<ShutdownStep>,
    pub timeout_ms: u64,
    pub wall_message: Option<String>,
}

/// An individual shutdown step.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownStep {
    pub description: String,
    pub action: ShutdownAction,
    pub timeout_ms: u64,
    pub status: ShutdownStepStatus,
}

/// Actions performed during shutdown.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShutdownAction {
    /// Broadcast wall message to all terminals.
    WallMessage(String),
    /// Signal agents to save state and disconnect.
    NotifyAgents,
    /// Send SIGTERM to a service, wait for graceful exit.
    StopService { name: String, signal: i32 },
    /// Force kill a service that didn't stop gracefully.
    ForceKillService { name: String },
    /// Flush filesystem buffers.
    SyncFilesystems,
    /// Unmount all filesystems.
    UnmountFilesystems,
    /// Deactivate swap.
    SwapOff,
    /// Deactivate LUKS volumes.
    CloseLuks,
    /// Final kernel call (reboot/poweroff/halt).
    KernelAction(ShutdownType),
}

/// Status of a shutdown step.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShutdownStepStatus {
    Pending,
    InProgress,
    Complete,
    Failed(String),
    Skipped,
}

impl fmt::Display for ShutdownStepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Complete => write!(f, "complete"),
            Self::Failed(e) => write!(f, "failed: {}", e),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

/// Plan for switching between runlevels at runtime.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunlevelSwitchPlan {
    pub from: Runlevel,
    pub to: Runlevel,
    pub services_to_start: Vec<String>,
    pub services_to_stop: Vec<String>,
    pub drop_to_shell: bool,
}

/// Result of executing a runlevel switch.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunlevelSwitchResult {
    /// Runlevel we switched from.
    pub from: Runlevel,
    /// Runlevel we switched to.
    pub to: Runlevel,
    /// Services that were successfully stopped.
    pub stopped: Vec<String>,
    /// Services that were successfully started.
    pub started: Vec<String>,
    /// Error messages for any failures.
    pub errors: Vec<String>,
    /// Whether the caller should drop to an interactive shell.
    pub drop_to_shell: bool,
}

// ---------------------------------------------------------------------------
// Health check execution types
// ---------------------------------------------------------------------------

/// Result of executing a single health check.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub service: String,
    pub check_type: String,
    pub passed: bool,
    pub latency_ms: u64,
    pub message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

/// Tracks consecutive health check failures for a service.
#[derive(Debug, Clone, Default)]
pub struct HealthTracker {
    /// Internal consecutive failure counts — use `failure_count()` to query.
    failures: HashMap<String, u32>,
}

impl HealthTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a health check result. Returns true if the service should
    /// be restarted (consecutive failures >= threshold).
    #[must_use]
    pub fn record(&mut self, service: &str, passed: bool, threshold: u32) -> bool {
        if passed {
            self.failures.remove(service);
            false
        } else {
            let count = self.failures.entry(service.to_string()).or_insert(0);
            *count += 1;
            *count >= threshold
        }
    }

    /// Get current consecutive failure count for a service.
    #[must_use]
    pub fn failure_count(&self, service: &str) -> u32 {
        self.failures.get(service).copied().unwrap_or(0)
    }

    /// Reset tracking for a service (e.g. after restart).
    pub fn reset(&mut self, service: &str) {
        self.failures.remove(service);
    }
}

// ---------------------------------------------------------------------------
// Process spawn specification
// ---------------------------------------------------------------------------

/// Everything needed to spawn a service process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSpec {
    pub binary: PathBuf,
    pub args: Vec<String>,
    pub environment: HashMap<String, String>,
    pub working_dir: Option<PathBuf>,
    pub stdout_log: Option<PathBuf>,
    pub stderr_log: Option<PathBuf>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    /// Resource limits to apply after spawn via `prlimit(1)`.
    pub resource_limits: Option<ResourceLimits>,
    /// Log rotation configuration.
    pub log_config: Option<LogConfig>,
}

impl ProcessSpec {
    /// Build a ProcessSpec from a ServiceDefinition.
    #[must_use]
    pub fn from_service(def: &ServiceDefinition) -> Self {
        Self {
            binary: def.binary_path.clone(),
            args: def.args.clone(),
            environment: def.environment.clone(),
            working_dir: None,
            stdout_log: Some(PathBuf::from(format!(
                "/var/log/agnos/services/{}.log",
                def.name
            ))),
            stderr_log: Some(PathBuf::from(format!(
                "/var/log/agnos/services/{}.err",
                def.name
            ))),
            uid: None,
            gid: None,
            resource_limits: def.resource_limits.clone(),
            log_config: def.log_config.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Emergency shell
// ---------------------------------------------------------------------------

/// Configuration for the emergency recovery shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyShellConfig {
    /// Path to the shell binary.
    pub shell_path: PathBuf,
    /// Environment variables for the emergency shell.
    pub environment: HashMap<String, String>,
    /// Message displayed before dropping to the shell.
    pub banner: String,
    /// Whether to require root password before granting access.
    pub require_auth: bool,
    /// Hex-encoded SHA-256 hash of the emergency shell password.
    /// Only used when `require_auth` is `true`.
    pub auth_password_hash: Option<String>,
}

impl Default for EmergencyShellConfig {
    fn default() -> Self {
        let mut env = HashMap::new();
        env.insert("HOME".into(), "/root".into());
        env.insert("TERM".into(), "linux".into());
        env.insert("PATH".into(), "/usr/sbin:/usr/bin:/sbin:/bin".into());
        env.insert("SHELL".into(), "/usr/bin/agnoshi".into());

        Self {
            shell_path: PathBuf::from("/usr/bin/agnoshi"),
            environment: env,
            banner: concat!(
                "\n",
                "========================================\n",
                "  AGNOS Emergency Shell\n",
                "========================================\n",
                "\n",
                "The system has entered emergency mode.\n",
                "Use 'argonaut status' to check service state.\n",
                "Use 'argonaut start <service>' to start services.\n",
                "Use 'exit' to continue boot or 'reboot' to restart.\n",
                "\n",
            )
            .into(),
            require_auth: false,
            auth_password_hash: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Crash action
// ---------------------------------------------------------------------------

/// Action to take when a service crashes.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CrashAction {
    /// Restart the service after the given delay.
    Restart { delay_ms: u64 },
    /// Don't restart; service exited normally or policy is Never.
    Ignore,
    /// Stop trying to restart; too many failures.
    GiveUp { reason: String },
}

// ---------------------------------------------------------------------------
// Safe command (injection-resistant)
// ---------------------------------------------------------------------------

/// A structured command representation to avoid shell injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeCommand {
    pub binary: String,
    pub args: Vec<String>,
}

impl fmt::Display for SafeCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.binary)?;
        for arg in &self.args {
            write!(f, " {}", arg)?;
        }
        Ok(())
    }
}

impl SafeCommand {
    /// Format as a display string (for logging only — NOT for shell execution).
    #[must_use]
    pub fn display(&self) -> String {
        self.to_string()
    }
}
