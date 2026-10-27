import { detectPlatform } from '../input/utils/platform';

export type AssetRequestType = 'thumb' | 'preview' | 'glb';

/**
 * Returns the base URI for custom protocol requests based on operating system.
 * - macOS / Linux: <protocol>://localhost
 * - Windows / Android: http://<protocol>.localhost
 */
export function getProtocolBase(protocol: string = 'asset'): string {
    const isWindows = detectPlatform() === 'windows';
    return isWindows ? `http://${protocol}.localhost` : `${protocol}://localhost`;
}

/**
 * Resolves an asset URL for a given asset ID and optional request type (thumb, preview, glb).
 * Cross-platform compatible for macOS, Linux, and Windows WebView2.
 */
export function getAssetUrl(assetId?: string | null, type?: AssetRequestType | string): string {
    if (!assetId) return '';
    const base = getProtocolBase('asset');
    if (!type) {
        return `${base}/${assetId}`;
    }
    return `${base}/${assetId}?type=${type}`;
}
