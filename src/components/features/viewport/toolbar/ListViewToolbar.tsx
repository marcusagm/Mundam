import { SearchToolbar } from '../../search/SearchToolbar';
import { Component, JSX } from 'solid-js';
import { HistoryNavigation } from './HistoryNavigation';
import { SortConfiguration } from './SortConfiguration';
import { ViewConfiguration } from './ViewConfiguration';
import './list-view-toolbar.css';

/**
 * Renders the toolbar for the main list view, containing history navigation, search, sorting, and view options.
 *
 * @returns {JSX.Element} The list view toolbar container.
 *
 * @example
 * ```tsx
 * import { ListViewToolbar } from '@/components/features/viewport/ListViewToolbar';
 * <ListViewToolbar />
 * ```
 */
export const ListViewToolbar: Component<{ actions?: JSX.Element }> = properties => {
    return (
        <div class="list-view-toolbar">
            <HistoryNavigation />

            {/* Search Bar */}
            <div class="toolbar-search">
                <SearchToolbar />
            </div>

            {/* Sort & View Controls */}
            <div class="toolbar-group">
                {properties.actions}
                <SortConfiguration />

                <div class="toolbar-separator" />

                <ViewConfiguration />
            </div>
        </div>
    );
};
