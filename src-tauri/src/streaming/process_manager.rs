//! Process Manager for FFmpeg Transcoding
//!
//! Tracks active FFmpeg processes and allows cancellation for rapid seeking.

use std::collections::HashMap;
use std::time::Instant;
use tokio::process::Child;

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

impl ProcessManager {
    /// Create a new process manager
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
        }
    }

    /// Register a new transcoding process
    pub fn register(&mut self, key: &str, child: &Child) {
        let info = ProcessInfo {
            process_id: child.id(),
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

            // The actual process killing happens when the Child is dropped
            // or we could use kill() if we stored the Child handle
        }
    }

    /// Check if a segment is currently being processed
    pub fn is_processing(&self, key: &str) -> bool {
        self.processes.contains_key(key)
    }

    /// Clean up old/orphaned processes (older than timeout)
    pub fn cleanup_stale(&mut self, timeout_secs: u64) {
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let now = Instant::now();

        self.processes.retain(|key, info| {
            let elapsed = now.duration_since(info.started_at);
            if elapsed > timeout {
                println!("WARN: Cleaning up stale process for {}", key);
                false
            } else {
                true
            }
        });
    }

    /// Get number of active processes
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
