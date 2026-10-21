/* eslint-disable max-lines */
import type { ShortcutDefinition } from '../types';

/**
 * Behaviour traits shared by every shortcut of a scope. Keeping them here lets the
 * dispatcher stay scope-agnostic while widgets inside each scope keep native key handling.
 */
const SCOPE_TRAITS: Record<string, Partial<ShortcutDefinition>> = {
    viewport: { yieldToInteractive: true },
    'image-viewer': { yieldToNavigationWidgets: true },
    'duplicate-finder': { yieldToNavigationWidgets: true },
    'video-player': { yieldToNavigationWidgets: true },
    'audio-player': { yieldToNavigationWidgets: true }
};

/** True when the focused element itself carries the given data attribute. */
function isFocusedElementMarked(dataAttribute: string): boolean {
    return typeof document !== 'undefined' && !!document.activeElement?.hasAttribute(dataAttribute);
}

/**
 * Default shortcut definitions for the application.
 */
const BASE_SHORTCUTS: ShortcutDefinition[] = [
    // Global scope
    {
        name: 'Focus Search',
        description: 'Focus the search input',
        keys: 'Meta+KeyK',
        scope: 'global',
        command: 'app:focus-search',
        category: 'Navigation',
        ignoreInputs: false,
        preventDefault: true
    },
    {
        name: 'Settings',
        description: 'Open application settings',
        keys: 'Meta+Comma',
        scope: 'global',
        command: 'app:settings',
        category: 'Application'
    },
    {
        name: 'Toggle Sidebars',
        description: 'Hide or show the library sidebar and inspector',
        keys: 'Meta+KeyB',
        scope: 'global',
        command: 'app:toggle-sidebars',
        category: 'Navigation'
    },
    {
        name: 'Mouse Back',
        description: 'Go back in filter history with the mouse back button',
        keys: 'BackClick',
        scope: 'global',
        command: 'app:history-back',
        category: 'Navigation',
        ignoreInputs: false,
        system: true
    },
    {
        name: 'Mouse Forward',
        description: 'Go forward in filter history with the mouse forward button',
        keys: 'ForwardClick',
        scope: 'global',
        command: 'app:history-forward',
        category: 'Navigation',
        ignoreInputs: false,
        system: true
    },

    // Viewport Interaction
    {
        name: 'Move Up',
        description: 'Navigate up in grid/list',
        keys: 'ArrowUp',
        scope: 'viewport',
        command: 'viewport:move-up',
        category: 'Navigation'
    },
    {
        name: 'Move Down',
        description: 'Navigate down in grid/list',
        keys: 'ArrowDown',
        scope: 'viewport',
        command: 'viewport:move-down',
        category: 'Navigation'
    },
    {
        name: 'Move Left',
        description: 'Navigate left in grid',
        keys: 'ArrowLeft',
        scope: 'viewport',
        command: 'viewport:move-left',
        category: 'Navigation'
    },
    {
        name: 'Move Right',
        description: 'Navigate right in grid',
        keys: 'ArrowRight',
        scope: 'viewport',
        command: 'viewport:move-right',
        category: 'Navigation'
    },
    {
        name: 'Go to Start',
        description: 'Navigate to first item',
        keys: 'Home',
        scope: 'viewport',
        command: 'viewport:home',
        category: 'Navigation'
    },
    {
        name: 'Go to End',
        description: 'Navigate to last item',
        keys: 'End',
        scope: 'viewport',
        command: 'viewport:end',
        category: 'Navigation'
    },
    {
        name: 'Toggle Selection',
        description: 'Select/deselect focused item',
        keys: 'Space',
        scope: 'viewport',
        command: 'viewport:toggle-select',
        category: 'Selection'
    },
    {
        name: 'Open Item',
        description: 'Open focused item',
        keys: 'Enter',
        scope: 'viewport',
        command: 'viewport:open',
        category: 'Navigation'
    },
    {
        name: 'Add to Selection',
        description: 'Add item to current selection',
        keys: 'Shift+Space',
        scope: 'viewport',
        command: 'viewport:select-add',
        category: 'Selection',
        ignoreInputs: true
    },
    {
        name: 'Deselect All',
        description: 'Clear the current selection',
        keys: 'Escape',
        scope: 'viewport',
        command: 'viewport:deselect-all',
        category: 'Selection',
        ignoreInputs: true
    },
    {
        name: 'Select All Items',
        description: 'Select all visible items',
        keys: 'Meta+KeyA',
        scope: 'viewport',
        command: 'viewport:select-all',
        category: 'Selection',
        ignoreInputs: true
    },
    {
        name: 'Move to Trash',
        description: 'Move selected or focused items to the trash',
        keys: 'Delete',
        scope: 'viewport',
        command: 'viewport:move-to-trash',
        category: 'Selection',
        ignoreInputs: true
    },

    // Image Viewer scope
    {
        name: 'Close Viewer',
        description: 'Close the image viewer',
        keys: 'Escape',
        scope: 'image-viewer',
        command: 'viewer:close',
        category: 'Viewer',
        ignoreInputs: false
    },
    {
        name: 'Zoom In',
        description: 'Increase zoom level',
        keys: 'Equal',
        scope: 'image-viewer',
        command: 'viewer:zoom-in',
        category: 'Viewer'
    },
    {
        name: 'Zoom Out',
        description: 'Decrease zoom level',
        keys: 'Minus',
        scope: 'image-viewer',
        command: 'viewer:zoom-out',
        category: 'Viewer'
    },
    {
        name: 'Fit to Screen',
        description: 'Fit image to screen',
        keys: 'Meta+Digit0',
        scope: 'image-viewer',
        command: 'viewer:fit-screen',
        category: 'Viewer'
    },
    {
        name: 'Original Size',
        description: 'Show image at 100% zoom',
        keys: 'Meta+Digit1',
        scope: 'image-viewer',
        command: 'viewer:original-size',
        category: 'Viewer'
    },
    {
        name: 'Pan Tool',
        description: 'Activate pan tool',
        keys: 'KeyH',
        scope: 'image-viewer',
        command: 'viewer:tool-pan',
        category: 'Viewer'
    },
    {
        name: 'Rotate Tool',
        description: 'Activate rotate tool',
        keys: 'KeyR',
        scope: 'image-viewer',
        command: 'viewer:tool-rotate',
        category: 'Viewer'
    },
    {
        name: 'Previous Item',
        description: 'Go to previous item',
        keys: 'ArrowLeft',
        scope: 'image-viewer',
        command: 'viewer:previous',
        category: 'Viewer'
    },
    {
        name: 'Next Item',
        description: 'Go to next item',
        keys: 'ArrowRight',
        scope: 'image-viewer',
        command: 'viewer:next',
        category: 'Viewer'
    },
    {
        name: 'Play/Pause Slideshow',
        description: 'Toggle slideshow playback',
        keys: 'Space',
        scope: 'image-viewer',
        command: 'viewer:slideshow-toggle',
        category: 'Viewer'
    },
    {
        name: 'Flip Horizontal',
        description: 'Flip image horizontally',
        keys: 'Shift+KeyH',
        scope: 'image-viewer',
        command: 'viewer:flip-h',
        category: 'Viewer'
    },
    {
        name: 'Flip Vertical',
        description: 'Flip image vertically',
        keys: 'Shift+KeyV',
        scope: 'image-viewer',
        command: 'viewer:flip-v',
        category: 'Viewer'
    },

    // Duplicate finder scope
    {
        name: 'Next Group',
        description: 'Select the next duplicate group',
        keys: 'ArrowDown',
        scope: 'duplicate-finder',
        command: 'duplicates:next-group',
        category: 'Duplicates'
    },
    {
        name: 'Previous Group',
        description: 'Select the previous duplicate group',
        keys: 'ArrowUp',
        scope: 'duplicate-finder',
        command: 'duplicates:previous-group',
        category: 'Duplicates'
    },
    {
        name: 'Next Candidate',
        description: 'Focus the next file of the selected group',
        keys: 'ArrowRight',
        scope: 'duplicate-finder',
        command: 'duplicates:next-candidate',
        category: 'Duplicates'
    },
    {
        name: 'Previous Candidate',
        description: 'Focus the previous file of the selected group',
        keys: 'ArrowLeft',
        scope: 'duplicate-finder',
        command: 'duplicates:previous-candidate',
        category: 'Duplicates'
    },
    {
        name: 'Toggle Selection',
        description: 'Select the focused group or toggle the focused file',
        keys: 'Space',
        scope: 'duplicate-finder',
        command: 'duplicates:toggle-selection',
        category: 'Duplicates',
        allowOnInteractive: true,
        enabledWhen: () => isFocusedElementMarked('data-duplicate-selectable')
    },
    {
        name: 'Ignore Group',
        description: 'Ignore the selected duplicate group',
        keys: 'Delete',
        scope: 'duplicate-finder',
        command: 'duplicates:ignore-group',
        category: 'Duplicates'
    },
    {
        name: 'Keep Selected',
        description: 'Keep the selected files, or the largest one when none is selected',
        keys: 'KeyK',
        scope: 'duplicate-finder',
        command: 'duplicates:keep-selected',
        category: 'Duplicates'
    },
    {
        name: 'Open Split View',
        description: 'Compare the files of the selected group side by side',
        keys: 'KeyS',
        scope: 'duplicate-finder',
        command: 'duplicates:open-split-view',
        category: 'Duplicates'
    },
    {
        name: 'Scan / Cancel Scan',
        description: 'Start a duplicate scan, or cancel the one in progress',
        keys: 'KeyR',
        scope: 'duplicate-finder',
        command: 'duplicates:toggle-scan',
        category: 'Duplicates'
    },
    {
        name: 'Configure Rules',
        description: 'Open the duplicate detection settings',
        keys: 'KeyC',
        scope: 'duplicate-finder',
        command: 'duplicates:open-rules',
        category: 'Duplicates'
    },

    // Video player scope
    {
        name: 'Play/Pause',
        description: 'Toggle video playback',
        keys: 'Space',
        scope: 'video-player',
        command: 'video:toggle-play',
        category: 'Player'
    },
    {
        name: 'Play/Pause (Alternative)',
        description: 'Toggle video playback',
        keys: 'KeyK',
        scope: 'video-player',
        command: 'video:toggle-play',
        category: 'Player'
    },
    {
        name: 'Seek Backward',
        description: 'Rewind 5 seconds',
        keys: 'ArrowLeft',
        scope: 'video-player',
        command: 'video:seek-backward',
        category: 'Player'
    },
    {
        name: 'Seek Backward (Alternative)',
        description: 'Rewind 5 seconds',
        keys: 'KeyJ',
        scope: 'video-player',
        command: 'video:seek-backward',
        category: 'Player'
    },
    {
        name: 'Seek Forward',
        description: 'Skip forward 5 seconds',
        keys: 'ArrowRight',
        scope: 'video-player',
        command: 'video:seek-forward',
        category: 'Player'
    },
    {
        name: 'Seek Forward (Alternative)',
        description: 'Skip forward 5 seconds',
        keys: 'KeyL',
        scope: 'video-player',
        command: 'video:seek-forward',
        category: 'Player'
    },
    {
        name: 'Volume Up',
        description: 'Increase the volume by 10%',
        keys: 'ArrowUp',
        scope: 'video-player',
        command: 'video:volume-up',
        category: 'Player'
    },
    {
        name: 'Volume Down',
        description: 'Decrease the volume by 10%',
        keys: 'ArrowDown',
        scope: 'video-player',
        command: 'video:volume-down',
        category: 'Player'
    },
    {
        name: 'Toggle Fullscreen',
        description: 'Enter or leave fullscreen',
        keys: 'KeyF',
        scope: 'video-player',
        command: 'video:toggle-fullscreen',
        category: 'Player'
    },
    {
        name: 'Toggle Mute',
        description: 'Mute or unmute the video',
        keys: 'KeyM',
        scope: 'video-player',
        command: 'video:toggle-mute',
        category: 'Player'
    },

    // Audio player scope
    {
        name: 'Play/Pause',
        description: 'Toggle audio playback',
        keys: 'Space',
        scope: 'audio-player',
        command: 'audio:toggle-play',
        category: 'Player'
    },
    {
        name: 'Seek Backward',
        description: 'Rewind 5 seconds',
        keys: 'ArrowLeft',
        scope: 'audio-player',
        command: 'audio:seek-backward',
        category: 'Player'
    },
    {
        name: 'Seek Forward',
        description: 'Skip forward 5 seconds',
        keys: 'ArrowRight',
        scope: 'audio-player',
        command: 'audio:seek-forward',
        category: 'Player'
    },
    {
        name: 'Toggle Mute',
        description: 'Mute or unmute the audio',
        keys: 'KeyM',
        scope: 'audio-player',
        command: 'audio:toggle-mute',
        category: 'Player'
    },

    // Search and modal scopes
    {
        name: 'Close Search',
        description: 'Clear the search query or blur the search input',
        keys: 'Escape',
        scope: 'search',
        command: 'search:close',
        category: 'Search',
        ignoreInputs: false
    },
    {
        name: 'Close Modal',
        description: 'Close the active modal',
        keys: 'Escape',
        scope: 'modal',
        command: 'modal:close',
        category: 'Modal',
        ignoreInputs: false
    }
];

export const DEFAULT_SHORTCUTS: ShortcutDefinition[] = BASE_SHORTCUTS.map(definition => ({
    ...SCOPE_TRAITS[definition.scope ?? 'global'],
    ...definition
}));
