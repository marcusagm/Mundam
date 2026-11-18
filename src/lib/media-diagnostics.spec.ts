import { describe, expect, it, vi } from 'vitest';
import { observeMediaDiagnostics } from './media-diagnostics';
import { LifecycleManager } from '../core/utils/LifecycleManager';

vi.mock('../core/utils/LifecycleManager', () => ({
    LifecycleManager: { logTelemetry: vi.fn().mockResolvedValue(undefined) }
}));

describe('media diagnostics', () => {
    it('reports decoded frame state without the source URL and removes listeners', () => {
        const media = document.createElement('video');
        Object.defineProperty(media, 'currentSrc', {
            value: 'http://localhost/private?token=secret'
        });
        Object.defineProperty(media, 'videoWidth', { value: 1280 });
        Object.defineProperty(media, 'videoHeight', { value: 720 });
        const dispose = observeMediaDiagnostics(media);
        media.dispatchEvent(new Event('loadeddata'));
        const loggedMessages = vi.mocked(LifecycleManager.logTelemetry).mock.calls;
        const payload = loggedMessages[loggedMessages.length - 1][2];
        expect(JSON.parse(payload)).toMatchObject({
            videoWidth: 1280,
            videoHeight: 720,
            transport: 'http'
        });
        expect(payload).not.toContain('secret');
        expect(payload).not.toContain('private');
        const calls = vi.mocked(LifecycleManager.logTelemetry).mock.calls.length;
        dispose();
        media.dispatchEvent(new Event('error'));
        expect(LifecycleManager.logTelemetry).toHaveBeenCalledTimes(calls);
    });
});
