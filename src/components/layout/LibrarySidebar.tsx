import { Component } from "solid-js";
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from "../ui/Resizable";
import { LibrarySidebarPanel } from "../features/library/LibrarySidebarPanel";
import { FolderTreeSidebarPanel } from "../features/library/FolderTreeSidebarPanel";
import { TagTreeSidebarPanel } from "../features/tags/TagTreeSidebarPanel";
import "./library-sidebar.css";

export const LibrarySidebar: Component = () => {
    return (
        <aside class="library-sidebar">
            <ResizablePanelGroup direction="vertical">
                <ResizablePanel id="sidebar-library" defaultSize={25} minSize={10} class="panel-lib">
                    <LibrarySidebarPanel />
                </ResizablePanel>
                
                <ResizableHandle />
                
                <ResizablePanel id="sidebar-folders" defaultSize={45} minSize={15} class="panel-folders">
                    <FolderTreeSidebarPanel />
                </ResizablePanel>
                
                <ResizableHandle />
                
                <ResizablePanel id="sidebar-tags" defaultSize={30} minSize={15} class="panel-tags">
                    <TagTreeSidebarPanel />
                </ResizablePanel>
            </ResizablePanelGroup>
        </aside>
    );
};

