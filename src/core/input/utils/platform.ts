import type { ModifierKey, Platform } from '../types';

let cachedPlatform: Platform | null = null;

/**
 * Detects the current operating system platform.
 *
 * @returns The detected platform ('mac', 'windows', or 'linux').
 */
export function detectPlatform(): Platform {
    if (cachedPlatform) return cachedPlatform;

    if (typeof navigator === 'undefined') {
        cachedPlatform = 'windows';
        return cachedPlatform;
    }

    const userAgent = navigator.userAgent.toLowerCase();

    if (userAgent.includes('mac')) {
        cachedPlatform = 'mac';
    } else if (userAgent.includes('linux')) {
        cachedPlatform = 'linux';
    } else {
        cachedPlatform = 'windows';
    }

    return cachedPlatform;
}

/**
 * Checks if the current platform is macOS.
 */
export function isMac(): boolean {
    return detectPlatform() === 'mac';
}

/**
 * Resolves the primary modifier key for the given or current platform.
 * macOS uses 'Meta' (Command), while Windows and Linux use 'Ctrl'.
 *
 * @param platform - Optional explicit platform to evaluate.
 * @returns The primary modifier key ('Meta' or 'Ctrl').
 */
export function getPrimaryModifier(platform?: Platform): ModifierKey {
    const plat = platform ?? detectPlatform();
    return plat === 'mac' ? 'Meta' : 'Ctrl';
}

/**
 * Overrides the cached platform detection for testing purposes.
 * Pass null to restore default detection.
 *
 * @param platform - Platform override, or null to reset.
 */
export function setPlatformForTesting(platform: Platform | null): void {
    cachedPlatform = platform;
}
