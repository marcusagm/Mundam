import { Component, JSX, createEffect } from 'solid-js';
import { assetDragSource } from '../../../../core/dnd';
import { useAssetDropZone } from '../../../../core/hooks/useAssetDropZone';

/**
 * Properties for the internal asset item container logic.
 */
export interface AssetItemContainerProperties {
    /**
     * Asset unique identifier
     * @type {number}
     */
    id: string;

    /**
     * File path to the asset
     * @type {string}
     */
    path: string;

    /**
     * Thumbnail path, if available
     * @type {string | null}
     */
    thumbnailPath: string | null;

    /**
     * Original filename string
     * @type {string}
     */
    filename: string;

    /**
     * True if currently selected by the user
     * @type {boolean}
     */
    isSelected: boolean;

    /**
     * True if currently focused in the view
     * @type {boolean | undefined}
     */
    isFocused?: boolean;

    /**
     * CSS style properties for virtual placement
     * @type {JSX.CSSProperties}
     */
    style: JSX.CSSProperties;

    /**
     * CSS class properties for virtual placement
     * @type {string | undefined}
     */
    class?: string;

    /**
     * Handler when clicked/selected
     * @param {number} id - The clicked asset ID
     * @param {Object} modifiers - Shift or Mutli select keys active
     * @returns {void}
     */
    onSelect: (id: string, modifiers: { multi: boolean; shift: boolean }) => void;

    /**
     * Handler when double clicked to open
     * @param {number} id - The opened asset ID
     * @returns {void}
     */
    onOpen: (id: string) => void;

    /**
     * Context menu firing handler
     * @param {MouseEvent} event - Native mouse event
     * @param {number} id - Target asset ID
     * @returns {void}
     */
    onContextMenu?: (event: MouseEvent, id: string) => void;

    /**
     * Function to fetch selected IDs for Drag and Drop
     * @returns {(number | string)[]}
     */
    getSelectedIds: () => string[];

    /**
     * Function to fetch asset's core info for Drag and Drop
     * @param {number} id - Requesting asset ID
     * @returns {{ path: string; thumbnail_path: string | null } | undefined}
     */
    getItemInfo: (id: string) => { path: string; thumbnail_path: string | null } | undefined;

    /**
     * Render prop receiving layout states (active drop, focused, selected).
     */
    children: (state: { isDropTarget: boolean }) => JSX.Element;
}

/**
 * Asset Drag Source Directive
 *
 * @type {import('solid-js').Directive}
 */
void assetDragSource;

/**
 * AssetItemContainer - Logical Drop/Drag Container
 *
 * It manages ARIA roles, event listening for opening and selection,
 * Drag and Drop hooks, and Virtualization basic layouts, leaving the visual
 * representation to child render props.
 *
 * @param {AssetItemContainerProperties} containerProperties - Properties to operate.
 * @returns {JSX.Element} Abstract wrapper for an asset.
 */
export const AssetItemContainer: Component<AssetItemContainerProperties> = containerProperties => {
    /**
     * Reference to the focusable element
     */
    let focusReference: HTMLDivElement | undefined;

    /**
     * Drop zone state and handlers
     */
    const { isDropTarget, dragHandlers } = useAssetDropZone(() => containerProperties.id);

    /**
     * Sync native focus when virtual focus changes
     */
    createEffect(() => {
        const grid = focusReference?.closest('[role="grid"]');
        const activeElement = document.activeElement;
        if (
            containerProperties.isFocused &&
            focusReference &&
            grid &&
            (activeElement === grid ||
                (activeElement?.matches('[role="gridcell"]') &&
                    activeElement.closest('[role="grid"]') === grid))
        ) {
            focusReference.focus({ preventScroll: true });
        }
    });

    /**
     * Native click handler
     *
     * @param {MouseEvent} event - Native click event
     * @returns {void}
     */
    const handleClick = (event: MouseEvent): void => {
        event.stopPropagation();
        containerProperties.onSelect(containerProperties.id, {
            multi: event.metaKey || event.ctrlKey,
            shift: event.shiftKey
        });
    };

    /**
     * Double click handler
     *
     * @returns {void}
     */
    const handleDoubleClick = (): void => {
        containerProperties.onOpen(containerProperties.id);
    };

    /**
     * Right click/Context handler
     *
     * @param {MouseEvent} event - Native mouse event
     * @returns {void}
     */
    const handleContextMenu = (event: MouseEvent): void => {
        containerProperties.onContextMenu?.(event, containerProperties.id);
    };

    return (
        <div
            ref={focusReference}
            use:assetDragSource={{
                id: containerProperties.id,
                path: containerProperties.path,
                thumbnailPath: containerProperties.thumbnailPath,
                isSelected: containerProperties.isSelected,
                getSelectedIds: containerProperties.getSelectedIds,
                getItemInfo: containerProperties.getItemInfo
            }}
            class={`virtual-item virtual-masonry-item ${
                containerProperties.isSelected ? 'selected' : ''
            } ${containerProperties.isFocused ? 'focused' : ''} ${
                isDropTarget() ? 'drop-target-active' : ''
            } ${containerProperties.class || ''}`}
            style={containerProperties.style}
            role="gridcell"
            aria-selected={containerProperties.isSelected}
            aria-label={`Asset: ${containerProperties.filename}`}
            tabIndex={containerProperties.isFocused ? 0 : -1}
            onClick={handleClick}
            onDblClick={handleDoubleClick}
            onContextMenu={handleContextMenu}
            {...dragHandlers}
        >
            {containerProperties.children({ isDropTarget: isDropTarget() })}
        </div>
    );
};
