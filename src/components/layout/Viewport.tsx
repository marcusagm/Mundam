import { Component, Show } from "solid-js";
import { useViewport } from "../../core/hooks";
import { ListView } from "../features/viewport/ListView";
import { ItemView } from "../features/itemview/ItemView";
import "./viewport.css";

export const Viewport: Component = () => {
    const viewport = useViewport();

    return (
        <main class="viewport-root">
            <ListView />
            
            <Show when={viewport.mode() === "item"}>
                <ItemView />
            </Show>
        </main>
    );
};
