import { cleanup, renderHook } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { LifecycleManager } from '../utils/LifecycleManager';
import { useShutdownMedia } from './useShutdownMedia';

vi.mock('../utils/LifecycleManager', () => ({
    LifecycleManager: {
        registerListener: vi.fn(),
        logTelemetry: vi.fn().mockResolvedValue(undefined)
    }
}));

afterEach(() => {
    cleanup();
    document.body.replaceChildren();
    vi.restoreAllMocks();
    vi.clearAllMocks();
});

describe('media during shutdown', () => {
    it('pauses current audio/video and prevents playback while draining', () => {
        const pause = vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {});
        const audio = document.createElement('audio');
        const video = document.createElement('video');
        document.body.append(audio, video);
        renderHook(useShutdownMedia);
        audio.dispatchEvent(new Event('play'));
        expect(pause).not.toHaveBeenCalled();

        const [event, receive] = vi.mocked(LifecycleManager.registerListener).mock.calls[0];
        expect(event).toBe('shutdown:requested');
        receive(undefined);
        expect(pause).toHaveBeenCalledTimes(2);
        expect(audio.muted).toBe(true);
        expect(video.autoplay).toBe(false);

        const lateAudio = document.createElement('audio');
        document.body.append(lateAudio);
        lateAudio.dispatchEvent(new Event('play'));
        expect(pause).toHaveBeenCalledTimes(3);
        expect(lateAudio.muted).toBe(true);

        cleanup();
        lateAudio.dispatchEvent(new Event('play'));
        expect(pause).toHaveBeenCalledTimes(3);
    });
});
