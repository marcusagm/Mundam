use std::collections::HashMap;
use std::path::{Path, PathBuf};

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;

use crate::ffmpeg::get_ffmpeg_path;

/// Manage linear transcoding sessions (Live HLS)
#[derive(Clone)]
pub struct LinearManager {
    sessions: Arc<RwLock<HashMap<String, LinearSession>>>,
    app_handle: tauri::AppHandle,
}

struct LinearSession {
    #[allow(dead_code)]
    process_id: Option<u32>,
    temp_dir: PathBuf,
    last_access: Instant,
    child: Option<Child>,
}

impl LinearManager {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            app_handle,
        }
    }

    /// Get valid session or start a new one
    pub async fn get_or_start(&self, file_path: &Path, quality: &str) -> Result<PathBuf, String> {
        let key = file_path.to_string_lossy().to_string();

        // 1. Check if active session exists
        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(&key) {
                // Check if process is still running
                if let Some(child) = &mut session.child {
                    // Try_wait returns Ok(None) if running, Ok(Some(status)) if exited, Err if error
                    match child.try_wait() {
                        Ok(None) => {
                            // Still running
                            session.last_access = Instant::now();
                            return Ok(session.temp_dir.clone());
                        }
                        Ok(Some(status)) => {
                            eprintln!("Linear ffmpeg exited prematurely: {}", status);
                            // It exited, so we need to restart it? Or maybe it finished?
                            // For HLS Live of a file, if it finished, the playlist is complete.
                            // We can still serve files from temp dir.
                            // But usually we treat "session" as "active transcoding".
                            // If index.m3u8 exists and has ENDLIST, we are good.
                            if session.temp_dir.join("index.m3u8").exists() {
                                session.last_access = Instant::now();
                                return Ok(session.temp_dir.clone());
                            }
                            // Else, fall through to restart
                        }
                        Err(e) => {
                            eprintln!("Error checking child status: {}", e);
                            // Fall through to restart
                        }
                    }
                }
            }
            // If we didn't return, we need to start fresh.
        }

        // 2. Start new session
        // Create temp dir
        let temp_dir_base = std::env::temp_dir().join("mundam_linear");
        // Ensure base exists
        if !temp_dir_base.exists() {
            tokio::fs::create_dir_all(&temp_dir_base).await.map_err(|e| e.to_string())?;
        }

        let session_id = uuid::Uuid::new_v4().to_string();
        let temp_dir = temp_dir_base.join(&session_id);
        tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| e.to_string())?;

        let ffmpeg_path = get_ffmpeg_path(Some(&self.app_handle))
            .ok_or("FFmpeg not found")?;

        // bitrate based on quality
        let video_bitrate = match quality {
            "high" => "5000k",
            "preview" => "1000k",
            _ => "2500k"
        };

        // Spawn FFmpeg
        let mut cmd = Command::new(ffmpeg_path);

        cmd.args([
            "-hide_banner",
            "-loglevel", "error",
            "-i", &key,
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-tune", "zerolatency",
            "-c:a", "aac",
            "-b:a", "128k",
            "-b:v", video_bitrate,
            "-f", "hls",
            "-hls_time", "4",
            "-hls_list_size", "0",
            "-hls_segment_filename", "segment_%05d.ts",
            // "-hls_flags", "delete_segments", // Don't delete, we might want to seek back
            "index.m3u8"
        ]);

        cmd.current_dir(&temp_dir);
        // Inherit stdio for debugging, or null to silence
        // cmd.stdout(Stdio::null());
        // cmd.stderr(Stdio::null());

        let child = cmd.spawn().map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;
        let pid = child.id();

        let session = LinearSession {
            process_id: pid,
            temp_dir: temp_dir.clone(),
            last_access: Instant::now(),
            child: Some(child),
        };

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(key, session);
        }

        Ok(temp_dir)
    }

    /// Clean up stale sessions
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
                println!("INFO: Cleaning up linear session for {}", key);
                // Kill process
                if let Some(mut child) = session.child.take() {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }

                // Remove temp dir
                let _ = tokio::fs::remove_dir_all(&session.temp_dir).await;
            }
        }
    }

    /// Get the temp directory for an active session
    pub async fn get_temp_dir(&self, file_path: &Path) -> Option<PathBuf> {
        let key = file_path.to_string_lossy().to_string();
        let sessions = self.sessions.read().await;
        sessions.get(&key).map(|s| s.temp_dir.clone())
    }
}
