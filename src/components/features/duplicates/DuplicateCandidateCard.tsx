import { Component, Show } from 'solid-js';
import { Button, Badge } from '../../ui';
import { CheckCircle2, Trash2 } from 'lucide-solid';
import { Thumbnail } from '../viewport/assets/Thumbnail';
import { DuplicateCandidate } from './types';

/**
 * Properties for the DuplicateCandidateCard component.
 */
export interface DuplicateCandidateCardProperties {
    /** The candidate asset to display. */
    candidate: DuplicateCandidate;
    /** Whether this candidate is currently selected for keeping. */
    isSelected: boolean;
    /** Whether a resolution action is currently in progress. */
    processing: boolean;
    /** Callback to toggle the selection state of this candidate. */
    onToggle: (id: string) => void;
    /** Callback to immediately resolve the group keeping only this candidate. */
    onKeepOnlyThis: (id: string) => void;
}

/**
 * A card representing a single duplicate candidate asset.
 * Renders the asset thumbnail, metadata, selection state, and action buttons.
 * Trashed assets are rendered in a degraded state with no interactive actions.
 *
 * @param {DuplicateCandidateCardProperties} props - Component properties.
 * @returns {JSX.Element} The rendered candidate card.
 *
 * @example
 * ```tsx
 * <DuplicateCandidateCard
 *   candidate={candidate}
 *   isSelected={selectedCandidates().has(candidate.id)}
 *   processing={processing()}
 *   onToggle={toggleCandidate}
 *   onKeepOnlyThis={handleKeepOnlyThis}
 * />
 * ```
 */
export const DuplicateCandidateCard: Component<DuplicateCandidateCardProperties> = props => {
    return (
        <div
            role="checkbox"
            aria-checked={props.isSelected}
            aria-disabled={props.candidate.isTrashed}
            tabIndex={props.candidate.isTrashed ? -1 : 0}
            class={`candidate-card ${props.isSelected ? 'is-selected' : ''} ${props.candidate.isTrashed ? 'is-trashed' : ''}`}
            onClick={() => {
                if (!props.candidate.isTrashed) props.onToggle(props.candidate.id);
            }}
            onKeyDown={event => {
                if (props.candidate.isTrashed) return;
                if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    props.onToggle(props.candidate.id);
                }
            }}
        >
            <div class="candidate-card-body">
                <Show when={props.candidate.isTrashed}>
                    <div class="candidate-trashed-overlay">
                        <Trash2 size={48} />
                        <span>Moved to Trash</span>
                    </div>
                </Show>
                <div class="candidate-header">
                    <h3 class="candidate-name">{props.candidate.name}</h3>
                    <div class="candidate-badges">
                        <Show when={props.isSelected}>
                            <div class="candidate-selected-icon">
                                <CheckCircle2
                                    size={20}
                                    fill="currentColor"
                                    color="var(--bg-secondary)"
                                />
                            </div>
                        </Show>
                        <Show when={props.candidate.isFavorite}>
                            <Badge variant="secondary">Favorite</Badge>
                        </Show>
                        <Badge variant="secondary">
                            {(props.candidate.score * 100).toFixed(0)}%
                        </Badge>
                    </div>
                </div>

                <div class="candidate-preview">
                    <Thumbnail
                        id={props.candidate.id}
                        src={props.candidate.path}
                        thumbnail={props.candidate.thumbnailUrl || null}
                        alt={props.candidate.name}
                        mediaType={props.candidate.mediaType}
                        state={props.candidate.state}
                    />
                </div>

                <div class="candidate-details">
                    <div class="candidate-detail-item full-width">
                        <span class="candidate-detail-label">Path</span>
                        <span class="candidate-detail-value">{props.candidate.path}</span>
                    </div>
                    <div class="candidate-detail-item">
                        <span class="candidate-detail-label">Format</span>
                        <span class="candidate-detail-value">{props.candidate.format}</span>
                    </div>
                    <div class="candidate-detail-item">
                        <span class="candidate-detail-label">Size</span>
                        <span class="candidate-detail-value">{props.candidate.size}</span>
                    </div>
                    <div class="candidate-detail-item">
                        <span class="candidate-detail-label">Dimensions</span>
                        <span class="candidate-detail-value">{props.candidate.dimensions}</span>
                    </div>
                    <div class="candidate-detail-item">
                        <span class="candidate-detail-label">Created</span>
                        <span class="candidate-detail-value">
                            {new Date(props.candidate.createdAt).toLocaleString()}
                        </span>
                    </div>
                    <div class="candidate-detail-item">
                        <span class="candidate-detail-label">Modified</span>
                        <span class="candidate-detail-value">
                            {new Date(props.candidate.updatedAt).toLocaleString()}
                        </span>
                    </div>
                    <Show when={props.candidate.tags && props.candidate.tags.length > 0}>
                        <div class="candidate-detail-item full-width">
                            <span class="candidate-detail-label">Tags</span>
                            <span class="candidate-detail-value">
                                {props.candidate.tags.map(t => t.name).join(', ')}
                            </span>
                        </div>
                    </Show>
                    <Show when={props.candidate.notes}>
                        <div class="candidate-detail-item full-width">
                            <span class="candidate-detail-label">Notes</span>
                            <span class="candidate-detail-value">{props.candidate.notes}</span>
                        </div>
                    </Show>
                </div>
            </div>

            <div class="candidate-actions">
                <Button
                    class="candidate-button"
                    disabled={props.processing || props.candidate.isTrashed}
                    onClick={() => props.onKeepOnlyThis(props.candidate.id)}
                >
                    Keep Only This
                </Button>
            </div>
        </div>
    );
};
