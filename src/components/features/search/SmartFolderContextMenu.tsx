import { Component, createMemo } from "solid-js";
import { Edit, Trash2 } from "lucide-solid";
import { ContextMenu, ContextMenuItem } from "../../ui/ContextMenu";
import { SmartFolder } from "../../../core/store/metadataStore";

interface SmartFolderContextMenuProps {
    x: number;
    y: number;
    isOpen: boolean;
    folder: SmartFolder | null;
    onClose: () => void;
    onEdit: (folder: SmartFolder) => void;
    onDelete: (folder: SmartFolder) => void;
}

export const SmartFolderContextMenu: Component<SmartFolderContextMenuProps> = (props) => {
    const items = createMemo<ContextMenuItem[]>(() => {
        const folder = props.folder;
        if (!folder) return [];

        return [
            { 
                type: 'item', 
                label: 'Edit Smart Folder', 
                icon: Edit, 
                action: () => props.onEdit(folder) 
            },
            { type: 'separator' },
            {
                type: 'item', 
                label: 'Delete', 
                danger: true, 
                icon: Trash2,
                action: () => props.onDelete(folder)
            }
        ];
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
