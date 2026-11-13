import { createSignal } from 'solid-js';
import { cleanup, fireEvent, render } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { VideoPlayer } from './index';

vi.mock('../../../core/input', () => ({
    createFocusScope: () => ({ whenFocused: (callback: unknown) => callback }),
    useCommands: () => {}
}));
vi.mock('./useFullscreen', () => ({
    useFullscreen: () => ({ isFullscreen: () => false, toggleFullscreen: () => {} })
}));
vi.mock('./VideoControls', () => ({ VideoControls: () => null }));
vi.mock('../../../lib/stream-utils', () => ({
    isHlsUrl: (source: string) => source.includes('.m3u8')
}));
vi.mock('../../../lib/hls-player', () => ({
    HlsPlayerManager: class {
        static isSupported() {
            return true;
        }
        attach(media: HTMLMediaElement) {
            // Model the MediaSource URL owned by hls.js on Windows.
            media.src = 'blob:http://localhost/hls-media-source';
        }
        destroy() {}
    }
}));

afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.useRealTimers();
});

describe('video HLS source ownership', () => {
    it('delegates engine selection when the WebView advertises native HLS', () => {
        vi.spyOn(HTMLMediaElement.prototype, 'canPlayType').mockReturnValue('probably');
        const { container } = render(() => <VideoPlayer src="http://127.0.0.1:9876/first.m3u8" />);
        expect(container.querySelector('video')!.getAttribute('src')).toBe(
            'blob:http://localhost/hls-media-source'
        );
    });

    it('preserves the MediaSource URL when changing HLS assets, and restores native playback', () => {
        const [source, setSource] = createSignal('http://127.0.0.1:9876/first.m3u8');
        const { container } = render(() => <VideoPlayer src={source()} />);
        const video = container.querySelector('video')!;
        expect(video.getAttribute('src')).toBe('blob:http://localhost/hls-media-source');

        setSource('http://127.0.0.1:9876/second.m3u8');
        expect(video.getAttribute('src')).toBe('blob:http://localhost/hls-media-source');

        setSource('http://asset.localhost/native.mp4');
        expect(video.getAttribute('src')).toBe('http://asset.localhost/native.mp4');

        setSource('http://127.0.0.1:9876/third.m3u8');
        expect(video.getAttribute('src')).toBe('blob:http://localhost/hls-media-source');
    });

    it('leaves recovery to hls.js instead of resetting MediaSource with load()', () => {
        vi.useFakeTimers();
        const load = vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(() => {});
        const { container } = render(() => <VideoPlayer src="http://127.0.0.1:9876/first.m3u8" />);
        fireEvent.error(container.querySelector('video')!);
        vi.advanceTimersByTime(3100);
        expect(load).not.toHaveBeenCalled();
        expect(container.querySelector('.ui-video-transcoding')).toBeNull();
    });
});
