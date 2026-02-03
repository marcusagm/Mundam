use std::path::{Path, PathBuf};
use std::process::Command;
use crate::thumbnails::icon;
use tauri::Manager;

/// Entry point for 3D model thumbnail generation.
/// 
/// This pipeline follows the "Universal Pipeline" strategy:
/// 1. **Ingest & Convert**: Uses `assimp` CLI (bundled or system) to convert the proprietary model (FBX, OBJ, BLEND) 
///    into a standardized **Binary GLTF (.glb)**.
/// 2. **Cache**: The .glb is saved in the thumbnails directory alongside the image thumbnail.
/// 3. **Thumbnail**: Currently generates a generic file type icon for the grid view.
/// 
/// # Returns
/// The filename of the generated thumbnail (webp), NOT the GLB path.
pub fn generate_model_preview(
    input_path: &Path,
    thumbnails_dir: &Path,
    hashed_filename: &str, 
    size_px: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    
    // 1. Derive GLB cache path: "{hash}.glb"
    let stem = Path::new(hashed_filename)
        .file_stem()
        .ok_or("Invalid filename structure")?
        .to_str()
        .ok_or("Invalid filename encoding")?;
        
    let glb_filename = format!("{}.glb", stem);
    let glb_path = thumbnails_dir.join(&glb_filename);

    // 2. Convert to GLB if cache doesn't exist
    if !glb_path.exists() {
        // Find Assimp binary (Bundled -> System)
        let assimp_bin = get_assimp_path_best_effort();
        
        match convert_to_glb(&assimp_bin, input_path, &glb_path) {
            Ok(_) => {
                // Success
            },
            Err(e) => {
                eprintln!("Model3D Warning: Could not create 3D Preview (GLB) for {:?}. Reason: {}", input_path.file_name(), e);
            }
        }
    }

    // 3. Generate Visual Thumbnail (Screenshot of 3D)
    // Since we now have a .glb file, we can try to extract a thumbnail from it.
    // However, headless 3D rendering is complex (needs EGL/Vulkan). 
    //
    // PLAN B: Use the GLB file directly if possible? No, we need an image for the grid.
    //
    // Currently, Assimp CLI does NOT generate screenshots.
    // To solve this properly without a complex 3D engine, we will stick to the Icon fallback 
    // BUT we create a TODO for "3D Thumbnailer".
    //
    // For now, we will continue copying the icon so the frontend works.
    
    // Check if we can use ffmpeg to snapshot the GLB? No, ffmpeg doesn't render 3D.
    // Check if we can use a library? 'three-d' needs window context.
    
    // Current Best Effort: Fallback to Icon, but correctly linked.
    let icon_relative = icon::get_or_generate_icon(input_path, thumbnails_dir, size_px)?;
    
    let icon_source = thumbnails_dir.join(&icon_relative);
    let icon_dest = thumbnails_dir.join(hashed_filename);
    
    if icon_source.exists() {
        if let Err(e) = std::fs::copy(&icon_source, &icon_dest) {
             eprintln!("Model3D Error: Failed to copy icon {:?} to {:?}: {}", icon_source, icon_dest, e);
        } else {
             return Ok(hashed_filename.to_string());
        }
    }

    Ok(hashed_filename.to_string())
}

/// Helper to find Assimp path without AppHandle (Best Effort)
/// Replicates the logic from ffmpeg.rs but for assimp
fn get_assimp_path_best_effort() -> PathBuf {
    // 1. Try resolving relative to executable (dev mode/bundle logic)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(target_dir) = exe_path.parent() { // debug/
            if let Some(build_dir) = target_dir.parent() { // target/
                 if let Some(src_tauri) = build_dir.parent() { // src-tauri/
                    let root = src_tauri.join("assimp");
                    
                    #[cfg(target_os = "macos")]
                    {
                        let p = root.join("macos").join("assimp");
                        if p.exists() { return p; }
                    }

                    #[cfg(target_os = "windows")]
                    {
                        let p = root.join("windows-x64").join("assimp.exe");
                        if p.exists() { return p; }
                    }

                    #[cfg(target_os = "linux")]
                    {
                        let p = root.join("linux").join("assimp");
                        if p.exists() { return p; }
                    }

                    // Legacy/Root fallback
                    let p = root.join("assimp");
                    if p.exists() { return p; }
                 }
            }
        }
    }
    
    // 2. Fallback to system PATH
    PathBuf::from("assimp")
}

/// Wraps the `assimp export` CLI command.
fn convert_to_glb(binary: &Path, input: &Path, output: &Path) -> Result<(), String> {
    // Command: assimp export <input> <output>
    let output_str = output.to_str().ok_or("Invalid output path")?;
    
    let result = Command::new(binary)
        .arg("export")
        .arg(input)
        .arg(output_str)
        .output();
        
    match result {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("Exit Code {}: {}", output.status, stderr))
            }
        },
        Err(e) => {
            Err(format!("Command execution failed: {}", e))
        }
    }
}
