import { createEffect, on, onCleanup, Accessor, Setter } from 'solid-js';
import { isHlsUrl } from '../../../lib/stream-utils';
import { HlsPlayerManager } from '../../../lib/hls-player';

interface UseHlsAttachmentProps {
    sourceUrlAccessor: Accessor<string>;
    videoElementAccessor: Accessor<HTMLVideoElement | undefined>;
    setErrorState: Setter<string | null>;
}

/**
 * Custom hook to handle HLS (HTTP Live Streaming) integration using `hls.js`.
 * Seamlessly checks for native support or attaches `HlsPlayerManager` for playback.
 *
 * @param props - Object containing accessors for URL, video element, and an error setter
 *
 * @example
 * ```tsx
 * useHlsAttachment({ sourceUrlAccessor, videoElementAccessor, setErrorState });
 * ```
 */
export function useHlsAttachment(props: UseHlsAttachmentProps) {
    let hlsPlayerManagerInstance: HlsPlayerManager | null = null;

    createEffect(
        on(
            [() => props.sourceUrlAccessor(), () => props.videoElementAccessor()],
            ([sourceUrl, videoElement]) => {
                if (hlsPlayerManagerInstance) {
                    hlsPlayerManagerInstance.destroy();
                    hlsPlayerManagerInstance = null;
                }

                if (!videoElement || !sourceUrl) {
                    return;
                }

                if (isHlsUrl(sourceUrl)) {
                    if (
                        HlsPlayerManager.isSupported() ||
                        videoElement.canPlayType('application/vnd.apple.mpegurl')
                    ) {
                        hlsPlayerManagerInstance = new HlsPlayerManager({ debug: false });
                        hlsPlayerManagerInstance.attach(videoElement, sourceUrl);
                    } else {
                        props.setErrorState('HLS playback not supported in this browser');
                    }
                }
            }
        )
    );

    onCleanup(() => {
        if (hlsPlayerManagerInstance) {
            hlsPlayerManagerInstance.destroy();
            hlsPlayerManagerInstance = null;
        }
    });
}
