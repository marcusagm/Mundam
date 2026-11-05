import { Component, createMemo } from 'solid-js';
import { Plus, Pencil, Palette, Trash2, Check } from 'lucide-solid';
import { ContextMenu, ContextMenuItem } from '../../ui/ContextMenu';
import { TreeNode } from '../../ui/TreeView';
import { ColorPicker } from '../../ui/ColorPicker';
import { useMetadata } from '../../../core/hooks';

interface TagContextMenuProps {
    coordinateX: number;
    coordinateY: number;
    isOpen: boolean;
    node: TreeNode | null;
    onClose: () => void;
    onAddChild: (id: string) => void;
    onRename: (id: string) => void;
    onDelete: (node: TreeNode) => void;
}

/**
 * Context menu for tag tree nodes.
 * Provides actions for adding, renaming, deleting, and changing tag colors.
 *
 * @param {TagContextMenuProps} properties - Component properties.
 * @returns {import('solid-js').JSX.Element} The rendered context menu.
 */
export const TagContextMenu: Component<TagContextMenuProps> = properties => {
    const metadata = useMetadata();

    /**
     * Updates a tag's color and refreshes the metadata.
     *
     * @param {number} tagId - The unique identifier of the tag.
     * @param {string} newColor - The new hexadecimal color code.
     */
    const handleColorChange = async (tagId: string, newColor: string) => {
        await metadata.updateTag(tagId, null, newColor);
    };

    const contextMenuItems = createMemo<ContextMenuItem[]>(() => {
        const treeNode = properties.node;
        if (!treeNode) return [];

        const tag = metadata.tags.find(tag => tag.id === String(treeNode.id));
        const inheritanceItems: ContextMenuItem[] = tag?.parent_id
            ? [
                  {
                      type: 'item',
                      label: 'Use Parent Color',
                      icon: tag.inherits_color ? Check : Palette,
                      action: () => {
                          void metadata.updateTag(tag.id, null, null, undefined, undefined, true);
                      }
                  }
              ]
            : [];
        return [
            ...inheritanceItems,
            {
                type: 'item',
                label: 'Add Child Tag',
                icon: Plus,
                action: () => properties.onAddChild(String(treeNode.id))
            },
            {
                type: 'item',
                label: 'Rename',
                icon: Pencil,
                action: () => properties.onRename(String(treeNode.id))
            },
            {
                type: 'submenu',
                label: 'Change Color',
                icon: Palette,
                items: [
                    {
                        type: 'custom',
                        content: (
                            <ColorPicker
                                color={treeNode.iconColor || '#cccccc'}
                                onChange={newColor =>
                                    handleColorChange(String(treeNode.id), newColor)
                                }
                            />
                        )
                    }
                ]
            },
            { type: 'separator' },
            {
                type: 'item',
                label: 'Delete',
                danger: true,
                icon: Trash2,
                action: () => properties.onDelete(treeNode)
            }
        ];
    });

    return (
        <ContextMenu
            coordinateX={properties.coordinateX}
            coordinateY={properties.coordinateY}
            items={contextMenuItems()}
            isOpen={properties.isOpen}
            onClose={properties.onClose}
        />
    );
};
