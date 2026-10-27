//! Linear HLS Session Manager
//!
//! Manages temporary HLS sessions for formats that require full-file transcoding
//! or live-like playback (e.g., SWF, legacy mov, high-bitrate mkv).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{error, info};

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

/// Manager for linear transcoding sessions
#[derive(Clone)]
pub struct LinearManager {
    /// Active sessions keyed by canonical file path
    pub sessions: Arc<RwLock<HashMap<String, LinearSession>>>,
    /// Handle to the Tauri application
    app_handle: tauri::AppHandle,
}

impl LinearManager {
    /// Creates a new LinearManager instance
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
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
                        }
                    }
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
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-preset",
            "ultrafast",
            "-tune",
            "zerolatency",
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

        cmd.current_dir(&temp_dir);
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        #[cfg(windows)]
        cmd.creation_flags(0x08000000);

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn FFmpeg: {}", e))?;

        let session = LinearSession {
            child: Some(child),
            temp_dir: temp_dir.clone(),
            last_access: Instant::now(),
        };

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(key, session);
        }

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
