import { createSignal } from 'solid-js';
import { cleanup, render } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { VideoPlayer } from './VideoPlayer';

const [loading, setLoading] = createSignal(true);
const [probeError, setProbeError] = createSignal<string | null>(null);
vi.mock('../../../../../core/hooks/useVideoSource', () => ({
    useVideoSource: () => ({
        videoUrl: () => 'http://asset.localhost/native.mp4',
        probeError,
        probeResult: Object.defineProperty(() => undefined, 'loading', { get: () => loading() })
    })
}));
vi.mock('../../../../ui', () => ({
    VideoPlayer: (props: { src: string }) => <video src={props.src} />
}));
vi.mock('../../../../../core/store/transcodeStore', () => ({
    transcodeState: { quality: () => 'standard' }
}));

afterEach(() => {
    cleanup();
    setLoading(true);
    setProbeError(null);
});

describe('video preview availability', () => {
    it('shows an available source without waiting for a slow or failed metadata probe', () => {
        const onReady = vi.fn();
        const view = render(() => (
            <VideoPlayer assetId="asset" path="movie.mp4" onReady={onReady} />
        ));
        const player = view.container.querySelector('video');
        expect(player).not.toBeNull();
        expect(onReady).toHaveBeenCalled();
        expect(view.queryByText('Analyzing video...')).toBeNull();

        setProbeError('Probe timed out');
        setLoading(false);
        expect(view.container.querySelector('video')).toBe(player);
        expect(view.queryByText('Failed to analyze video')).toBeNull();
    });
});
