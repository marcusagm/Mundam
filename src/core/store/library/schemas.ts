import { type AssetItem } from '../../../types';

export interface BatchChangeAddedItem extends AssetItem {
    folder_id: string;
    old_folder_id?: string;
}

export interface BatchChangeRemovedItem {
    id: string;
    folder_id: string;
    tag_ids: string[];
}

export interface BatchChangePayload {
    /** Consolidated operations use a single authoritative refresh and their own feedback. */
    operation?: 'folder_removal';
    added?: BatchChangeAddedItem[];
    removed?: BatchChangeRemovedItem[];
    updated?: BatchChangeAddedItem[];
    needs_refresh?: boolean;
}
