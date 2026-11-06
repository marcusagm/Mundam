import { buildFolderHierarchy, isFolderDescendant } from './folderHierarchy';
import { Component, createMemo, createSignal } from 'solid-js';
import { Folder } from 'lucide-solid';
import { TreeView } from '../../ui/TreeView';
import { useMetadata } from '../../../core/hooks';
import { resolveFolderColor } from './folderPresentation';

/** Controlled destination picker properties. */
export interface FolderDestinationPickerProps {
    /** Selected catalog ID. */
    value: string;
    /** Subtree that must not be offered as a destination. */
    excludedFolderId?: string;
    /** Blocks selection while an operation is pending. */
    isDisabled: boolean;
    /** Receives the selected catalog ID. */
    onSelect: (identifier: string) => void;
}

/** Selects a destination using the accessible shared tree.
 * @param props - Selection, subtree exclusion and interaction state.
 * @returns Folder tree excluding recursive destinations.
 * @example <FolderDestinationPicker value="" isDisabled={false} onSelect={() => {}} />
 */
export const FolderDestinationPicker: Component<FolderDestinationPickerProps> = props => {
    const metadata = useMetadata();
    const [expanded, setExpanded] = createSignal<Set<string | number>>(
        new Set(metadata.locations.filter(folder => !folder.parent_id).map(folder => folder.id))
    );
    const folderIndex = createMemo(
        () => new Map(metadata.locations.map(folder => [folder.id, folder]))
    );
    const nodes = createMemo(() => {
        const folders = metadata.locations.filter(
            folder =>
                !props.excludedFolderId ||
                !isFolderDescendant(folder.id, props.excludedFolderId, folderIndex())
        );
        const index = folderIndex();
        return buildFolderHierarchy(folders, folder => ({
            id: folder.id,
            label: folder.name,
            icon: Folder,
            iconColor: resolveFolderColor(folder.id, index),
            children: []
        }));
    });
    return (
        <div role="group" aria-label="Destination folder" aria-disabled={props.isDisabled}>
            <p>Destination folder</p>
            <div class="folder-destination-tree">
                <TreeView
                    items={nodes()}
                    selectedIds={props.value ? [props.value] : []}
                    expandedIds={expanded()}
                    onToggle={identifier => {
                        if (props.isDisabled) return;
                        setExpanded(previous => {
                            const next = new Set(previous);
                            if (next.has(identifier)) next.delete(identifier);
                            else next.add(identifier);
                            return next;
                        });
                    }}
                    onSelect={node => {
                        if (!props.isDisabled) props.onSelect(String(node.id));
                    }}
                    draggable={false}
                    acceptedDragTypes={[]}
                />
            </div>
            <p aria-live="polite">
                {metadata.locations.find(folder => folder.id === props.value)?.path ||
                    'Choose a folder in the tree.'}
            </p>
        </div>
    );
};
