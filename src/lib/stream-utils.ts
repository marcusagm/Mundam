/**
 * Stream utilities for handling media transcoding
 * Provides functions to detect if a file needs transcoding and generate appropriate URLs
 * Now powered by the central Format Store (Backend Source of Truth)
 */

import { invokeCommand as invoke } from './api';
import { formatActions } from '../core/store/formatStore';
import {
    HLS_SERVER_URL,
    getHlsPlaylistUrl,
    getStreamingToken,
    type VideoProbeResult
} from './hls-player';
import { getAssetUrl } from '../core/utils/assetUrl';

export { HLS_SERVER_URL };

// Re-export HLS utilities for convenience
export {
    getHlsPlaylistUrl,
    getHlsProbeUrl,
    probeVideo,
    isHlsServerAvailable,
    isHlsUrl,
    getStreamingToken,
    type VideoProbeResult
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
    const strategy = formatActions.getPlaybackStrategy(getExtension(path));
    return strategy === 'audioHls' || strategy === 'audioTranscode';
}

/**
 * Check if a file extension requires HLS for audio (deprecated, use specific helpers)
 */
export function needsHlsAudio(path: string): boolean {
    const strategy = formatActions.getPlaybackStrategy(getExtension(path));
    return strategy === 'audioHls' || strategy === 'audioLinearHls';
}

/**
 * Check if a file needs unknown kind of transcoding
 */
export function needsTranscoding(path: string): boolean {
    const strategy = formatActions.getPlaybackStrategy(getExtension(path));
    return (
        strategy === 'transcode' ||
        strategy === 'linearHls' ||
        strategy === 'hls' ||
        strategy === 'audioTranscode' ||
        strategy === 'audioHls' ||
        strategy === 'audioLinearHls'
    );
}

/**
 * Check if a file is natively supported
 */
export function isNativeFormat(path: string): boolean {
    const strategy = formatActions.getPlaybackStrategy(getExtension(path));
    return strategy === 'native';
}

/**
 * Get the appropriate audio URL for an asset
 */
export function getAudioUrl(
    assetId: string,
    path: string,
    quality: TranscodeQuality = 'standard'
): string {
    if (needsLinearAudio(path)) {
        // Linear HLS (Live/Async)
        const tokenSuffix = getStreamingToken() ? `&token=${getStreamingToken()}` : '';
        return `${HLS_SERVER_URL}/hls-live/${assetId}/index.m3u8?quality=${quality}&mode=audio${tokenSuffix}`;
    }

    if (needsStandardHlsAudio(path)) {
        // Standard HLS (Playlist/VOD)
        return getHlsPlaylistUrl(assetId, quality);
    }

    // Default to asset protocol (direct file access)
    return getAssetUrl(assetId);
}

function requiresLinearHls(path: string, probe?: VideoProbeResult | null): boolean {
    if (needsLinearTranscoding(path)) return true;
    if (!probe || !probe.video_codec) return false;
    return ['mjpeg', 'flv1', 'vp6f'].includes(probe.video_codec);
}

function requiresStandardHls(path: string, probe?: VideoProbeResult | null): boolean {
    return needsHlsTranscoding(path) || (probe !== undefined && probe !== null && !probe.is_native);
}

/**
 * Get the appropriate video URL for a file path.
 * Optionally accepts a probe result to check for specific codecs that require linear transcoding.
 */
export function getVideoUrl(
    assetId: string,
    path: string,
    quality: TranscodeQuality = 'standard',
    probe?: VideoProbeResult | null
): string {
    if (requiresLinearHls(path, probe)) {
        // Linear HLS (Live) - e.g. SWF or MJPEG
        const tokenSuffix = getStreamingToken() ? `&token=${getStreamingToken()}` : '';
        return `${HLS_SERVER_URL}/hls-live/${assetId}/index.m3u8?quality=${quality}&mode=live${tokenSuffix}`;
    }

    if (requiresStandardHls(path, probe)) {
        // Standard HLS (VOD) - e.g. MKV, AVI or non-native codec found via probe
        return getHlsPlaylistUrl(assetId, quality);
    }

    // Native format (MP4/MOV) - Direct file access over custom protocol
    return getAssetUrl(assetId);
}

/**
 * Get the media type from file extension
 * Delegates to the central store which has the authoritative categorization.
 */
export function getMediaType(
    path: string
): 'audio' | 'video' | 'image' | 'font' | 'model3d' | 'project' | 'vector' | 'archive' | 'unknown' {
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
            { id: 'high', label: 'High', videoBitrate: 10000000, audioBitrate: 320000 }
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
    return await invoke<CacheStats>('get_library_cache_stats');
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
