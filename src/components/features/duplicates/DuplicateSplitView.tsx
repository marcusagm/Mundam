import { Component, Show } from 'solid-js';
import { Modal, ResizablePanelGroup, ResizablePanel, ResizableHandle } from '../../ui';
import { DuplicateCandidate } from './types';
import { ImageViewer } from '../itemview/renderers/image/ImageViewer';
import { ItemViewProvider } from '../itemview/ItemViewContext';
import './duplicate-split-view.css';

/**
 * Properties for the DuplicateSplitView component.
 */
export interface DuplicateSplitViewProperties {
    /** Whether the split view modal is open. */
    isOpen: boolean;
    /** Callback to close the modal. */
    onClose: () => void;
    /** The first candidate to display on the left panel. */
    candidateA: DuplicateCandidate | null;
    /** The second candidate to display on the right panel. */
    candidateB: DuplicateCandidate | null;
}

/**
 * A full-screen modal for side-by-side visual comparison of two duplicate candidates.
 * Each panel is independently resizable via a drag handle. Images are loaded through
 * the `asset://` custom Tauri protocol for full-resolution preview access.
 *
 * @param {DuplicateSplitViewProperties} props - Component properties.
 * @returns {JSX.Element} The rendered split view modal.
 *
 * @example
 * ```tsx
 * <DuplicateSplitView
 *   isOpen={isSplitViewOpen()}
 *   onClose={() => setIsSplitViewOpen(false)}
 *   candidateA={candidates[0]}
 *   candidateB={candidates[1]}
 * />
 * ```
 */
export const DuplicateSplitView: Component<DuplicateSplitViewProperties> = props => {
    return (
        <Modal
            isOpen={props.isOpen}
            onClose={props.onClose}
            title="Synchronized Split View"
            size="full"
        >
            <div class="duplicate-split-view">
                <ItemViewProvider>
                    <ResizablePanelGroup direction="horizontal" class="split-view-body">
                        <ResizablePanel id="split-a" defaultSize={50}>
                            <Show when={props.candidateA}>
                                {candidateA => (
                                    <div class="split-panel-content">
                                        <div class="split-panel-info">
                                            <span class="split-panel-name">
                                                {candidateA().name}
                                            </span>
                                            <div class="split-panel-meta">
                                                <span>{candidateA().dimensions}</span>
                                                <span>{candidateA().size}</span>
                                            </div>
                                        </div>
                                        <div class="split-panel-image">
                                            <ImageViewer
                                                src={`asset://localhost/${candidateA().id}?type=preview`}
                                                alt={candidateA().name}
                                            />
                                        </div>
                                    </div>
                                )}
                            </Show>
                        </ResizablePanel>
                        <ResizableHandle />
                        <ResizablePanel id="split-b" defaultSize={50}>
                            <Show when={props.candidateB}>
                                {candidateB => (
                                    <div class="split-panel-content">
                                        <div class="split-panel-info">
                                            <span class="split-panel-name">
                                                {candidateB().name}
                                            </span>
                                            <div class="split-panel-meta">
                                                <span>{candidateB().dimensions}</span>
                                                <span>{candidateB().size}</span>
                                            </div>
                                        </div>
                                        <div class="split-panel-image">
                                            <ImageViewer
                                                src={`asset://localhost/${candidateB().id}?type=preview`}
                                                alt={candidateB().name}
                                            />
                                        </div>
                                    </div>
                                )}
                            </Show>
                        </ResizablePanel>
                    </ResizablePanelGroup>
                </ItemViewProvider>
            </div>
        </Modal>
    );
};
