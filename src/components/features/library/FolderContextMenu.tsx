import { Component, createMemo } from 'solid-js';
import { Trash2, FolderPlus, Edit2, FolderInput, Palette, FolderOpen, Copy } from 'lucide-solid';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { ContextMenu, ContextMenuItem, ContextMenuCheckboxItem } from '../../ui/ContextMenu';
import { TreeNode } from '../../ui/TreeView';
import { ColorPicker } from '../../ui/ColorPicker';
import { resolveFolderColor } from './folderPresentation';
import { executeLibraryOperation } from '../../../core/store/library/folderActions';
import { useFilters, useMetadata } from '../../../core/hooks';
import { FolderDialogAction } from './FolderActionsDialog';

/** Actions for indexed folders; unlinking roots remains distinct from deletion. */
export const FolderContextMenu: Component<{
    coordinateX: number;
    coordinateY: number;
    isOpen: boolean;
    node: TreeNode | null;
    onClose: () => void;
    onDelete: (node: TreeNode) => void;
    onRename: (id: string) => void;
    onAddChild: (id: string) => void;
    onAction: (
        action: Extract<FolderDialogAction, 'moveFolder' | 'deleteFolder'>,
        folderId: string
    ) => void;
}> = props => {
    const filters = useFilters();
    const metadata = useMetadata();
    const items = createMemo<ContextMenuItem[]>(() => {
        const node = props.node;
        if (!node) return [];
        const data = node.data as { folderIdentifier: string; path: string; isRoot: boolean };
        const folder = metadata.locations.find(folder => folder.id === data.folderIdentifier);
        const updateColor = (color: string | null, inheritsColor: boolean) => {
            void executeLibraryOperation({
                kind: 'folderColor',
                folderId: data.folderIdentifier,
                color,
                inheritsColor
            }).catch(() => undefined);
        };
        const onAction = props.onAction;
        const action = (kind: Extract<FolderDialogAction, 'moveFolder' | 'deleteFolder'>) => () =>
            onAction(kind, data.folderIdentifier);
        const result: ContextMenuItem[] = [
            {
                type: 'item',
                label: 'Create Subfolder',
                icon: FolderPlus,
                action: () => props.onAddChild(data.folderIdentifier)
            },
            ...(folder?.parent_id
                ? [
                      {
                          type: 'custom' as const,
                          content: (
                              <ContextMenuCheckboxItem
                                  label="Use Parent Color"
                                  checked={folder.inherits_color ?? true}
                                  onCheckedChange={checked =>
                                      updateColor(folder.color || null, checked)
                                  }
                                  onClose={props.onClose}
                              />
                          )
                      }
                  ]
                : []),
            {
                type: 'submenu',
                label: 'Change Color',
                icon: Palette,
                items: [
                    {
                        type: 'custom',
                        content: (
                            <ColorPicker
                                color={
                                    resolveFolderColor(data.folderIdentifier, metadata.locations) ||
                                    '#cccccc'
                                }
                                onChange={color => updateColor(color, false)}
                            />
                        )
                    }
                ]
            },
            ...(!folder?.inherits_color && folder?.color
                ? [
                      {
                          type: 'item' as const,
                          label: 'Reset Color',
                          icon: Palette,
                          action: () => updateColor(null, false)
                      }
                  ]
                : []),
            {
                type: 'item',
                label: 'Reveal in OS',
                icon: FolderOpen,
                action: () => {
                    void revealItemInDir(data.path).catch(console.error);
                }
            },
            {
                type: 'item',
                label: 'Copy Path',
                icon: Copy,
                action: () => {
                    void writeText(data.path).catch(console.error);
                }
            }
        ];
        result.push(
            {
                type: 'item',
                label: 'Rename Folder',
                icon: Edit2,
                action: () => props.onRename(String(node.id))
            },
            {
                type: 'item',
                label: 'Move Folder',
                icon: FolderInput,
                action: action('moveFolder')
            }
        );
        if (!data.isRoot)
            result.push({
                type: 'item',
                label: 'Delete Folder',
                icon: Trash2,
                danger: true,
                action: action('deleteFolder')
            });
        result.push(
            { type: 'separator' },
            {
                type: 'custom',
                content: (
                    <ContextMenuCheckboxItem
                        label="Recursive View"
                        checked={filters.folderRecursiveView}
                        onCheckedChange={filters.setFolderRecursiveView}
                        onClose={props.onClose}
                    />
                )
            }
        );
        if (data.isRoot)
            result.push(
                { type: 'separator' },
                {
                    type: 'item',
                    label: 'Unlink Folder',
                    icon: Trash2,
                    danger: true,
                    action: () => props.onDelete(node)
                }
            );
        return result;
    });
    return (
        <ContextMenu
            coordinateX={props.coordinateX}
            coordinateY={props.coordinateY}
            items={items()}
            isOpen={props.isOpen}
            onClose={props.onClose}
        />
    );
};
