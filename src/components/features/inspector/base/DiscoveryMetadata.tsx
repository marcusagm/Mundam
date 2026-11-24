import { Component, Show, createSignal, untrack, createEffect } from 'solid-js';
import { Check, Pencil, X } from 'lucide-solid';
import { openUrl } from '@tauri-apps/plugin-opener';
import { Button, Input, TagInput } from '../../../ui';
import { itemActions } from '../../../../core/store/library/itemActions';
import { type AssetItem } from '../../../../types';
import './discovery-metadata.css';

/** Asset identity remains stable while either inline editor has a pending request. */
interface DiscoveryMetadataProperties {
    /** Selected catalog projection. */
    item: AssetItem;
}

/** Shared inspector fields for user-authored search terms and asset provenance. */
export const DiscoveryMetadata: Component<DiscoveryMetadataProperties> = properties => {
    const [isSavingKeywords, setIsSavingKeywords] = createSignal(false);
    const [keywordError, setKeywordError] = createSignal('');
    const [keywords, setKeywords] = createSignal(untrack(() => properties.item.keywords || []));
    let pendingKeywords: string[] | undefined;

    createEffect(() => {
        if (!isSavingKeywords() && !keywordError()) {
            setKeywords(properties.item.keywords || []);
        }
    });

    /** Coalesces rapid edits while preserving input focus and the latest local draft. */
    const handleKeywordsChange = async (nextKeywords: string[]): Promise<void> => {
        setKeywords(nextKeywords);
        pendingKeywords = nextKeywords;
        if (isSavingKeywords()) return;
        setIsSavingKeywords(true);
        setKeywordError('');
        try {
            while (pendingKeywords) {
                const replacement = pendingKeywords;
                pendingKeywords = undefined;
                const asset = await itemActions.updateItemDiscovery(properties.item.id, {
                    keywords: replacement
                });
                if (!pendingKeywords) setKeywords(asset.keywords || []);
            }
        } catch {
            pendingKeywords = keywords();
            setKeywordError('Keywords could not be saved. Please try again.');
        } finally {
            setIsSavingKeywords(false);
        }
    };

    return (
        <>
            <div class="inspector-field-group" aria-busy={isSavingKeywords()}>
                <span class="inspector-label">Keywords</span>
                <TagInput
                    ariaLabel="Asset keywords"
                    allowCustomValues
                    value={keywords().map(keyword => ({
                        id: keyword.toLowerCase(),
                        label: keyword
                    }))}
                    onChange={keywords =>
                        void handleKeywordsChange(keywords.map(keyword => keyword.label))
                    }
                    placeholder="Add keywords, separated by commas"
                />
                <p class="inspector-discovery-hint">
                    <Show
                        when={isSavingKeywords()}
                        fallback="Separate keywords with a comma or press Enter."
                    >
                        <span role="status">Saving keywords...</span>
                    </Show>
                </p>
                <Show when={keywordError()}>
                    <p class="inspector-discovery-error" role="alert">
                        {keywordError()}
                    </p>
                    <Button
                        variant="ghost"
                        size="xs"
                        onClick={() => void handleKeywordsChange(keywords())}
                    >
                        Retry saving keywords
                    </Button>
                </Show>
            </div>
            <SourceUrlField item={properties.item} />
        </>
    );
};

/** Inline origin editor preserving the current link until persistence succeeds. */
const SourceUrlField: Component<DiscoveryMetadataProperties> = properties => {
    const [isEditing, setIsEditing] = createSignal(false);
    const [draft, setDraft] = createSignal('');
    const [isSaving, setIsSaving] = createSignal(false);
    const [error, setError] = createSignal('');

    const handleStartEdit = (): void => {
        setDraft(properties.item.source_url || '');
        setError('');
        setIsEditing(true);
    };

    const handleSave = async (): Promise<void> => {
        const sourceUrl = draft().trim();
        if (sourceUrl && !isValidSourceUrl(sourceUrl)) {
            setError('Enter an HTTP or HTTPS URL without credentials.');
            return;
        }
        setIsSaving(true);
        setError('');
        try {
            await itemActions.updateItemDiscovery(properties.item.id, { sourceUrl });
            setIsEditing(false);
        } catch {
            setError('Source URL could not be saved. Your changes are still here.');
        } finally {
            setIsSaving(false);
        }
    };

    const handleOpen = (event: MouseEvent): void => {
        event.preventDefault();
        const sourceUrl = properties.item.source_url;
        if (sourceUrl && isValidSourceUrl(sourceUrl)) {
            void openUrl(sourceUrl).catch(() => setError('The source link could not be opened.'));
        }
    };

    return (
        <div class="inspector-field-group" aria-busy={isSaving()}>
            <span class="inspector-label">Source URL</span>
            <div class="inspector-source-row">
                <Show
                    when={isEditing()}
                    fallback={
                        <>
                            <Show
                                when={properties.item.source_url}
                                fallback={
                                    <span class="inspector-source-empty">
                                        No source link added yet.
                                    </span>
                                }
                            >
                                <a
                                    class="inspector-source-link"
                                    href={properties.item.source_url!}
                                    onClick={handleOpen}
                                    title={properties.item.source_url!}
                                >
                                    {properties.item.source_url}
                                </a>
                            </Show>
                            <Button
                                variant="ghost"
                                size="icon-xs"
                                onClick={handleStartEdit}
                                aria-label="Edit source URL"
                                title="Edit source URL"
                            >
                                <Pencil size={14} />
                            </Button>
                        </>
                    }
                >
                    <Input
                        value={draft()}
                        onInput={event => setDraft(event.currentTarget.value)}
                        aria-label="Source URL"
                        placeholder="https://example.com/reference"
                        type="url"
                        size="sm"
                        autofocus
                        disabled={isSaving()}
                        wrapperClass="inspector-source-input"
                        error={!!error()}
                        onKeyDown={event => {
                            if (event.isComposing || isSaving()) return;
                            if (event.key === 'Enter') {
                                event.preventDefault();
                                void handleSave();
                            }
                            if (event.key === 'Escape') {
                                event.stopPropagation();
                                setIsEditing(false);
                                setError('');
                            }
                        }}
                    />
                    <Button
                        variant="primary"
                        size="icon-xs"
                        disabled={isSaving()}
                        onClick={() => void handleSave()}
                        aria-label="Save source URL"
                        title="Save source URL"
                    >
                        <Check size={14} />
                    </Button>
                    <Button
                        variant="ghost"
                        size="icon-xs"
                        disabled={isSaving()}
                        onClick={() => {
                            setIsEditing(false);
                            setError('');
                        }}
                        aria-label="Cancel source URL editing"
                        title="Cancel editing"
                    >
                        <X size={14} />
                    </Button>
                </Show>
            </div>
            <Show when={error()}>
                <p class="inspector-discovery-error" role="alert">
                    {error()}
                </p>
            </Show>
        </div>
    );
};

/** Accepts browser page origins while preventing executable schemes and embedded credentials. */
function isValidSourceUrl(value: string): boolean {
    try {
        const parsed = new URL(value);
        return (
            ['http:', 'https:'].includes(parsed.protocol) &&
            !!parsed.hostname &&
            !parsed.username &&
            !parsed.password
        );
    } catch {
        return false;
    }
}
