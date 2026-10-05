import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createRoot, createSignal } from 'solid-js';
import { render } from '@solidjs/testing-library';
import { useVirtualList } from './hooks/useVirtualList';
import { VirtualList } from './VirtualList';
import type { VirtualListController } from './types';

interface TestItem {
    id: string;
    label: string;
}

describe('VirtualList virtualization module', () => {
    beforeEach(() => {
        vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
            return setTimeout(() => callback(performance.now()), 0);
        });
        vi.stubGlobal('cancelAnimationFrame', (handleIdentifier: number) => {
            clearTimeout(handleIdentifier);
        });
        class MockResizeObserver {
            observe = vi.fn();
            unobserve = vi.fn();
            disconnect = vi.fn();
        }
        vi.stubGlobal('ResizeObserver', MockResizeObserver);
    });

    describe('useVirtualList hook', () => {
        it('calculates total height and visible range with fixed height and gap', () => {
            createRoot(disposeRoot => {
                const sampleItemList: TestItem[] = Array.from({ length: 100 }, (_, itemIndex) => ({
                    id: `item-${itemIndex}`,
                    label: `Item ${itemIndex}`
                }));

                const [scrollOffset, setScrollOffset] = createSignal(0);
                const [viewportHeight] = createSignal(400);

                const {
                    totalHeightInPixels,
                    visibleRange,
                    renderedSliceOfItems,
                    getItemOffsetInPixels
                } = useVirtualList({
                    items: () => sampleItemList,
                    itemHeight: 50,
                    gap: 10,
                    overscanCount: 2,
                    scrollOffset,
                    viewportHeight
                });

                // Total height = 100 * 60 - 10 = 5990px
                expect(totalHeightInPixels()).toBe(5990);

                // At scroll 0, viewport 400:
                // rawStartIndex = 0, visibleCount = ceil(400 / 60) = 7, rawEndIndex = 7
                // with overscan 2: startIndex = max(0, 0 - 2) = 0, endIndex = min(100, 7 + 2) = 9
                expect(visibleRange().startIndex).toBe(0);
                expect(visibleRange().endIndex).toBe(9);
                expect(renderedSliceOfItems().length).toBe(9);
                expect(getItemOffsetInPixels(0)).toBe(0);
                expect(getItemOffsetInPixels(1)).toBe(60);
                expect(getItemOffsetInPixels(5)).toBe(300);

                // Now scroll down by 600px (item index 10)
                setScrollOffset(600);
                // rawStartIndex = 10, rawEndIndex = 17
                // startIndex = 10 - 2 = 8, endIndex = 17 + 2 = 19
                expect(visibleRange().startIndex).toBe(8);
                expect(visibleRange().endIndex).toBe(19);
                expect(renderedSliceOfItems().length).toBe(11);

                disposeRoot();
            });
        });

        it('supports variable item heights via dynamic resolver function', () => {
            createRoot(disposeRoot => {
                const sampleItemList: TestItem[] = Array.from({ length: 20 }, (_, itemIndex) => ({
                    id: `item-${itemIndex}`,
                    label: `Item ${itemIndex}`
                }));

                const [scrollOffset] = createSignal(0);
                const [viewportHeight] = createSignal(300);

                const {
                    totalHeightInPixels,
                    getItemHeightInPixels,
                    getItemOffsetInPixels,
                    visibleRange
                } = useVirtualList({
                    items: () => sampleItemList,
                    itemHeight: (_item, itemIndex) => (itemIndex % 2 === 0 ? 50 : 100),
                    gap: 0,
                    overscanCount: 1,
                    scrollOffset,
                    viewportHeight
                });

                // 10 items of 50px + 10 items of 100px = 1500px
                expect(totalHeightInPixels()).toBe(1500);
                expect(getItemHeightInPixels(0)).toBe(50);
                expect(getItemHeightInPixels(1)).toBe(100);
                expect(getItemOffsetInPixels(0)).toBe(0);
                expect(getItemOffsetInPixels(1)).toBe(50);
                expect(getItemOffsetInPixels(2)).toBe(150);

                expect(visibleRange().startIndex).toBe(0);
                expect(visibleRange().endIndex).toBeGreaterThan(0);

                disposeRoot();
            });
        });

        it('calculates scroll offsets for index alignments accurately', () => {
            createRoot(disposeRoot => {
                const sampleItemList: TestItem[] = Array.from({ length: 50 }, (_, itemIndex) => ({
                    id: `item-${itemIndex}`,
                    label: `Item ${itemIndex}`
                }));

                const [scrollOffset] = createSignal(200);
                const [viewportHeight] = createSignal(500);

                const { calculateScrollOffsetForIndex } = useVirtualList({
                    items: () => sampleItemList,
                    itemHeight: 100,
                    gap: 0,
                    overscanCount: 2,
                    scrollOffset,
                    viewportHeight
                });

                // Target index 10: itemTop = 1000, itemHeight = 100
                expect(calculateScrollOffsetForIndex(10, 'start')).toBe(1000);
                // end: 1100 - 500 = 600
                expect(calculateScrollOffsetForIndex(10, 'end')).toBe(600);
                // center: 1000 - (500 - 100)/2 = 800
                expect(calculateScrollOffsetForIndex(10, 'center')).toBe(800);

                disposeRoot();
            });
        });
    });

    describe('VirtualList component', () => {
        it('renders fallback when item collection is empty', () => {
            const { getByText } = render(() => (
                <VirtualList<TestItem>
                    items={[]}
                    itemHeight={50}
                    fallback={<div>No records found</div>}
                >
                    {item => <div>{item.label}</div>}
                </VirtualList>
            ));

            expect(getByText('No records found')).toBeTruthy();
        });

        it('renders rendered items and provides imperative controller', () => {
            let controllerHandle: VirtualListController | undefined;

            const sampleItemList: TestItem[] = [
                { id: '1', label: 'First Item' },
                { id: '2', label: 'Second Item' },
                { id: '3', label: 'Third Item' }
            ];

            const { container, getByText } = render(() => (
                <VirtualList<TestItem>
                    items={sampleItemList}
                    itemHeight={100}
                    containerHeight={400}
                    controller={handle => {
                        controllerHandle = handle;
                    }}
                >
                    {item => <span>{item.label}</span>}
                </VirtualList>
            ));

            expect(getByText('First Item')).toBeTruthy();
            expect(getByText('Second Item')).toBeTruthy();
            expect(getByText('Third Item')).toBeTruthy();
            expect(controllerHandle).toBeDefined();
            expect(typeof controllerHandle?.scrollToIndex).toBe('function');
            expect(typeof controllerHandle?.scrollToTop).toBe('function');
            expect(typeof controllerHandle?.scrollToBottom).toBe('function');

            const virtualTrack = container.querySelector('.ui-virtual-list-track') as HTMLElement;
            expect(virtualTrack).toBeTruthy();
            expect(virtualTrack.style.height).toBe('300px');
        });
    });
});
