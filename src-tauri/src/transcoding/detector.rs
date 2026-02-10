use std::path::Path;
use crate::formats::{PlaybackStrategy, MediaType as FormatMediaType};

/// Media type for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Audio,
    Video,
    Unknown,
}

/// Check if a file path requires transcoding for playback (i.e. not native)
pub fn needs_transcoding(path: &Path) -> bool {
    !is_native_format(path)
}

/// Check if a file is natively supported (no transcoding needed)
pub fn is_native_format(path: &Path) -> bool {
    if let Some(format) = crate::formats::FileFormat::detect(path) {
        matches!(format.playback, PlaybackStrategy::Native)
    } else {
        false
    }
}

/// Determine the media type from file extension
pub fn get_media_type(path: &Path) -> MediaType {
    if let Some(format) = crate::formats::FileFormat::detect(path) {
        match format.type_category {
            FormatMediaType::Audio => MediaType::Audio,
            FormatMediaType::Video => MediaType::Video,
            _ => MediaType::Unknown,
        }
    } else {
        MediaType::Unknown
    }
}

/// Check if a file extension needs audio-only transcoding
#[allow(dead_code)]
pub fn is_audio_transcode(path: &Path) -> bool {
    if let Some(format) = crate::formats::FileFormat::detect(path) {
        matches!(format.type_category, FormatMediaType::Audio) &&
        !matches!(format.playback, PlaybackStrategy::Native)
    } else {
        false
    }
}

/// Check if a file extension needs video transcoding
#[allow(dead_code)]
pub fn is_video_transcode(path: &Path) -> bool {
    if let Some(format) = crate::formats::FileFormat::detect(path) {
        matches!(format.type_category, FormatMediaType::Video) &&
        !matches!(format.playback, PlaybackStrategy::Native)
    } else {
        false
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
