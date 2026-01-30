import { Component, For, Show, createSignal } from "solid-js";
import { FolderHeart } from "lucide-solid";
import { useMetadata, useFilters } from "../../../core/hooks";
import { SidebarPanel } from "../../ui/SidebarPanel";
import { SmartFolderContextMenu } from "./SmartFolderContextMenu";
import { AdvancedSearchModal } from "./AdvancedSearchModal";
import { SmartFolderDeleteModal } from "./SmartFolderDeleteModal";
import { SearchGroup } from "../../../core/store/filterStore";
import { SmartFolder } from "../../../core/store/metadataStore";
import { cn } from "../../../lib/utils";
import "./smart-folders.css";

export const SmartFoldersSidebarPanel: Component = () => {
    const metadata = useMetadata();
    const filters = useFilters();

    // Context Menu State
    const [contextMenuOpen, setContextMenuOpen] = createSignal(false);
    const [contextMenuPos, setContextMenuPos] = createSignal({ x: 0, y: 0 });
    const [selectedFolder, setSelectedFolder] = createSignal<SmartFolder | null>(null);

    // Edit Modal State
    const [isEditModalOpen, setIsEditModalOpen] = createSignal(false);
    const [folderToEdit, setFolderToEdit] = createSignal<SmartFolder | null>(null);

    // Delete Modal State
    const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);
    const [folderToDelete, setFolderToDelete] = createSignal<SmartFolder | null>(null);

    const handleSelect = (json: string) => {
        if (isActive(json)) {
            filters.setAdvancedSearch(null);
            return;
        }

        try {
            const query = JSON.parse(json) as SearchGroup;
            filters.setAdvancedSearch(query);
        } catch (e) {
            console.error("Failed to parse smart folder query", e);
        }
    };

    const handleContextMenu = (e: MouseEvent, folder: SmartFolder) => {
        e.preventDefault();
        setContextMenuPos({ x: e.clientX, y: e.clientY });
        setSelectedFolder(folder);
        setContextMenuOpen(true);
    };

    // const handleOpenContextMenuBtn = (e: MouseEvent, folder: SmartFolder) => {
    //     e.stopPropagation();
    //     const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    //     setContextMenuPos({ x: rect.left, y: rect.bottom });
    //     setSelectedFolder(folder);
    //     setContextMenuOpen(true);
    // };

    const handleEdit = (folder: SmartFolder) => {
        setFolderToEdit(folder);
        setIsEditModalOpen(true);
    };

    const handleDeleteClick = (folder: SmartFolder) => {
        setFolderToDelete(folder);
        setIsDeleteModalOpen(true);
    };

    const isActive = (json: string) => {
        if (!filters.advancedSearch) return false;
        return JSON.stringify(filters.advancedSearch) === json;
    };

    return (
        <SidebarPanel title="Smart Folders" class="panel-smart-folders">
            <div class="smart-folders-list">
                <Show when={metadata.smartFolders.length === 0}>
                    <div class="smart-folders-empty">
                        No smart folders yet. Create one in Advanced Search!
                    </div>
                </Show>
                <For each={metadata.smartFolders}>
                    {(folder) => (
                        <div 
                            class={cn("nav-item smart-folder-item", isActive(folder.query_json) && "active")}
                            onClick={() => handleSelect(folder.query_json)}
                            onContextMenu={(e) => handleContextMenu(e, folder)}
                        >
                            <FolderHeart size={16} />
                            <span class="folder-name">{folder.name}</span>
                            
                            {/* <button 
                                class="icon-btn mini-btn" 
                                onClick={(e) => handleOpenContextMenuBtn(e, folder)}
                            >
                                <MoreVertical size={14} />
                            </button> */}
                        </div>
                    )}
                </For>
            </div>

            <SmartFolderContextMenu 
                x={contextMenuPos().x} 
                y={contextMenuPos().y} 
                isOpen={contextMenuOpen()} 
                folder={selectedFolder()}
                onClose={() => setContextMenuOpen(false)}
                onEdit={handleEdit}
                onDelete={handleDeleteClick}
            />

            <SmartFolderDeleteModal 
                isOpen={isDeleteModalOpen()}
                onClose={() => setIsDeleteModalOpen(false)}
                folder={folderToDelete()}
            />

            <Show when={isEditModalOpen()}>
                <AdvancedSearchModal 
                    isOpen={isEditModalOpen()} 
                    onClose={() => setIsEditModalOpen(false)}
                    isSmartFolderMode={true}
                    initialId={folderToEdit()?.id}
                    initialName={folderToEdit()?.name}
                    initialQuery={folderToEdit() ? JSON.parse(folderToEdit()!.query_json) : undefined}
                    onSave={(name, query, id) => metadata.saveSmartFolder(name, query, id)}
                />
            </Show>
        </SidebarPanel>
    );
};

