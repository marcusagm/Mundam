//! Video Probe Module
//!
//! Uses ffprobe to extract video metadata and determine if format is native.

use serde::Serialize;
use std::path::Path;
use std::process::{Output, Stdio};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::{watch, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::error;

use super::process_manager::{CancellationReason, ProcessManager};
use crate::feature::transcoding::detector;
use crate::processing::transcoding::resolve_transcoding_tools;

#[derive(Debug, thiserror::Error)]
enum ProbeProcessError {
    #[error("Probe cancelled: {0:?}")]
    Cancelled(CancellationReason),
    #[error("Probe process supervision failed: {0}")]
    Supervision(#[from] std::io::Error),
}

pub(super) fn is_shutdown_cancellation(error: &(dyn std::error::Error + 'static)) -> bool {
    matches!(
        error.downcast_ref::<ProbeProcessError>(),
        Some(ProbeProcessError::Cancelled(CancellationReason::Shutdown))
    )
}

/// Video information returned by probe
#[derive(Debug, Clone, Serialize)]
pub struct VideoInfo {
    /// Duration in seconds
    pub duration_secs: f64,
    /// Whether the format is natively playable in WebView
    pub is_native: bool,
    /// Video codec (e.g., "h264", "vp9")
    pub video_codec: Option<String>,
    /// Audio codec (e.g., "aac", "opus")
    pub audio_codec: Option<String>,
    /// Container format (e.g., "mp4", "mkv")
    pub container: Option<String>,
    /// Resolution width
    pub width: Option<u32>,
    /// Resolution height
    pub height: Option<u32>,
}

/// Get video information using ffprobe
pub async fn get_video_info(
    app_handle: &tauri::AppHandle,
    registry: &crate::core::formats::registry::FormatRegistry,
    path: &Path,
    process_manager: &Arc<RwLock<ProcessManager>>,
    shutdown_token: &CancellationToken,
) -> Result<VideoInfo, Box<dyn std::error::Error + Send + Sync>> {
    let started = std::time::Instant::now();
    if shutdown_token.is_cancelled() {
        return Err(ProbeProcessError::Cancelled(CancellationReason::Shutdown).into());
    }
    // Check if file exists
    if !tokio::fs::try_exists(path).await? {
        return Err(format!("File not found: {:?}", path).into());
    }
    if shutdown_token.is_cancelled() {
        return Err(ProbeProcessError::Cancelled(CancellationReason::Shutdown).into());
    }

    // Get transcoding tools (ffprobe)
    let tools = resolve_transcoding_tools(Some(app_handle))?;
    let ffprobe_path = tools.ffprobe;

    // Fast path for MIDI: bypass ffprobe since it fails on sequence files
    if let Some(ext) = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
    {
        if ext == "mid" || ext == "midi" {
            let midi_path = path.to_path_buf();
            let duration_secs = tokio::task::spawn_blocking(move || {
                crate::processing::media::extractors::midi_renderer::get_midi_length(&midi_path)
                    .unwrap_or(0.0)
            })
            .await?;
            return Ok(VideoInfo {
                duration_secs,
                is_native: false, // requires transcoding
                video_codec: None,
                audio_codec: Some("midi".to_string()),
                container: Some("midi".to_string()),
                width: None,
                height: None,
            });
        }
    }

    // Run ffprobe with JSON output
    let mut cmd = Command::new(&ffprobe_path);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    cmd.args([
        "-v",
        "error",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        &path.to_string_lossy(),
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    let owner = start_probe_process(&mut cmd, process_manager, shutdown_token).await?;
    let output = owner.await??;
    tracing::debug!(status = %output.status, elapsed_ms = started.elapsed().as_millis(), "FFprobe completed");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("FFprobe failed for {:?}: {}", path, stderr);
        return Err(format!("FFprobe failed: {}", stderr).into());
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    // Extract duration from format
    let duration_secs = json["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    // Extract format name
    let container = json["format"]["format_name"]
        .as_str()
        .map(|s| s.split(',').next().unwrap_or(s).to_string());

    // Find video and audio streams
    let streams = json["streams"].as_array();

    let mut video_codec = None;
    let mut audio_codec = None;
    let mut width = None;
    let mut height = None;

    if let Some(streams) = streams {
        for stream in streams {
            let codec_type = stream["codec_type"].as_str().unwrap_or("");
            let codec_name = stream["codec_name"].as_str();

            match codec_type {
                "video" if video_codec.is_none() => {
                    video_codec = codec_name.map(String::from);
                    width = stream["width"].as_u64().map(|v| v as u32);
                    height = stream["height"].as_u64().map(|v| v as u32);
                }
                "audio" if audio_codec.is_none() => {
                    audio_codec = codec_name.map(String::from);
                }
                _ => {}
            }
        }
    }

    // Determine if native using existing detector
    let is_native =
        detector::is_native_format(registry, path) && is_codec_native(&video_codec, &audio_codec);

    Ok(VideoInfo {
        duration_secs,
        is_native,
        video_codec,
        audio_codec,
        container,
        width,
        height,
    })
}

async fn start_probe_process(
    cmd: &mut Command,
    process_manager: &Arc<RwLock<ProcessManager>>,
    shutdown_token: &CancellationToken,
) -> Result<tokio::task::JoinHandle<Result<Output, ProbeProcessError>>, ProbeProcessError> {
    let mut manager = process_manager.write().await;
    if shutdown_token.is_cancelled() {
        return Err(ProbeProcessError::Cancelled(CancellationReason::Shutdown));
    }
    let key = format!("probe:{}", uuid::Uuid::new_v4());
    let (id, cancellation) = manager
        .register(key)
        .ok_or(ProbeProcessError::Cancelled(CancellationReason::Shutdown))?;
    cmd.kill_on_drop(true);
    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(error) => {
            manager.remove(id);
            return Err(ProbeProcessError::Supervision(error));
        }
    };
    tracing::debug!(pid = ?child.id(), "FFprobe process started");
    let process_manager = process_manager.clone();
    let owner = tokio::spawn(async move {
        let result = wait_for_probe(child, cancellation).await;
        let mut manager = process_manager.write().await;
        match &result {
            Err(ProbeProcessError::Supervision(error)) => {
                error!(%error, "FFprobe process supervision failed");
                manager.record_owner_failure();
            }
            Ok(output) if !output.status.success() => error!(
                status = %output.status,
                stderr = %String::from_utf8_lossy(&output.stderr),
                "FFprobe exited unsuccessfully"
            ),
            _ => {}
        }
        manager.remove(id);
        result
    });
    Ok(owner)
}

async fn wait_for_probe(
    mut child: tokio::process::Child,
    mut cancellation: watch::Receiver<Option<CancellationReason>>,
) -> Result<Output, ProbeProcessError> {
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("FFprobe stdout is unavailable"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| std::io::Error::other("FFprobe stderr is unavailable"))?;
    let read_stdout = async {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).await.map(|_| bytes)
    };
    let read_stderr = async {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).await.map(|_| bytes)
    };
    let wait = async {
        tokio::select! {
            biased;
            result = child.wait() => Ok::<_, std::io::Error>((result?, None)),
            changed = cancellation.changed() => {
                changed.map_err(std::io::Error::other)?;
                let reason = (*cancellation.borrow_and_update())
                    .ok_or_else(|| std::io::Error::other("Missing FFprobe cancellation reason"))?;
                if let Some(status) = child.try_wait()? {
                    return Ok((status, None));
                }
                let termination = child.start_kill();
                let status = child.wait().await?;
                if let Err(error) = termination {
                    tracing::warn!(?reason, %error, %status, "FFprobe termination request failed; process exit was awaited");
                    return Ok((status, None));
                }
                tracing::debug!(?reason, %status, "FFprobe termination requested and exit awaited");
                Ok((status, Some(reason)))
            }
        }
    };
    let (status, stdout, stderr) = tokio::join!(wait, read_stdout, read_stderr);
    let (status, reason) = status?;
    let stdout = stdout?;
    let stderr = stderr?;
    if let Some(reason) = reason.filter(|_| super::segment::termination_confirmed(&status)) {
        return Err(ProbeProcessError::Cancelled(reason));
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// Check if video/audio codecs are natively supported in WebView
fn is_codec_native(video_codec: &Option<String>, audio_codec: &Option<String>) -> bool {
    // Native video codecs
    let native_video = match video_codec {
        Some(codec) => matches!(
            codec.as_str(),
            "h264" | "avc1" | "avc" | // H.264
            "vp8" // VP8
        ),
        None => true, // No video stream is OK
    };

    // Native audio codecs
    let native_audio = match audio_codec {
        Some(codec) => matches!(
            codec.as_str(),
            "aac" | "mp3" | "mp2" |    // Common web codecs
            "flac" |                    // FLAC
            "pcm_s16le" | "pcm_s24le" | "pcm_f32le" | // PCM variants (WAV)
            "opus" | "vorbis" // Modern open codecs
        ),
        None => true, // No audio stream is OK
    };

    native_video && native_audio
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    #[test]
    fn probe_process_child() {
        match std::env::var("MUNDAM_PROBE_TEST_CHILD").as_deref() {
            Ok("wait") => std::thread::sleep(Duration::from_secs(30)),
            Ok("success") => println!(r#"{{"format":{{"duration":"2.5"}}}}"#),
            Ok("failure") => {
                eprintln!("probe test failure");
                std::process::exit(7);
            }
            _ => {}
        }
    }

    fn child_command(mode: &str) -> std::io::Result<Command> {
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args([
                "--exact",
                "delivery::streaming::probe::tests::probe_process_child",
                "--nocapture",
            ])
            .env("MUNDAM_PROBE_TEST_CHILD", mode)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        Ok(command)
    }

    #[tokio::test]
    async fn probe_preserves_process_output_and_failure_status() -> TestResult {
        let manager = Arc::new(RwLock::new(ProcessManager::new()));
        let token = CancellationToken::new();
        let owner = start_probe_process(&mut child_command("success")?, &manager, &token).await?;
        let output = tokio::time::timeout(Duration::from_secs(5), owner).await???;
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains(r#""duration":"2.5""#));

        let owner = start_probe_process(&mut child_command("failure")?, &manager, &token).await?;
        let output = tokio::time::timeout(Duration::from_secs(5), owner).await???;
        assert_eq!(output.status.code(), Some(7));
        assert!(String::from_utf8_lossy(&output.stderr).contains("probe test failure"));
        let manager = manager.read().await;
        assert_eq!(manager.active_count(), 0);
        assert_eq!(manager.owner_failures(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn shutdown_reaps_probe_even_when_request_is_dropped() -> TestResult {
        let manager = Arc::new(RwLock::new(ProcessManager::new()));
        let token = CancellationToken::new();
        let owner = start_probe_process(&mut child_command("wait")?, &manager, &token).await?;
        drop(owner);
        assert_eq!(manager.read().await.active_count(), 1);
        token.cancel();
        assert_eq!(manager.write().await.cancel_all(), 1);
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if manager.read().await.active_count() == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await?;
        assert_eq!(manager.read().await.owner_failures(), 0);
        assert!(matches!(
            start_probe_process(&mut child_command("wait")?, &manager, &token).await,
            Err(ProbeProcessError::Cancelled(CancellationReason::Shutdown))
        ));
        Ok(())
    }

    #[tokio::test]
    async fn stale_probe_is_stopped_and_awaited() -> TestResult {
        let manager = Arc::new(RwLock::new(ProcessManager::new()));
        let token = CancellationToken::new();
        let owner = start_probe_process(&mut child_command("wait")?, &manager, &token).await?;
        tokio::time::sleep(Duration::from_millis(1)).await;
        manager.write().await.cleanup_stale(0);
        let result = tokio::time::timeout(Duration::from_secs(5), owner).await??;
        #[cfg(unix)]
        assert!(matches!(
            result,
            Err(ProbeProcessError::Cancelled(CancellationReason::Stale))
        ));
        #[cfg(not(unix))]
        assert!(!result?.status.success());
        assert_eq!(manager.read().await.active_count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn admission_racing_shutdown_never_leaves_a_process_untracked() -> TestResult {
        let manager = Arc::new(RwLock::new(ProcessManager::new()));
        let token = CancellationToken::new();
        let mut command = child_command("wait")?;
        let start = start_probe_process(&mut command, &manager, &token);
        let shutdown = async {
            token.cancel();
            manager.write().await.cancel_all();
        };
        let (result, ()) = tokio::join!(start, shutdown);
        match result {
            Ok(owner) => {
                let result = tokio::time::timeout(Duration::from_secs(5), owner).await??;
                #[cfg(unix)]
                assert!(matches!(
                    result,
                    Err(ProbeProcessError::Cancelled(CancellationReason::Shutdown))
                ));
                #[cfg(not(unix))]
                assert!(!result?.status.success());
            }
            Err(error) => assert!(matches!(
                error,
                ProbeProcessError::Cancelled(CancellationReason::Shutdown)
            )),
        }
        assert_eq!(manager.read().await.active_count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn admission_rejects_cancelled_token_and_closed_registry() -> TestResult {
        let manager = Arc::new(RwLock::new(ProcessManager::new()));
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        let error =
            match start_probe_process(&mut child_command("wait")?, &manager, &cancelled).await {
                Ok(_) => return Err("cancelled token admitted a probe".into()),
                Err(error) => error,
            };
        assert!(is_shutdown_cancellation(&error));

        manager.write().await.cancel_all();
        let error = match start_probe_process(
            &mut child_command("wait")?,
            &manager,
            &CancellationToken::new(),
        )
        .await
        {
            Ok(_) => return Err("closed registry admitted a probe".into()),
            Err(error) => error,
        };
        assert!(is_shutdown_cancellation(&error));
        assert_eq!(manager.read().await.active_count(), 0);
        Ok(())
    }
}
