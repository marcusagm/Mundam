import type { TreeNode } from '../../ui/TreeView';

/** Minimal hierarchy contract, independent of presentation or platform paths. */
export interface FolderHierarchyEntry {
    /** Stable catalog identity. */
    id: string;
    /** Immediate parent identity; null denotes a root. */
    parent_id: string | null;
}

/** Builds tree links once per metadata revision for navigation and destination selection.
 * @param entries - Flat authoritative hierarchy.
 * @param createNode - Presentation adapter for each entry.
 * @returns Root nodes sorted by label; missing parents remain visible at the root.
 * @example buildFolderHierarchy([], folder => ({ id: folder.id, label: folder.id }));
 */
export function buildFolderHierarchy<T extends FolderHierarchyEntry>(
    entries: T[],
    createNode: (entry: T) => TreeNode
): TreeNode[] {
    const nodes = new Map(entries.map(entry => [entry.id, createNode(entry)]));
    const parents = new Map(entries.map(entry => [entry.id, entry.parent_id]));
    const cycles = new Map<string, boolean>();
    const hasCycle = (identifier: string): boolean => {
        const trail = new Set<string>();
        let current: string | null = identifier;
        while (current && !trail.has(current) && !cycles.has(current)) {
            trail.add(current);
            current = parents.get(current) ?? null;
        }
        const cyclic = !!current && (trail.has(current) || cycles.get(current) === true);
        for (const visited of trail) cycles.set(visited, cyclic);
        return cyclic;
    };
    const roots: TreeNode[] = [];
    for (const entry of entries) {
        const node = nodes.get(entry.id)!;
        const parent = entry.parent_id && !hasCycle(entry.id) && nodes.get(entry.parent_id);
        if (parent) (parent.children ??= []).push(node);
        else roots.push(node);
    }
    return roots.sort((firstNode, secondNode) => firstNode.label.localeCompare(secondNode.label));
}

/** Tests ancestry without relying on platform path separators.
 * @param identifier - Proposed descendant, including the ancestor itself.
 * @param ancestor - Folder whose subtree must be excluded.
 * @param folders - Metadata or a cached hierarchy index.
 * @returns Whether the node belongs to that subtree.
 * @example isFolderDescendant('child', 'parent', []);
 */
export function isFolderDescendant(
    identifier: string,
    ancestor: string,
    folders: FolderHierarchyEntry[] | ReadonlyMap<string, FolderHierarchyEntry>
): boolean {
    const index = Array.isArray(folders)
        ? new Map(folders.map(folder => [folder.id, folder]))
        : folders;
    const visited = new Set<string>();
    let current = index.get(identifier);
    while (current && !visited.has(current.id)) {
        if (current.id === ancestor) return true;
        visited.add(current.id);
        current = current.parent_id ? index.get(current.parent_id) : undefined;
    }
    return false;
}
