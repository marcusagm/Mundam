import { APP_CONFIG } from '../../../config/constants';
import type { FolderNode } from '../metadata/metadataState';
import type { BatchChangeAddedItem } from './schemas';

/** Path fields that an observation may patch without replacing authored metadata. */
type AssetPathUpdate = Pick<
    BatchChangeAddedItem,
    'id' | 'folder_id' | 'path' | 'filename' | 'modified_at'
>;

/** Captured folder view used to classify a batch independently of reactive state. */
export interface LibraryBatchView {
    /** Explicit selection; an empty selection includes all folders. */
    selectedFolderIds: readonly string[];
    /** Whether selected folders include descendants. */
    recursive: boolean;
    /** Authoritative parent relationships. */
    locations: readonly Pick<FolderNode, 'id' | 'parent_id'>[];
    /** Identities already present in the viewport. */
    currentItems: readonly { id: string }[];
}

/** Minimal changes needed to retain known assets and discover newly visible ones. */
export interface LibraryBatchUpdatePlan {
    /** Confirmed path patches for known assets still in view. */
    updates: AssetPathUpdate[];
    /** Known assets that moved outside the selected folders. */
    removedIds: Set<string>;
    /** An unknown asset entered the view and requires an authoritative refresh. */
    needsRefresh: boolean;
}

function belongsToSubtree(
    childId: string,
    rootId: string,
    parents: ReadonlyMap<string, string | null>
): boolean {
    let current: string | null = childId;
    let depth = 0;
    while (current && depth < APP_CONFIG.MAX_FOLDER_DEPTH) {
        if (current === rootId) return true;
        current = parents.get(current) ?? null;
        depth++;
    }
    return false;
}

/** Classifies path changes against the current folder view without mutating stores.
 * @param updatedItems - Confirmed path and folder ownership changes.
 * @param view - Folder selection, ancestry and known viewport identities.
 * @returns Path patches, moved-out identities and whether a full refresh is needed.
 * @example planLibraryBatchUpdates([], { selectedFolderIds: [], recursive: false, locations: [], currentItems: [] });
 */
export function planLibraryBatchUpdates(
    updatedItems: readonly AssetPathUpdate[],
    view: LibraryBatchView
): LibraryBatchUpdatePlan {
    const parents = new Map(view.locations.map(location => [location.id, location.parent_id]));
    const knownIds = new Set(view.currentItems.map(item => item.id));
    const plan: LibraryBatchUpdatePlan = {
        updates: [],
        removedIds: new Set(),
        needsRefresh: false
    };
    for (const updatedItem of updatedItems) {
        const isNowInView =
            view.selectedFolderIds.length === 0 ||
            view.selectedFolderIds.some(folderId =>
                view.recursive
                    ? belongsToSubtree(updatedItem.folder_id, folderId, parents)
                    : updatedItem.folder_id === folderId
            );
        if (!isNowInView) {
            if (knownIds.has(updatedItem.id)) plan.removedIds.add(updatedItem.id);
        } else if (knownIds.has(updatedItem.id)) {
            plan.updates.push(updatedItem);
        } else {
            plan.needsRefresh = true;
        }
    }
    return plan;
}
