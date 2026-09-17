import { Component, For, Show } from 'solid-js';
import { DuplicateCandidate } from './types';
import { TechnicalFieldEntry } from './hooks/useMetadataMerge';

export const TechnicalFieldRow: Component<{
    field: TechnicalFieldEntry;
    discardedCandidates: DuplicateCandidate[];
    chosenAssetId: string | null;
    onChoiceChange: (assetId: string | null) => void;
}> = rowProperties => {
    return (
        <tr class="merge-table-row">
            <td class="merge-table-cell merge-table-field-name">{rowProperties.field.fieldName}</td>
            <For each={rowProperties.discardedCandidates}>
                {candidate => {
                    const value = rowProperties.field.valuesByAssetId[candidate.id];
                    const isChosen = () => rowProperties.chosenAssetId === candidate.id;
                    return (
                        <td class="merge-table-cell">
                            <Show
                                when={value !== undefined}
                                fallback={<span class="merge-table-empty">—</span>}
                            >
                                <button
                                    class={`merge-table-value-btn ${isChosen() ? 'is-chosen' : ''}`}
                                    onClick={() =>
                                        rowProperties.onChoiceChange(
                                            isChosen() ? null : candidate.id
                                        )
                                    }
                                    title={`Use value from ${candidate.name}`}
                                >
                                    <span class="merge-table-value-text">{value}</span>
                                    <Show when={isChosen()}>
                                        <span class="merge-table-chosen-badge">✓ Selected</span>
                                    </Show>
                                </button>
                            </Show>
                        </td>
                    );
                }}
            </For>
            <td class="merge-table-cell">
                <button
                    class={`merge-table-skip-btn ${rowProperties.chosenAssetId === null ? 'is-active' : ''}`}
                    onClick={() => rowProperties.onChoiceChange(null)}
                >
                    Skip
                </button>
            </td>
        </tr>
    );
};
