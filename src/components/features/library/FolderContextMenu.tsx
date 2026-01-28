import { Component, createMemo } from "solid-js";
import { Trash2 } from "lucide-solid";
import { ContextMenu, ContextMenuItem } from "../../ui/ContextMenu";
import { Checkbox } from "../../ui/Checkbox";
import { TreeNode } from "../../ui/TreeView";
import { useFilters } from "../../../core/hooks";

interface FolderNodeData {
    folderId: number;
    path: string;
    name: string;
    isRoot: boolean;
}

interface FolderContextMenuProps {
    x: number;
    y: number;
    isOpen: boolean;
    node: TreeNode | null;
    onClose: () => void;
    onDelete: (node: TreeNode) => void;
}

export const FolderContextMenu: Component<FolderContextMenuProps> = (props) => {
    const filters = useFilters();

    const items = createMemo<ContextMenuItem[]>(() => {
        const node = props.node;
        if (!node) return [];

        const data = node.data as FolderNodeData;

        // Custom Item for Recursive View Toggle using Checkbox
        // The Checkbox component handles its own check state visual, 
        // we just need to bind it to the filter state.
        const recursiveViewItem: ContextMenuItem = {
            type: 'custom',
            content: (
                <div 
                    class="ui-context-menu-item" 
                    onClick={(e) => {
                        e.stopPropagation();
                        // This click handler on the wrapper is fallback if Checkbox doesn't capture full row.
                        // But Checkbox is interactive.
                        // Actually, 'custom' item in ContextMenu is rendered inside a wrapper.
                        // We want the whole row to be clickable to toggle.
                        // The Checkbox component has a label prop which renders text.
                    }}
                    style={{ padding: '0 8px', height: '32px', display: 'flex', "align-items": 'center' }}
                >
                    <Checkbox 
                        label="Recursive View" 
                        checked={filters.folderRecursiveView}
                        onCheckedChange={(checked) => {
                            filters.setFolderRecursiveView(checked);
                            // Close menu after toggle? Usually toggle items stay open or close.
                            // Standard context menu toggles usually close.
                            props.onClose(); 
                        }}
                        // Styling to match context menu text
                        class="ui-context-menu-checkbox"
                    />
                </div>
            )
        };

        const menuItems: ContextMenuItem[] = [
            recursiveViewItem,
        ];
        
        // Delete Option for Root folders
        if (data.isRoot) {
             menuItems.push({ type: 'separator' });
             menuItems.push({
                type: 'item',
                label: "Remove Folder",
                icon: Trash2,
                danger: true,
                action: () => props.onDelete(node)
            });
        }
        
        return menuItems;
    });

    return (
        <ContextMenu 
            x={props.x} 
            y={props.y} 
            items={items()} 
            isOpen={props.isOpen} 
            onClose={props.onClose}
        />
    );
};
