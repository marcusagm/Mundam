import { describe, it, expect, vi, beforeEach } from 'vitest';
import { getAssetUrl, getProtocolBase } from './assetUrl';
import * as platformUtil from '../input/utils/platform';

describe('assetUrl utility', () => {
    beforeEach(() => {
        vi.restoreAllMocks();
    });

    it('generates Windows WebView2 URL when platform is windows', () => {
        vi.spyOn(platformUtil, 'detectPlatform').mockReturnValue('windows');

        expect(getProtocolBase('asset')).toBe('http://asset.localhost');
        expect(getAssetUrl('12345')).toBe('http://asset.localhost/12345');
        expect(getAssetUrl('12345', 'thumb')).toBe('http://asset.localhost/12345?type=thumb');
        expect(getAssetUrl('12345', 'preview')).toBe('http://asset.localhost/12345?type=preview');
        expect(getAssetUrl('12345', 'glb')).toBe('http://asset.localhost/12345?type=glb');
    });

    it('generates macOS/Linux URL when platform is mac', () => {
        vi.spyOn(platformUtil, 'detectPlatform').mockReturnValue('mac');

        expect(getProtocolBase('asset')).toBe('asset://localhost');
        expect(getAssetUrl('12345')).toBe('asset://localhost/12345');
        expect(getAssetUrl('12345', 'thumb')).toBe('asset://localhost/12345?type=thumb');
        expect(getAssetUrl('12345', 'preview')).toBe('asset://localhost/12345?type=preview');
        expect(getAssetUrl('12345', 'glb')).toBe('asset://localhost/12345?type=glb');
    });

    it('handles empty or null assetId gracefully', () => {
        expect(getAssetUrl('')).toBe('');
        expect(getAssetUrl(null)).toBe('');
        expect(getAssetUrl(undefined)).toBe('');
    });
});
