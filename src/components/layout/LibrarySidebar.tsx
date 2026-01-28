import { Component } from "solid-js";
import { LibrarySidebarPanel } from "../features/library/LibrarySidebarPanel";
import { FolderTreeSidebarPanel } from "../features/library/FolderTreeSidebarPanel";
import { TagTreeSidebarPanel } from "../features/tags/TagTreeSidebarPanel";
import "./library-sidebar.css";

export const LibrarySidebar: Component = () => {
    return (
        <aside class="library-sidebar">
            <LibrarySidebarPanel />
            <FolderTreeSidebarPanel />
            <TagTreeSidebarPanel />
        </aside>
    );
};

