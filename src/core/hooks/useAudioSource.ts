import { createSignal, createEffect, on } from 'solid-js';
import {
    type TranscodeQuality,
    getAudioUrl
} from '../../lib/stream-utils';

/**
 * Custom hook to manage audio source URL generation.
 * Consolidates streaming strategy logic used in both AudioRenderer and AudioInspector.
 */
export function useAudioSource(
    pathAccessor: () => string | undefined,
    qualityAccessor: () => TranscodeQuality = () => 'standard'
) {
    const [audioUrl, setAudioUrl] = createSignal('');

    // Update URL when path or quality changes
    createEffect(
        on(
            () => [pathAccessor(), qualityAccessor()] as const,
            ([path, q]) => {
                if (!path) {
                    setAudioUrl('');
                    return;
                }

                // Delegate URL construction to central logic in stream-utils
                const url = getAudioUrl(path, q);
                setAudioUrl(url);
            }
        )
    );

    return {
        audioUrl
    };
}
