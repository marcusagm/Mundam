//! Extraction utilities for 3D model formats.
//!
//! Provides shared extraction logic for 3D model files including Blender
//! thumbnail extraction and Assimp-based format conversion.

use crate::core::error::{AppError, AppResult};
use tracing::error;

/// Extracts an embedded JPEG thumbnail from a Blender `.blend` file.
///
/// Blender files embed a JPEG thumbnail in the REND block. This function
/// scans the raw binary data for JPEG start (`FF D8 FF`) and end (`FF D9`)
/// markers to extract it.
///
/// # Arguments
///
/// * `path` - The path to the Blender file on disk.
///
/// # Returns
///
/// The raw JPEG bytes of the embedded thumbnail.
///
/// # Errors
///
/// * `AppError::Io` - If the file cannot be read.
/// * `AppError::FormatNotSupported` - If no JPEG thumbnail is found.
pub fn extract_blender_thumbnail(path: &std::path::Path) -> AppResult<Vec<u8>> {
    let data = std::fs::read(path).map_err(AppError::Io)?;
    if let Some(jpeg_start_position) = data.windows(3).position(|window| window == b"\xFF\xD8\xFF")
    {
        if let Some(jpeg_end_offset) = data[jpeg_start_position..]
            .windows(2)
            .position(|window| window == b"\xFF\xD9")
        {
            return Ok(
                data[jpeg_start_position..jpeg_start_position + jpeg_end_offset + 2].to_vec(),
            );
        }
    }
    Err(AppError::FormatNotSupported(
        "No thumbnail in Blender file".into(),
    ))
}

/// Converts a 3D model file to GLB format using Assimp.
///
/// Spawns an Assimp subprocess to export the source file as GLB v2. The
/// converted file is written to a temporary directory and then read back
/// into memory.
///
/// # Arguments
///
/// * `source_path` - The path to the source 3D model file.
/// * `asset_id` - The unique identifier for the asset (used for temp file naming).
///
/// # Returns
///
/// A tuple of (GLB bytes, MIME type).
///
/// # Errors
///
/// * `AppError::Transcoding` - If Assimp is not found or conversion fails.
/// * `AppError::Io` - If file I/O operations fail.
pub async fn convert_to_glb_with_assimp(
    source_path: &std::path::Path,
    asset_id: &str,
) -> AppResult<(Vec<u8>, String)> {
    let tools = crate::processing::transcoding::resolve_transcoding_tools::<tauri::Wry>(None)?;
    let assimp_binary = tools
        .assimp
        .ok_or_else(|| AppError::ToolUnavailable("assimp".into()))?;

    let temporary_directory = std::env::temp_dir().join(format!("mundam_3d_{}", asset_id));
    tokio::fs::create_dir_all(&temporary_directory)
        .await
        .map_err(AppError::Io)?;

    let output_glb_path = temporary_directory.join(format!("{}.glb", asset_id));

    let source_path_owned = source_path.to_path_buf();
    let output_glb_path_clone = output_glb_path.clone();
    let asset_id_owned = asset_id.to_string();

    let command_output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(assimp_binary)
            .arg("export")
            .arg(&source_path_owned)
            .arg(&output_glb_path_clone)
            .arg("-fglb2")
            .output()
            .map_err(AppError::Io)
    })
    .await
    .map_err(|_| AppError::ExtractionProcessTimeout)??;

    if !command_output.status.success() {
        let error_message = String::from_utf8_lossy(&command_output.stderr);
        error!(
            "Assimp conversion failed for {}: {}",
            asset_id_owned, error_message
        );
        return Err(AppError::Transcoding(format!(
            "Assimp failed: {}",
            error_message
        )));
    }

    let glb_data = tokio::fs::read(&output_glb_path)
        .await
        .map_err(AppError::Io)?;

    let _ = tokio::fs::remove_file(&output_glb_path).await;
    let _ = tokio::fs::remove_dir(&temporary_directory).await;

    Ok((glb_data, "model/gltf-binary".to_string()))
}
