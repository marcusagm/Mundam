import { Component, createMemo, createSignal } from 'solid-js';
import { Folder } from 'lucide-solid';
import { TreeView, type TreeNode } from '../../ui/TreeView';
import { useMetadata } from '../../../core/hooks';
import { isFolderDescendant, resolveFolderColor } from './folderPresentation';

/** Selects an indexed destination using the existing accessible folder tree. */
export const FolderDestinationPicker: Component<{
    value: string;
    excludedFolderId?: string;
    isDisabled: boolean;
    onSelect: (identifier: string) => void;
}> = props => {
    const metadata = useMetadata();
    const [expanded, setExpanded] = createSignal<Set<string | number>>(
        new Set(metadata.locations.filter(folder => !folder.parent_id).map(folder => folder.id))
    );
    const nodes = createMemo(() => {
        const folders = metadata.locations.filter(
            folder =>
                !props.excludedFolderId ||
                !isFolderDescendant(folder.id, props.excludedFolderId, metadata.locations)
        );
        const nodeMap = new Map<string, TreeNode>();
        const roots: TreeNode[] = [];
        for (const folder of folders)
            nodeMap.set(folder.id, {
                id: folder.id,
                label: folder.name,
                icon: Folder,
                iconColor: resolveFolderColor(folder.id, metadata.locations),
                children: []
            });
        for (const folder of folders) {
            const node = nodeMap.get(folder.id)!;
            const parent = folder.parent_id && nodeMap.get(folder.parent_id);
            if (parent) parent.children!.push(node);
            else roots.push(node);
        }
        return roots;
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
