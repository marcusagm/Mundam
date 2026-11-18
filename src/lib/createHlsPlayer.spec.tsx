import { cleanup, render } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createHlsPlayer } from './createHlsPlayer';

vi.mock('../core/utils/LifecycleManager', () => ({
    LifecycleManager: { logTelemetry: vi.fn().mockResolvedValue(undefined) }
}));
afterEach(cleanup);

describe('native HTTP playback', () => {
    it.each(['audio', 'video'] as const)(
        'loads a native %s response without parsing an HLS manifest',
        kind => {
            const media = document.createElement(kind);
            const source = 'http://127.0.0.1:9876/stream/native-id?token=session';
            let player!: ReturnType<typeof createHlsPlayer>;
            render(() => {
                player = createHlsPlayer(
                    () => media,
                    () => source
                );
                return null;
            });
            expect(player.isHlsActive()).toBe(false);
            expect(player.getManager()).toBeNull();
            expect(media.src).toBe(source);
            expect(player.isLoading()).toBe(true);
            media.dispatchEvent(new Event('loadeddata'));
            expect(player.isLoading()).toBe(false);
            expect(player.hasError()).toBe(false);
        }
    );
});
