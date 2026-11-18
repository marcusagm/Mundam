//! Native setup and ordered cleanup for sessions owned by the IPC worker.

use super::StreamInput;
use crate::core::error::{AppError, AppResult};
use crate::feature::transcoding::profiles::TranscodingProfile;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::{Child, Command};
use tokio_util::sync::CancellationToken;

pub(super) struct StreamSession {
    pub process: Option<Child>,
    pub last_accessed: Instant,
    pub output_dir: PathBuf,
}

impl StreamSession {
    pub async fn stop(&mut self) -> AppResult<()> {
        if let Some(child) = self.process.as_mut() {
            let exited = match child.try_wait() {
                Ok(status) => status.is_some(),
                Err(error) => {
                    tracing::warn!(%error, "IPC HLS child status failed; attempting termination and wait");
                    false
                }
            };
            if !exited {
                if let Err(error) = child.start_kill() {
                    tracing::warn!(%error, "IPC HLS kill request failed; awaiting actual exit");
                }
                child.wait().await?;
            }
            self.process = None;
        }
        // Remove files only after confirming that their writer has exited.
        match tokio::fs::remove_dir_all(&self.output_dir).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

pub(super) async fn start_child(
    input: StreamInput,
    output_dir: PathBuf,
    cancellation: CancellationToken,
) -> AppResult<Child> {
    // Synchronous tool discovery and encoder probing are awaited by this owner.
    // They are not interruptible; a lifecycle deadline remains a degraded R4 outcome.
    let (tools, encoder) = tokio::task::spawn_blocking(|| {
        let tools = crate::processing::transcoding::resolve_transcoding_tools::<tauri::Wry>(None)?;
        let encoder = crate::processing::transcoding::resolve_h264_encoder(&tools.ffmpeg)?;
        Ok::<_, AppError>((tools, encoder))
    })
    .await
    .map_err(|error| AppError::Internal(format!("IPC HLS setup owner failed: {error}")))??;
    if cancellation.is_cancelled() {
        return Err(AppError::Cancelled);
    }
    let profile = TranscodingProfile::resolve_for_hls(
        &input.original_path,
        input.mime_type.as_deref(),
        encoder,
    );
    let mut command = Command::new(tools.ffmpeg);
    command
        .args(&profile.input_args)
        .arg("-i")
        .arg(&input.original_path)
        .args(&profile.output_args)
        .arg("-hls_segment_filename")
        .arg(output_dir.join("segment_%05d.ts"))
        .arg(output_dir.join("playlist.m3u8"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    Ok(command.spawn()?)
}
