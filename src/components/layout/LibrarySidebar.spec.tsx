import { cleanup, fireEvent, render, screen, within } from '@solidjs/testing-library';
import { createSignal } from 'solid-js';
import { afterEach, expect, it, vi } from 'vitest';
import type { PluginInspectorSection } from '../../lib/pluginInspector';
import { LibrarySidebar } from './LibrarySidebar';

vi.mock('../features/library/LibrarySidebarPanel', () => ({
    LibrarySidebarPanel: () => 'Library'
}));
vi.mock('../features/library/FolderTreeSidebarPanel', () => ({
    FolderTreeSidebarPanel: () => 'Folders'
}));
vi.mock('../features/tags/TagTreeSidebarPanel', () => ({ TagTreeSidebarPanel: () => 'Tags' }));
vi.mock('../features/search/SmartFoldersSidebarPanel', () => ({
    SmartFoldersSidebarPanel: () => 'Smart Folders'
}));

const reviewSection: PluginInspectorSection = {
    pluginId: 'org.example.reference-review',
    pluginName: 'Reference review',
    sectionId: 'review',
    title: 'Review',
    sidebarLinkLabel: 'Open review details',
    fields: []
};

afterEach(() => {
    cleanup();
    localStorage.removeItem('sidebar-layout-v3');
});

it('keeps native panels mounted at their saved sizes while authorized groups change', () => {
    const layout = JSON.stringify({ 'sidebar-tags': 25, 'sidebar-library': 32 });
    localStorage.setItem('sidebar-layout-v3', layout);
    const [sections, setSections] = createSignal<PluginInspectorSection[]>([]);
    const [eligible, setEligible] = createSignal(false);
    const openSection = vi.fn();
    const { container } = render(() => (
        <LibrarySidebar
            pluginSections={sections()}
            canOpenPluginLinks={eligible()}
            onOpenPluginSection={openSection}
        />
    ));
    const library = container.querySelector<HTMLElement>('[data-panel-id="sidebar-library"]');
    const tags = container.querySelector<HTMLElement>('[data-panel-id="sidebar-tags"]');
    expect(library?.style.height).toBe('32%');
    expect(tags?.style.height).toBe('25%');
    expect(container.querySelectorAll('.ui-resizable-handle')).toHaveLength(3);
    expect(screen.queryByRole('region', { name: 'Plugins' })).toBeNull();

    setSections([
        reviewSection,
        { ...reviewSection, pluginId: 'org.other.review', pluginName: 'Other' },
        { ...reviewSection, sectionId: 'inspector-only', sidebarLinkLabel: undefined }
    ]);
    const plugins = screen.getByRole('region', { name: 'Plugins' });
    const group = within(plugins).getByRole('button', { name: 'Reference review' });
    fireEvent.click(group);
    const link = within(plugins).getByRole<HTMLButtonElement>('button', {
        name: 'Open review details'
    });
    expect(link.disabled).toBe(true);
    expect(within(plugins).getByText('Select one asset to open plugin details.')).toBeTruthy();
    setEligible(true);
    expect(link.disabled).toBe(false);
    fireEvent.click(link);
    expect(openSection).toHaveBeenCalledExactlyOnceWith('org.example.reference-review:review');

    setSections([]);
    expect(screen.queryByRole('region', { name: 'Plugins' })).toBeNull();
    expect(container.querySelector('[data-panel-id="sidebar-library"]')).toBe(library);
    expect(tags?.style.height).toBe('25%');
    expect(localStorage.getItem('sidebar-layout-v3')).toBe(layout);
});
