import { Button, Tooltip } from '../../ui';
import { Component, createSignal, Show, For, createMemo } from 'solid-js';
import { Search, SlidersHorizontal, Funnel, X, Sparkles } from 'lucide-solid';
import { useFilters, useMetadata, useNotification } from '../../../core/hooks';
import { SearchGroup } from '../../../core/store/filter';
import { Input } from '../../ui/Input';
import { Popover } from '../../ui/Popover';
import { AdvancedSearchModal } from './AdvancedSearchModal';
import { createConditionalScope, useCommands, shortcutStore } from '../../../core/input';
import { formatShortcutForDisplay } from '../../../core/input/normalizer';
import { cn } from '../../../lib/utils';
import './search-toolbar.css';

/**
 * Renders the global search toolbar containing text search, filter shortcuts, and advanced search toggles.
 *
 * @returns {JSX.Element} The search toolbar interface.
 *
 * @example
 * ```tsx
 * import { SearchToolbar } from '@/components/features/search/SearchToolbar';
 * <SearchToolbar />
 * ```
 */
export const SearchToolbar: Component = () => {
    /**
     * Filters for the search toolbar.
     */
    const filters = useFilters();

    /**
     * Metadata for the search toolbar.
     */
    const metadata = useMetadata();

    /**
     * Notification for the search toolbar.
     */
    const notification = useNotification();

    /**
     * Modal open state for the search toolbar.
     */
    const [isModalOpen, setIsModalOpen] = createSignal(false);

    /**
     * Focused state for the search toolbar.
     */
    const [isFocused, setIsFocused] = createSignal(false);

    /**
     * Input reference for the search toolbar.
     */
    let inputRef: HTMLInputElement | undefined;

    // Input System - Scope only active when searching or focused
    createConditionalScope('search', () => !!filters.searchQuery || isFocused());

    /**
     * Focus search shortcut for the search toolbar.
     */
    const focusSearchShortcut = createMemo(() =>
        shortcutStore.getByNameAndScope('Focus Search', 'global')
    );

    /**
     * Shortcut label for the search toolbar.
     */
    const shortcutLabel = createMemo(() => {
        const s = focusSearchShortcut();
        if (!s) return 'Cmd+K';
        const keys = Array.isArray(s.keys) ? s.keys[0] : s.keys;
        return formatShortcutForDisplay(keys);
    });

    /**
     * Commands for the search toolbar.
     */
    useCommands({
        'app:focus-search': () => {
            inputRef?.focus();
        },
        'search:close': () => {
            if (
                activeFiltersList().length > 0 ||
                filters.searchQuery ||
                document.activeElement === inputRef
            ) {
                if (filters.searchQuery) {
                    filters.setSearch('');
                } else if (document.activeElement === inputRef) {
                    inputRef?.blur();
                }
            }
        }
    });

    /**
     * Current smart folder for the search toolbar.
     */
    const currentSmartFolder = createMemo(() => {
        if (!filters.advancedSearch) return null;
        const currentJson = JSON.stringify(filters.advancedSearch);
        return metadata.smartFolders.find(sf => sf.query_json === currentJson);
    });

    /**
     * Active filters list for the search toolbar.
     */
    const activeFiltersList = () => {
        const list: { type: string; label: string; value: string; onRemove: () => void }[] = [];

        if (filters.selectedTags.length > 0) {
            filters.selectedTags.forEach(id => {
                const tag = metadata.tags.find(t => t.id === id);
                if (tag) {
                    list.push({
                        type: 'tag',
                        label: 'Tag',
                        value: tag.name,
                        onRemove: () => filters.toggleTag(id)
                    });
                }
            });
        }

        filters.selectedFolderIds.forEach(folderId => {
            const folder = metadata.locations.find(f => f.id === folderId);
            if (folder) {
                list.push({
                    type: 'folder',
                    label: 'Folder',
                    value: folder.name,
                    onRemove: () => filters.selectFolder(folderId, true)
                });
            }
        });

        if (filters.filterUntagged) {
            list.push({
                type: 'untagged',
                label: 'Filter',
                value: 'Untagged',
                onRemove: () => filters.setUntagged(false)
            });
        }

        if (filters.advancedSearch) {
            const smartFolder = currentSmartFolder();

            list.push({
                type: 'advanced',
                label: smartFolder ? 'Smart Folder' : 'Advanced',
                value: smartFolder
                    ? smartFolder.name
                    : `${filters.advancedSearch.items.length} criteria active`,
                onRemove: () => filters.setAdvancedSearch(null)
            });
        }

        return list;
    };

    return (
        <div class="search-toolbar">
            <div class="search-input-wrapper">
                <Input
                    ref={inputRef}
                    placeholder={`Search references (${shortcutLabel()})`}
                    value={filters.searchQuery}
                    onInput={e => filters.setSearch(e.currentTarget.value)}
                    onFocus={() => setIsFocused(true)}
                    onBlur={() => setIsFocused(false)}
                    leftIcon={<Search size={14} />}
                    class="search-input"
                />
                <div class="search-actions">
                    <Show when={activeFiltersList().length > 0}>
                        <Popover
                            placement="bottom-end"
                            trigger={
                                <Tooltip content="Active Filters" placement="bottom">
                                    <Button
                                        variant="ghost"
                                        size="icon-xs"
                                        class="search-action-btn active"
                                    >
                                        <Funnel />
                                    </Button>
                                </Tooltip>
                            }
                        >
                            <div class="active-filters-popover">
                                <div class="active-filters-header">
                                    <h4>Active Filters</h4>
                                    <Button
                                        variant="ghost"
                                        size="xs"
                                        onClick={() => filters.clearAll()}
                                    >
                                        Clear All
                                    </Button>
                                </div>
                                <div class="active-filters-list">
                                    <For each={activeFiltersList()}>
                                        {filter => (
                                            <div class="active-filter-item">
                                                <span class="filter-label">{filter.label}:</span>
                                                <span class="filter-value">{filter.value}</span>
                                                <Button
                                                    variant="ghost"
                                                    size="icon-xs"
                                                    class="remove-filter-btn"
                                                    onClick={filter.onRemove}
                                                >
                                                    <X />
                                                </Button>
                                            </div>
                                        )}
                                    </For>
                                </div>
                            </div>
                        </Popover>
                    </Show>

                    <Tooltip
                        content={filters.searchFuzzy ? 'Fuzzy Match: ON' : 'Fuzzy Match: OFF'}
                        placement="bottom"
                    >
                        <Button
                            variant="ghost"
                            size="icon-xs"
                            class={cn('search-action-btn', !!filters.searchFuzzy && 'active')}
                            onClick={() => filters.setSearchFuzzy(!filters.searchFuzzy)}
                        >
                            <Sparkles />
                        </Button>
                    </Tooltip>

                    <Tooltip content="Advanced Search" placement="bottom">
                        <Button
                            variant="ghost"
                            size="icon-xs"
                            class={cn('search-action-btn', !!filters.advancedSearch && 'active')}
                            onClick={() => setIsModalOpen(true)}
                        >
                            <SlidersHorizontal />
                        </Button>
                    </Tooltip>
                </div>
            </div>

            <AdvancedSearchModal
                isOpen={isModalOpen()}
                onClose={() => setIsModalOpen(false)}
                isSmartFolderMode={!!currentSmartFolder()}
                initialIdentifier={currentSmartFolder()?.id}
                initialName={currentSmartFolder()?.name}
                initialQuery={filters.advancedSearch || undefined}
                onSave={(name: string, query: SearchGroup, id?: string) =>
                    metadata.saveSmartFolder(name, query, id).then(result => {
                        if (result.success) {
                            notification.success(
                                id ? 'Smart Folder Updated' : 'Smart Folder Created',
                                `Saved "${name}"`
                            );
                        } else {
                            notification.error(
                                result.error?.message || 'Failed to Save Smart Folder'
                            );
                        }
                        return result;
                    })
                }
            />
        </div>
    );
};
