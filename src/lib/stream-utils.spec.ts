import { afterEach, beforeEach, describe, it, expect, vi } from 'vitest';
import { getVideoUrl, getAudioUrl } from './stream-utils';
import { getAssetUrl } from '../core/utils/assetUrl';
import { setPlatformForTesting } from '../core/input/utils/platform';
import { initStreamingToken, isHlsUrl } from './hls-player';

vi.mock('./api', () => ({ invokeCommand: vi.fn().mockResolvedValue('test-session-token') }));
afterEach(() => setPlatformForTesting(null));
vi.mock('../core/store/formatStore', () => {
    const strategyMap: Record<string, string> = {
        mp4: 'native',
        mov: 'native',
        mkv: 'hls',
        avi: 'hls',
        swf: 'linearHls',
        mpg: 'linearHls',
        mp3: 'native',
        wav: 'native',
        flac: 'audioTranscode',
        ogg: 'audioTranscode'
    };

    return {
        formatActions: {
            getPlaybackStrategy: vi.fn((ext: string) => strategyMap[ext.toLowerCase()] || 'native'),
            getMediaType: vi.fn()
        }
    };
});

describe('stream-utils URL generation', () => {
    beforeEach(() => setPlatformForTesting('windows'));
    describe('getVideoUrl', () => {
        it('should return asset:// URL for native MP4 videos', () => {
            const path = '/Movies/test_video.mp4';
            const assetId = '12345';
            const url = getVideoUrl(assetId, path, 'standard');
            expect(url).toBe(getAssetUrl(assetId));
        });

        it('should return HLS URL for standard transcoding videos (MKV)', () => {
            const path = '/Movies/test_video.mkv';
            const assetId = '12345';
            const url = getVideoUrl(assetId, path, 'standard');
            expect(url).toContain(`/playlist/${assetId}/playlist.m3u8`);
        });

        it('should return linear HLS URL for live transcoding videos (SWF)', () => {
            const path = '/Movies/test_video.swf';
            const assetId = '12345';
            const url = getVideoUrl(assetId, path, 'high');
            expect(url).toContain('/hls-live/');
            expect(url).toContain('mode=live');
            expect(url).toContain('quality=high');
        });
    });

    describe('getAudioUrl', () => {
        it('should return asset:// URL for native audio', () => {
            const path = '/Music/test_audio.mp3';
            const assetId = '12345';
            const url = getAudioUrl(assetId, path, 'standard');
            expect(url).toBe(getAssetUrl(assetId));
        });

        it('should return HLS URL for transcoded audio', () => {
            const path = '/Music/test_audio.flac';
            const assetId = '12345';
            const url = getAudioUrl(assetId, path, 'standard');
            expect(url).toContain(`/playlist/${assetId}/playlist.m3u8`);
            expect(url).toContain('quality=standard');
        });
    });
});

describe('native Linux media transport', () => {
    it('uses authenticated HTTP for native video and audio without changing HLS', async () => {
        setPlatformForTesting('linux');
        await initStreamingToken();
        for (const path of ['/Movies/sample.mp4', '/Movies/sample.mov']) {
            expect(isHlsUrl(getVideoUrl('native-id', path))).toBe(false);
            expect(getVideoUrl('native-id', path)).toBe(
                'http://127.0.0.1:9876/stream/native-id?token=test-session-token'
            );
        }
        for (const path of ['/Music/sample.mp3', '/Music/sample.wav']) {
            expect(isHlsUrl(getAudioUrl('native-id', path))).toBe(false);
            expect(getAudioUrl('native-id', path)).toBe(
                'http://127.0.0.1:9876/stream/native-id?token=test-session-token'
            );
        }
        expect(getVideoUrl('hls-id', '/Movies/sample.mkv')).toContain('/playlist/hls-id/');
    });

    it('preserves native transport on macOS and Windows', () => {
        for (const platform of ['mac', 'windows'] as const) {
            setPlatformForTesting(platform);
            expect(getVideoUrl('native-id', '/Movies/sample.mp4')).toBe(getAssetUrl('native-id'));
            expect(getAudioUrl('native-id', '/Music/sample.mp3')).toBe(getAssetUrl('native-id'));
        }
    });
});

describe('HLS resource classification', () => {
    it.each([
        ['http://127.0.0.1:9876/stream/native-id?token=session.m3u8', false],
        ['http://127.0.0.1:9876/stream/native-id', false],
        ['http://127.0.0.1:9876/probe/native-id?_=1', false],
        ['http://127.0.0.1:9876/health', false],
        ['asset://localhost/native-id', false],
        ['blob:http://localhost/media-source', false],
        ['http://127.0.0.1:9876/playlist/id/playlist.m3u8?quality=standard&token=session', true],
        ['http://127.0.0.1:9876/hls-live/id/index.m3u8?mode=live', true],
        ['https://example.test/video.M3U8#position', true],
        ['/playlist/id/playlist.m3u8?token=session', true]
    ])('classifies the requested resource %s', (source, expected) => {
        expect(isHlsUrl(source)).toBe(expected);
    });
});
