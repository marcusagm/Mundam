use serde::Serialize;
use strum_macros::{Display, EnumIter};

#[derive(Debug, Clone, Serialize, EnumIter, Display, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Project, // ex: .psd, .ai
    Vector,  // ex: .svg, .pdf
    Archive, // ex: .zip
    Model3D, // ex: .blend, .fbx
    Font,
    Document, // ex: .txt, .md, .doc
    Unknown,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ThumbnailStrategy {
    NativeImage,     // Rust native decoders (image-rs, zune-jpeg)
    Ffmpeg,          // Video and complex formats
    Webview,         // SVG, HTML
    ZipPreview,      // Affinity, OpenOffice etc
    NativeExtractor, // For formats where we extract a preview (Affinity, RAW, PSD)
    Raw,             // FlashRaw based extraction for difficult formats
    Model3D,         // Uses Assimp to convert to GLB
    Font,            // Resvg with custom font loading
    Icon,            // Fallback for files without preview
    None,
}

impl ThumbnailStrategy {
    /// Determines whether the given strategy should be processed in an isolated Heavy queue.
    pub fn is_heavy(&self) -> bool {
        matches!(
            self,
            ThumbnailStrategy::Ffmpeg
                | ThumbnailStrategy::Raw
                | ThumbnailStrategy::NativeExtractor
                | ThumbnailStrategy::Model3D
        )
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStrategy {
    Native,         // Direct browser support (mp4, mp3)
    Hls,            // Standard HLS for most formats (webm, mkv, avi, etc.)
    LinearHls,      // Live/Linear HLS for specific formats (swf, mpg, mpeg)
    AudioHls,       // Standard HLS for audio (opus, ogg, etc.)
    AudioLinearHls, // Linear HLS for audio
    Transcode,      // Legacy transcoding (kept for compatibility if needed, but HLS preferred)
    AudioTranscode, // Legacy audio transcoding
    Conversion,     // Conversion strategy (e.g. 3D formats to GLB)
    None,           // No playback support
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PreviewStrategy {
    BrowserNative,   // Directly renderable by browser (JPEG, PNG, WebP, SVG, etc.)
    Raw,             // Extraction via FlashRaw/FlashRaw
    Ffmpeg,          // Extraction via FFmpeg (HEIC, HDR, AVIF)
    NativeExtractor, // Specialty extraction (PSD, Affinity, ZIP-based)
    Convert,         // On-the-fly conversion using 'image' crate (DDS, TGA, EXR)
    Assimp,          // 3D Model conversion/rendering via Assimp
    None,            // No preview available
}
