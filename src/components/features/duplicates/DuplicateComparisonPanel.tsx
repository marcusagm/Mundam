import { Component, For, Show, createSignal, createMemo } from 'solid-js';
import { Button, Badge } from '../../ui';
import { Columns } from 'lucide-solid';
import { DuplicateGroup, DuplicateCandidate } from './types';
import { DuplicateSplitView } from './DuplicateSplitView';
import { DuplicateCandidateCard } from './DuplicateCandidateCard';
import { MetadataMergeModal } from './MetadataMergeModal';
import { getGroupTypeBadgeVariant, formatGroupTypeLabel } from '../../../lib/duplicates';
import { createShortcut } from '../../../core/input';
import './duplicate-comparison-panel.css';

export interface DuplicateComparisonPanelProperties {
    /** The duplicate group to display and resolve */
    group: DuplicateGroup;
    /** Callback triggered when the group has been resolved */
    onResolve: (groupId: string, action: string, keptAssetIds?: string[]) => Promise<void>;
}

/**
 * Displays the detail panel for a selected duplicate group.
 *
 * After the user selects assets to keep and clicks "Keep Selected" or
 * "Keep Only This", a MetadataMergeModal opens so the user can review
 * metadata from discarded candidates field-by-field before confirming.
 *
 * @param {DuplicateComparisonPanelProperties} props - Component properties.
 * @returns {JSX.Element} The rendered comparison panel.
 */
export const DuplicateComparisonPanel: Component<DuplicateComparisonPanelProperties> = props => {
    const [selectedCandidates, setSelectedCandidates] = createSignal<Set<string>>(new Set());
    const [processing, setProcessing] = createSignal(false);
    const [isSplitViewOpen, setIsSplitViewOpen] = createSignal(false);

    /** IDs of assets the user has chosen to keep, pending merge confirmation. */
    const [pendingKeptIds, setPendingKeptIds] = createSignal<string[]>([]);
    const [isMergeModalOpen, setIsMergeModalOpen] = createSignal(false);

    const toggleCandidate = (id: string) => {
        const updatedSet = new Set(selectedCandidates());
        if (updatedSet.has(id)) {
            updatedSet.delete(id);
        } else {
            updatedSet.add(id);
        }
        setSelectedCandidates(updatedSet);
    };

    /** Finds the candidate with the largest file size, ignoring trashed ones. */
    const largestCandidate = createMemo(() => {
        const validCandidates = props.group.candidates.filter(candidate => !candidate.isTrashed);
        if (validCandidates.length === 0) return null;
        return validCandidates.reduce(
            (previous: DuplicateCandidate, current: DuplicateCandidate) => {
                return current.sizeBytes > previous.sizeBytes ? current : previous;
            }
        );
    });

    /** Finds the candidate created earliest, ignoring trashed ones. */
    const oldestCandidate = createMemo(() => {
        const validCandidates = props.group.candidates.filter(candidate => !candidate.isTrashed);
        if (validCandidates.length === 0) return null;
        return validCandidates.reduce((oldest: DuplicateCandidate, current: DuplicateCandidate) =>
            new Date(current.createdAt).getTime() < new Date(oldest.createdAt).getTime()
                ? current
                : oldest
        );
    });

    /** Finds the first candidate marked as favorite, if any. */
    const favoriteCandidate = createMemo(() => {
        return (
            props.group.candidates.find((candidate: DuplicateCandidate) => candidate.isFavorite) ||
            null
        );
    });

    /**
     * Derives the kept and discarded candidate lists from the pending kept IDs.
     * Used to populate the MetadataMergeModal.
     */
    const keptCandidates = createMemo(() =>
        props.group.candidates.filter(candidate => pendingKeptIds().includes(candidate.id))
    );

    const discardedCandidates = createMemo(() =>
        props.group.candidates.filter(
            candidate => !pendingKeptIds().includes(candidate.id) && !candidate.isTrashed
        )
    );

    /**
     * Opens the MetadataMergeModal with the given kept asset IDs.
     * The modal handles the actual resolve call after the user confirms.
     *
     * @param {string[]} keptIds - The asset IDs the user chose to keep.
     */
    const openMergeModal = (keptIds: string[]) => {
        setPendingKeptIds(keptIds);
        setIsMergeModalOpen(true);
    };

    const handleIgnoreGroup = async () => {
        setProcessing(true);
        try {
            await props.onResolve(props.group.id, 'ignore_group');
        } catch (error) {
            console.error('Failed to ignore group:', error);
        } finally {
            setProcessing(false);
        }
    };

    const handleKeepSelected = () => {
        if (selectedCandidates().size === 0) return;
        openMergeModal(Array.from(selectedCandidates()));
    };

    const handleKeepOnlyThis = (candidateId: string) => {
        openMergeModal([candidateId]);
    };

    const handleSmartAction = (candidate: DuplicateCandidate | null) => {
        if (!candidate) return;
        handleKeepOnlyThis(candidate.id);
    };

    const handleMergeModalClose = () => {
        setIsMergeModalOpen(false);
        setPendingKeptIds([]);
        setProcessing(false);
    };

    const handleOpenSplitView = () => {
        setIsSplitViewOpen(true);
    };

    const splitViewCandidates = createMemo(() => {
        const selectedCandidateIds = Array.from(selectedCandidates());
        const validCandidates = props.group.candidates.filter(candidate => !candidate.isTrashed);

        let firstCandidate = null;
        let secondCandidate = null;

        if (selectedCandidateIds.length >= 2) {
            firstCandidate =
                props.group.candidates.find(
                    candidate => candidate.id === selectedCandidateIds[0]
                ) || null;
            secondCandidate =
                props.group.candidates.find(
                    candidate => candidate.id === selectedCandidateIds[1]
                ) || null;
        } else if (validCandidates.length >= 2) {
            firstCandidate = validCandidates[0];
            secondCandidate = validCandidates[1];
        }

        return { firstCandidate, secondCandidate };
    });

    createShortcut({
        keys: 'd',
        scope: 'viewport',
        system: true,
        action: () => {
            if (!processing()) handleIgnoreGroup();
        }
    });

    createShortcut({
        keys: 'Backspace',
        scope: 'viewport',
        system: true,
        action: () => {
            if (!processing()) handleIgnoreGroup();
        }
    });

    createShortcut({
        keys: 'Delete',
        scope: 'viewport',
        system: true,
        action: () => {
            if (!processing()) handleIgnoreGroup();
        }
    });

    createShortcut({
        keys: 'k',
        scope: 'viewport',
        system: true,
        action: () => {
            if (processing()) return;
            if (selectedCandidates().size > 0) {
                handleKeepSelected();
            } else if (largestCandidate()) {
                handleSmartAction(largestCandidate());
            }
        }
    });

    return (
        <div class="comparison-panel">
            <div class="comparison-header">
                <div class="comparison-title-container">
                    <h2>Group Details</h2>
                    <div class="comparison-meta">
                        <Badge variant={getGroupTypeBadgeVariant(props.group.type)}>
                            {formatGroupTypeLabel(props.group.type)}
                        </Badge>
                        <span class="comparison-confidence">
                            Confidence: {(props.group.confidence * 100).toFixed(0)}%
                        </span>
                    </div>
                </div>
                <div class="comparison-actions">
                    <Button
                        variant="secondary"
                        onClick={handleOpenSplitView}
                        disabled={
                            props.group.candidates.filter(candidate => !candidate.isTrashed)
                                .length < 2
                        }
                    >
                        <Columns size={16} class="duplicate-comparison-button-icon" />
                        Split View
                    </Button>
                    <Button variant="secondary" onClick={handleIgnoreGroup} disabled={processing()}>
                        Ignore Group
                    </Button>
                    <Button
                        disabled={selectedCandidates().size === 0 || processing()}
                        onClick={handleKeepSelected}
                    >
                        Keep Selected
                    </Button>
                </div>
            </div>

            <Show when={props.group.candidates.length > 1}>
                <div class="smart-actions-bar">
                    <span class="smart-actions-label">Quick Actions</span>
                    <div class="smart-actions-buttons">
                        <Button
                            variant="secondary"
                            size="sm"
                            disabled={processing() || !largestCandidate()}
                            onClick={() => handleSmartAction(largestCandidate())}
                        >
                            Keep Largest
                        </Button>
                        <Button
                            variant="secondary"
                            size="sm"
                            disabled={processing() || !oldestCandidate()}
                            onClick={() => handleSmartAction(oldestCandidate())}
                        >
                            Keep Oldest
                        </Button>
                        <Show when={favoriteCandidate()}>
                            <Button
                                variant="secondary"
                                size="sm"
                                disabled={processing()}
                                onClick={() => handleSmartAction(favoriteCandidate())}
                            >
                                Keep Favorite
                            </Button>
                        </Show>
                    </div>
                </div>
            </Show>

            <div class="comparison-grid">
                <For each={props.group.candidates}>
                    {candidate => (
                        <DuplicateCandidateCard
                            candidate={candidate}
                            isSelected={selectedCandidates().has(candidate.id)}
                            processing={processing()}
                            onToggle={toggleCandidate}
                            onKeepOnlyThis={handleKeepOnlyThis}
                        />
                    )}
                </For>
            </div>

            <DuplicateSplitView
                isOpen={isSplitViewOpen()}
                onClose={() => setIsSplitViewOpen(false)}
                candidateA={splitViewCandidates().firstCandidate}
                candidateB={splitViewCandidates().secondCandidate}
            />

            <MetadataMergeModal
                isOpen={isMergeModalOpen()}
                groupId={props.group.id}
                candidates={props.group.candidates}
                keptCandidates={keptCandidates()}
                discardedCandidates={discardedCandidates()}
                onResolve={props.onResolve}
                onClose={handleMergeModalClose}
            />
        </div>
    );
};
