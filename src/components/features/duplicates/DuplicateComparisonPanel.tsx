import { Component, For, Show, createSignal, createMemo } from 'solid-js';
import { Button, Badge } from '../../ui';
import { CheckCircle2, Trash2 } from 'lucide-solid';
import { Thumbnail } from '../viewport/assets/Thumbnail';
import { DuplicateGroup, DuplicateCandidate } from './types';
import './duplicate-comparison-panel.css';
export interface DuplicateComparisonPanelProperties {
    /** The duplicate group to display and resolve */
    group: DuplicateGroup;
    /** Callback triggered when the group has been resolved */
    onResolve: (groupId: string, action: string, keptAssetIds?: string[]) => Promise<void>;
}

export const DuplicateComparisonPanel: Component<DuplicateComparisonPanelProperties> = props => {
    const [selectedCandidates, setSelectedCandidates] = createSignal<Set<string>>(new Set());
    const [processing, setProcessing] = createSignal(false);

    const toggleCandidate = (id: string) => {
        const newSet = new Set(selectedCandidates());
        if (newSet.has(id)) {
            newSet.delete(id);
        } else {
            newSet.add(id);
        }
        setSelectedCandidates(newSet);
    };

    /** Finds the candidate with the largest file size, ignoring trashed ones. */
    const largestCandidate = createMemo(() => {
        const validCandidates = props.group.candidates.filter(c => !c.isTrashed);
        if (validCandidates.length === 0) return null;
        return validCandidates.reduce((prev: DuplicateCandidate, current: DuplicateCandidate) => {
            return current.sizeBytes > prev.sizeBytes ? current : prev;
        });
    });

    /** Finds the candidate created earliest, ignoring trashed ones. */
    const oldestCandidate = createMemo(() => {
        const validCandidates = props.group.candidates.filter(c => !c.isTrashed);
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

    const handleKeepSelected = async () => {
        if (selectedCandidates().size === 0) return;

        setProcessing(true);
        try {
            const keptIds = Array.from(selectedCandidates());
            await props.onResolve(props.group.id, 'custom_selection', keptIds);
        } catch (error) {
            console.error('Failed to keep selected candidates:', error);
        } finally {
            setProcessing(false);
        }
    };

    const handleKeepOnlyThis = async (candidateId: string) => {
        setProcessing(true);
        try {
            await props.onResolve(props.group.id, 'custom_selection', [candidateId]);
        } catch (error) {
            console.error('Failed to keep candidate:', error);
        } finally {
            setProcessing(false);
        }
    };

    const handleSmartAction = async (candidate: DuplicateCandidate | null) => {
        if (!candidate) return;
        await handleKeepOnlyThis(candidate.id);
    };

    return (
        <div class="comparison-panel">
            <div class="comparison-header">
                <div class="comparison-title-container">
                    <h2>Group Details</h2>
                    <div class="comparison-meta">
                        <Badge>{props.group.type}</Badge>
                        <span class="comparison-confidence">
                            Confidence: {(props.group.confidence * 100).toFixed(0)}%
                        </span>
                    </div>
                </div>
                <div class="comparison-actions">
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
                    {candidate => {
                        const isSelected = () => selectedCandidates().has(candidate.id);
                        return (
                            <div
                                tabIndex={candidate.isTrashed ? -1 : 0}
                                class={`candidate-card ${isSelected() ? 'is-selected' : ''} ${candidate.isTrashed ? 'is-trashed' : ''}`}
                                onClick={() => {
                                    if (!candidate.isTrashed) toggleCandidate(candidate.id);
                                }}
                                onKeyDown={e => {
                                    if (candidate.isTrashed) return;
                                    if (e.key === 'Enter' || e.key === ' ') {
                                        e.preventDefault();
                                        toggleCandidate(candidate.id);
                                    }
                                }}
                            >
                                <div class="candidate-card-body" style={{ position: 'relative' }}>
                                    <Show when={candidate.isTrashed}>
                                        <div class="candidate-trashed-overlay">
                                            <Trash2 size={48} />
                                            <span>Moved to Trash</span>
                                        </div>
                                    </Show>
                                    <div class="candidate-header">
                                        <h3 class="candidate-name">{candidate.name}</h3>
                                        <div class="candidate-badges">
                                            <Show when={isSelected()}>
                                                <div class="candidate-selected-icon">
                                                    <CheckCircle2
                                                        size={20}
                                                        fill="currentColor"
                                                        color="var(--bg-secondary)"
                                                    />
                                                </div>
                                            </Show>
                                            <Show when={candidate.isFavorite}>
                                                <Badge variant="secondary">Favorite</Badge>
                                            </Show>
                                            <Badge variant="secondary">
                                                {(candidate.score * 100).toFixed(0)}%
                                            </Badge>
                                        </div>
                                    </div>

                                    <div class="candidate-preview">
                                        <Thumbnail
                                            id={candidate.id}
                                            src={candidate.path}
                                            thumbnail={candidate.thumbnailUrl || null}
                                            alt={candidate.name}
                                            mediaType={candidate.mediaType}
                                            state={candidate.state}
                                        />
                                    </div>

                                    <div class="candidate-details">
                                        <div class="candidate-detail-item full-width">
                                            <span class="candidate-detail-label">Path</span>
                                            <span class="candidate-detail-value">
                                                {candidate.path}
                                            </span>
                                        </div>
                                        <div class="candidate-detail-item">
                                            <span class="candidate-detail-label">Format</span>
                                            <span class="candidate-detail-value">
                                                {candidate.format}
                                            </span>
                                        </div>
                                        <div class="candidate-detail-item">
                                            <span class="candidate-detail-label">Size</span>
                                            <span class="candidate-detail-value">
                                                {candidate.size}
                                            </span>
                                        </div>
                                        <div class="candidate-detail-item">
                                            <span class="candidate-detail-label">Dimensions</span>
                                            <span class="candidate-detail-value">
                                                {candidate.dimensions}
                                            </span>
                                        </div>
                                        <div class="candidate-detail-item">
                                            <span class="candidate-detail-label">Created</span>
                                            <span class="candidate-detail-value">
                                                {new Date(candidate.createdAt).toLocaleString()}
                                            </span>
                                        </div>
                                        <div class="candidate-detail-item">
                                            <span class="candidate-detail-label">Modified</span>
                                            <span class="candidate-detail-value">
                                                {new Date(candidate.updatedAt).toLocaleString()}
                                            </span>
                                        </div>
                                        <Show when={candidate.tags && candidate.tags.length > 0}>
                                            <div class="candidate-detail-item full-width">
                                                <span class="candidate-detail-label">Tags</span>
                                                <span class="candidate-detail-value">
                                                    {candidate.tags.join(', ')}
                                                </span>
                                            </div>
                                        </Show>
                                        <Show when={candidate.notes}>
                                            <div class="candidate-detail-item full-width">
                                                <span class="candidate-detail-label">Notes</span>
                                                <span class="candidate-detail-value">
                                                    {candidate.notes}
                                                </span>
                                            </div>
                                        </Show>
                                    </div>
                                </div>

                                <div class="candidate-actions">
                                    <Button
                                        class="candidate-button"
                                        disabled={processing()}
                                        onClick={() => handleKeepOnlyThis(candidate.id)}
                                    >
                                        Keep Only This
                                    </Button>
                                </div>
                            </div>
                        );
                    }}
                </For>
            </div>
        </div>
    );
};
