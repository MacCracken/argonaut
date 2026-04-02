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

use crate::types::{ProcessSpec, SafeCommand};

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
#[derive(Debug)]
pub struct SpawnedProcess {
    /// The OS child process handle.
    child: Child,
    /// Service name this process belongs to.
    pub service_name: String,
    /// PID of the spawned process.
    pub pid: u32,
    /// When the process was spawned.
    pub started_at: Instant,
    /// Path to stdout log file.
    pub stdout_log: Option<PathBuf>,
    /// Path to stderr log file.
    pub stderr_log: Option<PathBuf>,
}

impl SpawnedProcess {
    /// Check if the process has exited without blocking.
    /// Returns `Some(exit_code)` if exited, `None` if still running.
    pub fn try_wait(&mut self) -> Result<Option<i32>> {
        match self.child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code().unwrap_or(-1);
                debug!(
                    service = %self.service_name,
                    pid = self.pid,
                    exit_code = code,
                    "process exited"
                );
                Ok(Some(code))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                error!(
                    service = %self.service_name,
                    pid = self.pid,
                    error = %e,
                    "failed to check process status"
                );
                Err(e).context("try_wait failed")
            }
        }
    }

    /// Block until the process exits. Returns the exit code.
    pub fn wait(&mut self) -> Result<i32> {
        let status = self
            .child
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
    }

    /// Send a signal to the process.
    pub fn signal(&self, sig: Signal) -> Result<()> {
        let nix_pid = Pid::from_raw(self.pid as i32);
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
        let nix_pid = Pid::from_raw(self.pid as i32);
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
        child,
        service_name: service_name.to_string(),
        pid,
        started_at: Instant::now(),
        stdout_log,
        stderr_log,
    })
}

/// Spawn a process from a `SafeCommand` (used for boot/shutdown helper commands).
///
/// Unlike service processes, these are one-shot commands that run to
/// completion. Stdout/stderr are inherited from the parent.
pub fn run_command(cmd: &SafeCommand) -> Result<i32> {
    info!(command = %cmd, "executing command");

    let output = Command::new(&cmd.binary)
        .args(&cmd.args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("failed to execute command: {}", cmd))?;

    let code = output.status.code().unwrap_or(-1);

    if output.status.success() {
        debug!(command = %cmd, exit_code = code, "command completed successfully");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            command = %cmd,
            exit_code = code,
            stderr = %stderr.trim(),
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
        if self.processes.contains_key(&name) {
            warn!(
                service = %name,
                "replacing existing process table entry"
            );
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
                match proc.wait() {
                    Ok(code) => results.push((name.clone(), code)),
                    Err(e) => error!(service = %name, error = %e, "failed to wait after SIGKILL"),
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
        };

        let mut proc = spawn_process(&spec, "stderr-test").unwrap();
        proc.wait().unwrap();

        let content = std::fs::read_to_string(&stderr_path).unwrap();
        assert!(content.contains("error-output"));
    }
}
