import { expect, it, vi } from 'vitest';
import {
    initializeMotionPreference,
    motionPreference,
    saveMotionPreference
} from './motionPreference';

it('follows system changes, honors explicit overrides and reads preferences from other windows', () => {
    const media = Object.assign(new EventTarget(), { matches: true });
    vi.stubGlobal('matchMedia', () => media);
    try {
        initializeMotionPreference();
        const effective = () => document.documentElement.dataset.reducedMotion;
        expect(effective()).toBe('true');
        saveMotionPreference('full');
        media.dispatchEvent(new Event('change'));
        expect(effective()).toBe('false');
        media.matches = false;
        saveMotionPreference('reduce');
        expect(effective()).toBe('true');
        saveMotionPreference('system');
        expect(effective()).toBe('false');
        media.matches = true;
        media.dispatchEvent(new Event('change'));
        expect(effective()).toBe('true');
        localStorage.setItem('mundam-motion-preference', 'full');
        window.dispatchEvent(new StorageEvent('storage', { key: 'mundam-motion-preference' }));
        expect(motionPreference()).toBe('full');
        expect(effective()).toBe('false');
    } finally {
        localStorage.removeItem('mundam-motion-preference');
        vi.unstubAllGlobals();
    }
});
