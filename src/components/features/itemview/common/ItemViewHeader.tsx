import { Component, Show } from 'solid-js';
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
            {/*
             * macOS: left spacer that mirrors the native traffic lights area.
             * Windows/Linux: no left-side placeholder needed.
             */}
            <Show when={platform === 'mac'}>
                <div class="item-view-header-platform-area" />
            </Show>

            <span class="item-view-header-title" title={properties.filename} data-tauri-drag-region>
                {properties.filename}
            </span>

            {/*
             * macOS: an invisible balance div keeps the title visually centred
             *        (mirrors the left traffic-lights spacer).
             * Windows/Linux: WindowControls on the far right per OS convention.
             */}
            <Show when={platform === 'mac'}>
                <div class="item-view-header-balance" />
            </Show>
            <Show when={platform !== 'mac'}>
                <div class="item-view-header-platform-area">
                    <WindowControls />
                </div>
            </Show>
        </header>
    );
};
