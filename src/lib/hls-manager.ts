/**
 * HLS Player Manager
 *
 * Manages the lifecycle of an hls.js instance attached to a video element.
 */

import Hls from 'hls.js';
import { type HlsPlayerOptions } from './hls-player';
import { LifecycleManager } from '../core/utils/LifecycleManager';
import { detectPlatform } from '../core/input/utils/platform';

/** Default options for HLS player */
const DEFAULT_OPTIONS: Required<HlsPlayerOptions> = {
    debug: false,
    autoStartLoad: true,
    seekDebounceMs: 150
};

export class HlsPlayerManager {
    private hlsInstance: Hls | null = null;
    private mediaElement: HTMLMediaElement | null = null;
    private mergedOptions: Required<HlsPlayerOptions>;
    private seekDebounceTimeout: ReturnType<typeof setTimeout> | null = null;
    private removeMediaListeners: (() => void) | null = null;

    constructor(options: HlsPlayerOptions = {}) {
        this.mergedOptions = { ...DEFAULT_OPTIONS, ...options };
    }

    /** Check if HLS is supported in the current browser */
    static isSupported(): boolean {
        return Hls.isSupported();
    }

    /** Attach the HLS player to a media element and load a playlist */
    attach(mediaElement: HTMLMediaElement, playlistUrl: string): void {
        this.destroy();
        this.mediaElement = mediaElement;
        const reportMediaState = (event: Event) => {
            void LifecycleManager.logTelemetry(
                event.type === 'error' ? 'warn' : 'debug',
                'HlsPlayer',
                JSON.stringify({
                    event: event.type,
                    mediaErrorCode: mediaElement.error?.code ?? null,
                    readyState: mediaElement.readyState,
                    networkState: mediaElement.networkState,
                    currentTime: mediaElement.currentTime,
                    paused: mediaElement.paused
                })
            );
        };
        const mediaEvents = ['loadedmetadata', 'playing', 'waiting', 'stalled', 'error'];
        mediaEvents.forEach(event => mediaElement.addEventListener(event, reportMediaState));
        this.removeMediaListeners = () => {
            mediaEvents.forEach(event => mediaElement.removeEventListener(event, reportMediaState));
        };

        // Keep WebKit's native path; prefer supervised MediaSource on WebView2.
        if (
            mediaElement.canPlayType('application/vnd.apple.mpegurl') &&
            (detectPlatform() !== 'windows' || !Hls.isSupported())
        ) {
            void LifecycleManager.logTelemetry('info', 'HlsPlayer', 'Attaching native HLS');
            mediaElement.src = playlistUrl;
            return;
        }

        if (!Hls.isSupported()) {
            void LifecycleManager.logTelemetry('error', 'HlsPlayer', 'MediaSource HLS unavailable');
            console.error('HLS is not supported in this browser');
            return;
        }

        this.hlsInstance = new Hls({
            debug: this.mergedOptions.debug,
            autoStartLoad: this.mergedOptions.autoStartLoad,
            maxBufferLength: 30,
            maxMaxBufferLength: 60,
            maxBufferSize: 60 * 1024 * 1024,
            fragLoadingMaxRetry: 3,
            manifestLoadingMaxRetry: 3,
            levelLoadingMaxRetry: 3,
            manifestLoadingTimeOut: 60000,
            manifestLoadingMaxRetryTimeout: 60000,
            fragLoadingTimeOut: 60000,
            fragLoadingMaxRetryTimeout: 60000
        });

        this.hlsInstance.on(Hls.Events.MEDIA_ATTACHED, () => {
            this.hlsInstance?.loadSource(playlistUrl);
        });

        this.hlsInstance.on(Hls.Events.ERROR, (_event, data) => {
            // Do not serialize ErrorData: URLs and loader contexts contain session tokens.
            void LifecycleManager.logTelemetry(
                data.fatal ? 'error' : 'warn',
                'HlsPlayer',
                JSON.stringify({
                    event: 'hlsError',
                    type: data.type,
                    details: data.details,
                    fatal: data.fatal,
                    httpStatus: data.response?.code ?? null,
                    segment: data.frag?.sn ?? null,
                    mediaErrorCode: mediaElement.error?.code ?? null,
                    readyState: mediaElement.readyState,
                    networkState: mediaElement.networkState
                })
            );
            if (data.fatal) {
                this.handleFatalError(data.type);
            }
        });
        this.hlsInstance.on(Hls.Events.MANIFEST_PARSED, (_event, data) => {
            void LifecycleManager.logTelemetry(
                'info',
                'HlsPlayer',
                JSON.stringify({ event: 'manifestParsed', levels: data.levels.length })
            );
        });
        void LifecycleManager.logTelemetry('info', 'HlsPlayer', 'Attaching MediaSource HLS');
        this.hlsInstance.attachMedia(mediaElement);
    }

    /** Handle fatal HLS errors with recovery attempts */
    private handleFatalError(errorType: string): void {
        switch (errorType) {
            case Hls.ErrorTypes.NETWORK_ERROR:
                console.error('HLS network error, trying to recover...');
                this.hlsInstance?.startLoad();
                break;
            case Hls.ErrorTypes.MEDIA_ERROR:
                console.error('HLS media error, trying to recover...');
                this.hlsInstance?.recoverMediaError();
                break;
            default:
                console.error('HLS fatal error, destroying...');
                this.destroy();
                break;
        }
    }

    /** Detach the HLS player from the media element */
    detach(): void {
        this.removeMediaListeners?.();
        this.removeMediaListeners = null;
        if (this.seekDebounceTimeout) {
            clearTimeout(this.seekDebounceTimeout);
            this.seekDebounceTimeout = null;
        }

        if (this.hlsInstance) {
            this.hlsInstance.detachMedia();
        }

        if (this.mediaElement) {
            this.mediaElement.removeAttribute('src');
            this.mediaElement.load();
            this.mediaElement = null;
        }
    }

    /** Destroy the HLS player instance */
    destroy(): void {
        this.detach();

        if (this.hlsInstance) {
            this.hlsInstance.destroy();
            this.hlsInstance = null;
        }
    }

    /** Start loading the stream */
    startLoad(startPosition?: number): void {
        this.hlsInstance?.startLoad(startPosition ?? -1);
    }

    /** Stop loading the stream */
    stopLoad(): void {
        this.hlsInstance?.stopLoad();
    }

    /** Debounced seek — useful for scrubbing */
    debouncedSeek(time: number): void {
        if (this.seekDebounceTimeout) {
            clearTimeout(this.seekDebounceTimeout);
        }

        this.seekDebounceTimeout = setTimeout(() => {
            if (this.mediaElement) {
                this.mediaElement.currentTime = time;
            }
        }, this.mergedOptions.seekDebounceMs);
    }

    /** Get the underlying hls.js instance */
    getHls(): Hls | null {
        return this.hlsInstance;
    }

    /** Get the media element */
    getMediaElement(): HTMLMediaElement | null {
        return this.mediaElement;
    }
}
