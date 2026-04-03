//! Process execution — spawn, track, signal, and reap service processes.
//!
//! This module bridges the gap between argonaut's type-level planning
//! (boot sequences, shutdown plans, service definitions) and actual
//! OS-level process management.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use tracing::{debug, error, info, warn};

use crate::types::{LogConfig, ProcessSpec, SafeCommand};

// ---------------------------------------------------------------------------
// Environment file loading
// ---------------------------------------------------------------------------

/// Load environment variables from a file.
///
/// Format: one `KEY=VALUE` per line. Lines starting with `#` are comments.
/// Empty lines are ignored. Values may be optionally quoted with `"` or `'`.
/// The first `=` splits the key from the value.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn load_environment_file(path: &Path) -> Result<HashMap<String, String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read environment file: {}", path.display()))?;

    let mut env = HashMap::new();
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            warn!(
                file = %path.display(),
                line = line_num + 1,
                content = trimmed,
                "skipping malformed environment line (no '=')"
            );
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            warn!(
                file = %path.display(),
                line = line_num + 1,
                "skipping environment line with empty key"
            );
            continue;
        }
        // Strip optional quotes from value (must be at least 2 chars for paired quotes)
        let value = value.trim();
        let value = if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };
        env.insert(key.to_string(), value.to_string());
    }

    info!(
        file = %path.display(),
        vars = env.len(),
        "loaded environment file"
    );
    Ok(env)
}

/// Load and merge environment variables from multiple files.
///
/// Files are loaded in order — later files override earlier ones.
/// Missing files are skipped with a warning (not an error).
#[must_use]
pub fn load_environment_files(paths: &[PathBuf]) -> HashMap<String, String> {
    let mut merged = HashMap::new();
    for path in paths {
        match load_environment_file(path) {
            Ok(env) => merged.extend(env),
            Err(e) => {
                warn!(
                    path = %path.display(),
                    error = %e,
                    "skipping missing or unreadable environment file"
                );
            }
        }
    }
    merged
}

// ---------------------------------------------------------------------------
// Log rotation
// ---------------------------------------------------------------------------

/// Rotate a log file if it exceeds the configured maximum size.
///
/// Renames `.log` → `.log.1`, `.log.1` → `.log.2`, etc., up to
/// `config.max_files`. The oldest file beyond the limit is deleted.
///
/// Does nothing if the file doesn't exist or is under the size limit.
fn rotate_log_if_needed(path: &Path, config: &LogConfig) -> Result<()> {
    let size = match fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(anyhow::anyhow!(
                "failed to stat log file {}: {}",
                path.display(),
                e
            ));
        }
    };

    if size < config.max_size_bytes {
        return Ok(());
    }

    let path_str = path.display().to_string();
    info!(
        path = %path_str,
        size_bytes = size,
        max_bytes = config.max_size_bytes,
        "rotating log file"
    );

    // Delete oldest rotated file if it exists
    let oldest = format!("{path_str}.{}", config.max_files);
    let _ = fs::remove_file(&oldest);

    // Shift existing rotated files: .log.N → .log.N+1 (backwards)
    for i in (1..config.max_files).rev() {
        let from = format!("{path_str}.{i}");
        let to = format!("{path_str}.{}", i + 1);
        if Path::new(&from).exists()
            && let Err(e) = fs::rename(&from, &to)
        {
            warn!(from = %from, to = %to, error = %e, "failed to rotate log file");
        }
    }

    // Rename current log to .log.1
    let rotated = format!("{path_str}.1");
    if let Err(e) = fs::rename(path, &rotated) {
        warn!(
            from = %path_str,
            to = %rotated,
            error = %e,
            "failed to rotate current log file"
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// PID file reading
// ---------------------------------------------------------------------------

/// Read a PID from a PID file.
///
/// The file should contain a single integer (the PID), optionally
/// followed by whitespace. Validates that the PID is positive and
/// that the process is alive.
///
/// # Errors
///
/// Returns an error if the file cannot be read, the content is not
/// a valid PID, or the process is not alive.
pub fn read_pid_file(path: &Path) -> Result<u32> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read PID file: {}", path.display()))?;
    let pid_str = content.trim();
    let pid: u32 = pid_str
        .parse()
        .with_context(|| format!("invalid PID in {}: '{}'", path.display(), pid_str))?;
    if pid == 0 {
        bail!("PID file {} contains PID 0", path.display());
    }
    // Check the process is alive
    let nix_pid =
        Pid::from_raw(i32::try_from(pid).with_context(|| format!("PID {} exceeds i32::MAX", pid))?);
    match signal::kill(nix_pid, None) {
        Ok(()) => {}
        Err(nix::errno::Errno::ESRCH) => {
            bail!("PID {} from {} is not alive (ESRCH)", pid, path.display());
        }
        Err(e) => {
            warn!(
                pid = pid,
                error = %e,
                "kill(0) check returned unexpected error, assuming alive"
            );
        }
    }
    info!(path = %path.display(), pid = pid, "read PID from file");
    Ok(pid)
}

// ---------------------------------------------------------------------------
// Log file management
// ---------------------------------------------------------------------------

/// Ensure the log directory exists for a given service.
fn ensure_log_dir(log_path: &Path) -> io::Result<()> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Open a log file for append (create if missing).
fn open_log_file(path: &Path) -> Result<File> {
    ensure_log_dir(path)?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open log file: {}", path.display()))
}

// ---------------------------------------------------------------------------
// Spawned process handle
// ---------------------------------------------------------------------------

/// A running service process with its metadata.
///
/// For simple services, `child` holds the `Child` handle from spawn.
/// For forking services, `child` is `None` — the original parent
/// has exited and we track the forked child by PID only (signaling
/// via `nix::sys::signal::kill`).
#[derive(Debug)]
pub struct SpawnedProcess {
    /// The OS child process handle (`None` for forked services where
    /// the parent has exited and we're tracking the child by PID).
    child: Option<Child>,
    /// Service name this process belongs to.
    pub service_name: String,
    /// PID of the tracked process.
    pub pid: u32,
    /// When the process was spawned (or adopted).
    pub started_at: Instant,
    /// Path to stdout log file.
    pub stdout_log: Option<PathBuf>,
    /// Path to stderr log file.
    pub stderr_log: Option<PathBuf>,
    /// Whether this is a forked process (parent exited, tracking child).
    pub forked: bool,
}

impl SpawnedProcess {
    /// Create a `SpawnedProcess` for a forked daemon where we only
    /// know the child PID (read from a PID file or sd_notify MAINPID).
    #[must_use]
    pub fn from_forked_pid(service_name: &str, pid: u32) -> Self {
        Self {
            child: None,
            service_name: service_name.to_string(),
            pid,
            started_at: Instant::now(),
            stdout_log: None,
            stderr_log: None,
            forked: true,
        }
    }
}

impl SpawnedProcess {
    /// Check if the process has exited without blocking.
    /// Returns `Some(exit_code)` if exited, `None` if still running.
    pub fn try_wait(&mut self) -> Result<Option<i32>> {
        if let Some(ref mut child) = self.child {
            match child.try_wait().context("try_wait failed")? {
                Some(status) => {
                    let code = status.code().unwrap_or(-1);
                    debug!(
                        service = %self.service_name,
                        pid = self.pid,
                        exit_code = code,
                        "process exited"
                    );
                    Ok(Some(code))
                }
                None => Ok(None),
            }
        } else {
            // Forked process — check liveness via kill(pid, 0)
            if self.is_alive() {
                Ok(None)
            } else {
                debug!(
                    service = %self.service_name,
                    pid = self.pid,
                    "forked process is no longer alive"
                );
                Ok(Some(-1))
            }
        }
    }

    /// Block until the process exits. Returns the exit code.
    pub fn wait(&mut self) -> Result<i32> {
        if let Some(ref mut child) = self.child {
            let status = child
                .wait()
                .with_context(|| format!("wait failed for service '{}'", self.service_name))?;
            let code = status.code().unwrap_or(-1);
            info!(
                service = %self.service_name,
                pid = self.pid,
                exit_code = code,
                elapsed_ms = self.started_at.elapsed().as_millis() as u64,
                "process exited"
            );
            Ok(code)
        } else {
            // Forked process — poll liveness until dead
            let start = Instant::now();
            while self.is_alive() {
                std::thread::sleep(Duration::from_millis(50));
                if start.elapsed() > Duration::from_secs(30) {
                    bail!(
                        "timed out waiting for forked process '{}' (pid {})",
                        self.service_name,
                        self.pid
                    );
                }
            }
            info!(
                service = %self.service_name,
                pid = self.pid,
                elapsed_ms = self.started_at.elapsed().as_millis() as u64,
                "forked process exited"
            );
            Ok(-1) // No exit code available for forked processes
        }
    }

    /// Convert the stored PID to a nix `Pid`, failing if it exceeds `i32::MAX`.
    fn nix_pid(&self) -> Result<Pid> {
        let raw = i32::try_from(self.pid)
            .with_context(|| format!("PID {} exceeds i32::MAX", self.pid))?;
        Ok(Pid::from_raw(raw))
    }

    /// Send a signal to the process.
    pub fn signal(&self, sig: Signal) -> Result<()> {
        let nix_pid = self.nix_pid()?;
        signal::kill(nix_pid, sig).with_context(|| {
            format!(
                "failed to send {} to service '{}' (pid {})",
                sig, self.service_name, self.pid
            )
        })?;
        info!(
            service = %self.service_name,
            pid = self.pid,
            signal = %sig,
            "sent signal to process"
        );
        Ok(())
    }

    /// Send SIGTERM for graceful shutdown.
    pub fn terminate(&self) -> Result<()> {
        self.signal(Signal::SIGTERM)
    }

    /// Send SIGKILL for forced shutdown.
    pub fn kill(&self) -> Result<()> {
        self.signal(Signal::SIGKILL)
    }

    /// Check if the process is still alive (signal 0).
    #[must_use]
    pub fn is_alive(&self) -> bool {
        let Ok(nix_pid) = self.nix_pid() else {
            return false;
        };
        signal::kill(nix_pid, None).is_ok()
    }

    /// Graceful stop: SIGTERM, wait up to `timeout`, then SIGKILL if needed.
    /// Returns the exit code.
    pub fn stop(&mut self, timeout: Duration) -> Result<i32> {
        info!(
            service = %self.service_name,
            pid = self.pid,
            timeout_ms = timeout.as_millis() as u64,
            "stopping process"
        );

        // Send SIGTERM
        if let Err(e) = self.terminate() {
            // Process may already be dead
            warn!(
                service = %self.service_name,
                pid = self.pid,
                error = %e,
                "SIGTERM failed, process may have already exited"
            );
        }

        // Poll for exit
        let deadline = Instant::now() + timeout;
        loop {
            match self.try_wait()? {
                Some(code) => return Ok(code),
                None => {
                    if Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }

        // Timed out — SIGKILL
        warn!(
            service = %self.service_name,
            pid = self.pid,
            timeout_ms = timeout.as_millis() as u64,
            "process did not exit after SIGTERM, sending SIGKILL"
        );
        if let Err(e) = self.kill() {
            warn!(
                service = %self.service_name,
                pid = self.pid,
                error = %e,
                "SIGKILL failed"
            );
        }

        // Wait for SIGKILL to take effect (should be near-instant)
        self.wait()
    }

    /// How long this process has been running.
    #[must_use]
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}

// ---------------------------------------------------------------------------
// Process spawning
// ---------------------------------------------------------------------------

/// Spawn a service process from a `ProcessSpec`.
///
/// Opens log files for stdout/stderr if paths are specified, configures
/// the environment, and spawns the child process.
pub fn spawn_process(spec: &ProcessSpec, service_name: &str) -> Result<SpawnedProcess> {
    let binary_str = spec.binary.display().to_string();

    info!(
        service = service_name,
        binary = %binary_str,
        args = ?spec.args,
        "spawning service process"
    );

    let mut cmd = Command::new(&spec.binary);
    cmd.args(&spec.args);

    // Environment
    for (key, val) in &spec.environment {
        cmd.env(key, val);
    }

    // Working directory
    if let Some(ref wd) = spec.working_dir {
        cmd.current_dir(wd);
    }

    // Log rotation — rotate before opening if configured
    if let Some(ref log_config) = spec.log_config {
        if let Some(ref path) = spec.stdout_log
            && let Err(e) = rotate_log_if_needed(path, log_config)
        {
            warn!(service = service_name, path = %path.display(), error = %e, "stdout log rotation failed");
        }
        if let Some(ref path) = spec.stderr_log
            && let Err(e) = rotate_log_if_needed(path, log_config)
        {
            warn!(service = service_name, path = %path.display(), error = %e, "stderr log rotation failed");
        }
    }

    // Stdout — fall back to /dev/null if log file can't be opened
    let stdout_log = if let Some(ref path) = spec.stdout_log {
        match open_log_file(path) {
            Ok(file) => {
                cmd.stdout(Stdio::from(file));
                Some(path.clone())
            }
            Err(e) => {
                warn!(
                    service = service_name,
                    path = %path.display(),
                    error = %e,
                    "cannot open stdout log, falling back to /dev/null"
                );
                cmd.stdout(Stdio::null());
                None
            }
        }
    } else {
        cmd.stdout(Stdio::null());
        None
    };

    // Stderr — fall back to /dev/null if log file can't be opened
    let stderr_log = if let Some(ref path) = spec.stderr_log {
        match open_log_file(path) {
            Ok(file) => {
                cmd.stderr(Stdio::from(file));
                Some(path.clone())
            }
            Err(e) => {
                warn!(
                    service = service_name,
                    path = %path.display(),
                    error = %e,
                    "cannot open stderr log, falling back to /dev/null"
                );
                cmd.stderr(Stdio::null());
                None
            }
        }
    } else {
        cmd.stderr(Stdio::null());
        None
    };

    // Don't inherit stdin
    cmd.stdin(Stdio::null());

    // Privilege separation — not yet implemented in safe code.
    // Return an error rather than silently ignoring uid/gid.
    if spec.uid.is_some() || spec.gid.is_some() {
        bail!(
            "uid/gid privilege drop is not yet implemented for service '{}'. \
             Set uid/gid to None, or implement privilege drop in the PID 1 binary.",
            service_name
        );
    }

    let child = cmd.spawn().with_context(|| {
        format!(
            "failed to spawn service '{}' ({})",
            service_name, binary_str
        )
    })?;

    let pid = child.id();
    info!(
        service = service_name,
        pid = pid,
        binary = %binary_str,
        "service process spawned"
    );

    Ok(SpawnedProcess {
        child: Some(child),
        service_name: service_name.to_string(),
        pid,
        started_at: Instant::now(),
        stdout_log,
        stderr_log,
        forked: false,
    })
}

/// Spawn a process from a `SafeCommand` (used for boot/shutdown helper commands).
///
/// Unlike service processes, these are one-shot commands that run to
/// completion. Stdout/stderr are inherited from the parent.
pub fn run_command(cmd: &SafeCommand) -> Result<i32> {
    info!(command = %cmd, "executing command");

    let mut child = Command::new(&cmd.binary)
        .args(&cmd.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to execute command: {}", cmd))?;

    // Note: stderr is read best-effort after wait(). The pipe buffer
    // may be incomplete if the process wrote more than the OS buffer size.
    let status = child
        .wait()
        .with_context(|| format!("failed to wait for command: {}", cmd))?;
    let code = status.code().unwrap_or(-1);

    if status.success() {
        debug!(command = %cmd, exit_code = code, "command completed successfully");
    } else {
        // Read stderr for diagnostic (bounded read)
        let stderr_msg = child
            .stderr
            .take()
            .and_then(|mut s| {
                let mut buf = vec![0u8; 4096];
                io::Read::read(&mut s, &mut buf)
                    .ok()
                    .map(|n| String::from_utf8_lossy(&buf[..n]).trim().to_string())
            })
            .unwrap_or_default();
        warn!(
            command = %cmd,
            exit_code = code,
            stderr = %stderr_msg,
            "command failed"
        );
    }

    Ok(code)
}

/// Run a sequence of `SafeCommand`s in order, stopping on first failure.
/// Returns the index and exit code of the failed command, or `Ok(())` if
/// all succeed.
pub fn run_command_sequence(cmds: &[SafeCommand]) -> Result<()> {
    for (i, cmd) in cmds.iter().enumerate() {
        let code = run_command(cmd)?;
        if code != 0 {
            bail!(
                "command {} of {} failed with exit code {}: {}",
                i + 1,
                cmds.len(),
                code,
                cmd
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Process table — tracks all running service processes
// ---------------------------------------------------------------------------

/// Tracks all running service processes for the init system.
#[derive(Debug, Default)]
pub struct ProcessTable {
    /// Map of service name → spawned process.
    processes: HashMap<String, SpawnedProcess>,
}

impl ProcessTable {
    /// Create a new empty process table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a spawned process into the table.
    /// If a process for the service already exists, the old entry is
    /// replaced (caller should have stopped it first).
    pub fn insert(&mut self, process: SpawnedProcess) {
        let name = process.service_name.clone();
        if let Some(mut old) = self.processes.remove(&name) {
            warn!(
                service = %name,
                old_pid = old.pid,
                new_pid = process.pid,
                "replacing process table entry — killing old process"
            );
            let _ = old.kill();
            let _ = old.wait();
        }
        self.processes.insert(name, process);
    }

    /// Remove and return a process from the table.
    pub fn remove(&mut self, service_name: &str) -> Option<SpawnedProcess> {
        self.processes.remove(service_name)
    }

    /// Get a reference to a tracked process.
    #[must_use]
    pub fn get(&self, service_name: &str) -> Option<&SpawnedProcess> {
        self.processes.get(service_name)
    }

    /// Get a mutable reference to a tracked process.
    pub fn get_mut(&mut self, service_name: &str) -> Option<&mut SpawnedProcess> {
        self.processes.get_mut(service_name)
    }

    /// Check if a service has a tracked process.
    #[must_use]
    pub fn contains(&self, service_name: &str) -> bool {
        self.processes.contains_key(service_name)
    }

    /// Number of tracked processes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.processes.len()
    }

    /// Whether the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.processes.is_empty()
    }

    /// Iterate over all tracked processes.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &SpawnedProcess)> {
        self.processes.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Iterate mutably over all tracked processes.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut SpawnedProcess)> {
        self.processes.iter_mut().map(|(k, v)| (k.as_str(), v))
    }

    /// Reap any processes that have exited. Returns a list of
    /// (service_name, exit_code) for reaped processes.
    pub fn reap_exited(&mut self) -> Vec<(String, i32)> {
        let mut exited = Vec::new();

        for (name, proc) in &mut self.processes {
            match proc.try_wait() {
                Ok(Some(code)) => {
                    exited.push((name.clone(), code));
                }
                Ok(None) => {} // still running
                Err(e) => {
                    error!(service = %name, error = %e, "failed to reap process");
                }
            }
        }

        // Remove reaped processes from the table
        for (name, _) in &exited {
            self.processes.remove(name);
        }

        exited
    }

    /// Stop all processes gracefully with the given timeout per process.
    /// Returns a list of (service_name, exit_code) for all stopped processes.
    pub fn stop_all(&mut self, timeout: Duration) -> Vec<(String, i32)> {
        let names: Vec<String> = self.processes.keys().cloned().collect();
        let mut results = Vec::new();

        // Send SIGTERM to all first
        for name in &names {
            if let Some(proc) = self.processes.get(name)
                && let Err(e) = proc.terminate()
            {
                warn!(service = %name, error = %e, "SIGTERM failed during stop_all");
            }
        }

        // Wait for all with timeout
        let deadline = Instant::now() + timeout;
        let mut remaining: Vec<String> = names.clone();

        while !remaining.is_empty() && Instant::now() < deadline {
            remaining.retain(|name| {
                if let Some(proc) = self.processes.get_mut(name) {
                    match proc.try_wait() {
                        Ok(Some(code)) => {
                            results.push((name.clone(), code));
                            false // remove from remaining
                        }
                        _ => true, // still waiting
                    }
                } else {
                    false
                }
            });
            if !remaining.is_empty() {
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        // SIGKILL anything still alive
        for name in &remaining {
            if let Some(proc) = self.processes.get_mut(name) {
                warn!(service = %name, pid = proc.pid, "sending SIGKILL after timeout");
                let _ = proc.kill();
                // Wait briefly for SIGKILL to take effect (non-blocking with cap)
                let kill_deadline = Instant::now() + Duration::from_millis(500);
                loop {
                    match proc.try_wait() {
                        Ok(Some(code)) => {
                            results.push((name.clone(), code));
                            break;
                        }
                        Ok(None) => {
                            if Instant::now() >= kill_deadline {
                                error!(service = %name, pid = proc.pid, "process did not exit after SIGKILL");
                                results.push((name.clone(), -1));
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(e) => {
                            error!(service = %name, error = %e, "failed to wait after SIGKILL");
                            results.push((name.clone(), -1));
                            break;
                        }
                    }
                }
            }
        }

        // Clear the table
        self.processes.clear();

        results
    }

    /// Get the PID for a service, if tracked.
    #[must_use]
    pub fn pid_of(&self, service_name: &str) -> Option<u32> {
        self.processes.get(service_name).map(|p| p.pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn true_spec() -> ProcessSpec {
        ProcessSpec {
            binary: PathBuf::from("/usr/bin/true"),
            args: vec![],
            environment: HashMap::new(),
            working_dir: None,
            stdout_log: None,
            stderr_log: None,
            uid: None,
            gid: None,
            resource_limits: None,
            log_config: None,
        }
    }

    fn false_spec() -> ProcessSpec {
        ProcessSpec {
            binary: PathBuf::from("/usr/bin/false"),
            args: vec![],
            environment: HashMap::new(),
            working_dir: None,
            stdout_log: None,
            stderr_log: None,
            uid: None,
            gid: None,
            resource_limits: None,
            log_config: None,
        }
    }

    fn sleep_spec(secs: &str) -> ProcessSpec {
        ProcessSpec {
            binary: PathBuf::from("/usr/bin/sleep"),
            args: vec![secs.to_string()],
            environment: HashMap::new(),
            working_dir: None,
            stdout_log: None,
            stderr_log: None,
            uid: None,
            gid: None,
            resource_limits: None,
            log_config: None,
        }
    }

    // --- spawn_process ---

    #[test]
    fn spawn_true_exits_zero() {
        let mut proc = spawn_process(&true_spec(), "test-true").unwrap();
        let code = proc.wait().unwrap();
        assert_eq!(code, 0);
    }

    #[test]
    fn spawn_false_exits_nonzero() {
        let mut proc = spawn_process(&false_spec(), "test-false").unwrap();
        let code = proc.wait().unwrap();
        assert_ne!(code, 0);
    }

    #[test]
    fn spawn_nonexistent_binary_fails() {
        let spec = ProcessSpec {
            binary: PathBuf::from("/nonexistent/binary"),
            args: vec![],
            environment: HashMap::new(),
            working_dir: None,
            stdout_log: None,
            stderr_log: None,
            uid: None,
            gid: None,
            resource_limits: None,
            log_config: None,
        };
        let result = spawn_process(&spec, "bad-binary");
        assert!(result.is_err());
    }

    #[test]
    fn spawn_has_valid_pid() {
        let mut proc = spawn_process(&sleep_spec("10"), "test-sleep").unwrap();
        assert!(proc.pid > 0);
        assert!(proc.is_alive());
        let _ = proc.kill();
        let _ = proc.wait();
    }

    // --- try_wait ---

    #[test]
    fn try_wait_running_returns_none() {
        let mut proc = spawn_process(&sleep_spec("10"), "test-sleep").unwrap();
        let result = proc.try_wait().unwrap();
        assert!(result.is_none());
        let _ = proc.kill();
        let _ = proc.wait();
    }

    #[test]
    fn try_wait_exited_returns_code() {
        let mut proc = spawn_process(&true_spec(), "test-true").unwrap();
        // Wait a bit for it to exit
        std::thread::sleep(Duration::from_millis(100));
        let result = proc.try_wait().unwrap();
        assert_eq!(result, Some(0));
    }

    // --- signal / terminate / kill ---

    #[test]
    fn terminate_stops_sleep() {
        let mut proc = spawn_process(&sleep_spec("60"), "test-sleep").unwrap();
        assert!(proc.is_alive());
        proc.terminate().unwrap();
        let code = proc.wait().unwrap();
        // SIGTERM gives -1 (signal exit)
        assert_ne!(code, 0);
    }

    #[test]
    fn kill_stops_sleep() {
        let mut proc = spawn_process(&sleep_spec("60"), "test-sleep").unwrap();
        proc.kill().unwrap();
        let code = proc.wait().unwrap();
        assert_ne!(code, 0);
    }

    // --- stop (graceful) ---

    #[test]
    fn stop_graceful_exits_cleanly() {
        let mut proc = spawn_process(&sleep_spec("60"), "test-sleep").unwrap();
        let code = proc.stop(Duration::from_secs(2)).unwrap();
        // Should have been killed by SIGTERM
        assert_ne!(code, 0);
        assert!(!proc.is_alive());
    }

    // --- uptime ---

    #[test]
    fn uptime_increases() {
        let mut proc = spawn_process(&sleep_spec("10"), "test-sleep").unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(proc.uptime() >= Duration::from_millis(50));
        let _ = proc.kill();
        let _ = proc.wait();
    }

    // --- run_command ---

    #[test]
    fn run_command_true() {
        let cmd = SafeCommand {
            binary: "true".to_string(),
            args: vec![],
        };
        let code = run_command(&cmd).unwrap();
        assert_eq!(code, 0);
    }

    #[test]
    fn run_command_false() {
        let cmd = SafeCommand {
            binary: "false".to_string(),
            args: vec![],
        };
        let code = run_command(&cmd).unwrap();
        assert_ne!(code, 0);
    }

    #[test]
    fn run_command_echo() {
        let cmd = SafeCommand {
            binary: "echo".to_string(),
            args: vec!["hello".to_string()],
        };
        let code = run_command(&cmd).unwrap();
        assert_eq!(code, 0);
    }

    // --- run_command_sequence ---

    #[test]
    fn run_command_sequence_all_succeed() {
        let cmds = vec![
            SafeCommand {
                binary: "true".to_string(),
                args: vec![],
            },
            SafeCommand {
                binary: "true".to_string(),
                args: vec![],
            },
        ];
        assert!(run_command_sequence(&cmds).is_ok());
    }

    #[test]
    fn run_command_sequence_stops_on_failure() {
        let cmds = vec![
            SafeCommand {
                binary: "true".to_string(),
                args: vec![],
            },
            SafeCommand {
                binary: "false".to_string(),
                args: vec![],
            },
            SafeCommand {
                binary: "true".to_string(),
                args: vec![],
            },
        ];
        let err = run_command_sequence(&cmds).unwrap_err();
        assert!(err.to_string().contains("command 2 of 3 failed"));
    }

    // --- ProcessTable ---

    #[test]
    fn process_table_insert_and_get() {
        let mut table = ProcessTable::new();
        let proc = spawn_process(&sleep_spec("10"), "svc-a").unwrap();
        let pid = proc.pid;
        table.insert(proc);

        assert!(table.contains("svc-a"));
        assert_eq!(table.len(), 1);
        assert_eq!(table.pid_of("svc-a"), Some(pid));

        // Cleanup
        table.get_mut("svc-a").unwrap().kill().unwrap();
        table.get_mut("svc-a").unwrap().wait().unwrap();
    }

    #[test]
    fn process_table_remove() {
        let mut table = ProcessTable::new();
        let proc = spawn_process(&sleep_spec("10"), "svc-a").unwrap();
        table.insert(proc);

        let mut removed = table.remove("svc-a").unwrap();
        assert!(table.is_empty());

        let _ = removed.kill();
        let _ = removed.wait();
    }

    #[test]
    fn process_table_reap_exited() {
        let mut table = ProcessTable::new();
        table.insert(spawn_process(&true_spec(), "svc-fast").unwrap());

        // Wait for it to exit
        std::thread::sleep(Duration::from_millis(200));

        let reaped = table.reap_exited();
        assert_eq!(reaped.len(), 1);
        assert_eq!(reaped[0].0, "svc-fast");
        assert_eq!(reaped[0].1, 0);
        assert!(table.is_empty());
    }

    #[test]
    fn process_table_stop_all() {
        let mut table = ProcessTable::new();
        table.insert(spawn_process(&sleep_spec("60"), "svc-1").unwrap());
        table.insert(spawn_process(&sleep_spec("60"), "svc-2").unwrap());

        let results = table.stop_all(Duration::from_secs(2));
        assert_eq!(results.len(), 2);
        assert!(table.is_empty());
    }

    // --- stdout/stderr capture ---

    #[test]
    fn spawn_captures_stdout_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let stdout_path = dir.path().join("test.log");

        let spec = ProcessSpec {
            binary: PathBuf::from("/bin/echo"),
            args: vec!["hello from argonaut".to_string()],
            environment: HashMap::new(),
            working_dir: None,
            stdout_log: Some(stdout_path.clone()),
            stderr_log: None,
            uid: None,
            gid: None,
            resource_limits: None,
            log_config: None,
        };

        let mut proc = spawn_process(&spec, "echo-test").unwrap();
        proc.wait().unwrap();

        let content = std::fs::read_to_string(&stdout_path).unwrap();
        assert!(content.contains("hello from argonaut"));
    }

    #[test]
    fn spawn_captures_stderr_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let stderr_path = dir.path().join("test.err");

        // Use sh -c to write to stderr
        let spec = ProcessSpec {
            binary: PathBuf::from("/bin/sh"),
            args: vec!["-c".to_string(), "echo error-output >&2".to_string()],
            environment: HashMap::new(),
            working_dir: None,
            stdout_log: None,
            stderr_log: Some(stderr_path.clone()),
            uid: None,
            gid: None,
            resource_limits: None,
            log_config: None,
        };

        let mut proc = spawn_process(&spec, "stderr-test").unwrap();
        proc.wait().unwrap();

        let content = std::fs::read_to_string(&stderr_path).unwrap();
        assert!(content.contains("error-output"));
    }
}
