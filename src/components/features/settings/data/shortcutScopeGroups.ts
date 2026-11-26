import type { RegisteredShortcut } from '../../../../core/input/types';

export type ScopeGroup = {
    scope: string;
    label: string;
    shortcuts: RegisteredShortcut[];
};

const SCOPE_LABELS: Record<string, string> = {
    global: 'Global',
    'plugin-commands': 'Plugin commands',
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
    'plugin-commands',
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
    const seenShortcutKeys = new Set<string>();

    for (const shortcut of shortcuts) {
        const scope = shortcut.scope || 'global';
        const uniquenessKey = shortcut.command
            ? `${shortcut.command}::${scope}`
            : `${shortcut.name}::${scope}`;

        if (seenShortcutKeys.has(uniquenessKey)) {
            continue;
        }
        seenShortcutKeys.add(uniquenessKey);

        const displayGroup = shortcut.category === 'Plugins' ? 'plugin-commands' : scope;
        if (!groups.has(displayGroup)) {
            groups.set(displayGroup, []);
        }
        groups.get(displayGroup)!.push(shortcut);
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
