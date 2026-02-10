use serde::Serialize;
use std::path::Path;
use std::fs::File;
use std::io::Read;

pub mod types;
pub mod definitions;

pub use types::*;
pub use definitions::SUPPORTED_FORMATS;

#[derive(Debug, Clone, Serialize)]
pub struct FileFormat {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub mime_types: &'static [&'static str],
    pub type_category: MediaType,
    #[serde(skip)]
    pub strategy: ThumbnailStrategy,
    pub playback: PlaybackStrategy,
}

impl FileFormat {
    /// Detects the real format of the file.
    /// Priority: 1. Magic Bytes (infer) -> 2. Extension (mime_guess/fallback) -> 3. None
    pub fn detect(path: &Path) -> Option<&'static FileFormat> {
        // Optimization: Open file once if possible.
        // For simple usage, we just wrap detect_header.
        if let Ok(mut file) = File::open(path) {
            return Self::detect_header(&mut file, path);
        }

        // Fallback if file open fails (locked?) - rely on extension only
        Self::detect_extension(path)
    }

    /// Detects format from an open file handle (reads header and rewinds).
    /// Used to avoid re-opening files in high-performance loops.
    pub fn detect_header(file: &mut File, path_fallback: &Path) -> Option<&'static FileFormat> {
        // 1. Try reading first bytes (Header)
        // 1024 bytes is enough for almost all magic bytes (infer usually needs < 300)
        let mut buffer = [0u8; 1024];

        // Read header
        if file.read(&mut buffer).is_ok() {
            // Rewind file for subsequent use!
            let _ = std::io::Seek::seek(file, std::io::SeekFrom::Start(0));

            if let Some(kind) = infer::get(&buffer) {
                // Check registry for the MIME returned by infer
                if let Some(fmt) = SUPPORTED_FORMATS.iter().find(|f| f.mime_types.contains(&kind.mime_type())) {
                    return Some(fmt);
                }
            }
        }

        // 2. Fallback: Extension
        Self::detect_extension(path_fallback)
    }

    fn detect_extension(path: &Path) -> Option<&'static FileFormat> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            return SUPPORTED_FORMATS.iter().find(|f| f.extensions.contains(&ext_lower.as_str()));
        }
        None
    }

    /// Checks if the file extension is supported by the library.
    /// This is a fast check for indexer traversing.
    pub fn is_supported_extension(path: &Path) -> bool {
        Self::detect_extension(path).is_some()
    }
}
