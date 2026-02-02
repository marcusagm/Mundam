import { Component, Switch, Match } from "solid-js";
import { useLibrary, useFilters } from "../../../core/hooks";
import { ListViewToolbar } from "./ListViewToolbar";
import { VirtualMasonry } from "./VirtualMasonry";
import { VirtualGridView } from "./VirtualGridView";
import { VirtualListView } from "./VirtualListView";
import "./list-view.css";

export const ListView: Component = () => {
    const lib = useLibrary();
    const filters = useFilters();

    return (
        <div class="list-view">
            <ListViewToolbar />
            
            <div class="list-view-content">
                <Switch>
                    <Match when={filters.layout === "grid"}>
                        <VirtualGridView />
                    </Match>
                    <Match when={filters.layout === "list"}>
                        <VirtualListView />
                    </Match>
                    <Match when={filters.layout === "masonry-v" || filters.layout === "masonry-h"}>
                        <VirtualMasonry items={lib.items} mode={filters.layout as "masonry-v" | "masonry-h"} />
                    </Match>
                </Switch>
            </div>
        </div>
    );
};
