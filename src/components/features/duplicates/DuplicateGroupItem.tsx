import { Component, For, Show, createMemo } from 'solid-js';
import { Badge } from '../../ui';
import { Thumbnail } from '../viewport/assets/Thumbnail';
import { getGroupTypeBadgeVariant, formatGroupTypeLabel } from '../../../lib/duplicates';
import { DuplicateGroup } from './types';
import './duplicate-group-item.css';

export interface DuplicateGroupItemProperties {
    /** The duplicate group to render. */
    group: DuplicateGroup;
    /** Whether this item is currently selected. */
    isSelected: boolean;
    /** Callback when the user clicks this item. */
    onSelect: () => void;
}

/**
 * A single item in the duplicate groups sidebar list.
 * Shows a deck preview of thumbnails (like MultiInspector), the group type badge,
 * candidate count, and confidence score.
 */
export const DuplicateGroupItem: Component<DuplicateGroupItemProperties> = props => {
    const isIgnored = () => props.group.status === 'ignored';

    /**
     * Up to 3 candidates for the deck preview, reversed so the first candidate
     * renders on top (highest z-index).
     */
    const previewCandidates = createMemo(() =>
        (props.group.candidates || []).slice(0, 3).reverse()
    );

    return (
        <div
            role="button"
            tabIndex={0}
            onClick={() => props.onSelect()}
            onKeyDown={event => {
                if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    props.onSelect();
                }
            }}
            class={`group-list-item ${props.isSelected ? 'is-selected' : ''} ${isIgnored() ? 'is-ignored' : ''}`}
        >
            <div class="group-list-item-header">
                <div class="group-list-item-badges">
                    <Badge variant={getGroupTypeBadgeVariant(props.group.type)}>
                        {formatGroupTypeLabel(props.group.type)}
                    </Badge>
                    <Show when={isIgnored()}>
                        <Badge variant="secondary">Ignored</Badge>
                    </Show>
                </div>
                <span class="group-list-item-count">{props.group.candidateCount} files</span>
            </div>

            <div class="group-list-item-preview">
                <Show
                    when={props.group.candidates.length > 0}
                    fallback={
                        <div class="group-list-item-thumbnail-placeholder">
                            <span>Select to load {props.group.candidateCount} files</span>
                        </div>
                    }
                >
                    <div class="group-deck-container">
                        <div class="group-deck-wrapper">
                            <For each={previewCandidates()}>
                                {(candidate, index) => (
                                    <div
                                        class="group-deck-card"
                                        style={{
                                            top: `${index() * 3}px`,
                                            left: `${index() * 3}px`,
                                            right: `${(2 - index()) * 3}px`,
                                            bottom: `${(2 - index()) * 3}px`,
                                            transform: `rotate(${(index() - 1) * 3}deg)`,
                                            'z-index': index()
                                        }}
                                    >
                                        <Thumbnail
                                            id={candidate.id}
                                            src={candidate.path}
                                            thumbnail={candidate.thumbnailUrl || null}
                                            alt={candidate.name}
                                            mediaType={candidate.mediaType}
                                            state={candidate.state}
                                        />
                                    </div>
                                )}
                            </For>
                            <div class="group-deck-badge">{props.group.candidateCount}</div>
                        </div>
                    </div>
                    <span class="group-list-item-name">{props.group.candidates[0]?.name}</span>
                </Show>
            </div>

            <div class="group-list-item-confidence">
                Confidence: {(props.group.confidence * 100).toFixed(0)}%
            </div>
        </div>
    );
};
