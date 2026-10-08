import { Accessor, createMemo } from 'solid-js';
import { DuplicateCandidate } from '../types';

export interface SplitViewCandidatePair {
    firstCandidate: DuplicateCandidate | null;
    secondCandidate: DuplicateCandidate | null;
}

export interface CandidateAnalysisResult {
    largestCandidate: Accessor<DuplicateCandidate | null>;
    oldestCandidate: Accessor<DuplicateCandidate | null>;
    favoriteCandidate: Accessor<DuplicateCandidate | null>;
    splitViewCandidates: Accessor<SplitViewCandidatePair>;
}

/**
 * Hook providing derived metrics and candidate selections for duplicate comparison.
 *
 * @param {Accessor<DuplicateCandidate[]>} candidatesAccessor - Signal accessor for candidates in the current group.
 * @param {Accessor<Set<string>>} selectedCandidatesAccessor - Signal accessor for currently selected candidate IDs.
 * @returns {CandidateAnalysisResult} Computed candidate metrics and split view candidates.
 */
export function useCandidateAnalysis(
    candidatesAccessor: Accessor<DuplicateCandidate[]>,
    selectedCandidatesAccessor: Accessor<Set<string>>
): CandidateAnalysisResult {
    /** Finds the candidate with the largest file size, ignoring trashed ones. */
    const largestCandidate = createMemo(() => {
        const validCandidates = candidatesAccessor().filter(
            (candidate: DuplicateCandidate) => !candidate.isTrashed
        );
        if (validCandidates.length === 0) return null;
        return validCandidates.reduce(
            (previousCandidate: DuplicateCandidate, currentCandidate: DuplicateCandidate) => {
                return currentCandidate.sizeBytes > previousCandidate.sizeBytes
                    ? currentCandidate
                    : previousCandidate;
            }
        );
    });

    /** Finds the candidate created earliest, ignoring trashed ones. */
    const oldestCandidate = createMemo(() => {
        const validCandidates = candidatesAccessor().filter(
            (candidate: DuplicateCandidate) => !candidate.isTrashed
        );
        if (validCandidates.length === 0) return null;
        return validCandidates.reduce(
            (oldestFound: DuplicateCandidate, currentCandidate: DuplicateCandidate) =>
                new Date(currentCandidate.createdAt).getTime() <
                new Date(oldestFound.createdAt).getTime()
                    ? currentCandidate
                    : oldestFound
        );
    });

    /** Finds the first candidate marked as favorite, if any. */
    const favoriteCandidate = createMemo(() => {
        return (
            candidatesAccessor().find((candidate: DuplicateCandidate) => candidate.isFavorite) ||
            null
        );
    });

    /** Determines which two candidates should be displayed in split view comparison. */
    const splitViewCandidates = createMemo((): SplitViewCandidatePair => {
        const selectedCandidateIdentifiers = Array.from(selectedCandidatesAccessor());
        const validCandidates = candidatesAccessor().filter(
            (candidate: DuplicateCandidate) => !candidate.isTrashed
        );

        let firstCandidate: DuplicateCandidate | null = null;
        let secondCandidate: DuplicateCandidate | null = null;

        if (selectedCandidateIdentifiers.length >= 2) {
            firstCandidate =
                candidatesAccessor().find(
                    (candidate: DuplicateCandidate) =>
                        candidate.id === selectedCandidateIdentifiers[0]
                ) || null;
            secondCandidate =
                candidatesAccessor().find(
                    (candidate: DuplicateCandidate) =>
                        candidate.id === selectedCandidateIdentifiers[1]
                ) || null;
        } else if (validCandidates.length >= 2) {
            firstCandidate = validCandidates[0];
            secondCandidate = validCandidates[1];
        }

        return { firstCandidate, secondCandidate };
    });

    return {
        largestCandidate,
        oldestCandidate,
        favoriteCandidate,
        splitViewCandidates
    };
}
