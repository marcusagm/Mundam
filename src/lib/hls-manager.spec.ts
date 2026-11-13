import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import Hls from 'hls.js';
import { HlsPlayerManager } from './hls-manager';
import { LifecycleManager } from '../core/utils/LifecycleManager';
import { setPlatformForTesting } from '../core/input/utils/platform';

vi.mock('../core/utils/LifecycleManager', () => ({
    LifecycleManager: { logTelemetry: vi.fn().mockResolvedValue(undefined) }
}));
vi.mock('hls.js', () => ({
    default: class {
        static isSupported = () => true;
        static Events = {
            MEDIA_ATTACHED: 'attached',
            ERROR: 'error',
            MANIFEST_PARSED: 'manifest'
        };
        static ErrorTypes = { NETWORK_ERROR: 'network', MEDIA_ERROR: 'media' };
        listeners = new Map<string, (event: string, data: unknown) => void>();
        on(event: string, callback: (event: string, data: unknown) => void) {
            this.listeners.set(event, callback);
        }
        attachMedia() {
            this.emit('attached', {});
        }
        emit(event: string, data: unknown) {
            this.listeners.get(event)?.(event, data);
        }
        loadSource = vi.fn();
        startLoad = vi.fn();
        detachMedia = vi.fn();
        destroy = vi.fn();
    }
}));

beforeEach(() => {
    vi.clearAllMocks();
    setPlatformForTesting('windows');
    vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(() => {});
});
afterEach(() => {
    vi.restoreAllMocks();
    setPlatformForTesting(null);
});

describe('HLS diagnostics and lifecycle', () => {
    it('prefers MediaSource on Windows even when WebView2 advertises native HLS', () => {
        const manager = new HlsPlayerManager();
        const media = document.createElement('video');
        vi.spyOn(media, 'canPlayType').mockReturnValue('probably');
        manager.attach(media, 'http://localhost/playlist.m3u8');
        expect(manager.getHls()).not.toBeNull();
        expect(media.getAttribute('src')).toBeNull();
        manager.destroy();
    });

    it('preserves native HLS on macOS', () => {
        setPlatformForTesting('mac');
        const manager = new HlsPlayerManager();
        const media = document.createElement('video');
        vi.spyOn(media, 'canPlayType').mockReturnValue('probably');
        manager.attach(media, 'http://localhost/playlist.m3u8');
        expect(manager.getHls()).toBeNull();
        expect(media.src).toBe('http://localhost/playlist.m3u8');
        manager.destroy();
    });

    it('falls back to native HLS on Windows without MediaSource support', () => {
        vi.spyOn(Hls, 'isSupported').mockReturnValue(false);
        const manager = new HlsPlayerManager();
        const media = document.createElement('audio');
        vi.spyOn(media, 'canPlayType').mockReturnValue('probably');
        manager.attach(media, 'http://localhost/playlist.m3u8');
        expect(manager.getHls()).toBeNull();
        expect(media.src).toBe('http://localhost/playlist.m3u8');
        manager.destroy();
    });
    it('loads after attachment and reports errors without session URLs or raw error messages', () => {
        const manager = new HlsPlayerManager();
        const media = document.createElement('audio');
        manager.attach(media, 'http://127.0.0.1/playlist.m3u8?token=private-token');
        const instance = manager.getHls()!;
        expect(instance.loadSource).toHaveBeenCalledOnce();
        const emitter = instance as unknown as { emit: (event: string, data: unknown) => void };
        emitter.emit(Hls.Events.ERROR, {
            type: 'network',
            details: 'manifestLoadError',
            fatal: false,
            response: { code: 500, url: 'http://localhost/?token=private-token' },
            error: new Error('http://localhost/?token=private-token'),
            frag: { sn: 4 }
        });
        const messages = JSON.stringify(vi.mocked(LifecycleManager.logTelemetry).mock.calls);
        expect(messages).toContain('manifestLoadError');
        expect(messages).toContain('500');
        expect(messages).not.toContain('private-token');
        manager.destroy();
    });

    it('destroys the previous HLS instance and removes listeners when reattaching', () => {
        const manager = new HlsPlayerManager();
        const previousMedia = document.createElement('video');
        manager.attach(previousMedia, 'http://localhost/first.m3u8');
        const previousInstance = manager.getHls()!;
        manager.attach(document.createElement('audio'), 'http://localhost/second.m3u8');
        expect(previousInstance.destroy).toHaveBeenCalledOnce();
        vi.mocked(LifecycleManager.logTelemetry).mockClear();
        previousMedia.dispatchEvent(new Event('error'));
        expect(LifecycleManager.logTelemetry).not.toHaveBeenCalled();
        manager.destroy();
    });
});
