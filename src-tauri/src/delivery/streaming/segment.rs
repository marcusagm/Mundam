//! Segment Transcoder
//!
//! Transcodes video segments on-demand using FFmpeg.
//! Segments are cached to disk for subsequent requests.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::{watch, RwLock};
use tracing::info;

use super::process_manager::{CancellationReason, ProcessManager};
use crate::feature::transcoding::cache::TranscodeCache;
use crate::feature::transcoding::detector;
use crate::processing::transcoding::resolve_transcoding_tools;

/// A process was stopped by its owner after an explicit cancellation request.
#[derive(Debug, thiserror::Error)]
#[error("Segment process cancelled: {0:?}")]
pub struct SegmentCancelled(pub CancellationReason);

/// No new transcoding process can be started after shutdown closes admission.
#[derive(Debug, thiserror::Error)]
#[error("Segment process admission is closed during shutdown")]
pub struct SegmentAdmissionClosed;

/// Identifies one segment consistently across cache lookup and transcoding.
#[derive(Clone, Copy)]
pub struct SegmentRequest<'request> {
    /// Validated physical source path.
    pub file_path: &'request Path,
    /// Zero-based segment number.
    pub segment_index: u32,
    /// Duration in seconds used to compute the seek offset.
    pub segment_duration: f64,
    /// Existing encoder quality preset.
    pub quality: &'request str,
}

/// Gets a cached segment or generates it using the same request parameters.
///
/// # Errors
/// Returns unavailable tools, invalid sources, failed transcoding or cache read errors.
pub async fn get_segment(
    app_handle: &tauri::AppHandle,
    registry: &Arc<crate::core::formats::registry::FormatRegistry>,
    cache: &Arc<TranscodeCache>,
    process_manager: &Arc<RwLock<ProcessManager>>,
    request: SegmentRequest<'_>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let SegmentRequest {
        file_path,
        segment_index,
        quality,
        ..
    } = request;
    // Check if segment is already cached
    let cache_path = get_segment_cache_path(cache, file_path, segment_index, quality);

    if cache_path.exists() {
        // Serve from cache
        let data = tokio::fs::read(&cache_path).await?;
        tracing::debug!(
            segment_index,
            bytes = data.len(),
            "Segment served from cache"
        );
        return Ok(data);
    }

    // Generate segment key for process management
    let segment_key = format!("{}:{}", file_path.display(), segment_index);

    // Transcode the segment
    let started = std::time::Instant::now();
    let data =
        transcode_segment(app_handle, registry, process_manager, &segment_key, request).await?;
    tracing::debug!(
        segment_index,
        bytes = data.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "Segment transcoded successfully"
    );

    // Cache the segment to disk
    if let Some(parent) = cache_path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::write(&cache_path, &data).await.ok();

    Ok(data)
}

/// Transcode a single segment using FFmpeg
async fn transcode_segment(
    app_handle: &tauri::AppHandle,
    registry: &Arc<crate::core::formats::registry::FormatRegistry>,
    process_manager: &Arc<RwLock<ProcessManager>>,
    segment_key: &str,
    request: SegmentRequest<'_>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let SegmentRequest {
        file_path,
        segment_index,
        segment_duration,
        quality,
    } = request;
    let tools = resolve_transcoding_tools(Some(app_handle))?;
    let ffmpeg_path = tools.ffmpeg;

    let start_time = segment_index as f64 * segment_duration;

    // Detect media kind to adjust FFmpeg flags
    let media_kind = detector::get_media_kind(registry, file_path);
    let is_audio = media_kind == detector::MediaKind::Audio;
    let h264_encoder = if is_audio {
        None
    } else {
        Some(crate::processing::transcoding::resolve_h264_encoder(
            &ffmpeg_path,
        )?)
    };

    let mut cmd = Command::new(&ffmpeg_path);
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "warning",
        "-analyzeduration",
        "100M",
        "-probesize",
        "50M",
        "-ignore_unknown",
        "-fflags",
        "+genpts",
        "-ss",
        &format!("{:.3}", start_time),
        "-i",
        &file_path.to_string_lossy(),
        "-t",
        &format!("{:.3}", segment_duration),
    ]);

    if is_audio {
        // Audio-only configuration
        cmd.args([
            "-map", "0:a:0?", "-vn", "-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-ac", "2",
        ]);
    } else {
        let h264_encoder = h264_encoder.ok_or("H.264 encoder was not resolved for video")?;
        // Video configuration
        cmd.args([
            "-map",
            "0:v:0",
            "-map",
            "0:a:0?",
            "-sn",
            "-c:v",
            h264_encoder,
        ]);

        if h264_encoder == "libx264" {
            cmd.args(["-preset", "ultrafast"]);
            match quality {
                "preview" => cmd.args(["-crf", "30"]),
                "high" => cmd.args(["-crf", "18"]),
                _ => cmd.args(["-crf", "23"]),
            };
        } else {
            let video_bitrate = match quality {
                "preview" => "500k",
                "high" => "4000k",
                _ => "1800k",
            };
            cmd.args(["-b:v", video_bitrate]);
        }
        let scale = if quality == "preview" {
            "scale=-2:480"
        } else {
            "scale=trunc(iw/2)*2:trunc(ih/2)*2"
        };
        cmd.args(["-vf", scale]);

        if h264_encoder == "libx264" {
            cmd.args(["-profile:v", "high", "-level", "4.1"]);
        }
        cmd.args([
            "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "128k", "-ar", "48000",
        ]);
    }

    cmd.args(["-f", "mpegts", "-"]);

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(windows)]
    cmd.creation_flags(0x08000000);

    let owner = start_segment_process(&mut cmd, process_manager, segment_key).await?;
    let output = owner.await??;
    let status = output.status;
    let output_data = output.stdout;

    if !status.success() {
        let err_output = String::from_utf8_lossy(&output.stderr);
        info!("FFmpeg failed (segment {}): {}", segment_index, err_output);
        return Err(format!("FFmpeg failed (segment {}): {}", segment_index, err_output).into());
    }

    if output_data.is_empty() {
        return Err(format!("FFmpeg produced empty output for segment {}", segment_index).into());
    }

    Ok(output_data)
}

type SegmentOutput = Result<std::process::Output, Box<dyn std::error::Error + Send + Sync>>;

async fn start_segment_process(
    cmd: &mut Command,
    process_manager: &Arc<RwLock<ProcessManager>>,
    segment_key: &str,
) -> Result<tokio::task::JoinHandle<SegmentOutput>, Box<dyn std::error::Error + Send + Sync>> {
    cmd.kill_on_drop(true);
    let mut pm = process_manager.write().await;
    let (id, cancellation) = pm
        .register(segment_key.to_string())
        .ok_or(SegmentAdmissionClosed)?;
    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(error) => {
            pm.remove(id);
            return Err(error.into());
        }
    };
    let manager = process_manager.clone();
    let owner = tokio::spawn(async move {
        let output = wait_for_segment(child, cancellation).await;
        let owner_failed =
            matches!(&output, Err(error) if error.downcast_ref::<SegmentCancelled>().is_none());
        match &output {
            Err(error) if error.downcast_ref::<SegmentCancelled>().is_none() => {
                tracing::error!(%error, "Segment process owner failed");
            }
            Ok(output) if !output.status.success() => tracing::error!(
                status = %output.status,
                stderr = %String::from_utf8_lossy(&output.stderr),
                "Segment process exited unsuccessfully"
            ),
            _ => {}
        }
        let mut manager = manager.write().await;
        if owner_failed {
            manager.record_owner_failure();
        }
        manager.remove(id);
        output
    });
    drop(pm);
    Ok(owner)
}

async fn wait_for_segment(
    mut child: tokio::process::Child,
    mut cancellation: watch::Receiver<Option<CancellationReason>>,
) -> Result<std::process::Output, Box<dyn std::error::Error + Send + Sync>> {
    let mut stdout = child.stdout.take().ok_or("Segment stdout unavailable")?;
    let mut stderr = child.stderr.take().ok_or("Segment stderr unavailable")?;
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
            result = child.wait() => Ok((result?, None)),
            changed = cancellation.changed() => {
                changed.map_err(|error| std::io::Error::other(error.to_string()))?;
                let reason = *cancellation.borrow_and_update();
                let reason = reason.ok_or_else(|| std::io::Error::other("Missing segment cancellation reason"))?;
                if let Some(status) = child.try_wait()? {
                    return Ok((status, None));
                }
                let termination = child.start_kill();
                let status = child.wait().await?;
                if let Err(error) = termination {
                    tracing::warn!(?reason, %error, %status, "Segment termination request failed; process exit was awaited");
                    return Ok((status, None));
                }
                info!(?reason, %status, "Segment process termination awaited");
                Ok::<_, std::io::Error>((status, Some(reason)))
            }
        }
    };
    let (status, stdout, stderr) = tokio::join!(wait, read_stdout, read_stderr);
    let (status, reason) = status?;
    let stdout = stdout?;
    let stderr = stderr?;
    if let Some(reason) = reason.filter(|_| termination_confirmed(&status)) {
        return Err(Box::new(SegmentCancelled(reason)));
    }
    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

#[cfg(unix)]
pub(super) fn termination_confirmed(status: &std::process::ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt;
    status.signal() == Some(libc::SIGKILL)
}

#[cfg(not(unix))]
pub(super) fn termination_confirmed(_status: &std::process::ExitStatus) -> bool {
    false
}

/// Get the cache path for a segment
fn get_segment_cache_path(
    cache: &TranscodeCache,
    file_path: &Path,
    segment_index: u32,
    quality: &str,
) -> PathBuf {
    let cache_dir = cache.dir().join("hls_segments");

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    file_path.to_string_lossy().hash(&mut hasher);

    if let Ok(metadata) = std::fs::metadata(file_path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::SystemTime::UNIX_EPOCH) {
                duration.as_secs().hash(&mut hasher);
            }
        }
    }

    let file_hash = format!("{:016x}", hasher.finish());

    cache_dir.join(format!(
        "{}-{}-seg{:05}.ts",
        file_hash, quality, segment_index
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn child(script: &str) -> std::io::Result<tokio::process::Child> {
        Command::new("sh")
            .args(["-c", script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
    }

    #[tokio::test]
    async fn cancellation_reaps_child_and_is_typed(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let process = child("exec sleep 30")?;
        let (sender, receiver) = watch::channel(None);
        sender.send_replace(Some(CancellationReason::Shutdown));
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            wait_for_segment(process, receiver),
        )
        .await?;
        assert!(
            matches!(result, Err(error) if error.downcast_ref::<SegmentCancelled>().is_some_and(|cancelled| cancelled.0 == CancellationReason::Shutdown))
        );
        Ok(())
    }

    #[tokio::test]
    async fn genuine_process_failure_is_not_cancelled(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let process = child("printf failure >&2; exit 7")?;
        let (_sender, receiver) = watch::channel(None);
        let output = wait_for_segment(process, receiver).await?;
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stderr, b"failure");
        Ok(())
    }

    #[tokio::test]
    async fn dropped_request_remains_tracked_until_shutdown_reaps_process(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let manager = Arc::new(RwLock::new(ProcessManager::new()));
        let mut command = Command::new("sh");
        command.args(["-c", "exec sleep 30"]);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let owner = start_segment_process(&mut command, &manager, "segment").await?;
        drop(owner);
        assert_eq!(manager.write().await.cancel_all(), 1);
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if manager.read().await.active_count() == 0 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await?;
        let mut missing_command = Command::new("/nonexistent/mundam-segment-test");
        let error = match start_segment_process(&mut missing_command, &manager, "late").await {
            Ok(_) => return Err("late segment process was admitted".into()),
            Err(error) => error,
        };
        assert!(error.downcast_ref::<SegmentAdmissionClosed>().is_some());
        Ok(())
    }

    #[tokio::test]
    async fn owner_error_is_retained_after_process_is_unregistered(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let manager = Arc::new(RwLock::new(ProcessManager::new()));
        let mut command = Command::new("sh");
        command.args(["-c", "exec sleep 30"]);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let owner = start_segment_process(&mut command, &manager, "segment").await?;
        assert!(owner.await?.is_err());
        let manager = manager.read().await;
        assert_eq!(manager.active_count(), 0);
        assert_eq!(manager.owner_failures(), 1);
        Ok(())
    }
}
