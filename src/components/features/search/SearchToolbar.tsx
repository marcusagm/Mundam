import { Component, createSignal, Show, For } from "solid-js";
import { Search, SlidersHorizontal, Filter, X } from "lucide-solid";
import { useFilters, useMetadata } from "../../../core/hooks";
import { SearchGroup } from "../../../core/store/filterStore";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Popover } from "../../ui/Popover";
import { AdvancedSearchModal } from "./AdvancedSearchModal";
import "./search-toolbar.css";

export const SearchToolbar: Component = () => {
    const filters = useFilters();
    const metadata = useMetadata();
    const [isModalOpen, setIsModalOpen] = createSignal(false);

    // Filter helpers
    const activeFiltersList = () => {
        const list: { type: string, label: string, value: string, onRemove: () => void }[] = [];
        
        if (filters.selectedTags.length > 0) {
            filters.selectedTags.forEach(id => {
                const tag = metadata.tags.find(t => t.id === id);
                if (tag) {
                    list.push({
                        type: "tag",
                        label: "Tag",
                        value: tag.name,
                        onRemove: () => filters.toggleTag(id)
                    });
                }
            });
        }

        if (filters.selectedFolderId !== null) {
            const folder = metadata.locations.find(f => f.id === filters.selectedFolderId);
            if (folder) {
                list.push({
                    type: "folder",
                    label: "Folder",
                    value: folder.name,
                    onRemove: () => filters.setFolder(null)
                });
            }
        }

        if (filters.filterUntagged) {
            list.push({
                type: "untagged",
                label: "Filter",
                value: "Untagged",
                onRemove: () => filters.setUntagged(false)
            });
        }

        if (filters.advancedSearch) {
            list.push({
                type: "advanced",
                label: "Advanced",
                value: "Search Criteria Active",
                onRemove: () => filters.setAdvancedSearch(null)
            });
        }

        return list;
    };

    return (
        <div class="search-toolbar">
            <div class="search-input-wrapper">
                <Input 
                    placeholder="Search references (Ctrl+K)" 
                    value={filters.searchQuery}
                    onInput={(e) => filters.setSearch(e.currentTarget.value)}
                    leftIcon={<Search size={14} />}
                    class="search-input"
                />
                <div class="search-actions">
                    <Show when={activeFiltersList().length > 0}>
                        <Popover
                            trigger={
                                <Button 
                                    variant="ghost" 
                                    size="icon" 
                                    class="search-action-btn active"
                                    title="Active Filters"
                                >
                                    <Filter size={14} />
                                </Button>
                            }
                        >
                            <div class="active-filters-popover">
                                <div class="active-filters-header">
                                    <h4>Active Filters</h4>
                                    <Button variant="ghost" size="sm" onClick={() => filters.clearAll()}>Clear All</Button>
                                </div>
                                <div class="active-filters-list">
                                    <For each={activeFiltersList()}>
                                        {(filter) => (
                                            <div class="active-filter-item">
                                                <span class="filter-label">{filter.label}:</span>
                                                <span class="filter-value">{filter.value}</span>
                                                <button 
                                                    class="remove-filter-btn" 
                                                    onClick={filter.onRemove}
                                                >
                                                    <X size={12} />
                                                </button>
                                            </div>
                                        )}
                                    </For>
                                </div>
                            </div>
                        </Popover>
                    </Show>

                    <Button 
                        variant="ghost" 
                        size="icon" 
                        class="search-action-btn" 
                        title="Advanced Search"
                        onClick={() => setIsModalOpen(true)}
                    >
                        <SlidersHorizontal size={14} />
                    </Button>
                </div>
            </div>

            <AdvancedSearchModal 
                isOpen={isModalOpen()} 
                onClose={() => setIsModalOpen(false)} 
                isSmartFolderMode={true}
                onSave={(name: string, query: SearchGroup) => metadata.saveSmartFolder(name, query)}
            />
        </div>
    );
};
