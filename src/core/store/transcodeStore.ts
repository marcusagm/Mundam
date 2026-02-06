/**
 * Transcoding Store
 * Manages transcoding quality preferences with localStorage persistence
 */

import { createSignal } from 'solid-js';
import { type TranscodeQuality } from '../../lib/stream-utils';

const STORAGE_KEY = 'mundam-transcode-quality';

// Load initial value from localStorage
function loadInitialQuality(): TranscodeQuality {
    try {
        const stored = localStorage.getItem(STORAGE_KEY);
        if (stored && ['preview', 'standard', 'high'].includes(stored)) {
            return stored as TranscodeQuality;
        }
    } catch {
        // localStorage not available
    }
    return 'preview'; // Default to preview for faster, lighter transcoding
}

const [quality, setQualityInternal] = createSignal<TranscodeQuality>(loadInitialQuality());

function setQuality(newQuality: TranscodeQuality) {
    setQualityInternal(newQuality);
    try {
        localStorage.setItem(STORAGE_KEY, newQuality);
    } catch {
        // localStorage not available
    }
}

export const transcodeState = {
    quality,
};

export const transcodeActions = {
    setQuality,
};
