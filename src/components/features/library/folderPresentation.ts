import type { FolderHierarchyEntry } from './folderHierarchy';
/** Minimum folder metadata needed for presentation and destination filtering. */
export interface FolderPresentation extends FolderHierarchyEntry {
    /** Explicit custom color, retained while inheritance is enabled. */
    color?: string | null;
    /** Whether presentation resolves the ancestor color. */
    inherits_color?: boolean;
}

/** Resolves ancestor colors while protecting against malformed hierarchy cycles.
 * @param identifier - Folder whose icon is rendered.
 * @param folders - Metadata or a cached index for the current revision.
 * @returns Effective color or undefined for the system default.
 * @example resolveFolderColor('folder', []);
 */
export function resolveFolderColor(
    identifier: string,
    folders: FolderPresentation[] | ReadonlyMap<string, FolderPresentation>
): string | undefined {
    const index = Array.isArray(folders)
        ? new Map(folders.map(folder => [folder.id, folder]))
        : folders;
    const visited = new Set<string>();
    let current = index.get(identifier);
    while (current && !visited.has(current.id)) {
        visited.add(current.id);
        if (!current.inherits_color) return current.color || undefined;
        current = current.parent_id ? index.get(current.parent_id) : undefined;
    }
    return undefined;
}
