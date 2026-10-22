import { Component, createMemo, createSignal, Show } from 'solid-js';
import {
    ExternalLink,
    FolderOpen,
    Copy,
    Edit2,
    CopyPlus,
    Trash2,
    ArchiveRestore
} from 'lucide-solid';
import { ContextMenu, ContextMenuItem } from '../../../ui/ContextMenu';
import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
import { invokeCommand } from '../../../../lib/api';
import { AssetItem } from '../../../../types';
import { selectionState } from '../../../../core/store/selectionStore';
import { useLibrary } from '../../../../core/hooks/useLibrary';
import { useTrashActions } from '../../../../core/hooks/useTrashActions';
import { useNotification } from '../../../../core/hooks/useNotification';
import { useFilters } from '../../../../core/hooks/useFilters';
import { PromptModal } from '../../../ui/Modal/PromptModal';

/**
 * Interface for Component's Properties of AssetContextMenu.
 *
 * @param coordinateX - Coordinates X of mouse.
 * @param coordinateY - Coordinates Y of mouse.
 * @param isOpen - State of opening the context menu.
 * @param asset - Asset selected.
 * @param onClose - Function of closing the context menu.
 * @returns {JSX.Element} - Componente AssetContextMenu.
 */
export interface AssetContextMenuProps {
    coordinateX: number;
    coordinateY: number;
    isOpen: boolean;
    asset: AssetItem | null;
    onClose: () => void;
}

/**
 * Renderize the context menu of the asset.
 *
 * @param props - Component's Properties.
 * @returns {JSX.Element} - Component AssetContextMenu.
 */
export const AssetContextMenu: Component<AssetContextMenuProps> = props => {
    const { items: allItems, restoreFromTrashAssets } = useLibrary();
    const trash = useTrashActions();
    const notify = useNotification();
    const filters = useFilters();

    const [isRenameModalOpen, setIsRenameModalOpen] = createSignal(false);

    /**
     * Obtains the target assets for the context menu.
     * If the right-clicked asset is part of a multiple selection, returns all selected assets.
     * Otherwise, returns just the right-clicked asset.
     */
    const getTargetAssets = (): AssetItem[] => {
        if (!props.asset) return [];

        // Use string comparison since asset ids can be strings or numbers in selectionState
        const clickedIdStr = props.asset.id.toString();

        if (
            selectionState.selectedIds.includes(clickedIdStr) &&
            selectionState.selectedIds.length > 1
        ) {
            const selected = allItems.filter(i =>
                selectionState.selectedIds.includes(i.id.toString())
            );
            if (selected.length > 0) return selected;
        }

        return [props.asset];
    };

    const handleRestoreAssets = async (assets: AssetItem[], isMulti: boolean) => {
        const ids = assets.map(a => a.id);
        await restoreFromTrashAssets(ids);

        notify.success(
            isMulti ? `${assets.length} items restored` : 'Item restored',
            'Restored items have been moved back to their original locations'
        );
        props.onClose();
    };

    const handleMoveToTrashAssets = async (assets: AssetItem[]) => {
        await trash.moveToTrash(assets.map(a => a.id));
        props.onClose();
    };

    const handleOpenAssets = async (assets: AssetItem[]) => {
        for (const asset of assets) {
            try {
                if (asset.path) {
                    await openPath(asset.path);
                }
            } catch (error) {
                console.error(`Failed to open file ${asset.path}:`, error);
            }
        }
        props.onClose();
    };

    const handleRevealAsset = async (asset: AssetItem) => {
        try {
            if (asset.path) {
                await revealItemInDir(asset.path);
            }
        } catch (error) {
            console.error('Failed to reveal file:', error);
        }
        props.onClose();
    };

    const handleCopyFiles = async (assets: AssetItem[]) => {
        try {
            const paths = assets.map(a => a.path).filter(Boolean) as string[];
            await invokeCommand('copy_files_to_clipboard', { paths });
        } catch (error) {
            console.error('Failed to copy files to clipboard:', error);
        }
        props.onClose();
    };

    const handleCopyPaths = async (assets: AssetItem[]) => {
        try {
            const paths = assets
                .map(a => a.path)
                .filter(Boolean)
                .join('\n');
            await navigator.clipboard.writeText(paths);
        } catch (error) {
            console.error('Failed to copy paths:', error);
        }
        props.onClose();
    };

    // eslint-disable-next-line complexity
    const items = createMemo<ContextMenuItem[]>(() => {
        const assets = getTargetAssets();
        if (assets.length === 0) return [];

        const isMulti = assets.length > 1;

        const menuItems: ContextMenuItem[] = [
            {
                type: 'item',
                label: isMulti ? 'Open files' : 'Open file',
                icon: ExternalLink,
                action: () => handleOpenAssets(assets)
            }
        ];

        // Omit "Reveal in OS" if multiple assets are selected
        if (!isMulti && assets[0].path) {
            menuItems.push({
                type: 'item',
                label: 'Reveal in OS',
                icon: FolderOpen,
                action: () => handleRevealAsset(assets[0])
            });
        }

        menuItems.push({
            type: 'separator'
        });

        // Copy physical files to OS clipboard
        menuItems.push({
            type: 'item',
            label: isMulti ? 'Copy Files' : 'Copy File',
            icon: CopyPlus,
            action: () => handleCopyFiles(assets)
        });

        // Copy paths as text
        menuItems.push({
            type: 'item',
            label: isMulti ? 'Copy Paths' : 'Copy Path',
            icon: Copy,
            action: () => handleCopyPaths(assets)
        });

        menuItems.push({
            type: 'separator'
        });

        if (filters.filterTrash) {
            menuItems.push({
                type: 'item',
                label: isMulti ? 'Restore Files' : 'Restore File',
                icon: ArchiveRestore,
                action: () => handleRestoreAssets(assets, isMulti)
            });
        } else {
            menuItems.push({
                type: 'item',
                label: isMulti ? 'Move to Trash' : 'Move to Trash',
                icon: Trash2,
                action: () => handleMoveToTrashAssets(assets)
            });
        }

        // Rename option is only available for single selection
        if (!isMulti && assets[0].path) {
            menuItems.push({
                type: 'separator'
            });
            menuItems.push({
                type: 'item',
                label: 'Rename',
                icon: Edit2,
                action: () => {
                    setIsRenameModalOpen(true);
                    // Do not close the main context menu state fully yet, let the modal overlay take over
                }
            });
        }

        return menuItems;
    });

    const handleRenameConfirm = async (newName: string) => {
        const asset = props.asset;
        if (!asset || !asset.path) return;

        try {
            // Extrair o diretório do arquivo original e manter a extensão
            const pathParts = asset.path.split('/');
            const oldFilename = pathParts.pop() || '';
            const extensionMatch = oldFilename.match(/\.[^/.]+$/);
            const extension = extensionMatch ? extensionMatch[0] : '';

            // Garantir que a extensão original seja mantida se o usuário não digitar
            const finalNewName = newName.endsWith(extension) ? newName : `${newName}${extension}`;
            const newPath = [...pathParts, finalNewName].join('/');

            // Renomeia o arquivo fisicamente através do nosso próprio backend,
            // driblando os bloqueios de segurança exagerados da API de Frontend do Tauri V2.
            // O indexer do backend processará isso como um evento e atualizará o banco de dados via Heuristics.
            await invokeCommand('rename_file', { oldPath: asset.path, newPath });
        } catch (error) {
            console.error('Failed to rename file:', error);
        }
        props.onClose();
    };

    return (
        <>
            <ContextMenu
                coordinateX={props.coordinateX}
                coordinateY={props.coordinateY}
                items={items()}
                isOpen={props.isOpen && !isRenameModalOpen()}
                onClose={props.onClose}
            />

            <Show when={props.asset}>
                {asset => (
                    <PromptModal
                        isOpen={isRenameModalOpen()}
                        onClose={() => {
                            setIsRenameModalOpen(false);
                            props.onClose();
                        }}
                        onConfirm={handleRenameConfirm}
                        title="Rename Asset"
                        description="Enter the new name for this file. The extension will be preserved."
                        initialValue={asset().filename.replace(/\.[^/.]+$/, '')}
                        placeholder="New filename"
                        required={true}
                    />
                )}
            </Show>
        </>
    );
};
