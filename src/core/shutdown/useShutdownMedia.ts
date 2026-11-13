import { onCleanup } from 'solid-js';
import { LifecycleManager } from '../utils/LifecycleManager';

/** Pauses media immediately on shutdown, including playback started during draining. */
export function useShutdownMedia(): void {
    let closing = false;
    const pauseMedia = (media: HTMLMediaElement) => {
        media.autoplay = false;
        media.muted = true;
        media.pause();
    };
    const preventPlayback = (event: Event) => {
        if (closing && event.target instanceof HTMLMediaElement) {
            pauseMedia(event.target);
        }
    };
    document.addEventListener('play', preventPlayback, true);
    LifecycleManager.registerListener<void>('shutdown:requested', () => {
        closing = true;
        const media = document.querySelectorAll<HTMLMediaElement>('audio, video');
        media.forEach(pauseMedia);
        void LifecycleManager.logTelemetry(
            'info',
            'ShutdownMedia',
            `Paused media: ${media.length}`
        );
    });
    onCleanup(() => document.removeEventListener('play', preventPlayback, true));
}
