/** Minimum folder metadata needed for presentation and destination filtering. */
export interface FolderPresentation {
    id: string;
    parent_id: string | null;
    color?: string | null;
    inherits_color?: boolean;
}

/** Resolves live ancestor colors, protecting against malformed hierarchy cycles. */
export function resolveFolderColor(
    identifier: string,
    folders: FolderPresentation[]
): string | undefined {
    const visited = new Set<string>();
    let current = folders.find(folder => folder.id === identifier);
    while (current && !visited.has(current.id)) {
        visited.add(current.id);
        if (!current.inherits_color) return current.color || undefined;
        current = folders.find(folder => folder.id === current?.parent_id);
    }
    return undefined;
}

/** Tests ancestry without relying on platform-specific path separators. */
export function isFolderDescendant(
    identifier: string,
    ancestor: string,
    folders: FolderPresentation[]
): boolean {
    const visited = new Set<string>();
    let current = folders.find(folder => folder.id === identifier);
    while (current && !visited.has(current.id)) {
        if (current.id === ancestor) return true;
        visited.add(current.id);
        current = folders.find(folder => folder.id === current?.parent_id);
    }
    return false;
}
