//! Specialized extractor for Sketch files (.sketch).
//!
//! Sketch files are ZIP archives containing JSON metadata and assets.
//! Previews are usually stored in `previews/preview.png`.

use std::path::Path;
use std::io::Read;
// No custom error imports needed as we use Boxed error

/// Extracts the preview image from a Sketch file.
///
/// # Arguments
/// * `sketch_file_path` - Path to the .sketch file.
///
/// # Errors
/// Returns `Err` if the file is not a valid ZIP or if no preview is found.
///
/// # Examples
/// ```rust
/// let (data, mime) = extract_sketch_preview(path)?;
/// ```
pub fn extract_sketch_preview(sketch_file_path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let sketch_file = std::fs::File::open(sketch_file_path)?;
    let mut zip_archive = zip::ZipArchive::new(sketch_file)?;

    // Sketch standard: previews/preview.png
    // We check both lowercase and uppercase variants just in case
    let candidate_internal_paths = [
        "previews/preview.png",
        "Previews/preview.png",
    ];

    for internal_path in candidate_internal_paths {
        if let Ok(mut zip_entry) = zip_archive.by_name(internal_path) {
            let mut image_data_buffer = Vec::new();
            zip_entry.read_to_end(&mut image_data_buffer)?;

            // Log success if needed, but for now we follow "clean code" - no unnecessary logs
            return Ok((image_data_buffer, "image/png".to_string()));
        }
    }

    // Fallback: search for any PNG that looks like a preview
    for entry_index in 0..zip_archive.len() {
        let mut zip_entry = zip_archive.by_index(entry_index)?;

        let entry_name = zip_entry.name().to_lowercase();
        if entry_name.ends_with("preview.png") {
            let mut image_data_buffer = Vec::new();
            zip_entry.read_to_end(&mut image_data_buffer)?;
            return Ok((image_data_buffer, "image/png".to_string()));
        }
    }

    Err(format!("No preview found in Sketch file: {}", sketch_file_path.display()).into())
}
