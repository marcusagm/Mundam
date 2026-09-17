import { Component, Show, For } from 'solid-js';
import { Star, Heart } from 'lucide-solid';
import { Checkbox } from '../../ui/Checkbox';
import { UseMetadataMergeReturn } from './hooks/useMetadataMerge';

interface MetadataMergeSystemFieldsProperties {
    merge: UseMetadataMergeReturn;
}

export const MetadataMergeSystemFields: Component<MetadataMergeSystemFieldsProperties> = props => {
    return (
        <Show when={props.merge.suggestedRating > 0 || props.merge.suggestedIsFavorite}>
            <section class="merge-section">
                <div class="merge-section-header">
                    <Star size={16} />
                    <h3>System Metadata</h3>
                </div>
                <div class="merge-system-fields">
                    <Show when={props.merge.suggestedRating > 0}>
                        <div class="merge-field-row">
                            <div class="merge-field-label">
                                <Checkbox
                                    label="Rating"
                                    checked={props.merge.selectedRating() !== null}
                                    onCheckedChange={(checked: boolean) =>
                                        props.merge.setSelectedRating(
                                            checked ? props.merge.suggestedRating : null
                                        )
                                    }
                                />
                            </div>
                            <div
                                class="merge-rating-control"
                                style={{
                                    opacity: props.merge.selectedRating() === null ? 0.5 : 1
                                }}
                            >
                                <For each={[1, 2, 3, 4, 5]}>
                                    {starValue => (
                                        <button
                                            class={`merge-star ${(props.merge.selectedRating() ?? props.merge.suggestedRating) >= starValue ? 'is-filled' : ''}`}
                                            onClick={() => {
                                                const isCurrent =
                                                    props.merge.selectedRating() === starValue;
                                                props.merge.setSelectedRating(
                                                    isCurrent ? null : starValue
                                                );
                                            }}
                                            aria-label={`Set rating to ${starValue}`}
                                        >
                                            ★
                                        </button>
                                    )}
                                </For>
                            </div>
                        </div>
                    </Show>
                    <Show when={props.merge.suggestedIsFavorite}>
                        <div class="merge-field-row">
                            <div class="merge-field-label">
                                <div class="merge-field-checkbox-container">
                                    <Checkbox
                                        checked={props.merge.selectedIsFavorite() !== null}
                                        onCheckedChange={(checked: boolean) =>
                                            props.merge.setSelectedIsFavorite(checked ? true : null)
                                        }
                                    />
                                    <Heart size={14} />
                                    Favorite
                                </div>
                            </div>
                            <div class="merge-toggle-group">
                                <button
                                    class={`merge-toggle-btn ${props.merge.selectedIsFavorite() === true ? 'is-active' : ''}`}
                                    onClick={() => {
                                        const isCurrent = props.merge.selectedIsFavorite() === true;
                                        props.merge.setSelectedIsFavorite(isCurrent ? null : true);
                                    }}
                                >
                                    Mark as Favorite
                                </button>
                            </div>
                        </div>
                    </Show>
                </div>
            </section>
        </Show>
    );
};
