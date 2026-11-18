/**
 * HLS Player Utilities
 *
 * Types, constants, and utility functions for HLS streaming integration.
 * For the player manager class, see hls-manager.ts
 * For the SolidJS hook, see createHlsPlayer.ts
 *
 * Security: All streaming URLs include a session token generated at app boot.
 * The token is fetched once via Tauri IPC and cached for the session lifetime.
 */

import { invokeCommand as invoke } from './api';
import { fetch } from '@tauri-apps/plugin-http';

/**
 * Configuration options for the HLS player.
 */
export interface HlsPlayerOptions {
    /** Enable debug logging */
    debug?: boolean;
    /** Start loading immediately on attach */
    autoStartLoad?: boolean;
    /** Debounce delay for seek operations (ms) */
    seekDebounceMs?: number;
}

export interface HlsPlayerState {
    /** Whether the player is loading */
    isLoading: boolean;
    /** Whether the player has encountered an error */
    hasError: boolean;
    /** Error message if unknown */
    errorMessage: string | null;
    /** Current buffered percentage */
    buffered: number;
}

/** HLS streaming server base URL */
export const HLS_SERVER_URL = 'http://127.0.0.1:9876';

/** Cached session token for streaming server authentication */
let cachedStreamingToken: string | null = null;

/**
 * Initialize the streaming session token by fetching it from the backend.
 *
 * Must be called once during app initialization before unknown streaming URLs
 * are constructed. The token is cached for the lifetime of the session.
 */
export async function initStreamingToken(): Promise<void> {
    if (cachedStreamingToken) return;
    cachedStreamingToken = await invoke<string>('get_streaming_token');
}

/**
 * Get the current streaming token, or empty string if not yet initialized.
 *
 * Prefer calling `initStreamingToken()` during app boot to ensure
 * the token is available before unknown streaming requests.
 */
export function getStreamingToken(): string {
    return cachedStreamingToken ?? '';
}

/**
 * Build a query string suffix with the session token.
 * Returns `&token=xxx` if the token is available, or empty string otherwise.
 */
function buildTokenSuffix(): string {
    const token = getStreamingToken();
    return token ? `&token=${token}` : '';
}

/**
 * Get the HLS playlist URL for an asset
 * @param {string} assetId - UUID of the asset
 * @param {string} [quality='standard'] - Quality string parameter
 * @returns {string} The M3U8 playlist URL with authentication token
 */
export function getHlsPlaylistUrl(assetId: string, quality: string = 'standard'): string {
    return `${HLS_SERVER_URL}/playlist/${assetId}/playlist.m3u8?quality=${quality}${buildTokenSuffix()}`;
}

/**
 * Get the probe URL for an asset
 * @param {string} assetId - UUID of the asset
 * @returns {string} The probe endpoint URL with authentication token
 */
export function getHlsProbeUrl(assetId: string): string {
    return `${HLS_SERVER_URL}/probe/${assetId}?_=1${buildTokenSuffix()}`;
}

/**
 * Probe a video file to get metadata and native format detection
 */
export interface VideoProbeResult {
    duration_secs: number;
    is_native: boolean;
    video_codec: string | null;
    audio_codec: string | null;
    container: string | null;
    width: number | null;
    height: number | null;
}

/**
 * Probe an asset for metadata
 * @param {string} assetId - UUID of the asset
 * @returns {Promise<VideoProbeResult>} Video metadata including duration and native format detection
 * @throws {Error} Throw if probe request fails
 */
export async function probeVideo(assetId: string): Promise<VideoProbeResult> {
    const url = getHlsProbeUrl(assetId);
    const response = await fetch(url);

    if (!response.ok) {
        throw new Error(`Probe failed: ${response.statusText}`);
    }

    return response.json();
}

/**
 * Check if HLS streaming server is available
 */
export async function isHlsServerAvailable(): Promise<boolean> {
    try {
        const response = await fetch(`${HLS_SERVER_URL}/health`, {
            method: 'GET'
        });
        return response.ok;
    } catch {
        return false;
    }
}

/**
 * Check if a URL is an HLS playlist
 */
export function isHlsUrl(url: string): boolean {
    const resourcePath = url.split(/[?#]/, 1)[0];
    return /\.m3u8$/i.test(resourcePath);
}

// Re-export for backwards compatibility
export { HlsPlayerManager } from './hls-manager';
export { createHlsPlayer } from './createHlsPlayer';
