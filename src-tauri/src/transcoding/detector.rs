use std::path::Path;

/// Audio extensions that require transcoding (not natively supported by WebView)
const TRANSCODE_AUDIO: &[&str] = &[
    "ogg", "oga", "opus",  // Ogg container
    "wma",                  // Windows Media
    "ac3",                  // Dolby Digital
    "spx",                  // Speex
    "ra", "rm",             // RealAudio
    "mka",                  // Matroska Audio
];

/// Video extensions that require transcoding
const TRANSCODE_VIDEO: &[&str] = &[
    // Desktop containers
    "mkv",                  // Matroska
    "avi",                  // AVI
    "flv", "f4v",           // Flash Video
    "wmv", "asf",           // Windows Media
    "ogv",                  // Ogg Video
    "webm",                 // WebM (VP9 not supported on macOS WebView)
    // Broadcast/Professional
    "mpeg", "mpg", "m2v",   // MPEG-1/2
    "vob",                  // DVD Video
    "m2ts", "mts", "ts",    // MPEG Transport Stream
    "mxf",                  // Material Exchange Format
    "wtv",                  // Windows TV
    // Legacy/Mobile
    "3gp", "3g2",           // 3GPP
    "rm", "rmvb",           // RealMedia
    "swf",                  // Flash (limited support)
    "divx",                 // DivX
    "hevc",                 // Raw HEVC (container-less)
    "mjpeg",                // Motion JPEG
];

/// Native audio extensions (no transcoding needed)
const NATIVE_AUDIO: &[&str] = &[
    "mp3",                  // MPEG Layer 3
    "wav",                  // Waveform
    "aac",                  // Advanced Audio Coding
    "m4a", "m4r",           // MPEG-4 Audio
    "flac",                 // Free Lossless
    "mp2",                  // MPEG Layer 2
    "aiff", "aif",          // Audio Interchange
];

/// Native video extensions (no transcoding needed)
const NATIVE_VIDEO: &[&str] = &[
    "mp4", "m4v",           // MPEG-4
    "mov", "qt",            // QuickTime
];

/// Media type for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Audio,
    Video,
    Unknown,
}

/// Check if a file path requires transcoding for playback
pub fn needs_transcoding(path: &Path) -> bool {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    
    match ext {
        Some(e) => {
            TRANSCODE_AUDIO.contains(&e.as_str()) || TRANSCODE_VIDEO.contains(&e.as_str())
        }
        None => false,
    }
}

/// Check if a file is natively supported (no transcoding needed)
pub fn is_native_format(path: &Path) -> bool {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    
    match ext {
        Some(e) => {
            NATIVE_AUDIO.contains(&e.as_str()) || NATIVE_VIDEO.contains(&e.as_str())
        }
        None => false,
    }
}

/// Determine the media type from file extension
pub fn get_media_type(path: &Path) -> MediaType {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    
    match ext {
        Some(e) => {
            if TRANSCODE_AUDIO.contains(&e.as_str()) || NATIVE_AUDIO.contains(&e.as_str()) {
                MediaType::Audio
            } else if TRANSCODE_VIDEO.contains(&e.as_str()) || NATIVE_VIDEO.contains(&e.as_str()) {
                MediaType::Video
            } else {
                MediaType::Unknown
            }
        }
        None => MediaType::Unknown,
    }
}

/// Check if a file extension needs audio-only transcoding
pub fn is_audio_transcode(path: &Path) -> bool {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    
    match ext {
        Some(e) => TRANSCODE_AUDIO.contains(&e.as_str()),
        None => false,
    }
}

/// Check if a file extension needs video transcoding
pub fn is_video_transcode(path: &Path) -> bool {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    
    match ext {
        Some(e) => TRANSCODE_VIDEO.contains(&e.as_str()),
        None => false,
    }
}

/// Get the output extension for transcoded file
pub fn get_output_extension(path: &Path) -> &'static str {
    match get_media_type(path) {
        MediaType::Audio => "m4a",
        MediaType::Video => "mp4",
        MediaType::Unknown => "mp4", // Default to video container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_transcoding_audio() {
        assert!(needs_transcoding(Path::new("test.ogg")));
        assert!(needs_transcoding(Path::new("test.opus")));
        assert!(needs_transcoding(Path::new("test.wma")));
        assert!(!needs_transcoding(Path::new("test.mp3")));
        assert!(!needs_transcoding(Path::new("test.wav")));
    }

    #[test]
    fn test_needs_transcoding_video() {
        assert!(needs_transcoding(Path::new("test.mkv")));
        assert!(needs_transcoding(Path::new("test.avi")));
        assert!(needs_transcoding(Path::new("test.m2ts")));
        assert!(!needs_transcoding(Path::new("test.mp4")));
        assert!(!needs_transcoding(Path::new("test.mov")));
    }

    #[test]
    fn test_media_type() {
        assert_eq!(get_media_type(Path::new("test.ogg")), MediaType::Audio);
        assert_eq!(get_media_type(Path::new("test.mkv")), MediaType::Video);
        assert_eq!(get_media_type(Path::new("test.txt")), MediaType::Unknown);
    }
}
