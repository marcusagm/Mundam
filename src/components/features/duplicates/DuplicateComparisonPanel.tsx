import { Component, For, Show, createSignal, createMemo } from 'solid-js';
import { Button, Badge } from '../../ui';
import { Columns, Undo2 } from 'lucide-solid';
import { DuplicateGroup, DuplicateCandidate } from './types';
import { DuplicateSplitView } from './DuplicateSplitView';
import { DuplicateCandidateCard } from './DuplicateCandidateCard';
import { DuplicateResolutionBanner } from './DuplicateResolutionBanner';
import { DuplicateSmartActionsBar } from './DuplicateSmartActionsBar';
import { MetadataMergeModal } from './MetadataMergeModal';
import { useCandidateAnalysis } from './hooks/useCandidateAnalysis';
import { getGroupTypeBadgeVariant, formatGroupTypeLabel } from '../../../lib/duplicates';
import { createShortcut } from '../../../core/input';
import './duplicate-comparison-panel.css';

export interface DuplicateComparisonPanelProperties {
    /** The duplicate group to display and resolve */
    group: DuplicateGroup;
    /** Callback triggered when the group has been resolved */
    onResolve: (groupId: string, action: string, keptAssetIds?: string[]) => Promise<void>;
    /** Callback triggered to undo a resolution or reopen an ignored group */
    onUndo?: (groupId: string) => Promise<void>;
}

/**
 * Displays the detail panel for a selected duplicate group.
 *
 * After the user selects assets to keep and clicks "Keep Selected" or
 * "Keep Only This", a MetadataMergeModal opens so the user can review
 * metadata from discarded candidates field-by-field before confirming.
 *
 * @param {DuplicateComparisonPanelProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered comparison panel.
 */
export const DuplicateComparisonPanel: Component<
    DuplicateComparisonPanelProperties
> = properties => {
    const [selectedCandidateIdentifiers, setSelectedCandidateIdentifiers] = createSignal<
        Set<string>
    >(new Set());
    const [isProcessing, setIsProcessing] = createSignal(false);
    const [isSplitViewOpen, setIsSplitViewOpen] = createSignal(false);

    /** Identifiers of assets the user has chosen to keep, pending merge confirmation. */
    const [pendingKeptCandidateIdentifiers, setPendingKeptCandidateIdentifiers] = createSignal<
        string[]
    >([]);
    const [isMergeModalOpen, setIsMergeModalOpen] = createSignal(false);

    const toggleCandidate = (candidateIdentifier: string) => {
        const updatedSet = new Set(selectedCandidateIdentifiers());
        if (updatedSet.has(candidateIdentifier)) {
            updatedSet.delete(candidateIdentifier);
        } else {
            updatedSet.add(candidateIdentifier);
        }
        setSelectedCandidateIdentifiers(updatedSet);
    };

    const { largestCandidate, oldestCandidate, favoriteCandidate, splitViewCandidates } =
        useCandidateAnalysis(() => properties.group.candidates, selectedCandidateIdentifiers);

    /**
     * Derives the kept and discarded candidate lists from the pending kept IDs.
     * Used to populate the MetadataMergeModal.
     */
    const keptCandidates = createMemo(() =>
        properties.group.candidates.filter(candidate =>
            pendingKeptCandidateIdentifiers().includes(candidate.id)
        )
    );

    const discardedCandidates = createMemo(() =>
        properties.group.candidates.filter(
            candidate =>
                !pendingKeptCandidateIdentifiers().includes(candidate.id) && !candidate.isTrashed
        )
    );

    /**
     * Opens the MetadataMergeModal with the given kept asset IDs.
     * The modal handles the actual resolve call after the user confirms.
     *
     * @param {string[]} keptCandidateIdentifiers - The asset IDs the user chose to keep.
     */
    const openMergeModal = (keptCandidateIdentifiers: string[]) => {
        setPendingKeptCandidateIdentifiers(keptCandidateIdentifiers);
        setIsMergeModalOpen(true);
    };

    const handleIgnoreGroup = async () => {
        setIsProcessing(true);
        try {
            await properties.onResolve(properties.group.id, 'ignore_group');
        } catch (error) {
            console.error('Failed to ignore group:', error);
        } finally {
            setIsProcessing(false);
        }
    };

    const handleUndoAction = async () => {
        if (!properties.onUndo) return;
        setIsProcessing(true);
        try {
            await properties.onUndo(properties.group.id);
        } catch (error: unknown) {
            console.error('Failed to undo duplicate group:', error);
        } finally {
            setIsProcessing(false);
        }
    };

    const handleKeepSelected = () => {
        if (selectedCandidateIdentifiers().size === 0) return;
        openMergeModal(Array.from(selectedCandidateIdentifiers()));
    };

    const handleKeepOnlyThis = (candidateIdentifier: string) => {
        openMergeModal([candidateIdentifier]);
    };

    const handleSmartAction = (candidate: DuplicateCandidate | null) => {
        if (!candidate) return;
        handleKeepOnlyThis(candidate.id);
    };

    const handleMergeModalClose = () => {
        setIsMergeModalOpen(false);
        setPendingKeptCandidateIdentifiers([]);
        setIsProcessing(false);
    };

    const handleOpenSplitView = () => {
        setIsSplitViewOpen(true);
    };

    createShortcut({
        keys: 'd',
        scope: 'viewport',
        system: true,
        action: () => {
            if (!isProcessing()) handleIgnoreGroup();
        }
    });

    createShortcut({
        keys: 'Backspace',
        scope: 'viewport',
        system: true,
        action: () => {
            if (!isProcessing()) handleIgnoreGroup();
        }
    });

    createShortcut({
        keys: 'Delete',
        scope: 'viewport',
        system: true,
        action: () => {
            if (!isProcessing()) handleIgnoreGroup();
        }
    });

    createShortcut({
        keys: 'k',
        scope: 'viewport',
        system: true,
        action: () => {
            if (isProcessing()) return;
            if (selectedCandidateIdentifiers().size > 0) {
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
                        <Badge variant={getGroupTypeBadgeVariant(properties.group.type)}>
                            {formatGroupTypeLabel(properties.group.type)}
                        </Badge>
                        <span class="comparison-confidence">
                            Confidence: {(properties.group.confidence * 100).toFixed(0)}%
                        </span>
                    </div>
                </div>
                <div class="comparison-actions">
                    <Button
                        variant="secondary"
                        onClick={handleOpenSplitView}
                        disabled={
                            properties.group.candidates.filter(candidate => !candidate.isTrashed)
                                .length < 2
                        }
                    >
                        <Columns size={16} class="duplicate-comparison-button-icon" />
                        Split View
                    </Button>
                    <Show
                        when={properties.group.status === 'open'}
                        fallback={
                            <Button
                                variant="secondary"
                                onClick={handleUndoAction}
                                disabled={isProcessing()}
                            >
                                <Undo2 size={16} class="duplicate-comparison-button-icon" />
                                {properties.group.status === 'resolved'
                                    ? 'Undo Resolution'
                                    : 'Reopen Group'}
                            </Button>
                        }
                    >
                        <Button
                            variant="secondary"
                            onClick={handleIgnoreGroup}
                            disabled={isProcessing()}
                        >
                            Ignore Group
                        </Button>
                        <Button
                            disabled={selectedCandidateIdentifiers().size === 0 || isProcessing()}
                            onClick={handleKeepSelected}
                        >
                            Keep Selected
                        </Button>
                    </Show>
                </div>
            </div>

            <DuplicateResolutionBanner
                groupStatus={properties.group.status}
                isProcessing={isProcessing()}
                onUndoAction={handleUndoAction}
            />

            <Show
                when={properties.group.status === 'open' && properties.group.candidates.length > 1}
            >
                <DuplicateSmartActionsBar
                    largestCandidate={largestCandidate()}
                    oldestCandidate={oldestCandidate()}
                    favoriteCandidate={favoriteCandidate()}
                    isProcessing={isProcessing()}
                    onSelectCandidate={handleSmartAction}
                />
            </Show>

            <div class="comparison-grid">
                <For each={properties.group.candidates}>
                    {candidate => (
                        <DuplicateCandidateCard
                            candidate={candidate}
                            isSelected={selectedCandidateIdentifiers().has(candidate.id)}
                            processing={isProcessing()}
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
                groupId={properties.group.id}
                candidates={properties.group.candidates}
                keptCandidates={keptCandidates()}
                discardedCandidates={discardedCandidates()}
                onResolve={properties.onResolve}
                onClose={handleMergeModalClose}
            />
        </div>
    );
};
