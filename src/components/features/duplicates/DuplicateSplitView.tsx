import { Component, Show, Switch, Match, createSignal } from 'solid-js';
import {
    Modal,
    Button,
    Slider,
    ResizablePanelGroup,
    ResizablePanel,
    ResizableHandle
} from '../../ui';
import { DuplicateCandidate } from './types';
import { ImageViewer } from '../itemview/renderers/image/ImageViewer';
import { ItemViewProvider } from '../itemview/ItemViewContext';
import { DuplicateOverlayView } from './DuplicateOverlayView';
import './duplicate-split-view.css';

/**
 * Supported comparison visualization modes for the split view.
 */
export type SplitComparisonMode = 'split' | 'wipe' | 'difference' | 'onion';

/**
 * Properties for the DuplicateSplitView component.
 */
export interface DuplicateSplitViewProperties {
    /** Whether the split view modal is open. */
    isOpen: boolean;
    /** Callback to close the modal. */
    onClose: () => void;
    /** The first candidate to display on the left or base layer. */
    candidateA: DuplicateCandidate | null;
    /** The second candidate to display on the right or overlay layer. */
    candidateB: DuplicateCandidate | null;
}

/**
 * A full-screen comparison modal supporting side-by-side split, wipe curtain slider,
 * difference highlight, and onion-skin opacity blending between two duplicate candidates.
 *
 * @param {DuplicateSplitViewProperties} props - Component properties.
 * @returns {JSX.Element} The rendered split view modal.
 */
export const DuplicateSplitView: Component<DuplicateSplitViewProperties> = props => {
    const [comparisonMode, setComparisonMode] = createSignal<SplitComparisonMode>('split');
    const [wipeCurtainPercentage, setWipeCurtainPercentage] = createSignal<number>(50);
    const [onionSkinOpacityPercentage, setOnionSkinOpacityPercentage] = createSignal<number>(50);
    const [isDifferenceInverted, setIsDifferenceInverted] = createSignal<boolean>(false);

    return (
        <Modal
            isOpen={props.isOpen}
            onClose={props.onClose}
            title="Visual Comparison & Overlay"
            size="full"
        >
            <div class="duplicate-split-view">
                <div class="split-view-toolbar">
                    <div class="split-view-modes">
                        <Button
                            size="sm"
                            variant={comparisonMode() === 'split' ? 'primary' : 'ghost'}
                            onClick={() => setComparisonMode('split')}
                        >
                            Side by Side
                        </Button>
                        <Button
                            size="sm"
                            variant={comparisonMode() === 'wipe' ? 'primary' : 'ghost'}
                            onClick={() => setComparisonMode('wipe')}
                        >
                            Wipe Curtain
                        </Button>
                        <Button
                            size="sm"
                            variant={comparisonMode() === 'difference' ? 'primary' : 'ghost'}
                            onClick={() => setComparisonMode('difference')}
                        >
                            Difference Highlight
                        </Button>
                        <Button
                            size="sm"
                            variant={comparisonMode() === 'onion' ? 'primary' : 'ghost'}
                            onClick={() => setComparisonMode('onion')}
                        >
                            Onion Skin
                        </Button>
                    </div>

                    <div class="split-view-mode-controls">
                        <Show when={comparisonMode() === 'wipe'}>
                            <div class="split-slider-control">
                                <span class="split-control-label">
                                    Curtain: {wipeCurtainPercentage()}%
                                </span>
                                <Slider
                                    minimumValue={0}
                                    maximumValue={100}
                                    stepValue={1}
                                    value={wipeCurtainPercentage()}
                                    onValueChange={setWipeCurtainPercentage}
                                />
                            </div>
                        </Show>

                        <Show when={comparisonMode() === 'onion'}>
                            <div class="split-slider-control">
                                <span class="split-control-label">
                                    Opacity: {onionSkinOpacityPercentage()}%
                                </span>
                                <Slider
                                    minimumValue={0}
                                    maximumValue={100}
                                    stepValue={1}
                                    value={onionSkinOpacityPercentage()}
                                    onValueChange={setOnionSkinOpacityPercentage}
                                />
                            </div>
                        </Show>

                        <Show when={comparisonMode() === 'difference'}>
                            <div class="split-difference-info">
                                <span class="split-difference-hint">
                                    Identical pixels appear black; altered pixels are highlighted.
                                </span>
                                <Button
                                    size="xs"
                                    variant={isDifferenceInverted() ? 'primary' : 'outline'}
                                    onClick={() => setIsDifferenceInverted(previous => !previous)}
                                >
                                    {isDifferenceInverted() ? 'Inverted' : 'Standard'}
                                </Button>
                            </div>
                        </Show>
                    </div>
                </div>

                <div class="split-view-body">
                    <Switch>
                        <Match when={comparisonMode() === 'split'}>
                            <ItemViewProvider>
                                <ResizablePanelGroup direction="horizontal">
                                    <ResizablePanel id="split-panel-a" defaultSize={50}>
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
                                    <ResizablePanel id="split-panel-b" defaultSize={50}>
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
                        </Match>

                        <Match when={comparisonMode() !== 'split'}>
                            <Show when={props.candidateA && props.candidateB}>
                                <DuplicateOverlayView
                                    candidateA={props.candidateA!}
                                    candidateB={props.candidateB!}
                                    comparisonMode={
                                        comparisonMode() as 'wipe' | 'difference' | 'onion'
                                    }
                                    wipeCurtainPercentage={wipeCurtainPercentage()}
                                    onWipeCurtainChange={setWipeCurtainPercentage}
                                    onionSkinOpacityPercentage={onionSkinOpacityPercentage()}
                                    isDifferenceInverted={isDifferenceInverted()}
                                />
                            </Show>
                        </Match>
                    </Switch>
                </div>
            </div>
        </Modal>
    );
};
