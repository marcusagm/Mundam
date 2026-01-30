import { Component } from "solid-js";
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from "../ui/Resizable";
import { LibrarySidebarPanel } from "../features/library/LibrarySidebarPanel";
import { FolderTreeSidebarPanel } from "../features/library/FolderTreeSidebarPanel";
import { TagTreeSidebarPanel } from "../features/tags/TagTreeSidebarPanel";
import { SmartFoldersSidebarPanel } from "../features/search/SmartFoldersSidebarPanel";
import "./library-sidebar.css";

export const LibrarySidebar: Component = () => {
    const STORAGE_KEY = "sidebar-layout-v2"; // Increment version since we added a panel

    // Get persisted sizes or use defaults
    const getPersistedLayout = () => {
        try {
            const saved = localStorage.getItem(STORAGE_KEY);
            return saved ? JSON.parse(saved) : null;
        } catch {
            return null;
        }
    };

    const layout = getPersistedLayout();
    const librarySize = layout?.[0] ?? 15;
    const foldersSize = layout?.[1] ?? 35;
    const tagsSize = layout?.[2] ?? 30;
    const smartSize = layout?.[3] ?? 20;

    const handleLayoutChange = (sizes: number[]) => {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(sizes));
    };

    return (
        <aside class="library-sidebar">
            <ResizablePanelGroup direction="vertical" onLayout={handleLayoutChange}>
                <ResizablePanel id="sidebar-library" defaultSize={librarySize} minSize={10} class="panel-lib">
                    <LibrarySidebarPanel />
                </ResizablePanel>
                
                <ResizableHandle />
                
                <ResizablePanel id="sidebar-folders" defaultSize={foldersSize} minSize={15} class="panel-folders">
                    <FolderTreeSidebarPanel />
                </ResizablePanel>
                
                <ResizableHandle />
                
                <ResizablePanel id="sidebar-tags" defaultSize={tagsSize} minSize={15} class="panel-tags">
                    <TagTreeSidebarPanel />
                </ResizablePanel>

                <ResizableHandle />

                <ResizablePanel id="sidebar-smart" defaultSize={smartSize} minSize={10} class="panel-smart">
                    <SmartFoldersSidebarPanel />
                </ResizablePanel>
            </ResizablePanelGroup>
        </aside>
    );
};

