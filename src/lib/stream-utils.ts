/**
 * Stream utilities for handling media transcoding
 * Provides functions to detect if a file needs transcoding and generate appropriate URLs
 * Now powered by the central Format Store (Backend Source of Truth)
 */

import { invoke } from '@tauri-apps/api/core';
import { formatActions } from '../core/store/formatStore';
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
  const strategy = formatActions.getPlaybackStrategy(getExtension(path));
  return strategy === 'audioTranscode';
}

/**
 * Check if a file extension requires transcoding for video playback
 */
export function needsVideoTranscoding(path: string): boolean {
  const strategy = formatActions.getPlaybackStrategy(getExtension(path));
  return strategy === 'transcode';
}

/**
 * Check if a file extension requires linear transcoding (live HLS - e.g. SWF, MPG)
 */
export function needsLinearTranscoding(path: string): boolean {
  return formatActions.getPlaybackStrategy(getExtension(path)) === 'linearHls';
}

/**
 * Check if a file extension requires standard HLS transcoding (e.g. MKV, AVI)
 */
export function needsHlsTranscoding(path: string): boolean {
  return formatActions.getPlaybackStrategy(getExtension(path)) === 'hls';
}

/**
 * Check if a file extension requires Linear HLS for audio
 */
export function needsLinearAudio(path: string): boolean {
  return formatActions.getPlaybackStrategy(getExtension(path)) === 'audioLinearHls';
}

/**
 * Check if a file extension requires Standard HLS for audio
 */
export function needsStandardHlsAudio(path: string): boolean {
  return formatActions.getPlaybackStrategy(getExtension(path)) === 'audioHls';
}

/**
 * Check if a file extension requires HLS for audio (deprecated, use specific helpers)
 */
export function needsHlsAudio(path: string): boolean {
  const strategy = formatActions.getPlaybackStrategy(getExtension(path));
  return strategy === 'audioHls' || strategy === 'audioLinearHls';
}

/**
 * Check if a file needs any kind of transcoding
 */
export function needsTranscoding(path: string): boolean {
  const strategy = formatActions.getPlaybackStrategy(getExtension(path));
  return strategy === 'transcode' || strategy === 'linearHls' || strategy === 'hls' || strategy === 'audioTranscode' || strategy === 'audioHls' || strategy === 'audioLinearHls';
}

/**
 * Check if a file is natively supported
 */
export function isNativeFormat(path: string): boolean {
  const strategy = formatActions.getPlaybackStrategy(getExtension(path));
  return strategy === 'native';
}

/**
 * Get the appropriate audio URL for a file path
 */
export function getAudioUrl(path: string, quality: TranscodeQuality = 'standard'): string {
  const encodedPath = encodeURIComponent(path);

  if (needsLinearAudio(path)) {
     // Linear HLS (Live/Async)
     return `${HLS_SERVER_URL}/hls-live/${encodedPath}/index.m3u8?quality=${quality}&mode=audio`;
  }

  if (needsStandardHlsAudio(path)) {
     // Standard HLS (Playlist/VOD)
     return getHlsPlaylistUrl(path, quality);
  }

  if (needsAudioTranscoding(path)) {
    return `audio-stream://localhost/${encodedPath}?quality=${quality}`;
  }

  return `audio://localhost/${encodedPath}`;
}

/**
 * Get the appropriate video URL for a file path
 */
export function getVideoUrl(path: string, quality: TranscodeQuality = 'standard'): string {
  const encodedPath = encodeURIComponent(path);

  if (needsLinearTranscoding(path)) {
    // Linear HLS (Live) - e.g. SWF
    return `${HLS_SERVER_URL}/hls-live/${encodedPath}/index.m3u8?quality=${quality}&mode=live`;
  }

  if (needsHlsTranscoding(path)) {
    // Standard HLS (VOD) - e.g. MKV, AVI
    return getHlsPlaylistUrl(path, quality);
  }

  if (needsVideoTranscoding(path)) {
    return `video-stream://localhost/${encodedPath}?quality=${quality}`;
  }

  return `video://localhost/${encodedPath}`;
}

/**
 * Get the media type from file extension
 * Delegates to the central store which has the authoritative categorization.
 */
export function getMediaType(path: string): 'audio' | 'video' | 'image' | 'font' | 'model3d' | 'project' | 'archive' | 'unknown' {
  return formatActions.getMediaType(getExtension(path));
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
