//! Linear HLS Session Manager
//!
//! Manages temporary HLS sessions for formats that require full-file transcoding
//! or live-like playback (e.g., SWF, legacy mov, high-bitrate mkv).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, MutexGuard, RwLock};
use tracing::{error, info, warn};

use crate::processing::transcoding::resolve_transcoding_tools;

/// A single linear transcoding session
pub struct LinearSession {
    /// FFmpeg child process
    pub child: Option<Child>,
    /// Temporary directory where fragments are stored
    pub temp_dir: PathBuf,
    /// Last time this session was accessed by a player
    pub last_access: Instant,
}

/// Summary of attempts to drain linear sessions during shutdown.
pub struct LinearShutdownReport {
    /// Number of sessions processed.
    pub session_count: usize,
    /// Number of child processes whose state or termination could not be verified.
    pub process_failures: usize,
    /// Number of session directories that could not be removed.
    pub directory_failures: usize,
}

impl LinearShutdownReport {
    /// Whether every attempted cleanup completed without a reported failure.
    pub fn is_clean(&self) -> bool {
        self.process_failures == 0 && self.directory_failures == 0
    }
}

/// Manager for linear transcoding sessions
#[derive(Clone)]
pub struct LinearManager {
    /// Active sessions keyed by canonical file path
    pub sessions: Arc<RwLock<HashMap<String, LinearSession>>>,
    admission: Arc<LinearAdmission>,
    /// Handle to the Tauri application
    app_handle: tauri::AppHandle,
}

#[derive(Default)]
struct LinearAdmission {
    closed: AtomicBool,
    start: Mutex<()>,
}

impl LinearAdmission {
    async fn enter(&self) -> Result<MutexGuard<'_, ()>, String> {
        let start = self.start.lock().await;
        if self.closed.load(Ordering::Acquire) {
            return Err("Linear streaming admission is closed during shutdown".into());
        }
        Ok(start)
    }

    async fn close(&self) {
        self.closed.store(true, Ordering::Release);
        let _start = self.start.lock().await;
    }
}

impl LinearManager {
    /// Creates a new LinearManager instance
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            admission: Arc::new(LinearAdmission::default()),
            app_handle,
        }
    }

    /// Get valid session or start a new one
    pub async fn get_or_start(
        &self,
        asset_id: &str,
        file_path: &Path,
        quality: &str,
    ) -> Result<PathBuf, String> {
        let _admission = self.admission.enter().await?;
        let key = asset_id.to_string();

        // 1. Check if active session exists
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(&key) {
                if let Some(child) = &mut session.child {
                    match child.try_wait() {
                        Ok(None) => {
                            // Still running
                            session.last_access = Instant::now();
                            return Ok(session.temp_dir.clone());
                        }
                        Ok(Some(status)) => {
                            info!(
                                "Linear FFmpeg exited: {}. Checking for completion...",
                                status
                            );
                            if session.temp_dir.join("index.m3u8").exists() {
                                session.last_access = Instant::now();
                                return Ok(session.temp_dir.clone());
                            }
                        }
                        Err(e) => {
                            error!("Error checking linear child status: {}", e);
                            return Err(format!("Failed to check linear child status: {}", e));
                        }
                    }
                }
            }
        }

        let previous = self.sessions.write().await.remove(&key);
        if let Some(session) = previous {
            if let Err(error) = tokio::fs::remove_dir_all(&session.temp_dir).await {
                if error.kind() != std::io::ErrorKind::NotFound {
                    return Err(format!(
                        "Failed to remove previous linear session: {}",
                        error
                    ));
                }
            }
        }

        // 2. Start new session
        let app_data = self
            .app_handle
            .path()
            .app_local_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;

        let temp_dir_base = app_data.join("streams").join("linear");
        if !temp_dir_base.exists() {
            tokio::fs::create_dir_all(&temp_dir_base)
                .await
                .map_err(|e| format!("Failed to create linear base dir: {}", e))?;
        }

        let session_id = uuid::Uuid::new_v4().to_string();
        let temp_dir = temp_dir_base.join(&session_id);
        tokio::fs::create_dir_all(&temp_dir)
            .await
            .map_err(|e| format!("Failed to create session dir: {}", e))?;

        let tools = resolve_transcoding_tools::<tauri::Wry>(None)
            .map_err(|e| format!("FFmpeg tools not found: {:?}", e))?;
        let h264_encoder = crate::processing::transcoding::resolve_h264_encoder(&tools.ffmpeg)
            .map_err(|error| error.to_string())?;

        // Determine bitrate based on quality
        let video_bitrate = match quality {
            "high" => "5000k",
            "low" => "1000k",
            _ => "2500k",
        };

        let mut actual_input = file_path.to_path_buf();

        if let Some(ext) = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
        {
            if ext == "mid" || ext == "midi" {
                let wav_path = temp_dir.join("synthesized.wav");
                // Run the synthesis
                if let Err(e) =
                    crate::processing::media::extractors::midi_renderer::render_midi_to_wav(
                        file_path,
                        &wav_path,
                        Some(&self.app_handle),
                    )
                    .await
                {
                    error!("MIDI Synthesis failed: {}", e);
                    return Err(format!("MIDI synthesis failed: {}", e));
                }
                // FFmpeg will now transcode the synthesized WAV
                actual_input = wav_path;
            }
        }

        let mut cmd = Command::new(&tools.ffmpeg);
        cmd.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            &actual_input.to_string_lossy(),
            "-c:v",
            h264_encoder,
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-b:v",
            video_bitrate,
            "-f",
            "hls",
            "-hls_time",
            "4",
            "-hls_list_size",
            "0",
            "-hls_segment_filename",
            "segment_%05d.ts",
            "index.m3u8",
        ]);
        if h264_encoder == "libx264" {
            cmd.args(["-preset", "ultrafast", "-tune", "zerolatency"]);
        }

        cmd.current_dir(&temp_dir);
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        cmd.kill_on_drop(true);

        #[cfg(windows)]
        cmd.creation_flags(0x08000000);

        let mut sessions = self.sessions.write().await;
        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn FFmpeg: {}", e))?;

        let session = LinearSession {
            child: Some(child),
            temp_dir: temp_dir.clone(),
            last_access: Instant::now(),
        };

        sessions.insert(key, session);

        Ok(temp_dir)
    }

    /// Clean up sessions that have been inactive for longer than the timeout
    pub async fn cleanup(&self, timeout: Duration) {
        let mut sessions = self.sessions.write().await;
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (key, session) in sessions.iter() {
            if now.duration_since(session.last_access) > timeout {
                to_remove.push(key.clone());
            }
        }

        for key in to_remove {
            if let Some(mut session) = sessions.remove(&key) {
                info!("Cleaning up linear session for {}", key);
                if let Some(mut child) = session.child.take() {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
                let _ = tokio::fs::remove_dir_all(&session.temp_dir).await;
            }
        }
    }

    /// Terminate every active session and remove its temporary files.
    pub async fn shutdown(&self) -> LinearShutdownReport {
        self.admission.close().await;
        let mut sessions = self.sessions.write().await;
        let count = sessions.len();
        let started = Instant::now();
        let mut process_failures = 0;
        let mut directory_failures = 0;
        let mut running_processes = 0;
        let mut exited_processes = 0;
        let mut process_wait_ms = 0;
        let mut directory_cleanup_ms = 0;
        let mut slowest_session_ms = 0;
        info!(
            session_count = count,
            "Linear streaming: shutdown cleanup started"
        );
        for (_, mut session) in sessions.drain() {
            let session_started = Instant::now();
            let process_started = Instant::now();
            if let Some(mut child) = session.child.take() {
                match child.try_wait() {
                    Ok(Some(_)) => exited_processes += 1,
                    Ok(None) => {
                        running_processes += 1;
                        if let Err(error) = child.kill().await {
                            process_failures += 1;
                            warn!(%error, "Linear streaming: failed to terminate and wait for child");
                        }
                    }
                    Err(error) => {
                        process_failures += 1;
                        warn!(%error, "Linear streaming: failed to check child status");
                        if let Err(error) = child.kill().await {
                            warn!(%error, "Linear streaming: failed to terminate and wait for child");
                        }
                    }
                }
            }
            let session_process_ms = process_started.elapsed().as_millis() as u64;
            process_wait_ms += session_process_ms;
            let directory_started = Instant::now();
            if let Err(error) = tokio::fs::remove_dir_all(&session.temp_dir).await {
                if error.kind() != std::io::ErrorKind::NotFound {
                    directory_failures += 1;
                    warn!(%error, "Linear streaming: failed to remove session directory");
                }
            }
            let session_directory_ms = directory_started.elapsed().as_millis() as u64;
            directory_cleanup_ms += session_directory_ms;
            let session_elapsed_ms = session_started.elapsed().as_millis() as u64;
            slowest_session_ms = slowest_session_ms.max(session_elapsed_ms);
            tracing::debug!(
                process_wait_ms = session_process_ms,
                directory_cleanup_ms = session_directory_ms,
                elapsed_ms = session_elapsed_ms,
                "Linear streaming: session shutdown cleanup timing"
            );
        }
        info!(
            session_count = count,
            process_failures,
            directory_failures,
            running_processes,
            exited_processes,
            process_wait_ms,
            directory_cleanup_ms,
            slowest_session_ms,
            elapsed_ms = started.elapsed().as_millis() as u64,
            "Linear streaming: shutdown cleanup finished"
        );
        LinearShutdownReport {
            session_count: count,
            process_failures,
            directory_failures,
        }
    }

    /// Get active session
    pub async fn get_session(
        &self,
        asset_id: &str,
    ) -> Option<tokio::sync::RwLockReadGuard<'_, HashMap<String, LinearSession>>> {
        let sessions = self.sessions.read().await;
        if sessions.contains_key(asset_id) {
            Some(sessions)
        } else {
            None
        }
    }

    /// Get the temp directory for an active session
    pub async fn get_temp_dir(&self, asset_id: &str) -> Option<PathBuf> {
        let key = asset_id.to_string();
        let sessions = self.sessions.read().await;
        sessions.get(&key).map(|s| s.temp_dir.clone())
    }

    /// Update the last_access time for a session to prevent timeout
    pub async fn update_access(&self, asset_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(asset_id) {
            session.last_access = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_report_distinguishes_cleanup_failures() {
        let mut report = LinearShutdownReport {
            session_count: 2,
            process_failures: 0,
            directory_failures: 0,
        };
        assert!(report.is_clean());
        report.process_failures = 1;
        assert!(!report.is_clean());
        report.process_failures = 0;
        report.directory_failures = 1;
        assert!(!report.is_clean());
    }

    #[tokio::test]
    async fn shutdown_waits_for_admission_and_rejects_late_starts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let admission = Arc::new(LinearAdmission::default());
        let guard = admission.enter().await?;
        let queued = admission.clone();
        let waiting = tokio::spawn(async move {
            let _guard = queued.enter().await?;
            Ok::<(), String>(())
        });
        tokio::task::yield_now().await;
        let closing = admission.clone();
        let mut shutdown = tokio::spawn(async move { closing.close().await });
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut shutdown)
                .await
                .is_err()
        );
        assert!(admission.closed.load(Ordering::Acquire));
        drop(guard);
        tokio::time::timeout(Duration::from_secs(2), shutdown).await??;
        assert!(tokio::time::timeout(Duration::from_secs(2), waiting)
            .await??
            .is_err());
        assert!(admission.enter().await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn only_one_session_start_is_admitted_at_a_time() -> Result<(), Box<dyn std::error::Error>>
    {
        let admission = Arc::new(LinearAdmission::default());
        let guard = admission.enter().await?;
        let second = admission.clone();
        let mut waiting = tokio::spawn(async move {
            let _guard = second.enter().await?;
            Ok::<(), String>(())
        });
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut waiting)
                .await
                .is_err()
        );
        drop(guard);
        tokio::time::timeout(Duration::from_secs(2), waiting)
            .await??
            .map_err(std::io::Error::other)?;
        Ok(())
    }
}
