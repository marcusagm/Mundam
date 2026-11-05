import { invokeCommand as invoke } from './api';
import { type AssetItem } from '../types';

// We primarily use the Rust backend for DB operations now.
// This file wraps those invocations or provides legacy support where needed.

interface FolderNode {
    id: string;
    path: string;
    name: string;
    parent_id: string | null;
    color?: string | null;
    inherits_color?: boolean;
    is_root?: boolean;
}

export async function initDb() {
    // Database management is handled by the Backend (Rust).
    // No-op or perform specific frontend-only inits if needed
}

export async function addLocation(path: string) {
    const name = path.split(/[\\/]/).pop() || 'Unnamed Folder';
    return await invoke('create_folder', {
        payload: {
            parent_id: null,
            name,
            path
        }
    });
}

export async function getLocations() {
    return await invoke<FolderNode[]>('get_all_subfolders');
}

export async function getAssets(
    limit: number = 100,
    offset: number = 0
    // sortBy and sortOrder were removed as V2 handles default sorting or explicit via advanced search
) {
    return await invoke<AssetItem[]>('get_assets', {
        filter: {},
        page: { page: offset / limit + 1, pageSize: limit }
    });
}
