//! Media transcoding module for unsupported formats
//! 
//! Uses FFmpeg to transcode audio/video formats that are not natively
//! supported by the WebView (WKWebView/WebView2).

pub mod quality;
pub mod cache;
pub mod ffmpeg_pipe;
pub mod detector;

pub mod commands;

pub use quality::TranscodeQuality;
pub use cache::TranscodeCache;
pub use ffmpeg_pipe::FfmpegTranscoder;
