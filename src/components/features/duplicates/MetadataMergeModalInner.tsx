import { Component, For, Show, createMemo, untrack } from 'solid-js';
import { FileText, Tag, Database, Loader } from 'lucide-solid';
import { Button } from '../../ui/Button';
import { Checkbox } from '../../ui/Checkbox';
import {
    ModalContent,
    ModalHeader,
    ModalTitle,
    ModalCloseButton,
    ModalBody,
    ModalFooter
} from '../../ui/Modal';
import { MetadataMergeModalProperties } from './MetadataMergeModal';
import { useMetadataMerge } from './hooks/useMetadataMerge';
import { TechnicalFieldRow } from './TechnicalFieldRow';
import { MetadataMergeSystemFields } from './MetadataMergeSystemFields';
import './metadata-merge-modal.css';

export const MetadataMergeModalInner: Component<MetadataMergeModalProperties> = props => {
    // Untrack initial arrays to avoid ESLint warnings about using props outside tracked scope,
    // since useMetadataMerge captures the arrays once on mount.
    const initialCandidates = untrack(() => props.candidates);
    const initialKept = untrack(() => props.keptCandidates);
    const initialDiscarded = untrack(() => props.discardedCandidates);

    const merge = useMetadataMerge(initialCandidates, initialKept, initialDiscarded);

    const keptAssetIds = createMemo(() => props.keptCandidates.map(candidate => candidate.id));

    const hasMergeableContent = createMemo(() => {
        const hasRating = merge.suggestedRating > 0;
        const hasFavorite = merge.suggestedIsFavorite;
        const hasNotes = Boolean(merge.suggestedNotes);
        const hasTags = merge.discardedTagIds.size > 0;
        const hasTechnical = merge.divergentTechnicalFields().length > 0;
        return hasRating || hasFavorite || hasNotes || hasTags || hasTechnical;
    });

    const handleApplyAndResolve = async () => {
        try {
            await merge.applyMergeAndResolve(props.groupId, keptAssetIds(), props.onResolve);
            props.onClose();
        } catch {
            // Error is handled by api.ts global handler
        }
    };

    const handleSkipMerge = async () => {
        try {
            await props.onResolve(props.groupId, 'custom_selection', keptAssetIds());
            props.onClose();
        } catch {
            // Error is handled by api.ts global handler
        }
    };

    return (
        <ModalContent size="xl" class="metadata-merge-modal">
            <ModalHeader>
                <ModalTitle>Merge Metadata</ModalTitle>
                <ModalCloseButton />
            </ModalHeader>
            <ModalBody>
                <p class="merge-modal-subtitle">
                    Review metadata from discarded candidates before resolving. Deselect any fields
                    you want to skip.
                </p>
                <div class="merge-modal-kept-list">
                    <span class="merge-modal-kept-label">Keeping:</span>
                    <For each={props.keptCandidates}>
                        {candidate => <span class="merge-modal-kept-tag">{candidate.name}</span>}
                    </For>
                </div>

                <div class="merge-modal-body">
                    <Show
                        when={!merge.isLoadingExif()}
                        fallback={
                            <div class="merge-modal-loading">
                                <Loader size={24} class="merge-spinner" />
                                <span>Reading metadata from all candidates…</span>
                            </div>
                        }
                    >
                        <Show
                            when={hasMergeableContent()}
                            fallback={
                                <div class="merge-modal-empty">
                                    <p>No metadata differences found between candidates.</p>
                                </div>
                            }
                        >
                            <div class="merge-sections">
                                <MetadataMergeSystemFields merge={merge} />

                                <Show when={merge.suggestedNotes}>
                                    <section class="merge-section">
                                        <div class="merge-section-header">
                                            <FileText size={16} />
                                            <div class="merge-field-checkbox-container">
                                                <Checkbox
                                                    checked={merge.selectedNotes() !== null}
                                                    onCheckedChange={(checked: boolean) =>
                                                        merge.setSelectedNotes(
                                                            checked
                                                                ? (merge.suggestedNotes ?? null)
                                                                : null
                                                        )
                                                    }
                                                />
                                                <h3>Notes</h3>
                                            </div>
                                        </div>
                                        <textarea
                                            class="merge-notes-textarea"
                                            value={
                                                merge.selectedNotes() ?? merge.suggestedNotes ?? ''
                                            }
                                            onInput={event => {
                                                const val = (event.target as HTMLTextAreaElement)
                                                    .value;
                                                merge.setSelectedNotes(val || null);
                                            }}
                                            rows={4}
                                            placeholder="Notes to merge into kept asset…"
                                            disabled={merge.selectedNotes() === null}
                                            style={{
                                                opacity: merge.selectedNotes() === null ? 0.5 : 1
                                            }}
                                        />
                                    </section>
                                </Show>

                                <Show when={merge.discardedTagIds.size > 0}>
                                    <section class="merge-section">
                                        <div class="merge-section-header">
                                            <Tag size={16} />
                                            <h3>Tags from Discarded Candidates</h3>
                                            <span class="merge-section-hint">
                                                Deselect tags you don't want to carry over.
                                            </span>
                                        </div>
                                        <div class="merge-tags-grid">
                                            <For
                                                each={Array.from(
                                                    new Map(
                                                        props.discardedCandidates
                                                            .flatMap(c => c.tags)
                                                            .map(tag => [tag.id, tag])
                                                    ).values()
                                                )}
                                            >
                                                {tag => {
                                                    const isSelected = () =>
                                                        merge.selectedTagIds().has(tag.id);
                                                    return (
                                                        <Checkbox
                                                            label={tag.name}
                                                            checked={isSelected()}
                                                            onCheckedChange={() =>
                                                                merge.toggleTagId(tag.id)
                                                            }
                                                        />
                                                    );
                                                }}
                                            </For>
                                        </div>
                                    </section>
                                </Show>

                                <Show when={merge.divergentTechnicalFields().length > 0}>
                                    <section class="merge-section">
                                        <div class="merge-section-header">
                                            <Database size={16} />
                                            <h3>Technical Metadata (EXIF / XMP / IPTC)</h3>
                                            <span class="merge-section-hint">
                                                Fields where candidates have different values. Click
                                                a value to select it.
                                            </span>
                                        </div>
                                        <div class="merge-table-wrapper">
                                            <table class="merge-table">
                                                <thead>
                                                    <tr>
                                                        <th class="merge-table-th">Field</th>
                                                        <For each={props.discardedCandidates}>
                                                            {candidate => (
                                                                <th class="merge-table-th merge-table-th--source">
                                                                    From: {candidate.name}
                                                                </th>
                                                            )}
                                                        </For>
                                                        <th class="merge-table-th">Action</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    <For each={merge.divergentTechnicalFields()}>
                                                        {field => (
                                                            <TechnicalFieldRow
                                                                field={field}
                                                                discardedCandidates={
                                                                    props.discardedCandidates
                                                                }
                                                                chosenAssetId={
                                                                    merge.technicalFieldChoices()[
                                                                        field.fieldName
                                                                    ]?.chosenAssetId ?? null
                                                                }
                                                                onChoiceChange={assetId =>
                                                                    merge.setTechnicalFieldChoice(
                                                                        field.fieldName,
                                                                        {
                                                                            chosenAssetId: assetId
                                                                        }
                                                                    )
                                                                }
                                                            />
                                                        )}
                                                    </For>
                                                </tbody>
                                            </table>
                                        </div>
                                    </section>
                                </Show>
                            </div>
                        </Show>
                    </Show>
                </div>
            </ModalBody>
            <ModalFooter>
                <Button variant="secondary" onClick={handleSkipMerge} disabled={merge.isApplying()}>
                    Skip Merge & Resolve
                </Button>
                <Button
                    variant="primary"
                    onClick={handleApplyAndResolve}
                    disabled={merge.isApplying() || merge.isLoadingExif()}
                >
                    <Show when={merge.isApplying()} fallback="Apply Merge & Resolve">
                        <Loader size={14} class="merge-spinner merge-spinner--btn" />
                        Applying…
                    </Show>
                </Button>
            </ModalFooter>
        </ModalContent>
    );
};
