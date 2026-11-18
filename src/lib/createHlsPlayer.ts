/**
 * SolidJS hook for using HLS player
 *
 * Provides reactive state management for HLS streaming playback.
 */

import { onCleanup, createSignal, createEffect, Accessor } from 'solid-js';
import { type HlsPlayerOptions, isHlsUrl } from './hls-player';
import { HlsPlayerManager } from './hls-manager';
import { observeMediaDiagnostics } from './media-diagnostics';

/**
 * SolidJS hook for using HLS player
 *
 * @param mediaRef - Accessor for the media element reference
 * @param sourceAccessor - Accessor for the video source (file path or HLS URL)
 * @param options - HLS player options
 * @returns Player state and control functions
 */
export function createHlsPlayer(
    mediaRef: Accessor<HTMLMediaElement | undefined>,
    sourceAccessor: Accessor<string>,
    options: HlsPlayerOptions = {}
) {
    const [isLoading, setIsLoading] = createSignal(true);
    const [hasError, setHasError] = createSignal(false);
    const [errorMessage, setErrorMessage] = createSignal<string | null>(null);
    const [isHlsActive, setIsHlsActive] = createSignal(false);

    let manager: HlsPlayerManager | null = null;
    let disposeDiagnostics: (() => void) | undefined;

    createEffect(() => {
        disposeDiagnostics?.();
        disposeDiagnostics = undefined;
        const media = mediaRef();
        const source = sourceAccessor();

        if (!media || !source) return;

        disposeDiagnostics = observeMediaDiagnostics(media);
        setIsLoading(true);
        setHasError(false);
        setErrorMessage(null);

        if (manager) {
            manager.destroy();
            manager = null;
        }

        if (isHlsUrl(source)) {
            setIsHlsActive(true);
            attachHlsSource(media, source, options);
        } else {
            setIsHlsActive(false);
            attachNativeSource(media, source);
        }
    });

    function attachHlsSource(
        media: HTMLMediaElement,
        source: string,
        playerOptions: HlsPlayerOptions
    ): void {
        if (HlsPlayerManager.isSupported() || media.canPlayType('application/vnd.apple.mpegurl')) {
            manager = new HlsPlayerManager(playerOptions);
            manager.attach(media, source);

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
    }

    function attachNativeSource(media: HTMLMediaElement, source: string): void {
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

    onCleanup(() => {
        disposeDiagnostics?.();
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
        /** Debounced seek for scrubbing */
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
        /** Get the underlying manager instance */
        getManager: () => manager
    };
}
