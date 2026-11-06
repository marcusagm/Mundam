import type { TreeNode } from '../types';
import { createTreeDragItem } from '../../../../core/dnd/createTreeDragItem';
import { createRoot } from 'solid-js';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { currentDragItem, setDragItem } from '../../../../core/dnd/dnd-core';
import { useTreeDragDrop } from './useTreeDragDrop';

afterEach(() => setDragItem(null));

describe('tree drop targets', () => {
    it('preserves tag payloads and sibling reordering in the default mode', () => {
        createRoot(dispose => {
            try {
                const options = {
                    isEnabled: () => true,
                    isEditing: () => false,
                    dragType: () => 'TAG',
                    createDragItem: (node: TreeNode) => createTreeDragItem('TAG', node),
                    acceptedDragTypes: () => ['TAG'],
                    isValidDrop: () => true
                };
                const source = useTreeDragDrop({
                    ...options,
                    node: () => ({ id: 'source', label: 'Tag', data: { parent_id: 'parent' } })
                });
                const element = document.createElement('div');
                element.getBoundingClientRect = () => ({ top: 0, height: 100 }) as DOMRect;
                const event = {
                    preventDefault: vi.fn(),
                    stopPropagation: vi.fn(),
                    currentTarget: element,
                    clientY: 1,
                    dataTransfer: { setData: vi.fn(), dropEffect: 'none' }
                } as unknown as DragEvent;
                source.handleDragStart(event);
                expect(currentDragItem()).toEqual({
                    type: 'TAG',
                    payload: { id: 'source', name: 'Tag', parent_id: 'parent' }
                });
                const target = useTreeDragDrop({
                    ...options,
                    node: () => ({ id: 'target', label: 'Target' })
                });
                target.handleDragOver(event);
                expect(target.dropPosition()).toBe('before');
            } finally {
                dispose();
            }
        });
    });
    it('uses a folder payload and always nests when dropped near a row edge', async () => {
        await createRoot(async dispose => {
            try {
                const source = useTreeDragDrop({
                    node: () => ({
                        id: 'folder-source',
                        label: 'Source',
                        data: { path: '/source' }
                    }),
                    isEnabled: () => true,
                    isEditing: () => false,
                    dragType: () => 'FOLDER',
                    createDragItem: (node: TreeNode) => createTreeDragItem('FOLDER', node),
                    acceptedDragTypes: () => ['FOLDER'],
                    isValidDrop: () => true
                });
                const transfer = {
                    setData: vi.fn(),
                    getData: () => JSON.stringify(currentDragItem()),
                    dropEffect: 'none',
                    effectAllowed: 'all'
                };
                const element = document.createElement('div');
                element.getBoundingClientRect = () => ({ top: 0, height: 100 }) as DOMRect;
                const event = {
                    preventDefault: vi.fn(),
                    stopPropagation: vi.fn(),
                    dataTransfer: transfer,
                    currentTarget: element,
                    clientY: 1
                } as unknown as DragEvent;
                source.handleDragStart(event);
                expect(currentDragItem()).toEqual({
                    type: 'FOLDER',
                    payload: { id: 'folder-source', name: 'Source', path: '/source' }
                });
                const onDrop = vi.fn();
                const target = useTreeDragDrop({
                    node: () => ({ id: 'folder-target', label: 'Target' }),
                    isEnabled: () => true,
                    isEditing: () => false,
                    dragType: () => 'FOLDER',
                    acceptedDragTypes: () => ['FOLDER'],
                    isValidDrop: () => true,
                    dropMode: () => 'inside',
                    onDrop
                });
                target.handleDragOver(event);
                expect(target.dropPosition()).toBe('inside');
                expect(transfer.dropEffect).toBe('move');
                await target.handleDrop(event);
                expect(onDrop).toHaveBeenCalledWith(
                    expect.objectContaining({ type: 'FOLDER' }),
                    'folder-target',
                    'inside'
                );
            } finally {
                dispose();
            }
        });
    });
    it('accepts external assets while folder dragging remains disabled', async () => {
        const onDrop = vi.fn();
        const item = {
            type: 'ASSET' as const,
            payload: { id: 'asset', ids: ['asset'], filename: 'asset.png', path: '/asset.png' }
        };
        setDragItem(item);
        const element = document.createElement('div');
        const event = {
            preventDefault: vi.fn(),
            stopPropagation: vi.fn(),
            currentTarget: element,
            clientY: 0,
            dataTransfer: { dropEffect: 'none', getData: () => JSON.stringify(item) }
        } as unknown as DragEvent;
        await createRoot(async dispose => {
            try {
                const dragDrop = useTreeDragDrop({
                    node: () => ({ id: 'folder', label: 'Folder' }),
                    isEnabled: () => false,
                    isEditing: () => false,
                    dragType: () => 'FOLDER',
                    acceptedDragTypes: () => ['ASSET'],
                    isValidDrop: () => true,
                    onDrop
                });
                dragDrop.handleDragOver(event);
                expect(dragDrop.dropPosition()).toBe('inside');
                await dragDrop.handleDrop(event);
                expect(onDrop).toHaveBeenCalledWith(item, 'folder', 'inside');
                dragDrop.handleDragStart(event);
                expect(event.preventDefault).toHaveBeenCalled();
            } finally {
                dispose();
            }
        });
    });
    it('validates the decoded item rather than stale global state and awaits failures', async () => {
        const log = vi.spyOn(console, 'error').mockImplementation(() => undefined);
        const failure = new Error('Rejected movement');
        const onDrop = vi.fn().mockRejectedValue(failure);
        setDragItem({ type: 'TAG', payload: { id: 'stale', name: 'Stale' } });
        const item = { type: 'FOLDER', payload: { id: 'source', name: 'Source', path: '/source' } };
        await createRoot(async dispose => {
            try {
                const target = useTreeDragDrop({
                    node: () => ({ id: 'target', label: 'Target' }),
                    isEnabled: () => true,
                    isEditing: () => false,
                    dragType: () => 'FOLDER',
                    acceptedDragTypes: () => ['FOLDER'],
                    isValidDrop: () => true,
                    onDrop
                });
                await target.handleDrop(
                    {
                        preventDefault: vi.fn(),
                        stopPropagation: vi.fn(),
                        dataTransfer: { getData: () => JSON.stringify(item) }
                    } as unknown as DragEvent,
                    'inside'
                );
                expect(onDrop).toHaveBeenCalledWith(item, 'target', 'inside');
                expect(log).toHaveBeenCalledWith('Hierarchical drop operation failed:', failure);
                expect(currentDragItem()).toBeNull();
            } finally {
                dispose();
                log.mockRestore();
            }
        });
    });
});
