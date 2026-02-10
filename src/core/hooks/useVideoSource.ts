import { createSignal, createEffect, on, createResource } from 'solid-js';
import {
    type TranscodeQuality,
    probeVideo,
    isHlsServerAvailable,
    getVideoUrl,
    type VideoProbeResult
} from '../../lib/stream-utils';

/**
 * Custom hook to manage video source URL generation and probing.
 * Consolidates streaming strategy logic used in both ItemView and Inspector.
 */
export function useVideoSource(
    pathAccessor: () => string | undefined,
    qualityAccessor: () => TranscodeQuality = () => 'standard'
) {
    const [videoUrl, setVideoUrl] = createSignal('');
    const [probeError, setProbeError] = createSignal<string | null>(null);

    // Probe video when path changes
    const [probeResult] = createResource(
        pathAccessor,
        async (path): Promise<VideoProbeResult | null> => {
            if (!path) return null;

            try {
                // First check if HLS server is available
                const serverAvailable = await isHlsServerAvailable();
                if (!serverAvailable) {
                    console.log('VideoSourceHook: HLS server not available, using fallback');
                    setProbeError(null);
                    return null;
                }

                // Probe the video
                const result = await probeVideo(path);
                setProbeError(null);
                return result;
            } catch (e) {
                console.warn('VideoSourceHook: Video probe failed:', e);
                setProbeError(e instanceof Error ? e.message : 'Probe failed');
                return null;
            }
        }
    );

    // Update URL when path, quality, or probe result changes
    createEffect(
        on(
            () => [pathAccessor(), qualityAccessor(), probeResult()] as const,
            ([path, q, probe]) => {
                if (!path) {
                    setVideoUrl('');
                    return;
                }

                // Delegate URL construction to central logic in stream-utils
                const url = getVideoUrl(path, q, probe);
                setVideoUrl(url);
            }
        )
    );

    return {
        videoUrl,
        probeResult,
        probeError,
        isLoading: probeResult.loading
    };
}
