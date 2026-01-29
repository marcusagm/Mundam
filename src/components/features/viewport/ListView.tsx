import { Component } from "solid-js";
import { useLibrary } from "../../../core/hooks";
import { ListViewToolbar } from "./ListViewToolbar";
import { VirtualMasonry } from "./VirtualMasonry";
import "./list-view.css";

export const ListView: Component = () => {
    const lib = useLibrary();

    return (
        <div class="list-view">
            <ListViewToolbar />
            
            <div class="list-view-content">
                {/* Por enquanto apenas Masonry Vertical é suportado na implementação */}
                <VirtualMasonry items={lib.items} />
            </div>
        </div>
    );
};
