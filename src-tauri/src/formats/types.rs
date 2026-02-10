use serde::Serialize;
use strum_macros::{EnumIter, Display};

#[derive(Debug, Clone, Serialize, EnumIter, Display, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Project, // ex: .psd, .ai
    Archive, // ex: .zip
    Model3D,
    Font,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub enum ThumbnailStrategy {
    NativeImage, // Rust native decoders (image-rs, zune-jpeg)
    Ffmpeg,      // Video and complex formats
    Webview,     // SVG, HTML
    ZipPreview,  // Affinity, OpenOffice etc
    NativeExtractor, // For formats where we extract a preview (Affinity, RAW, PSD)
    Model3D,     // Uses Assimp to convert to GLB
    Font,        // Resvg with custom font loading
    Icon,        // Fallback for files without preview
    None,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStrategy {
    Native,          // Direct browser support (mp4, mp3)
    Hls,             // Standard HLS for most formats (webm, mkv, avi, etc.)
    LinearHls,       // Live/Linear HLS for specific formats (swf, mpg, mpeg)
    AudioHls,        // Standard HLS for audio (opus, ogg, etc.)
    AudioLinearHls,  // Linear HLS for audio
    Transcode,       // Legacy transcoding (kept for compatibility if needed, but HLS preferred)
    AudioTranscode,  // Legacy audio transcoding
    None,            // No playback support
}
