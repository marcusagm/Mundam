import { createSignal } from 'solid-js';

type MotionPreference = 'system' | 'reduce' | 'full';
const storageKey = 'mundam-motion-preference';
const [motionPreference, setMotionPreference] = createSignal<MotionPreference>('system');
export { motionPreference };

function parsePreference(value: string | null): MotionPreference {
    return value === 'reduce' || value === 'full' ? value : 'system';
}

/** Persists the user's choice for all windows on this device. */
export function saveMotionPreference(value: string): void {
    const preference = parsePreference(value);
    localStorage.setItem(storageKey, preference);
    setMotionPreference(preference);
    applyMotionPreference();
}

function applyMotionPreference(): void {
    const preference = motionPreference();
    const reduced =
        preference === 'reduce' ||
        (preference === 'system' && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
    document.documentElement.dataset.reducedMotion = String(reduced);
}

/** Initializes motion before rendering and follows OS and sibling-window changes. */
export function initializeMotionPreference(): void {
    setMotionPreference(parsePreference(localStorage.getItem(storageKey)));
    applyMotionPreference();
    window
        .matchMedia('(prefers-reduced-motion: reduce)')
        .addEventListener('change', applyMotionPreference);
    window.addEventListener('storage', event => {
        if (event.key === storageKey || event.key === null) {
            setMotionPreference(parsePreference(localStorage.getItem(storageKey)));
            applyMotionPreference();
        }
    });
}
