use serde::{Deserialize, Serialize};

/// Transcoding quality presets for video and audio
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TranscodeQuality {
    /// Fast transcoding, reasonable quality (CRF 28, 192kbps audio)
    Preview,
    /// Balanced quality and speed (CRF 23, 256kbps audio)
    #[default]
    Standard,
    /// High quality, slower transcoding (CRF 18, 320kbps audio)
    High,
}

impl TranscodeQuality {
    /// CRF value for quality-based encoding (lower = better quality)
    /// Using CRF instead of bitrate for better quality-to-size ratio
    pub fn crf(&self) -> u8 {
        match self {
            TranscodeQuality::Preview => 28,   // Fast, acceptable quality
            TranscodeQuality::Standard => 23,  // Good quality (x264 default)
            TranscodeQuality::High => 18,      // High quality
        }
    }

    /// Video bitrate in bits per second (fallback for streams)
    pub fn video_bitrate(&self) -> u32 {
        match self {
            TranscodeQuality::Preview => 4_000_000,   // 4 Mbps
            TranscodeQuality::Standard => 8_000_000,  // 8 Mbps
            TranscodeQuality::High => 15_000_000,     // 15 Mbps
        }
    }

    /// Audio bitrate in bits per second
    pub fn audio_bitrate(&self) -> u32 {
        match self {
            TranscodeQuality::Preview => 192_000,  // 192 kbps
            TranscodeQuality::Standard => 256_000, // 256 kbps
            TranscodeQuality::High => 320_000,     // 320 kbps
        }
    }

    /// FFmpeg preset for encoding speed/quality tradeoff
    pub fn ffmpeg_preset(&self) -> &'static str {
        match self {
            TranscodeQuality::Preview => "veryfast",
            TranscodeQuality::Standard => "medium",
            TranscodeQuality::High => "slow",
        }
    }

    /// Human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            TranscodeQuality::Preview => "Preview",
            TranscodeQuality::Standard => "Standard",
            TranscodeQuality::High => "High",
        }
    }

    /// Returns all available qualities
    pub fn all() -> &'static [TranscodeQuality] {
        &[
            TranscodeQuality::Preview,
            TranscodeQuality::Standard,
            TranscodeQuality::High,
        ]
    }

    /// Parse from string (for protocol URL parsing)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "preview" | "low" => Some(TranscodeQuality::Preview),
            "standard" | "medium" => Some(TranscodeQuality::Standard),
            "high" => Some(TranscodeQuality::High),
            _ => None,
        }
    }
}

impl std::fmt::Display for TranscodeQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_bitrates() {
        assert_eq!(TranscodeQuality::Preview.video_bitrate(), 4_000_000);
        assert_eq!(TranscodeQuality::Standard.audio_bitrate(), 256_000);
        assert_eq!(TranscodeQuality::High.ffmpeg_preset(), "slow");
        assert_eq!(TranscodeQuality::Standard.crf(), 23);
    }

    #[test]
    fn test_from_str() {
        assert_eq!(TranscodeQuality::from_str("preview"), Some(TranscodeQuality::Preview));
        assert_eq!(TranscodeQuality::from_str("HIGH"), Some(TranscodeQuality::High));
        assert_eq!(TranscodeQuality::from_str("invalid"), None);
    }
}
