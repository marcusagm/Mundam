import { duplicatesApi } from '../../../../lib/duplicates';
import { DuplicateGroup, DuplicateCandidate } from '../types';

/**
 * Result representing the outcome of preloading candidates for a single duplicate group.
 */
export interface PreloadCandidateResult {
    /** The duplicate group identifier */
    candidateGroupId: string;
    /** The loaded candidate assets, or null if loading failed */
    candidateList: DuplicateCandidate[] | null;
}

/**
 * Merges successfully preloaded candidates into the current duplicate group list.
 *
 * @param {DuplicateGroup[]} existingGroupList - Current duplicate groups.
 * @param {PreloadCandidateResult[]} preloadResultList - Results from batch fetching candidates.
 * @returns {{ updatedGroupList: DuplicateGroup[]; hasModifications: boolean }} Updated groups and modification flag.
 */
export function applyPreloadedCandidatesToGroups(
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
 * Fetches candidates for candidate groups, preventing duplicate in-flight requests.
 *
 * @param {string[]} candidateGroupsToLoad - Group IDs that need candidates loaded.
 * @param {Set<string>} inFlightLoads - Set tracking groups currently being fetched.
 * @returns {Promise<PreloadCandidateResult[]>} Array of preload results.
 */
export async function fetchCandidatesForGroups(
    candidateGroupsToLoad: string[],
    inFlightLoads: Set<string>
): Promise<PreloadCandidateResult[]> {
    for (const candidateGroupId of candidateGroupsToLoad) {
        inFlightLoads.add(candidateGroupId);
    }

    return Promise.all(
        candidateGroupsToLoad.map(async candidateGroupId => {
            try {
                const candidateList = await duplicatesApi.getDuplicateCandidates(candidateGroupId);
                return {
                    candidateGroupId,
                    candidateList
                };
            } catch (error: unknown) {
                console.error(`Failed to preload candidates for group ${candidateGroupId}:`, error);
                return {
                    candidateGroupId,
                    candidateList: null
                };
            } finally {
                inFlightLoads.delete(candidateGroupId);
            }
        })
    );
}
