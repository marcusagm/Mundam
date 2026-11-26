import { Component, Switch, Match, JSX } from 'solid-js';
import { useLibrary, useFilters } from '../../../../core/hooks';
import { ListViewToolbar } from '../toolbar/ListViewToolbar';
import { VirtualMasonry } from './VirtualMasonry';
import { VirtualGridView } from './VirtualGridView';
import { VirtualListView } from './VirtualListView';
import './list-view.css';

/**
 * Renders the main viewport layout, managing the active list view mode (Grid, List, or Masonry).
 *
 * @returns {JSX.Element} The active list view container.
 */
export const ListView: Component<{ toolbarActions?: JSX.Element }> = properties => {
    /**
     * Library store
     *
     * @returns {Library} The library store.
     */
    const lib = useLibrary();

    /**
     * Filters store
     *
     * @returns {Filters} The filters store.
     */
    const filters = useFilters();

    return (
        <div class="list-view">
            <ListViewToolbar actions={properties.toolbarActions} />

            <div class="list-view-content">
                <Switch>
                    <Match when={filters.layout === 'grid'}>
                        <VirtualGridView />
                    </Match>
                    <Match when={filters.layout === 'list'}>
                        <VirtualListView />
                    </Match>
                    <Match when={filters.layout === 'masonry-v' || filters.layout === 'masonry-h'}>
                        <VirtualMasonry
                            items={lib.items}
                            mode={filters.layout as 'masonry-v' | 'masonry-h'}
                        />
                    </Match>
                </Switch>
            </div>
        </div>
    );
};
