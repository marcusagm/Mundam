import { Component } from "solid-js";
import { LibrarySidebarPanel } from "../features/library/LibrarySidebarPanel";
import { FoldersSidebarPanel } from "../features/library/FoldersSidebarPanel";
import { TagTreeSidebarPanel } from "../features/tags/TagTreeSidebarPanel";
import "./library-sidebar.css";

export const LibrarySidebar: Component = () => {
    return (
        <aside class="library-sidebar">
            <LibrarySidebarPanel />
            <FoldersSidebarPanel />
            <TagTreeSidebarPanel />
        </aside>
    );
};
