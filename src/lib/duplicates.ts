import { invoke } from '@tauri-apps/api/core';
import { DuplicateGroup, DuplicateCandidate } from '../components/features/duplicates/types';

export interface BackendDuplicateGroup {
    id: string;
    rule_set_id: string;
    group_type: string;
    canonical_asset_id: string | null;
    confidence: number;
    status: string;
    candidate_count: number;
    created_at: string;
    updated_at: string;
}

export interface BackendDuplicateCandidate {
    group_id: string;
    asset_id: string;
    score: number;
    reasons: string;
    is_selected: boolean;
}

export interface BackendAsset {
    id: string;
    path: string;
    state: string;
    size: number | null;
    width: number | null;
    height: number | null;
    format: string;
    mime_type: string;
    thumbnail_path?: string | null;
    created_at: string;
    updated_at: string;
    deleted_at?: string | null;
}

export const duplicatesApi = {
    /**
     * Gets all duplicate groups by status.
     * @param status The status (e.g. 'open', 'resolved')
     */
    getDuplicateGroups: async (status: string): Promise<DuplicateGroup[]> => {
        const groups = await invoke<BackendDuplicateGroup[]>('get_duplicate_groups', { status });

        // Map backend DTO to frontend format
        return groups.map(group => ({
            id: group.id,
            type: group.group_type.toLowerCase() as DuplicateGroup['type'], // 'exact' | 'visual' | 'derived'
            status: group.status.toLowerCase() as DuplicateGroup['status'],
            confidence: group.confidence,
            candidateCount: group.candidate_count,
            candidatesLoaded: false,
            candidates: [] // We fetch candidates separately on selection
        }));
    },

    /**
     * Gets candidates for a given group.
     * @param groupId The group ID
     */
    getDuplicateCandidates: async (groupId: string): Promise<DuplicateCandidate[]> => {
        const backendCandidates = await invoke<BackendDuplicateCandidate[]>(
            'get_duplicate_candidates',
            { groupId }
        );

        // To build the full candidate, we ideally need to fetch the assets too.
        // For now, map the fields we know, and the view will fetch the asset info if missing,
        // or we just fetch the asset here.
        const candidates: DuplicateCandidate[] = [];

        for (const cand of backendCandidates) {
            // Fetch the actual asset using its ID
            const assetData = await invoke<BackendAsset | null>('get_asset', {
                id: cand.asset_id
            });

            if (assetData) {
                candidates.push({
                    id: cand.asset_id,
                    name: assetData.path.split(/[/\\]/).pop() || cand.asset_id,
                    size: assetData.size
                        ? `${(assetData.size / 1024 / 1024).toFixed(2)} MB`
                        : 'Unknown',
                    sizeBytes: assetData.size || 0,
                    dimensions:
                        assetData.width && assetData.height
                            ? `${assetData.width}x${assetData.height}`
                            : 'Unknown',
                    score: cand.score,
                    path: assetData.path,
                    format: assetData.format,
                    createdAt: assetData.created_at,
                    updatedAt: assetData.updated_at,
                    tags: [], // Could fetch if needed
                    isFavorite: false,
                    thumbnailUrl: assetData.thumbnail_path || undefined,
                    mediaType: assetData.mime_type,
                    state: assetData.state,
                    isTrashed: !!assetData.deleted_at
                });
            }
        }

        return candidates;
    },

    /**
     * Resolves a duplicate group.
     * @param groupId The group ID
     * @param action The resolution action (e.g., 'keep_oldest', 'ignore_group')
     * @param selectedAssetId Optional selected asset to keep
     */
    resolveDuplicateGroup: async (
        groupId: string,
        action: string,
        keptAssetIds?: string[]
    ): Promise<void> => {
        return invoke('resolve_duplicate_group', {
            groupId,
            action,
            keptAssetIds
        });
    },

    /**
     * Triggers a scan to find new duplicates.
     */
    startDuplicateScan: async (): Promise<void> => {
        return invoke('start_duplicate_scan');
    }
};
