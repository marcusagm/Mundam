import { Component, Show, createSignal, createMemo, onMount, onCleanup } from 'solid-js';
import { DuplicateCandidate } from './types';
import './duplicate-split-view.css';

/**
 * Properties for the DuplicateOverlayView component.
 */
export interface DuplicateOverlayViewProperties {
    /** The active comparison overlay mode. */
    comparisonMode: 'wipe' | 'difference' | 'onion';
    /** The percentage position of the wipe curtain (0 - 100). */
    wipeCurtainPercentage: number;
    /** Callback to update the wipe curtain position. */
    onWipeCurtainChange: (percentage: number) => void;
    /** The opacity percentage for onion skin blending (0 - 100). */
    onionSkinOpacityPercentage: number;
    /** Whether the difference highlight color mode is inverted. */
    isDifferenceInverted: boolean;
    /** The first candidate asset (base layer). */
    candidateA: DuplicateCandidate;
    /** The second candidate asset (target layer). */
    candidateB: DuplicateCandidate;
}

/**
 * Calculates the bounding width and height for the overlay container
 * so that both images occupy the identical aspect ratio frame.
 *
 * @param {number} availableWidth - Available stage width in pixels.
 * @param {number} availableHeight - Available stage height in pixels.
 * @param {number} imageAspectRatio - Aspect ratio (width / height) of the image.
 * @returns {{ width: number; height: number }} Bounding frame dimensions.
 */
function calculateOverlayFrameDimensions(
    availableWidth: number,
    availableHeight: number,
    imageAspectRatio: number
): { width: number; height: number } {
    const stageAspectRatio = availableWidth / availableHeight;
    if (stageAspectRatio > imageAspectRatio) {
        return {
            width: Math.round(availableHeight * imageAspectRatio),
            height: Math.round(availableHeight)
        };
    }
    return {
        width: Math.round(availableWidth),
        height: Math.round(availableWidth / imageAspectRatio)
    };
}

/**
 * Interactive visual overlay stage supporting Wipe Curtain,
 * Difference Highlight, and Onion Skin opacity blending.
 *
 * @param {DuplicateOverlayViewProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered overlay stage.
 */
export const DuplicateOverlayView: Component<DuplicateOverlayViewProperties> = properties => {
    const [isWipeDragging, setIsWipeDragging] = createSignal<boolean>(false);

    let stageElementReference: HTMLDivElement | undefined;
    let overlayContainerReference: HTMLDivElement | undefined;

    const [stageDimensions, setStageDimensions] = createSignal<{ width: number; height: number }>({
        width: 0,
        height: 0
    });
    const [imageNaturalDimensions, setImageNaturalDimensions] = createSignal<{
        width: number;
        height: number;
    } | null>(null);

    const fallbackCandidateDimensions = createMemo(() => {
        const rawDimensions = properties.candidateA.dimensions || properties.candidateB.dimensions;
        if (!rawDimensions) return null;
        const [widthString, heightString] = rawDimensions.split('x');
        const parsedWidth = parseFloat(widthString);
        const parsedHeight = parseFloat(heightString);
        if (parsedWidth > 0 && parsedHeight > 0) {
            return { width: parsedWidth, height: parsedHeight };
        }
        return null;
    });

    const activeImageDimensions = createMemo(() => {
        return imageNaturalDimensions() || fallbackCandidateDimensions() || null;
    });

    const handleImageLoad = (event: Event) => {
        const imageElement = event.currentTarget as HTMLImageElement;
        if (imageElement.naturalWidth > 0 && imageElement.naturalHeight > 0) {
            setImageNaturalDimensions({
                width: imageElement.naturalWidth,
                height: imageElement.naturalHeight
            });
        }
    };

    onMount(() => {
        if (!stageElementReference) return;
        const resizeObserver = new ResizeObserver(observedEntries => {
            for (const entry of observedEntries) {
                setStageDimensions({
                    width: entry.contentRect.width,
                    height: entry.contentRect.height
                });
            }
        });
        resizeObserver.observe(stageElementReference);
        onCleanup(() => resizeObserver.disconnect());
    });

    const overlayContainerComputedStyle = createMemo(() => {
        const stage = stageDimensions();
        const dimensions = activeImageDimensions();
        if (stage.width === 0 || stage.height === 0) {
            return { width: '100%', height: '100%' };
        }
        if (!dimensions || dimensions.width === 0 || dimensions.height === 0) {
            return { 'max-width': '100%', 'max-height': '100%' };
        }

        const availableWidth = Math.max(100, stage.width - 32);
        const availableHeight = Math.max(100, stage.height - 32);
        const imageRatio = dimensions.width / dimensions.height;
        const frameDimensions = calculateOverlayFrameDimensions(
            availableWidth,
            availableHeight,
            imageRatio
        );

        return {
            width: `${frameDimensions.width}px`,
            height: `${frameDimensions.height}px`
        };
    });

    const updateWipePercentageFromClientX = (clientX: number) => {
        if (!overlayContainerReference) return;
        const containerRectangle = overlayContainerReference.getBoundingClientRect();
        if (containerRectangle.width === 0) return;
        const relativeX = clientX - containerRectangle.left;
        const calculatedPercentage = Math.max(
            0,
            Math.min(100, Math.round((relativeX / containerRectangle.width) * 100))
        );
        properties.onWipeCurtainChange(calculatedPercentage);
    };

    const handleOverlayMouseDown = (event: MouseEvent) => {
        if (properties.comparisonMode !== 'wipe') return;
        event.preventDefault();
        setIsWipeDragging(true);
        updateWipePercentageFromClientX(event.clientX);
    };

    const handleOverlayMouseMove = (event: MouseEvent) => {
        if (properties.comparisonMode !== 'wipe' || !isWipeDragging()) return;
        event.preventDefault();
        updateWipePercentageFromClientX(event.clientX);
    };

    const handleOverlayMouseUp = () => {
        setIsWipeDragging(false);
    };

    const calculateTargetLayerStyle = () => {
        if (properties.comparisonMode === 'wipe') {
            return {
                'clip-path': `inset(0 0 0 ${properties.wipeCurtainPercentage}%)`
            };
        }
        if (properties.comparisonMode === 'onion') {
            return {
                opacity: `${properties.onionSkinOpacityPercentage / 100}`
            };
        }
        return {};
    };

    return (
        <div
            ref={stageElementReference}
            class="split-overlay-stage"
            onMouseMove={handleOverlayMouseMove}
            onMouseUp={handleOverlayMouseUp}
            onMouseLeave={handleOverlayMouseUp}
        >
            <div
                ref={overlayContainerReference}
                class={`split-overlay-container ${properties.comparisonMode === 'wipe' ? 'is-wipe-mode' : ''}`}
                style={overlayContainerComputedStyle()}
                onMouseDown={handleOverlayMouseDown}
            >
                <img
                    src={`asset://localhost/${properties.candidateA.id}?type=preview`}
                    alt={properties.candidateA.name}
                    class="split-overlay-layer base-layer"
                    onLoad={handleImageLoad}
                />
                <img
                    src={`asset://localhost/${properties.candidateB.id}?type=preview`}
                    alt={properties.candidateB.name}
                    class={`split-overlay-layer target-layer ${properties.comparisonMode === 'difference' ? (properties.isDifferenceInverted ? 'is-difference-inverted' : 'is-difference') : ''}`}
                    style={calculateTargetLayerStyle()}
                    onLoad={handleImageLoad}
                />
                <Show when={properties.comparisonMode === 'wipe'}>
                    <div
                        class="split-wipe-divider"
                        style={{ left: `${properties.wipeCurtainPercentage}%` }}
                    >
                        <div class="split-wipe-handle" />
                    </div>
                </Show>
            </div>
        </div>
    );
};
