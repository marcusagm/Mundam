import { LifecycleManager } from '../core/utils/LifecycleManager';

/**
 * Reports media state without serializing resource URLs, tokens or raw errors.
 * @param media - Element whose native decoder and video frames are being inspected.
 * @returns A disposer for diagnostic listeners.
 */
export function observeMediaDiagnostics(media: HTMLMediaElement): () => void {
    let lastSample = -Infinity;
    const report = (event: Event): void => {
        if (event.type === 'timeupdate') {
            if (media.paused || performance.now() - lastSample < 5000) return;
            lastSample = performance.now();
        }
        void LifecycleManager.logTelemetry(
            event.type === 'error' ? 'warn' : event.type === 'timeupdate' ? 'debug' : 'info',
            'MediaPlayback',
            JSON.stringify({
                event: event.type,
                transport: media.currentSrc.split(':', 1)[0] || null,
                mediaErrorCode: media.error?.code ?? null,
                readyState: media.readyState,
                networkState: media.networkState,
                currentTime: media.currentTime,
                paused: media.paused,
                ...getVideoDiagnostics(media)
            })
        );
    };
    const events = ['loadedmetadata', 'loadeddata', 'playing', 'error', 'resize', 'timeupdate'];
    events.forEach(event => media.addEventListener(event, report));
    return () => events.forEach(event => media.removeEventListener(event, report));
}

/**
 * Reads video frame counters without assuming the WebView exposes every API.
 * @param media - Current video or audio element.
 * @returns Dimensions and frame counters, or null for unavailable values.
 */
function getVideoDiagnostics(media: HTMLMediaElement) {
    if (!(media instanceof HTMLVideoElement)) {
        return {
            videoWidth: null,
            videoHeight: null,
            totalVideoFrames: null,
            droppedVideoFrames: null
        };
    }
    const quality = media.getVideoPlaybackQuality?.();
    const decodedFrames = (media as HTMLVideoElement & { webkitDecodedFrameCount?: number })
        .webkitDecodedFrameCount;
    return {
        videoWidth: media.videoWidth,
        videoHeight: media.videoHeight,
        totalVideoFrames: quality?.totalVideoFrames ?? decodedFrames ?? null,
        droppedVideoFrames: quality?.droppedVideoFrames ?? null
    };
}
