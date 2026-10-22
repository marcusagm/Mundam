import { Component } from 'solid-js';
import { WindowControls } from '../../../layout/TitleBar';
import { detectPlatform } from '../../../../core/input/utils/platform';
import '../item-view-header.css';

export interface ItemViewHeaderProperties {
    /** Name of the asset currently being viewed. */
    filename?: string;
}

/**
 * Header of the ItemView overlay. Since the overlay covers the application
 * TitleBar, this header mirrors its look and keeps the window draggable and
 * the window controls reachable, while displaying the asset file name.
 *
 * @param {ItemViewHeaderProperties} properties - Component properties.
 * @returns {JSX.Element} The rendered header.
 */
export const ItemViewHeader: Component<ItemViewHeaderProperties> = properties => {
    const platform = detectPlatform();

    return (
        <header
            class="item-view-header"
            classList={{ 'item-view-header--macos': platform === 'mac' }}
            data-tauri-drag-region
        >
            <div class="item-view-header-platform-area">
                <WindowControls />
            </div>
            <span class="item-view-header-title" title={properties.filename} data-tauri-drag-region>
                {properties.filename}
            </span>
            <div class="item-view-header-balance" />
        </header>
    );
};
