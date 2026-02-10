/**
 * Stream utilities for handling media transcoding
 * Provides functions to detect if a file needs transcoding and generate appropriate URLs
 */

import { invoke } from '@tauri-apps/api/core';

import { HLS_SERVER_URL, getHlsPlaylistUrl } from './hls-player';

export { HLS_SERVER_URL };

// Re-export HLS utilities for convenience
export {
  getHlsPlaylistUrl,
  getHlsProbeUrl,
  probeVideo,
  isHlsServerAvailable,
  isHlsUrl,
  type VideoProbeResult,
} from './hls-player';

export type TranscodeQuality = 'preview' | 'standard' | 'high';

export interface QualityOption {
  id: TranscodeQuality;
  label: string;
  videoBitrate: number;
  audioBitrate: number;
}

export interface CacheStats {
  directory: string;
  sizeBytes: number;
}


// Audio extensions that require transcoding
const TRANSCODE_AUDIO_EXTENSIONS = new Set([
  'ogg', 'oga', 'opus',   // Ogg container
  'wma',                  // Windows Media
  'ac3',                  // Dolby Digital (AC-3)
  'dts',                  // DTS
  'spx',                  // Speex
  'ra', 'rm',             // RealAudio
  'mka',                  // Matroska Audio
  'aiff', 'aif', 'aifc',  // AIFF
  'amr',                  // AMR (Mobile)
  'ape',                  // Monkey's Audio
  'wv',                   // WavPack
]);

// Video extensions that require transcoding
const TRANSCODE_VIDEO_EXTENSIONS = new Set([
  // Desktop containers
  'mkv',                  // Matroska
  'avi',                  // AVI
  'flv', 'f4v',           // Flash Video
  'wmv', 'asf',           // Windows Media
  'ogv',                  // Ogg Video
  'webm',                 // WebM (VP9 not supported on macOS WebView)
  // Broadcast/Professional
  'mpeg', 'mpg', 'm2v',   // MPEG-1/2
  'vob',                  // DVD Video
  'm2ts', 'mts', 'ts',    // MPEG Transport Stream
  'mxf',                  // Material Exchange Format
  'wtv',                  // Windows TV
  // Legacy/Mobile
  '3gp', '3g2',           // 3GPP
  'rmvb',                 // RealMedia
  'swf',                  // Flash (limited support)
  'divx',                 // DivX
  'hevc',                 // Raw HEVC
  'mjpeg', 'mjpg',        // Motion JPEG
]);

// Native audio extensions (no transcoding needed)
const NATIVE_AUDIO_EXTENSIONS = new Set([
  'mp3', 'wav', 'aac', 'm4a', 'm4r', 'flac', 'mp2',
]);

// Video extensions that require linear transcoding (live HLS)
const LINEAR_VIDEO_EXTENSIONS = new Set([
  'swf',                  // Flash
  'mpg', 'mpeg', 'm2v',   // MPEG-1/2 (Often has seeking issues)
  'wtv',                  // Windows TV (MPEG-2 based, poor seeking)
  'rm', 'rmvb',           // RealMedia (Proprietary, poor seeking)
  '3gp', '3g2',           // 3GPP (Mobile, erratic timestamps)
  'mjpeg', 'mjpg',        // Motion JPEG (Stream of images)
  'ogv',                  // Ogg Theora (Often fails segmentation)
]);

// Native video extensions (no transcoding needed)
const NATIVE_VIDEO_EXTENSIONS = new Set([
  'mp4', 'm4v', 'mov', 'qt',
]);

// HLS Audio extensions (use HLS to prevent UI freezing during load)
const HLS_AUDIO_EXTENSIONS = new Set([
  'opus', 'oga', 'ogg',   // Ogg container
  'wma',                  // Windows Media
  'ac3',                  // Dolby Digital
  'dts',                  // DTS
  'wv',                   // WavPack
  'aifc', 'amr', 'ape',   // Untested but likely problematic
]);

/**
 * Get the file extension from a path (lowercase, without dot)
 */
export function getExtension(path: string): string {
  const parts = path.split('.');
  return parts.length > 1 ? parts[parts.length - 1].toLowerCase() : '';
}

/**
 * Check if a file extension requires transcoding for audio playback
 */
export function needsAudioTranscoding(path: string): boolean {
  const ext = getExtension(path);
  return TRANSCODE_AUDIO_EXTENSIONS.has(ext);
}

/**
 * Check if a file extension requires transcoding for video playback
 */
export function needsVideoTranscoding(path: string): boolean {
  const ext = getExtension(path);
  return TRANSCODE_VIDEO_EXTENSIONS.has(ext);
}

/**
 * Check if a file extension requires linear transcoding (live HLS)
 */
export function needsLinearTranscoding(path: string): boolean {
  const ext = getExtension(path);
  return LINEAR_VIDEO_EXTENSIONS.has(ext);
}

/**
 * Check if a file extension requires HLS for audio (to avoid freezing)
 */
export function needsHlsAudio(path: string): boolean {
  const ext = getExtension(path);
  return HLS_AUDIO_EXTENSIONS.has(ext);
}

/**
 * Check if a file needs any kind of transcoding
 */
export function needsTranscoding(path: string): boolean {
  return needsAudioTranscoding(path) || needsVideoTranscoding(path);
}

/**
 * Check if a file is natively supported
 */
export function isNativeFormat(path: string): boolean {
  const ext = getExtension(path);
  return NATIVE_AUDIO_EXTENSIONS.has(ext) || NATIVE_VIDEO_EXTENSIONS.has(ext);
}

/**
 * Get the appropriate audio URL for a file path
 * Uses audio-stream:// for files that need transcoding, audio:// otherwise
 * Uses HLS for formats that cause UI freezing
 */
export function getAudioUrl(path: string, quality: TranscodeQuality = 'standard'): string {
  if (needsHlsAudio(path)) {
     return getHlsPlaylistUrl(path, quality);
  }

  const encodedPath = encodeURIComponent(path);

  if (needsAudioTranscoding(path)) {
    return `audio-stream://localhost/${encodedPath}?quality=${quality}`;
  }

  return `audio://localhost/${encodedPath}`;
}

/**
 * Get the appropriate video URL for a file path
 * Uses video-stream:// for files that need transcoding, video:// otherwise
 * Uses HLS Live for linear formats
 */
export function getVideoUrl(path: string, quality: TranscodeQuality = 'standard'): string {
  const encodedPath = encodeURIComponent(path);

  if (needsLinearTranscoding(path)) {
    return `${HLS_SERVER_URL}/hls-live/${encodedPath}/index.m3u8?quality=${quality}`;
  }

  if (needsVideoTranscoding(path)) {
    return `video-stream://localhost/${encodedPath}?quality=${quality}`;
  }

  return `video://localhost/${encodedPath}`;
}

/**
 * Get the media type from file extension
 */
export function getMediaType(path: string): 'audio' | 'video' | 'unknown' {
  const ext = getExtension(path);

  if (NATIVE_AUDIO_EXTENSIONS.has(ext) || TRANSCODE_AUDIO_EXTENSIONS.has(ext)) {
    return 'audio';
  }

  if (NATIVE_VIDEO_EXTENSIONS.has(ext) || TRANSCODE_VIDEO_EXTENSIONS.has(ext)) {
    return 'video';
  }

  return 'unknown';
}

// --- Tauri Commands ---

/**
 * Check if FFmpeg is available in the system
 */
export async function isFfmpegAvailable(): Promise<boolean> {
  try {
    return await invoke<boolean>('ffmpeg_available');
  } catch {
    return false;
  }
}

/**
 * Get available quality options
 */
export async function getQualityOptions(): Promise<QualityOption[]> {
  try {
    return await invoke<QualityOption[]>('get_quality_options');
  } catch {
    return [
      { id: 'preview', label: 'Preview', videoBitrate: 2000000, audioBitrate: 128000 },
      { id: 'standard', label: 'Standard', videoBitrate: 5000000, audioBitrate: 256000 },
      { id: 'high', label: 'High', videoBitrate: 10000000, audioBitrate: 320000 },
    ];
  }
}

/**
 * Check if a transcoded version is already cached
 */
export async function isCached(path: string, quality?: TranscodeQuality): Promise<boolean> {
  try {
    return await invoke<boolean>('is_cached', { path, quality });
  } catch {
    return false;
  }
}

/**
 * Get cache statistics
 */
export async function getCacheStats(): Promise<CacheStats> {
  return await invoke<CacheStats>('get_cache_stats');
}

/**
 * Clean up old cache entries
 */
export async function cleanupCache(maxAgeDays?: number): Promise<number> {
  return await invoke<number>('cleanup_cache', { maxAgeDays });
}

/**
 * Pre-transcode a file (returns cached path)
 */
export async function transcodeFile(path: string, quality?: TranscodeQuality): Promise<string> {
  return await invoke<string>('transcode_file', { path, quality });
}

/**
 * Get stream URL from backend (uses backend format detection)
 */
export async function getStreamUrl(path: string, quality?: TranscodeQuality): Promise<string> {
  return await invoke<string>('get_stream_url', { path, quality });
}
