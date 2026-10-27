//! Process Manager for FFmpeg Transcoding
//!
//! Tracks active FFmpeg processes and allows cancellation for rapid seeking.
//! This ensures that when a user seeks quickly, previous segment transcoding
//! processes are terminated to save CPU resources.

use std::collections::HashMap;
use std::time::Instant;
use tracing::{info, warn};

/// Information about an active process
struct ProcessInfo {
    /// Process handle/ID
    process_id: u32,
    /// When the process started
    started_at: Instant,
}

/// Manages active FFmpeg transcoding processes
#[derive(Default)]
pub struct ProcessManager {
    /// Active processes keyed by segment identifier (e.g., "asset_id:index:quality")
    processes: HashMap<String, ProcessInfo>,
}

#[cfg(unix)]
fn kill_process(pid: u32) {
    use std::process::Command;
    let _ = Command::new("kill").arg("-9").arg(pid.to_string()).output();
}

#[cfg(windows)]
fn kill_process(pid: u32) {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("taskkill");
    cmd.creation_flags(0x08000000);
    let _ = cmd
        .arg("/F")
        .arg("/PID")
        .arg(pid.to_string())
        .output();
}

impl ProcessManager {
    /// Create a new process manager
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
        }
    }

    /// Register a new transcoding process
    pub fn register(&mut self, key: String, pid: u32) {
        let info = ProcessInfo {
            process_id: pid,
            started_at: Instant::now(),
        };
        self.processes.insert(key, info);
    }

    /// Cancel a transcoding process by key
    pub fn cancel(&mut self, key: &str) {
        if let Some(info) = self.processes.remove(key) {
            let elapsed = info.started_at.elapsed();
            info!("Cancelled segment {} after {:?}", key, elapsed);
            kill_process(info.process_id);
        }
    }

    /// Remove a process from tracking without killing it (e.g. after successful completion)
    pub fn remove(&mut self, key: &str) {
        self.processes.remove(key);
    }

    /// Check if a segment is currently being processed
    pub fn is_processing(&self, key: &str) -> bool {
        self.processes.contains_key(key)
    }

    /// Clean up old/orphaned processes (older than timeout)
    pub fn cleanup_stale(&mut self, timeout_secs: u64) {
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (key, info) in &self.processes {
            if now.duration_since(info.started_at) > timeout {
                to_remove.push(key.clone());
            }
        }

        for key in to_remove {
            warn!("Cleaning up stale process for {}", key);
            self.cancel(&key);
        }
    }

    /// Get number of active processes
    pub fn active_count(&self) -> usize {
        self.processes.len()
    }
}
