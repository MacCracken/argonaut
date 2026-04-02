//! sd_notify compatible readiness notification.
//!
//! Implements the server side of the systemd notification protocol.
//! Services send `READY=1` (and optionally `STATUS=...`) to a Unix
//! datagram socket. Argonaut listens on this socket and transitions
//! the service from `Starting` to `Running` when `READY=1` is received.

use std::collections::HashMap;
use std::io;
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tracing::{debug, info, warn};

/// Parsed fields from an sd_notify message.
#[derive(Debug, Clone, Default)]
pub struct NotifyMessage {
    /// Whether READY=1 was present.
    pub ready: bool,
    /// STATUS=<string> if present.
    pub status: Option<String>,
    /// MAINPID=<pid> if present.
    pub main_pid: Option<u32>,
    /// All key=value pairs from the message.
    pub fields: HashMap<String, String>,
}

impl NotifyMessage {
    /// Parse an sd_notify message (newline-separated KEY=VALUE pairs).
    #[must_use]
    pub fn parse(data: &[u8]) -> Self {
        let text = String::from_utf8_lossy(data);
        let mut msg = NotifyMessage::default();

        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                msg.fields.insert(key.to_string(), value.to_string());

                match key {
                    "READY" => msg.ready = value == "1",
                    "STATUS" => msg.status = Some(value.to_string()),
                    "MAINPID" => msg.main_pid = value.parse().ok(),
                    _ => {}
                }
            }
        }

        msg
    }
}

/// A listener for sd_notify messages on a Unix datagram socket.
pub struct NotifyListener {
    socket: UnixDatagram,
    /// Path to the socket file.
    pub path: PathBuf,
}

impl NotifyListener {
    /// Create a new notify listener at the given socket path.
    ///
    /// The socket file is created (or replaced if it already exists).
    /// The path is typically `/run/argonaut/notify` or a temp path
    /// for testing.
    pub fn bind(path: &Path) -> io::Result<Self> {
        // Remove stale socket if it exists (atomic, no TOCTOU)
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let socket = UnixDatagram::bind(path)?;
        socket.set_nonblocking(true)?;

        info!(path = %path.display(), "notify listener bound");

        Ok(Self {
            socket,
            path: path.to_path_buf(),
        })
    }

    /// Set the receive timeout on the socket.
    pub fn set_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.socket.set_read_timeout(timeout)
    }

    /// Try to receive a notification without blocking.
    /// Returns `None` if no message is available.
    pub fn try_recv(&self) -> Option<NotifyMessage> {
        let mut buf = [0u8; 4096];
        match self.socket.recv(&mut buf) {
            Ok(n) => {
                let msg = NotifyMessage::parse(&buf[..n]);
                debug!(
                    ready = msg.ready,
                    status = ?msg.status,
                    main_pid = ?msg.main_pid,
                    "received notify message"
                );
                Some(msg)
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => None,
            Err(e) => {
                warn!(error = %e, "error receiving notify message");
                None
            }
        }
    }

    /// Drain all pending notifications. Returns all parsed messages.
    pub fn drain(&self) -> Vec<NotifyMessage> {
        let mut messages = Vec::new();
        while let Some(msg) = self.try_recv() {
            messages.push(msg);
        }
        messages
    }

    /// Get the socket path as a string, suitable for setting the
    /// `NOTIFY_SOCKET` environment variable on spawned services.
    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.path
    }
}

impl Drop for NotifyListener {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Send an sd_notify message to a socket path (client side, for testing).
pub fn send_notify(socket_path: &Path, message: &str) -> io::Result<()> {
    let sock = UnixDatagram::unbound()?;
    sock.send_to(message.as_bytes(), socket_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ready_message() {
        let msg = NotifyMessage::parse(b"READY=1\nSTATUS=running\n");
        assert!(msg.ready);
        assert_eq!(msg.status.as_deref(), Some("running"));
    }

    #[test]
    fn parse_not_ready() {
        let msg = NotifyMessage::parse(b"STATUS=starting\n");
        assert!(!msg.ready);
        assert_eq!(msg.status.as_deref(), Some("starting"));
    }

    #[test]
    fn parse_mainpid() {
        let msg = NotifyMessage::parse(b"READY=1\nMAINPID=42\n");
        assert!(msg.ready);
        assert_eq!(msg.main_pid, Some(42));
    }

    #[test]
    fn parse_empty() {
        let msg = NotifyMessage::parse(b"");
        assert!(!msg.ready);
        assert!(msg.status.is_none());
    }

    #[test]
    fn parse_preserves_all_fields() {
        let msg = NotifyMessage::parse(b"READY=1\nCUSTOM=hello\n");
        assert_eq!(msg.fields.get("CUSTOM").unwrap(), "hello");
    }

    #[test]
    fn listener_bind_and_recv() {
        let dir = tempfile::tempdir().unwrap();
        let sock_path = dir.path().join("notify.sock");

        let listener = NotifyListener::bind(&sock_path).unwrap();
        assert!(sock_path.exists());

        // Send a ready message
        send_notify(&sock_path, "READY=1\nSTATUS=ok\n").unwrap();

        let msg = listener.try_recv().unwrap();
        assert!(msg.ready);
        assert_eq!(msg.status.as_deref(), Some("ok"));
    }

    #[test]
    fn listener_try_recv_empty_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let sock_path = dir.path().join("notify.sock");
        let listener = NotifyListener::bind(&sock_path).unwrap();

        assert!(listener.try_recv().is_none());
    }

    #[test]
    fn listener_drain_multiple() {
        let dir = tempfile::tempdir().unwrap();
        let sock_path = dir.path().join("notify.sock");
        let listener = NotifyListener::bind(&sock_path).unwrap();

        send_notify(&sock_path, "STATUS=starting\n").unwrap();
        send_notify(&sock_path, "READY=1\n").unwrap();

        let msgs = listener.drain();
        assert_eq!(msgs.len(), 2);
        assert!(!msgs[0].ready);
        assert!(msgs[1].ready);
    }

    #[test]
    fn listener_cleanup_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let sock_path = dir.path().join("notify.sock");

        {
            let _listener = NotifyListener::bind(&sock_path).unwrap();
            assert!(sock_path.exists());
        }
        // After drop, socket file should be cleaned up
        assert!(!sock_path.exists());
    }
}
