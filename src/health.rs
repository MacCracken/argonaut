//! Health check execution — runs health and readiness checks against
//! live services and returns structured results.

use std::net::TcpStream;
use std::time::{Duration, Instant};

use chrono::Utc;
use tracing::{debug, warn};

use crate::types::{HealthCheck, HealthCheckResult, HealthCheckType, ReadyCheck};

// ---------------------------------------------------------------------------
// Execute a single health check
// ---------------------------------------------------------------------------

/// Execute a health check and return the result.
///
/// Supports HTTP GET, TCP connect, command execution, and process-alive
/// checks. Each check is subject to the configured timeout.
#[must_use]
pub fn execute_health_check(
    service_name: &str,
    check: &HealthCheck,
    pid: Option<u32>,
) -> HealthCheckResult {
    let start = Instant::now();
    let timeout = Duration::from_millis(check.timeout_ms);

    let (passed, message) = match &check.check_type {
        HealthCheckType::HttpGet(url) => execute_http_get(url, timeout),
        HealthCheckType::TcpConnect(host, port) => execute_tcp_connect(host, *port, timeout),
        HealthCheckType::Command(cmd) => execute_command_check(cmd, timeout),
        HealthCheckType::ProcessAlive => execute_process_alive(pid),
    };

    let latency = start.elapsed();

    let result = HealthCheckResult {
        service: service_name.to_string(),
        check_type: check.check_type.to_string(),
        passed,
        latency_ms: latency.as_millis() as u64,
        message,
        checked_at: Utc::now(),
    };

    debug!(
        service = service_name,
        passed = result.passed,
        latency_ms = result.latency_ms,
        check_type = %result.check_type,
        "health check executed"
    );

    result
}

/// Execute a readiness check. Retries up to `retries` times with
/// `retry_delay_ms` between attempts. Returns the final result.
pub fn execute_ready_check(
    service_name: &str,
    check: &ReadyCheck,
    pid: Option<u32>,
) -> HealthCheckResult {
    let timeout = Duration::from_millis(check.timeout_ms);
    let delay = Duration::from_millis(check.retry_delay_ms);
    let overall_start = Instant::now();

    for attempt in 0..=check.retries {
        let start = Instant::now();
        let per_check_timeout = timeout.saturating_sub(overall_start.elapsed());

        if per_check_timeout.is_zero() {
            return HealthCheckResult {
                service: service_name.to_string(),
                check_type: check.check_type.to_string(),
                passed: false,
                latency_ms: overall_start.elapsed().as_millis() as u64,
                message: Some("ready check timed out".to_string()),
                checked_at: Utc::now(),
            };
        }

        let (passed, message) = match &check.check_type {
            HealthCheckType::HttpGet(url) => execute_http_get(url, per_check_timeout),
            HealthCheckType::TcpConnect(host, port) => {
                execute_tcp_connect(host, *port, per_check_timeout)
            }
            HealthCheckType::Command(cmd) => execute_command_check(cmd, per_check_timeout),
            HealthCheckType::ProcessAlive => execute_process_alive(pid),
        };

        let latency = start.elapsed();

        if passed {
            debug!(
                service = service_name,
                attempt = attempt,
                latency_ms = latency.as_millis() as u64,
                "ready check passed"
            );
            return HealthCheckResult {
                service: service_name.to_string(),
                check_type: check.check_type.to_string(),
                passed: true,
                latency_ms: latency.as_millis() as u64,
                message,
                checked_at: Utc::now(),
            };
        }

        if attempt < check.retries {
            debug!(
                service = service_name,
                attempt = attempt,
                retries_left = check.retries - attempt,
                "ready check failed, retrying"
            );
            std::thread::sleep(delay);
        }
    }

    warn!(
        service = service_name,
        retries = check.retries,
        "ready check failed after all retries"
    );
    HealthCheckResult {
        service: service_name.to_string(),
        check_type: check.check_type.to_string(),
        passed: false,
        latency_ms: overall_start.elapsed().as_millis() as u64,
        message: Some(format!(
            "ready check failed after {} retries",
            check.retries
        )),
        checked_at: Utc::now(),
    }
}

// ---------------------------------------------------------------------------
// Individual check implementations
// ---------------------------------------------------------------------------

/// HTTP GET check — expects 2xx status line in response.
///
/// Uses raw TCP + HTTP/1.1 to avoid heavy dependencies. Only checks
/// the status code — does not parse the body.
fn execute_http_get(url: &str, timeout: Duration) -> (bool, Option<String>) {
    use std::io::{BufRead, BufReader, Write};

    // Parse URL: http://host:port/path
    let url = url.strip_prefix("http://").unwrap_or(url);
    let (host_port, path) = match url.find('/') {
        Some(i) => (&url[..i], &url[i..]),
        None => (url, "/"),
    };

    let addr: std::net::SocketAddr = match host_port.parse() {
        Ok(a) => a,
        Err(_) => {
            // Try adding default port 80
            match format!("{host_port}:80").parse() {
                Ok(a) => a,
                Err(e) => return (false, Some(format!("invalid URL: {e}"))),
            }
        }
    };

    let stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(s) => s,
        Err(e) => return (false, Some(format!("connect failed: {e}"))),
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let request = format!("GET {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n\r\n");

    let mut writer = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => return (false, Some(format!("stream clone failed: {e}"))),
    };
    if let Err(e) = writer.write_all(request.as_bytes()) {
        return (false, Some(format!("write failed: {e}")));
    }

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    if let Err(e) = reader.read_line(&mut status_line) {
        return (false, Some(format!("read failed: {e}")));
    }

    // Parse "HTTP/1.1 200 OK"
    let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return (false, Some(format!("invalid status line: {status_line}")));
    }
    match parts[1].parse::<u16>() {
        Ok(code) if (200..300).contains(&code) => (true, Some(format!("HTTP {code}"))),
        Ok(code) => (false, Some(format!("HTTP {code}"))),
        Err(e) => (false, Some(format!("invalid status code: {e}"))),
    }
}

/// TCP connect check — connect and immediately close.
fn execute_tcp_connect(host: &str, port: u16, timeout: Duration) -> (bool, Option<String>) {
    let addr = format!("{host}:{port}");
    let socket_addr: std::net::SocketAddr = match addr.parse() {
        Ok(a) => a,
        Err(e) => return (false, Some(format!("invalid address {addr}: {e}"))),
    };
    match TcpStream::connect_timeout(&socket_addr, timeout) {
        Ok(_stream) => (true, Some(format!("TCP connect to {addr} succeeded"))),
        Err(e) => (false, Some(format!("TCP connect to {addr} failed: {e}"))),
    }
}

/// Command check — run command, exit code 0 = healthy. Enforces timeout.
fn execute_command_check(cmd_str: &str, timeout: Duration) -> (bool, Option<String>) {
    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    if parts.is_empty() {
        return (false, Some("empty command".to_string()));
    }

    let mut child = match std::process::Command::new(parts[0])
        .args(&parts[1..])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return (false, Some(format!("spawn failed: {e}"))),
    };

    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code().unwrap_or(-1);
                return if code == 0 {
                    (true, None)
                } else {
                    (false, Some(format!("command exited with code {code}")))
                };
            }
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return (false, Some("command timed out".to_string()));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return (false, Some(format!("wait failed: {e}"))),
        }
    }
}

/// ProcessAlive check — `kill(pid, 0)` to test if process exists.
fn execute_process_alive(pid: Option<u32>) -> (bool, Option<String>) {
    match pid {
        Some(pid) => {
            let raw = match i32::try_from(pid) {
                Ok(r) => r,
                Err(_) => {
                    return (
                        false,
                        Some(format!("PID {pid} exceeds i32::MAX, cannot check")),
                    );
                }
            };
            let nix_pid = nix::unistd::Pid::from_raw(raw);
            match nix::sys::signal::kill(nix_pid, None) {
                Ok(()) => (true, None),
                Err(e) => (false, Some(format!("process {pid} not alive: {e}"))),
            }
        }
        None => (false, Some("no PID tracked".to_string())),
    }
}

// ---------------------------------------------------------------------------
// Health state
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

/// Health state of a service.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthState {
    /// No checks have run yet.
    Unknown,
    /// All recent checks passed.
    Healthy,
    /// Some checks failing but under threshold.
    Degraded,
    /// Consecutive failures exceeded threshold.
    Unhealthy,
}

impl std::fmt::Display for HealthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

// ---------------------------------------------------------------------------
// Health history ring buffer
// ---------------------------------------------------------------------------

/// Ring buffer of recent health check results for a service.
#[derive(Debug, Clone)]
pub struct HealthHistory {
    /// Fixed-size buffer of recent results.
    results: Vec<HealthCheckResult>,
    /// Maximum number of results to keep.
    capacity: usize,
    /// Current write position (wraps around).
    write_pos: usize,
    /// Total number of results recorded (may exceed capacity).
    total: usize,
    /// Current consecutive failure count.
    consecutive_failures: u32,
    /// Current health state.
    pub state: HealthState,
}

impl HealthHistory {
    /// Create a new history buffer with the given capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            results: Vec::with_capacity(capacity),
            capacity,
            write_pos: 0,
            total: 0,
            consecutive_failures: 0,
            state: HealthState::Unknown,
        }
    }

    /// Record a health check result. Updates state based on
    /// consecutive failures vs the threshold.
    pub fn record(&mut self, result: HealthCheckResult, failure_threshold: u32) {
        if result.passed {
            self.consecutive_failures = 0;
            self.state = HealthState::Healthy;
        } else {
            self.consecutive_failures += 1;
            if self.consecutive_failures >= failure_threshold {
                self.state = HealthState::Unhealthy;
            } else {
                self.state = HealthState::Degraded;
            }
        }

        // Insert into ring buffer
        if self.results.len() < self.capacity {
            self.results.push(result);
        } else {
            self.results[self.write_pos] = result;
        }
        self.write_pos = (self.write_pos + 1) % self.capacity;
        self.total += 1;
    }

    /// Current consecutive failure count.
    #[must_use]
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// Total checks recorded (including overwritten ones).
    #[must_use]
    pub fn total_checks(&self) -> usize {
        self.total
    }

    /// Number of results currently in the buffer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Get the most recent result.
    #[must_use]
    pub fn latest(&self) -> Option<&HealthCheckResult> {
        if self.results.is_empty() {
            None
        } else {
            let idx = if self.write_pos == 0 {
                self.results.len() - 1
            } else {
                self.write_pos - 1
            };
            Some(&self.results[idx])
        }
    }

    /// Iterate over all results in chronological order.
    pub fn iter(&self) -> impl Iterator<Item = &HealthCheckResult> {
        let (a, b) = if self.results.len() < self.capacity {
            (self.results.as_slice(), &[] as &[HealthCheckResult])
        } else {
            let (second, first) = self.results.split_at(self.write_pos);
            (first, second)
        };
        a.iter().chain(b.iter())
    }

    /// Reset the history.
    pub fn reset(&mut self) {
        self.results.clear();
        self.write_pos = 0;
        self.total = 0;
        self.consecutive_failures = 0;
        self.state = HealthState::Unknown;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_result(passed: bool) -> HealthCheckResult {
        HealthCheckResult {
            service: "test-svc".into(),
            check_type: "test".into(),
            passed,
            latency_ms: 5,
            message: None,
            checked_at: Utc::now(),
        }
    }

    // --- execute checks ---

    #[test]
    fn process_alive_check_self() {
        let pid = std::process::id();
        let (alive, _) = execute_process_alive(Some(pid));
        assert!(alive);
    }

    #[test]
    fn process_alive_no_pid() {
        let (alive, msg) = execute_process_alive(None);
        assert!(!alive);
        assert!(msg.unwrap().contains("no PID"));
    }

    #[test]
    fn command_check_true() {
        let (passed, _) = execute_command_check("true", Duration::from_secs(5));
        assert!(passed);
    }

    #[test]
    fn command_check_false() {
        let (passed, msg) = execute_command_check("false", Duration::from_secs(5));
        assert!(!passed);
        assert!(msg.unwrap().contains("exit"));
    }

    #[test]
    fn command_check_empty() {
        let (passed, _) = execute_command_check("", Duration::from_secs(5));
        assert!(!passed);
    }

    #[test]
    fn tcp_connect_refuses() {
        // Port 1 is almost certainly not listening
        let (passed, msg) = execute_tcp_connect("127.0.0.1", 1, Duration::from_millis(500));
        assert!(!passed);
        assert!(msg.is_some());
    }

    #[test]
    fn http_get_invalid_url() {
        let (passed, msg) =
            execute_http_get("http://127.0.0.1:1/health", Duration::from_millis(500));
        assert!(!passed);
        assert!(msg.is_some());
    }

    // --- HealthState ---

    #[test]
    fn health_state_display() {
        assert_eq!(HealthState::Unknown.to_string(), "unknown");
        assert_eq!(HealthState::Healthy.to_string(), "healthy");
        assert_eq!(HealthState::Degraded.to_string(), "degraded");
        assert_eq!(HealthState::Unhealthy.to_string(), "unhealthy");
    }

    // --- HealthHistory ---

    #[test]
    fn history_starts_unknown() {
        let h = HealthHistory::new(10);
        assert_eq!(h.state, HealthState::Unknown);
        assert!(h.is_empty());
        assert_eq!(h.consecutive_failures(), 0);
    }

    #[test]
    fn history_becomes_healthy_on_pass() {
        let mut h = HealthHistory::new(10);
        h.record(mock_result(true), 3);
        assert_eq!(h.state, HealthState::Healthy);
        assert_eq!(h.consecutive_failures(), 0);
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn history_becomes_degraded_on_failure() {
        let mut h = HealthHistory::new(10);
        h.record(mock_result(false), 3);
        assert_eq!(h.state, HealthState::Degraded);
        assert_eq!(h.consecutive_failures(), 1);
    }

    #[test]
    fn history_becomes_unhealthy_at_threshold() {
        let mut h = HealthHistory::new(10);
        h.record(mock_result(false), 3);
        assert_eq!(h.state, HealthState::Degraded);
        h.record(mock_result(false), 3);
        assert_eq!(h.state, HealthState::Degraded);
        h.record(mock_result(false), 3);
        assert_eq!(h.state, HealthState::Unhealthy);
        assert_eq!(h.consecutive_failures(), 3);
    }

    #[test]
    fn history_resets_on_pass() {
        let mut h = HealthHistory::new(10);
        h.record(mock_result(false), 3);
        h.record(mock_result(false), 3);
        h.record(mock_result(true), 3);
        assert_eq!(h.state, HealthState::Healthy);
        assert_eq!(h.consecutive_failures(), 0);
    }

    #[test]
    fn history_ring_buffer_wraps() {
        let mut h = HealthHistory::new(3);
        for _ in 0..5 {
            h.record(mock_result(true), 3);
        }
        assert_eq!(h.len(), 3); // capped at capacity
        assert_eq!(h.total_checks(), 5);
    }

    #[test]
    fn history_latest_returns_most_recent() {
        let mut h = HealthHistory::new(10);
        h.record(mock_result(true), 3);
        h.record(mock_result(false), 3);
        assert!(!h.latest().unwrap().passed);
    }

    #[test]
    fn history_reset_clears_all() {
        let mut h = HealthHistory::new(10);
        h.record(mock_result(false), 3);
        h.record(mock_result(false), 3);
        h.reset();
        assert_eq!(h.state, HealthState::Unknown);
        assert!(h.is_empty());
        assert_eq!(h.consecutive_failures(), 0);
        assert_eq!(h.total_checks(), 0);
    }

    // --- Serde ---

    #[test]
    fn serde_health_state() {
        for state in [
            HealthState::Unknown,
            HealthState::Healthy,
            HealthState::Degraded,
            HealthState::Unhealthy,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let back: HealthState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, back);
        }
    }
}
