import { Component, createSignal, createEffect, untrack, Show } from 'solid-js';
import { Info, FileText, Calendar, HardDrive, Heart, Pencil } from 'lucide-solid';
import { AccordionItem, AccordionHeader, AccordionContent, Toggle } from '../../../ui';
import { Button } from '../../../ui/Button';
import {
    assetRenameTarget,
    FolderActionsDialog,
    type FolderDialogTarget
} from '../../library/FolderActionsDialog';
import { StarRating } from './StarRating.tsx';
import { useLibrary } from '../../../../core/hooks';
import { itemActions } from '../../../../core/store/library/itemActions';
import { type AssetItem } from '../../../../types';
import { formatFileSize, formatToDisplay } from '../../../../utils/format';
import { DiscoveryMetadata } from './DiscoveryMetadata';
import './CommonMetadata.css';

/**
 * Interface for CommonMetadata component properties.
 */
interface CommonMetadataProps {
    /**
     * The item to display metadata for.
     */
    item: AssetItem | null;
}

/** Displays shared asset identity, user metadata and filesystem dates. */
// eslint-disable-next-line complexity
export const CommonMetadata: Component<CommonMetadataProps> = props => {
    /**
     * Creates a signal for the notes.
     * @returns The notes signal.
     */
    const [notes, setNotes] = createSignal(untrack(() => props.item?.notes || ''));

    /**
     * Gets the library instance.
     * @returns The library instance.
     */
    const [renameTarget, setRenameTarget] = createSignal<FolderDialogTarget | null>(null);
    const library = useLibrary();

    /**
     * Creates an effect to update the notes when the item changes.
     */
    createEffect(() => {
        setNotes(props.item?.notes || '');
    });

    /**
     * Handles the notes change event.
     * @param value - The new notes value.
     */
    const handleNotesChange = (value: string) => {
        setNotes(value);
        if (props.item) {
            library.updateItemNotes(props.item.id, value);
        }
    };

    /**
     * Handles the rating change event.
     * @param rating - The new rating value.
     */
    const handleRatingChange = (rating: number) => {
        if (props.item) {
            library.updateItemRating(props.item.id, rating);
        }
    };

    /**
     * Renders the CommonMetadata component.
     * @returns The rendered CommonMetadata component.
     */
    return (
        <AccordionItem value="common">
            <AccordionHeader title="General Info" icon={<Info size={14} />} />
            <AccordionContent>
                <div class="inspector-field-group">
                    <span class="inspector-label">Name</span>
                    <Show when={props.item}>
                        {item => (
                            <div class="inspector-filename-row">
                                <span class="inspector-filename">{item().filename}</span>
                                <Show when={item().path && !item().deleted_at}>
                                    <Button
                                        variant="ghost"
                                        size="icon-xs"
                                        aria-label="Rename file"
                                        title="Rename file"
                                        onClick={() => setRenameTarget(assetRenameTarget(item()))}
                                    >
                                        <Pencil size={14} />
                                    </Button>
                                </Show>
                            </div>
                        )}
                    </Show>
                    <FolderActionsDialog
                        target={renameTarget()}
                        onClose={() => setRenameTarget(null)}
                    />
                </div>

                <div class="inspector-grid">
                    <div class="inspector-field-group">
                        <label class="inspector-label">Favorite</label>
                        <Toggle
                            pressed={props.item?.is_favorite || false}
                            onPressedChange={() =>
                                props.item && itemActions.toggleItemFavorite(props.item.id)
                            }
                            variant="outline"
                            size="sm"
                            class="inspector-favorite-toggle"
                        >
                            <Heart
                                size={16}
                                fill={props.item?.is_favorite ? 'currentColor' : 'none'}
                                class={props.item?.is_favorite ? 'favorite-active-icon' : ''}
                            />
                            {props.item?.is_favorite ? 'Favorited' : 'Favorite'}
                        </Toggle>
                    </div>

                    <div class="inspector-field-group">
                        <label class="inspector-label">Rating</label>
                        <div class="inspector-rating-container">
                            <StarRating
                                rating={props.item?.rating || 0}
                                onChange={handleRatingChange}
                            />
                        </div>
                    </div>
                </div>

                <div class="inspector-grid">
                    <div class="inspector-meta-item">
                        <span class="inspector-meta-label">Type</span>
                        <span class="inspector-meta-value inspector-meta-value-type">
                            <FileText size={10} />
                            {props.item?.format || props.item?.filename.split('.').pop()}
                        </span>
                    </div>
                    <div class="inspector-meta-item">
                        <span class="inspector-meta-label">Size</span>
                        <span class="inspector-meta-value">
                            <HardDrive size={10} />
                            {props.item ? formatFileSize(props.item.size) : '-'}
                        </span>
                    </div>
                    <div class="inspector-meta-item">
                        <span class="inspector-meta-label">Created</span>
                        <span class="inspector-meta-value">
                            <Calendar size={10} />
                            {props.item ? formatToDisplay(props.item.created_at) : '-'}
                        </span>
                    </div>
                    <div class="inspector-meta-item">
                        <span class="inspector-meta-label">Modified</span>
                        <span class="inspector-meta-value">
                            <Calendar size={10} />
                            {props.item ? formatToDisplay(props.item.modified_at) : '-'}
                        </span>
                    </div>
                </div>

                <Show when={props.item} keyed>
                    {item => <DiscoveryMetadata item={item} />}
                </Show>
                <div class="inspector-field-group inspector-notes-group">
                    <label class="inspector-label">Notes</label>
                    <textarea
                        class="inspector-notes-input"
                        value={notes()}
                        onInput={event => handleNotesChange(event.currentTarget.value)}
                        placeholder="Add observations..."
                        rows={3}
                    />
                </div>
            </AccordionContent>
        </AccordionItem>
    );
};
