import type { RegisteredShortcut } from '../../../../core/input/types';

export type ScopeGroup = {
    scope: string;
    label: string;
    shortcuts: RegisteredShortcut[];
};

const SCOPE_LABELS: Record<string, string> = {
    global: 'Global',
    'image-viewer': 'Image Viewer',
    viewport: 'Viewport',
    'duplicate-finder': 'Duplicate Finder',
    'video-player': 'Video Player',
    'audio-player': 'Audio Player',
    search: 'Search',
    modal: 'Modal'
};

const SCOPE_ORDER = [
    'global',
    'viewport',
    'image-viewer',
    'duplicate-finder',
    'video-player',
    'audio-player',
    'search',
    'modal'
];

export function groupShortcutsByScope(
    shortcuts: RegisteredShortcut[]
): Map<string, RegisteredShortcut[]> {
    const groups = new Map<string, RegisteredShortcut[]>();
    for (const shortcut of shortcuts) {
        const scope = shortcut.scope || 'global';
        if (!groups.has(scope)) {
            groups.set(scope, []);
        }
        groups.get(scope)!.push(shortcut);
    }
    return groups;
}

function toScopeGroup(scope: string, shortcuts: RegisteredShortcut[]): ScopeGroup {
    return {
        scope,
        label: SCOPE_LABELS[scope] || scope,
        shortcuts: shortcuts.sort((a, b) => a.name.localeCompare(b.name))
    };
}

export function buildOrderedScopeGroups(groups: Map<string, RegisteredShortcut[]>): ScopeGroup[] {
    const result: ScopeGroup[] = [];

    for (const scope of SCOPE_ORDER) {
        const scopedShortcuts = groups.get(scope);
        if (scopedShortcuts && scopedShortcuts.length > 0) {
            result.push(toScopeGroup(scope, scopedShortcuts));
        }
    }

    for (const [scope, scopedShortcuts] of groups) {
        if (!SCOPE_ORDER.includes(scope) && scopedShortcuts.length > 0) {
            result.push(toScopeGroup(scope, scopedShortcuts));
        }
    }

    return result;
}
