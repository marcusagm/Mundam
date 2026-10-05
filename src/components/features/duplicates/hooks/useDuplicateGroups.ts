import { createSignal, createResource, createMemo, Resource, Setter, Accessor } from 'solid-js';
import { duplicatesApi } from '../../../../lib/duplicates';
import { DuplicateGroup, DuplicateCandidate } from '../types';
import { LifecycleManager } from '../../../../core/utils/LifecycleManager';

/**
 * Scan progress data emitted by the backend during a duplicate scan.
 */
export interface ScanProgress {
    /** Number of fingerprints processed so far. */
    processed: number;
    /** Number of fingerprints that matched a group. */
    matched: number;
    /** Number of new duplicate groups created. */
    groupsCreated: number;
    /** Total number of fingerprints to process in this phase. Zero means indeterminate. */
    total: number;
}

/**
 * Return type of the `useDuplicateGroups` hook.
 */
export interface UseDuplicateGroupsReturn {
    /** All fetched groups (open + ignored when enabled). */
    groups: Resource<DuplicateGroup[]>;
    /** The currently visible groups after applying the ignored filter. */
    visibleGroups: Accessor<DuplicateGroup[]>;
    /** The ID of the currently selected group. */
    selectedGroupId: () => string | null;
    /** Setter for the selected group ID. */
    setSelectedGroupId: Setter<string | null>;
    /** Selects a group and fetches its candidates on demand. */
    selectGroup: (groupId: string) => Promise<void>;
    /** Resolves a group with a given action. */
    resolveGroup: (groupId: string, action: string, keptAssetIds?: string[]) => Promise<void>;
    /** Triggers a manual deep scan. */
    startScan: () => Promise<void>;
    /** Cancels an ongoing scan. */
    cancelScan: () => Promise<void>;
    /** Whether to show ignored groups in the sidebar. */
    showIgnored: Accessor<boolean>;
    /** Setter for the showIgnored flag. */
    setShowIgnored: Setter<boolean>;
    /** The current group type filter. */
    groupTypeFilter: Accessor<'all' | 'exact' | 'visual' | 'derived'>;
    /** Setter for the group type filter. */
    setGroupTypeFilter: Setter<'all' | 'exact' | 'visual' | 'derived'>;
    /** Whether a scan is currently running. */
    isScanning: Accessor<boolean>;
    /** Current scan progress, or null if no scan is in progress. */
    scanProgress: Accessor<ScanProgress | null>;
    /** Count of new duplicate groups found in background scans since last reset. */
    newGroupsFoundCount: Accessor<number>;
    /** Resets the new-groups counter after the user acknowledges the notification. */
    resetNewGroupsCount: () => void;
    /** Preloads candidate items and thumbnail previews for a list of group IDs. */
    preloadGroupCandidates: (groupIdList: string[]) => Promise<void>;
}

/**
 * Hook to manage the state and actions of duplicate groups.
 * Handles fetching, selecting, filtering, and resolving groups,
 * and reacts to backend scan progress events via the LifecycleManager.
 *
 * @returns {UseDuplicateGroupsReturn} The state and actions for duplicate groups.
 */
export function useDuplicateGroups(): UseDuplicateGroupsReturn {
    const [selectedGroupId, setSelectedGroupId] = createSignal<string | null>(null);
    const [showIgnored, setShowIgnored] = createSignal(false);
    const [groupTypeFilter, setGroupTypeFilter] = createSignal<
        'all' | 'exact' | 'visual' | 'derived'
    >('all');
    const [isScanning, setIsScanning] = createSignal(false);
    const [scanProgress, setScanProgress] = createSignal<ScanProgress | null>(null);
    const [newGroupsFoundCount, setNewGroupsFoundCount] = createSignal(0);

    const [groups, { mutate, refetch }] = createResource(async () => {
        try {
            const openGroups = await duplicatesApi.getDuplicateGroups('open');
            const ignoredGroups = await duplicatesApi.getDuplicateGroups('ignored');
            return [...openGroups, ...ignoredGroups];
        } catch (error: unknown) {
            console.error('Failed to load duplicate groups:', error);
            return [];
        }
    });

    LifecycleManager.registerListener<{
        type: string;
        payload: unknown;
    }>('mundam://domain-event', payload => {
        const domainEvent = payload as { type: string; payload: Record<string, number> };
        if (domainEvent.type === 'DuplicateScanProgressed') {
            const eventData = domainEvent.payload;
            setScanProgress({
                processed: eventData.processed || 0,
                matched: eventData.matched || 0,
                groupsCreated: eventData.groups_created || 0,
                total: eventData.total || 0
            });
            setIsScanning(true);
        } else if (domainEvent.type === 'DuplicateScanFinished') {
            setIsScanning(false);
            setScanProgress(null);
            refetch();
        } else if (domainEvent.type === 'DuplicateGroupCreated') {
            setNewGroupsFoundCount(previous => previous + 1);
            refetch();
        }
    });

    /**
     * Derived list of groups visible in the sidebar after applying filters.
     */
    const visibleGroups = createMemo(() => {
        let filteredGroups = groups() || [];

        if (!showIgnored()) {
            filteredGroups = filteredGroups.filter(group => group.status !== 'ignored');
        }

        const typeFilter = groupTypeFilter();
        if (typeFilter !== 'all') {
            filteredGroups = filteredGroups.filter(group => group.type === typeFilter);
        }

        return filteredGroups;
    });

    /**
     * Selects a group and fetches its candidates if they are not already loaded.
     *
     * @param {string} groupId - The ID of the group to select.
     * @returns {Promise<void>}
     */
    const selectGroup = async (groupId: string): Promise<void> => {
        setSelectedGroupId(groupId);
        const currentGroups = groups();
        if (!currentGroups) return;

        const groupIndex = currentGroups.findIndex(group => group.id === groupId);
        if (groupIndex !== -1 && !currentGroups[groupIndex].candidatesLoaded) {
            try {
                const candidates = await duplicatesApi.getDuplicateCandidates(groupId);
                const updatedGroups = [...currentGroups];
                updatedGroups[groupIndex] = {
                    ...updatedGroups[groupIndex],
                    candidates,
                    candidatesLoaded: true,
                    candidateCount: candidates.length
                };
                mutate(updatedGroups);
            } catch (error: unknown) {
                console.error(`Failed to load candidates for group ${groupId}:`, error);
                const updatedGroups = [...currentGroups];
                updatedGroups[groupIndex] = {
                    ...updatedGroups[groupIndex],
                    candidatesLoaded: true
                };
                mutate(updatedGroups);
            }
        }
    };

    /**
     * Resolves a duplicate group using the provided action and updates local state.
     *
     * @param {string} groupId - The ID of the group to resolve.
     * @param {string} action - The action chosen by the user.
     * @param {string[]} [keptAssetIds] - The list of asset IDs to keep.
     * @returns {Promise<void>}
     * @throws Will re-throw backend errors so the caller can handle UI feedback.
     */
    const resolveGroup = async (
        groupId: string,
        action: string,
        keptAssetIds?: string[]
    ): Promise<void> => {
        try {
            await duplicatesApi.resolveDuplicateGroup(groupId, action, keptAssetIds);

            const currentGroups = groups();
            if (currentGroups) {
                if (action === 'ignore_group') {
                    mutate(
                        currentGroups.map(group =>
                            group.id === groupId ? { ...group, status: 'ignored' as const } : group
                        )
                    );
                } else {
                    mutate(currentGroups.filter(group => group.id !== groupId));
                }
            }

            if (selectedGroupId() === groupId) {
                setSelectedGroupId(null);
            }
        } catch (error: unknown) {
            console.error('Failed to resolve duplicate group:', error);
            throw error;
        }
    };

    /**
     * Triggers a manual deep scan and marks the scanning state.
     *
     * @returns {Promise<void>}
     */
    const startScan = async (): Promise<void> => {
        try {
            setIsScanning(true);
            setScanProgress(null);
            await duplicatesApi.startDuplicateScan();
        } catch (error: unknown) {
            console.error('Failed to start scan:', error);
            setIsScanning(false);
            throw error;
        }
    };

    /**
     * Cancels an ongoing scan.
     *
     * @returns {Promise<void>}
     */
    const cancelScan = async (): Promise<void> => {
        try {
            await duplicatesApi.cancelDuplicateScan();
            setIsScanning(false);
            setScanProgress(null);
        } catch (error: unknown) {
            console.error('Failed to cancel scan:', error);
        }
    };

    /**
     * Resets the background-scan new-groups counter after the user acknowledges
     * the notification banner or toast.
     *
     * @returns {void}
     */
    const resetNewGroupsCount = (): void => {
        setNewGroupsFoundCount(0);
    };

    /** Set tracking active candidate network requests to prevent duplicate in-flight fetches */
    const inFlightCandidateLoads = new Set<string>();

    interface PreloadCandidateResult {
        candidateGroupId: string;
        candidateList: DuplicateCandidate[] | null;
    }

    /**
     * Merges successfully preloaded candidates into the current duplicate group list.
     *
     * @param {DuplicateGroup[]} existingGroupList - Current duplicate groups.
     * @param {PreloadCandidateResult[]} preloadResultList - Results from batch fetching candidates.
     * @returns {{ updatedGroupList: DuplicateGroup[]; hasModifications: boolean }} Updated groups and modification flag.
     */
    function applyPreloadedCandidatesToGroups(
        existingGroupList: DuplicateGroup[],
        preloadResultList: PreloadCandidateResult[]
    ): { updatedGroupList: DuplicateGroup[]; hasModifications: boolean } {
        let hasModifications = false;
        const updatedGroupList = [...existingGroupList];

        for (const resultItem of preloadResultList) {
            if (resultItem.candidateList) {
                const groupIndex = updatedGroupList.findIndex(
                    groupItem => groupItem.id === resultItem.candidateGroupId
                );
                if (groupIndex !== -1) {
                    updatedGroupList[groupIndex] = {
                        ...updatedGroupList[groupIndex],
                        candidates: resultItem.candidateList,
                        candidatesLoaded: true,
                        candidateCount: resultItem.candidateList.length
                    };
                    hasModifications = true;
                }
            }
        }

        return { updatedGroupList, hasModifications };
    }

    /**
     * Preloads candidates and thumbnail previews for visible duplicate groups.
     * Prevents duplicate in-flight requests and batches state updates atomically.
     *
     * @param {string[]} groupIdList - Array of group IDs to preload candidates for.
     * @returns {Promise<void>}
     */
    const preloadGroupCandidates = async (groupIdList: string[]): Promise<void> => {
        const currentGroups = groups();
        if (!currentGroups || groupIdList.length === 0) {
            return;
        }

        const candidateGroupsToLoad = groupIdList.filter(candidateGroupId => {
            if (inFlightCandidateLoads.has(candidateGroupId)) {
                return false;
            }
            const foundGroup = currentGroups.find(groupItem => groupItem.id === candidateGroupId);
            return foundGroup && !foundGroup.candidatesLoaded;
        });

        if (candidateGroupsToLoad.length === 0) {
            return;
        }

        for (const candidateGroupId of candidateGroupsToLoad) {
            inFlightCandidateLoads.add(candidateGroupId);
        }

        try {
            const loadResults = await Promise.all(
                candidateGroupsToLoad.map(async candidateGroupId => {
                    try {
                        const candidateList =
                            await duplicatesApi.getDuplicateCandidates(candidateGroupId);
                        return {
                            candidateGroupId,
                            candidateList,
                            error: null
                        };
                    } catch (error: unknown) {
                        console.error(
                            `Failed to preload candidates for group ${candidateGroupId}:`,
                            error
                        );
                        return {
                            candidateGroupId,
                            candidateList: null,
                            error
                        };
                    } finally {
                        inFlightCandidateLoads.delete(candidateGroupId);
                    }
                })
            );

            const latestGroups = groups();
            if (!latestGroups) {
                return;
            }

            const { updatedGroupList, hasModifications } = applyPreloadedCandidatesToGroups(
                latestGroups,
                loadResults
            );

            if (hasModifications) {
                mutate(updatedGroupList);
            }
        } catch (error: unknown) {
            console.error('Error during batch candidate preloading:', error);
        }
    };

    return {
        groups,
        visibleGroups,
        selectedGroupId,
        setSelectedGroupId,
        selectGroup,
        preloadGroupCandidates,
        resolveGroup,
        startScan,
        cancelScan,
        showIgnored,
        setShowIgnored,
        groupTypeFilter,
        setGroupTypeFilter,
        isScanning,
        scanProgress,
        newGroupsFoundCount,
        resetNewGroupsCount
    };
}
