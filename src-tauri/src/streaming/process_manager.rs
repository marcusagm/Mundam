//! Process Manager for FFmpeg Transcoding
//!
//! Tracks active FFmpeg processes and allows cancellation for rapid seeking.

use std::collections::HashMap;
use std::time::Instant;

/// Manages active FFmpeg transcoding processes
pub struct ProcessManager {
    /// Active processes keyed by segment identifier
    processes: HashMap<String, ProcessInfo>,
}

/// Information about an active process
struct ProcessInfo {
    /// Process handle (if we're tracking it)
    #[allow(dead_code)]
    process_id: Option<u32>,
    /// When the process started
    started_at: Instant,
}

#[cfg(unix)]
fn kill_process(pid: u32) {
    use std::process::Command;
    Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .output()
        .ok();
}

#[cfg(windows)]
fn kill_process(pid: u32) {
    use std::process::Command;
    Command::new("taskkill")
        .arg("/F")
        .arg("/PID")
        .arg(pid.to_string())
        .output()
        .ok();
}

impl ProcessManager {
    /// Create a new process manager
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
        }
    }

    /// Register a new transcoding process
    #[allow(dead_code)]
    pub fn register(&mut self, key: &str, pid: u32) {
        let info = ProcessInfo {
            process_id: Some(pid),
            started_at: Instant::now(),
        };
        self.processes.insert(key.to_string(), info);
    }

    /// Cancel a transcoding process by key
    pub fn cancel(&mut self, key: &str) {
        if let Some(info) = self.processes.remove(key) {
            // Log cancellation
            let elapsed = info.started_at.elapsed();
            println!("INFO: Cancelled segment {} after {:?}", key, elapsed);

            // Kill the process if we have an ID
            if let Some(pid) = info.process_id {
                kill_process(pid);
            }
        }
    }

    /// Check if a segment is currently being processed
    #[allow(dead_code)]
    pub fn is_processing(&self, key: &str) -> bool {
        self.processes.contains_key(key)
    }

    /// Clean up old/orphaned processes (older than timeout)
    #[allow(dead_code)]
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
            println!("WARN: Cleaning up stale process for {}", key);
            self.cancel(&key); // This will remove from map AND kill process
        }
    }

    /// Get number of active processes
    #[allow(dead_code)]
    pub fn active_count(&self) -> usize {
        self.processes.len()
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_manager_basic() {
        let mut pm = ProcessManager::new();

        assert_eq!(pm.active_count(), 0);
        assert!(!pm.is_processing("test:0"));
    }
}
