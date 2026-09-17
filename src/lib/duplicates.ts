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

/**
 * Maps a raw backend asset + candidate pair into the frontend DuplicateCandidate type.
 *
 * @param {BackendDuplicateCandidate} backendCandidate - The raw candidate from the backend.
 * @param {BackendAsset} assetData - The full asset record fetched by ID.
 * @returns {DuplicateCandidate} The mapped frontend candidate object.
 */
function buildCandidateFromBackendData(
    backendCandidate: BackendDuplicateCandidate,
    assetData: BackendAsset
): DuplicateCandidate {
    const fileName = assetData.path.split(/[/\\]/).pop() || backendCandidate.asset_id;
    const formattedSize = assetData.size
        ? `${(assetData.size / 1024 / 1024).toFixed(2)} MB`
        : 'Unknown';
    const formattedDimensions =
        assetData.width && assetData.height ? `${assetData.width}x${assetData.height}` : 'Unknown';

    return {
        id: backendCandidate.asset_id,
        name: fileName,
        size: formattedSize,
        sizeBytes: assetData.size || 0,
        dimensions: formattedDimensions,
        score: backendCandidate.score,
        path: assetData.path,
        format: assetData.format,
        createdAt: assetData.created_at,
        updatedAt: assetData.updated_at,
        tags: [],
        isFavorite: false,
        thumbnailUrl: assetData.thumbnail_path || undefined,
        mediaType: assetData.mime_type,
        state: assetData.state,
        isTrashed: !!assetData.deleted_at
    };
}

export const duplicatesApi = {
    /**
     * Gets all duplicate groups filtered by status.
     *
     * @param {string} status - The status filter (e.g. 'open', 'ignored', 'resolved').
     * @returns {Promise<DuplicateGroup[]>} The mapped frontend duplicate groups.
     */
    getDuplicateGroups: async (status: string): Promise<DuplicateGroup[]> => {
        const groups = await invoke<BackendDuplicateGroup[]>('get_duplicate_groups', { status });

        return groups.map(group => ({
            id: group.id,
            type: group.group_type.toLowerCase() as DuplicateGroup['type'],
            status: group.status.toLowerCase() as DuplicateGroup['status'],
            confidence: group.confidence,
            candidateCount: group.candidate_count,
            candidatesLoaded: false,
            candidates: []
        }));
    },

    /**
     * Gets the full candidate list for a group, resolving each candidate's asset metadata.
     * This performs N+1 fetches (one `get_asset` per candidate) as a known trade-off for
     * simplicity. A batch endpoint should be implemented when groups grow large.
     *
     * @param {string} groupId - The group ID whose candidates to fetch.
     * @returns {Promise<DuplicateCandidate[]>} The mapped frontend candidates.
     */
    getDuplicateCandidates: async (groupId: string): Promise<DuplicateCandidate[]> => {
        const backendCandidates = await invoke<BackendDuplicateCandidate[]>(
            'get_duplicate_candidates',
            { groupId }
        );

        const candidatePromises = backendCandidates.map(async backendCandidate => {
            const assetData = await invoke<BackendAsset | null>('get_asset', {
                id: backendCandidate.asset_id
            });

            if (!assetData) return null;
            return buildCandidateFromBackendData(backendCandidate, assetData);
        });

        const resolvedCandidates = await Promise.all(candidatePromises);
        return resolvedCandidates.filter(
            (candidate): candidate is DuplicateCandidate => candidate !== null
        );
    },

    /**
     * Resolves a duplicate group with a given action.
     *
     * @param {string} groupId - The group ID to resolve.
     * @param {string} action - The resolution action (e.g., 'custom_selection', 'ignore_group').
     * @param {string[]} [keptAssetIds] - Optional list of asset IDs to keep.
     * @returns {Promise<void>}
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
     * Triggers a full duplicate scan on the backend (rehash + exact + visual).
     *
     * @returns {Promise<void>}
     */
    startDuplicateScan: async (): Promise<void> => {
        return invoke('start_duplicate_scan');
    },

    /**
     * Cancels an ongoing duplicate scan via its CancellationToken.
     *
     * @returns {Promise<void>}
     */
    cancelDuplicateScan: async (): Promise<void> => {
        return invoke('cancel_duplicate_scan');
    }
};
