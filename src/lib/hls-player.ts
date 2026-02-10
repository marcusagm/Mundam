/**
 * HLS Player Manager for SolidJS
 *
 * Wrapper around hls.js for seamless HLS streaming integration.
 * Designed for use with the HLS on-the-fly streaming server.
 */

import Hls from 'hls.js';
import { fetch } from '@tauri-apps/plugin-http';
import { onCleanup, createSignal, createEffect, Accessor } from 'solid-js';

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
    /** Error message if any */
    errorMessage: string | null;
    /** Current buffered percentage */
    buffered: number;
}

/** Default options for HLS player */
const DEFAULT_OPTIONS: Required<HlsPlayerOptions> = {
    debug: false,
    autoStartLoad: true,
    seekDebounceMs: 150,
};

/** HLS streaming server base URL */
export const HLS_SERVER_URL = 'http://127.0.0.1:9876';

/**
 * Get the HLS playlist URL for a video file
 * @param filePath - Absolute path to the video file
 * @returns The M3U8 playlist URL
 */
export function getHlsPlaylistUrl(filePath: string, quality: string = 'standard'): string {
    const encodedPath = encodeURIComponent(filePath);
    return `${HLS_SERVER_URL}/playlist/${encodedPath}?quality=${quality}`;
}

/**
 * Get the probe URL for a video file
 * @param filePath - Absolute path to the video file
 * @returns The probe endpoint URL
 */
export function getHlsProbeUrl(filePath: string): string {
    const encodedPath = encodeURIComponent(filePath);
    return `${HLS_SERVER_URL}/probe/${encodedPath}`;
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
 * Probe a video file for metadata
 * @param filePath - Absolute path to the video file
 * @returns Video metadata including duration and native format detection
 */
export async function probeVideo(filePath: string): Promise<VideoProbeResult> {
    const url = getHlsProbeUrl(filePath);
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
        console.log(`DEBUG: Checking HLS server at ${HLS_SERVER_URL}/health`);
        const response = await fetch(`${HLS_SERVER_URL}/health`, {
            method: 'GET',
        });
        console.log(`DEBUG: HLS server response status: ${response.status}`);
        return response.ok;
    } catch (error) {
        console.error('DEBUG: HLS server check failed:', error);
        return false;
    }
}

/**
 * Check if a URL is an HLS playlist
 */
export function isHlsUrl(url: string): boolean {
    return url.endsWith('.m3u8') || url.includes(HLS_SERVER_URL);
}

/**
 * HLS Player Manager class
 *
 * Manages the lifecycle of an hls.js instance attached to a video element.
 */
export class HlsPlayerManager {
    private hls: Hls | null = null;
    private mediaElement: HTMLMediaElement | null = null;
    private options: Required<HlsPlayerOptions>;
    private seekDebounceTimeout: ReturnType<typeof setTimeout> | null = null;

    constructor(options: HlsPlayerOptions = {}) {
        this.options = { ...DEFAULT_OPTIONS, ...options };
    }

    /**
     * Check if HLS is supported in the current browser
     */
    static isSupported(): boolean {
        return Hls.isSupported();
    }

    /**
     * Attach the HLS player to a media element and load a playlist
     * @param mediaElement - The media element to attach to
     * @param playlistUrl - The M3U8 playlist URL
     */
    attach(mediaElement: HTMLMediaElement, playlistUrl: string): void {
        // Detach any existing instance
        this.detach();

        this.mediaElement = mediaElement;

        // Check native HLS support (Safari)
        if (mediaElement.canPlayType('application/vnd.apple.mpegurl')) {
            // Safari has native HLS support
            mediaElement.src = playlistUrl;
            return;
        }

        if (!Hls.isSupported()) {
            console.error('HLS is not supported in this browser');
            return;
        }

        this.hls = new Hls({
            debug: this.options.debug,
            autoStartLoad: this.options.autoStartLoad,
            // Optimize for live-ish streaming
            maxBufferLength: 30,
            maxMaxBufferLength: 60,
            maxBufferSize: 60 * 1024 * 1024, // 60MB
            // Retry configuration
            fragLoadingMaxRetry: 3,
            manifestLoadingMaxRetry: 3,
            levelLoadingMaxRetry: 3,
        });

        this.hls.attachMedia(mediaElement);

        this.hls.on(Hls.Events.MEDIA_ATTACHED, () => {
            this.hls?.loadSource(playlistUrl);
        });

        this.hls.on(Hls.Events.ERROR, (_event, data) => {
            if (data.fatal) {
                switch (data.type) {
                    case Hls.ErrorTypes.NETWORK_ERROR:
                        console.error('HLS network error, trying to recover...');
                        this.hls?.startLoad();
                        break;
                    case Hls.ErrorTypes.MEDIA_ERROR:
                        console.error('HLS media error, trying to recover...');
                        this.hls?.recoverMediaError();
                        break;
                    default:
                        console.error('HLS fatal error, destroying...');
                        this.destroy();
                        break;
                }
            }
        });
    }

    /**
     * Detach the HLS player from the media element
     */
    detach(): void {
        if (this.seekDebounceTimeout) {
            clearTimeout(this.seekDebounceTimeout);
            this.seekDebounceTimeout = null;
        }

        if (this.hls) {
            this.hls.detachMedia();
        }

        if (this.mediaElement) {
            this.mediaElement.removeAttribute('src');
            this.mediaElement.load();
            this.mediaElement = null;
        }
    }

    /**
     * Destroy the HLS player instance
     */
    destroy(): void {
        this.detach();

        if (this.hls) {
            this.hls.destroy();
            this.hls = null;
        }
    }

    /**
     * Start loading the stream
     */
    startLoad(startPosition?: number): void {
        this.hls?.startLoad(startPosition ?? -1);
    }

    /**
     * Stop loading the stream
     */
    stopLoad(): void {
        this.hls?.stopLoad();
    }

    /**
     * Debounced seek - useful for scrubbing
     * @param time - Time to seek to in seconds
     */
    debouncedSeek(time: number): void {
        if (this.seekDebounceTimeout) {
            clearTimeout(this.seekDebounceTimeout);
        }

        this.seekDebounceTimeout = setTimeout(() => {
            if (this.mediaElement) {
                this.mediaElement.currentTime = time;
            }
        }, this.options.seekDebounceMs);
    }

    /**
     * Get the underlying hls.js instance
     */
    getHls(): Hls | null {
        return this.hls;
    }

    /**
     * Get the media element
     */
    getMediaElement(): HTMLMediaElement | null {
        return this.mediaElement;
    }
}

/**
 * SolidJS hook for using HLS player
 *
 * @param mediaRef - Accessor for the media element reference
 * @param src - Accessor for the video source (file path or HLS URL)
 * @param options - HLS player options
 * @returns Player state and control functions
 */
export function createHlsPlayer(
    mediaRef: Accessor<HTMLMediaElement | undefined>,
    src: Accessor<string>,
    options: HlsPlayerOptions = {}
) {
    const [isLoading, setIsLoading] = createSignal(true);
    const [hasError, setHasError] = createSignal(false);
    const [errorMessage, setErrorMessage] = createSignal<string | null>(null);
    const [isHlsActive, setIsHlsActive] = createSignal(false);

    let manager: HlsPlayerManager | null = null;

    // Effect to handle source changes
    createEffect(() => {
        const media = mediaRef();
        const source = src();

        if (!media || !source) return;

        // Reset state
        setIsLoading(true);
        setHasError(false);
        setErrorMessage(null);

        // Clean up previous manager
        if (manager) {
            manager.destroy();
            manager = null;
        }

        // Check if this is an HLS source
        if (isHlsUrl(source)) {
            setIsHlsActive(true);

            if (HlsPlayerManager.isSupported() || media.canPlayType('application/vnd.apple.mpegurl')) {
                manager = new HlsPlayerManager(options);
                manager.attach(media, source);

                // Listen for loading events
                media.addEventListener('loadeddata', () => setIsLoading(false), { once: true });
                media.addEventListener(
                    'error',
                    () => {
                        setHasError(true);
                        setErrorMessage('Failed to load HLS stream');
                        setIsLoading(false);
                    },
                    { once: true }
                );
            } else {
                setHasError(true);
                setErrorMessage('HLS is not supported in this browser');
                setIsLoading(false);
            }
        } else {
            // Regular media source
            setIsHlsActive(false);
            media.src = source;
            media.addEventListener('loadeddata', () => setIsLoading(false), { once: true });
            media.addEventListener(
                'error',
                () => {
                    setHasError(true);
                    setErrorMessage('Failed to load media');
                    setIsLoading(false);
                },
                { once: true }
            );
        }
    });

    // Cleanup on unmount
    onCleanup(() => {
        if (manager) {
            manager.destroy();
            manager = null;
        }
    });

    return {
        isLoading,
        hasError,
        errorMessage,
        isHlsActive,
        /**
         * Debounced seek for scrubbing
         */
        debouncedSeek: (time: number) => {
            if (manager && isHlsActive()) {
                manager.debouncedSeek(time);
            } else {
                const media = mediaRef();
                if (media) {
                    media.currentTime = time;
                }
            }
        },
        /**
         * Get the underlying manager instance
         */
        getManager: () => manager,
    };
}
