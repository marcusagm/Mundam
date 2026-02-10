import { Component, createMemo, Switch, Match } from 'solid-js';
import { useLibrary, useSelection } from '../../core/hooks';
import { Info } from 'lucide-solid';
import { ImageInspector } from '../features/inspector/image/ImageInspector';
import { AudioInspector } from '../features/inspector/audio/AudioInspector';
import { VideoInspector } from '../features/inspector/video/VideoInspector';
import { FontInspector } from '../features/inspector/font/FontInspector';
import { ModelInspector } from '../features/inspector/model/ModelInspector';
import { MultiInspector } from '../features/inspector/multi/MultiInspector';
import { getMediaType } from '../features/inspector/utils';
import './file-inspector.css';

export const FileInspector: Component = () => {
    const lib = useLibrary();
    const selection = useSelection();

    const selectionCount = createMemo(() => selection.selectedIds.length);

    const activeItem = createMemo(() => {
        if (selectionCount() === 0) return null;
        const ids = selection.selectedIds;
        const id = ids[ids.length - 1]; // Last selected
        return lib.items.find((i: { id: number }) => i.id === id) || null;
    });

    const selectedItems = createMemo(() => {
        return lib.items.filter((i: { id: number }) => selection.selectedIds.includes(i.id));
    });

    const fileType = createMemo(() => {
        const item = activeItem();
        if (!item) return 'unknown';
        return getMediaType(item.filename);
    });

    return (
        <div class="inspector-container">
            <div class="inspector-header">
                <span>Inspector</span>
                <span class="inspector-selection-badge">
                    {selectionCount() > 0 ? selectionCount() : 0}
                </span>
            </div>

            <Switch
                fallback={
                    <div class="inspector-empty-state">
                        <div class="empty-icon-circle">
                            <Info size={32} />
                        </div>
                        <h3>No Selection</h3>
                        <p>Select a file to view its details and metadata.</p>
                    </div>
                }
            >
                <Match when={selectionCount() > 1}>
                    <MultiInspector items={selectedItems()} />
                </Match>

                <Match when={selectionCount() === 1 && activeItem()}>
                    <Switch fallback={<ImageInspector item={activeItem()!} />}>
                        <Match when={fileType() === 'audio'}>
                            <AudioInspector item={activeItem()!} />
                        </Match>
                        <Match when={fileType() === 'video'}>
                            <VideoInspector item={activeItem()!} />
                        </Match>
                        <Match when={fileType() === 'font'}>
                            <FontInspector item={activeItem()!} />
                        </Match>
                        <Match when={fileType() === 'model3d'}>
                            <ModelInspector item={activeItem()!} />
                        </Match>
                        <Match when={fileType() === 'image'}>
                            <ImageInspector item={activeItem()!} />
                        </Match>
                    </Switch>
                </Match>
            </Switch>
        </div>
    );
};
