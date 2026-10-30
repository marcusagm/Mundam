import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render } from '@solidjs/testing-library';
import { createSignal } from 'solid-js';
import type { ThumbnailProcessingSnapshot } from '../../../../core/store/thumbnailProcessing';

const api = vi.hoisted(() => ({
    outcomes: new Map<string, ThumbnailProcessingSnapshot>(),
    ready: undefined as ((event: { payload: { id: string; path: string } }) => void) | undefined,
    rejectRepair: false,
    unavailable: false,
    invoke: vi.fn()
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke }));
vi.mock('@tauri-apps/api/event', () => ({
    listen: async (
        _name: string,
        listener: (event: { payload: { id: string; path: string } }) => void
    ) => {
        api.ready = listener;
        return vi.fn();
    }
}));

import { Thumbnail } from './Thumbnail';

function outcome(
    identifier: string,
    state: ThumbnailProcessingSnapshot['state'],
    path: string | null = null,
    generation = 0
): void {
    api.outcomes.set(identifier, { asset_id: identifier, state, generation, thumbnail_path: path });
}

describe('thumbnail extraction outcomes', () => {
    beforeEach(() => {
        vi.useFakeTimers();
        api.outcomes.clear();
        api.rejectRepair = false;
        api.unavailable = false;
        api.invoke.mockReset();
        api.invoke.mockImplementation(
            async (command: string, argumentsValue?: Record<string, unknown>) => {
                if (command === 'get_thumbnail_processing_status') {
                    if (api.unavailable) throw new Error('Status unavailable');
                    return (argumentsValue?.assetIds as string[]).flatMap(identifier => {
                        const snapshot = api.outcomes.get(identifier);
                        return snapshot ? [snapshot] : [];
                    });
                }
                if (command === 'request_thumbnail_regenerate' && api.rejectRepair)
                    throw new Error('Request rejected');
                return undefined;
            }
        );
    });
    afterEach(() => {
        cleanup();
        vi.useRealTimers();
        vi.restoreAllMocks();
    });

    it.each(['skipped', 'failed'] as const)(
        'shows an explained file icon for %s while AssetState is Indexed',
        async state => {
            outcome(state, state);
            const view = render(() => (
                <Thumbnail
                    id={state}
                    src="/file.raw"
                    alt="file.raw"
                    thumbnail={null}
                    state="Indexed"
                />
            ));
            await vi.advanceTimersByTimeAsync(35);
            expect(view.container.querySelector('.thumbnail-file-fallback')).not.toBeNull();
            expect(view.container.querySelector('.asset-placeholder')).toBeNull();
            expect(
                view.container.querySelector('.thumbnail-container')?.getAttribute('aria-busy')
            ).toBe('false');
            expect(
                view.container.querySelector('.thumbnail-container')?.getAttribute('title')
            ).toBeTruthy();
            expect(
                api.invoke.mock.calls.filter(
                    ([command]) => command === 'request_thumbnail_regenerate'
                )
            ).toHaveLength(0);
        }
    );

    it('recovers completed output from status polling even without a ready event', async () => {
        outcome('pending', 'pending');
        const view = render(() => (
            <Thumbnail
                id="pending"
                src="/file.png"
                alt="file.png"
                thumbnail={null}
                state="Indexed"
            />
        ));
        await vi.advanceTimersByTimeAsync(35);
        expect(view.container.querySelector('.asset-placeholder')).not.toBeNull();
        outcome('pending', 'succeeded', 'pending.webp', 2);
        await vi.advanceTimersByTimeAsync(2000);
        const image = view.container.querySelector('img');
        expect(image).not.toBeNull();
        expect(image?.getAttribute('src')).toContain('generation=2');
        if (!image) throw new Error('Expected generated image');
        fireEvent.load(image);
        expect(view.container.querySelector('.asset-placeholder')).toBeNull();
    });

    it('resubscribes when a virtualized card changes identity and ignores the previous ready event', async () => {
        outcome('first', 'skipped');
        outcome('second', 'pending');
        const [identifier, setIdentifier] = createSignal('first');
        const view = render(() => (
            <Thumbnail
                id={identifier()}
                src="/file.png"
                alt="file.png"
                thumbnail={null}
                state="Indexed"
            />
        ));
        await vi.advanceTimersByTimeAsync(35);
        setIdentifier('second');
        api.ready?.({ payload: { id: 'first', path: 'first.webp' } });
        await vi.advanceTimersByTimeAsync(35);
        expect(
            view.container.querySelector('[data-id="second"]')?.getAttribute('data-thumbnail-state')
        ).toBe('pending');
        expect(view.container.querySelector('img')).toBeNull();
        expect(view.container.querySelector('.asset-placeholder')).not.toBeNull();
    });

    it('ends a rejected cache repair and does not repeat it after virtualization', async () => {
        vi.spyOn(console, 'warn').mockImplementation(() => {});
        api.rejectRepair = true;
        outcome('broken', 'succeeded', 'broken.webp');
        let view = render(() => (
            <Thumbnail
                id="broken"
                src="/file.png"
                alt="file.png"
                thumbnail={null}
                state="Thumbnailed"
            />
        ));
        await vi.advanceTimersByTimeAsync(35);
        let image = view.container.querySelector('img');
        if (!image) throw new Error('Expected image before failure');
        fireEvent.error(image);
        await vi.advanceTimersByTimeAsync(35);
        expect(view.container.querySelector('.thumbnail-file-fallback')).not.toBeNull();
        expect(view.container.querySelector('.asset-placeholder')).toBeNull();
        view.unmount();
        view = render(() => (
            <Thumbnail
                id="broken"
                src="/file.png"
                alt="file.png"
                thumbnail={null}
                state="Thumbnailed"
            />
        ));
        await vi.advanceTimersByTimeAsync(35);
        image = view.container.querySelector('img');
        if (!image) throw new Error('Expected remounted image');
        fireEvent.error(image);
        await vi.advanceTimersByTimeAsync(35);
        expect(
            api.invoke.mock.calls.filter(([command]) => command === 'request_thumbnail_regenerate')
        ).toHaveLength(1);
        expect(view.container.querySelector('.asset-placeholder')).toBeNull();
    });

    it('keeps waiting when the initial status query is unavailable', async () => {
        vi.spyOn(console, 'warn').mockImplementation(() => {});
        api.unavailable = true;
        const view = render(() => (
            <Thumbnail
                id="unknown"
                src="/file.raw"
                alt="file.raw"
                thumbnail={null}
                state="Indexed"
            />
        ));
        await vi.advanceTimersByTimeAsync(35);
        expect(view.container.querySelector('.asset-placeholder')).not.toBeNull();
        expect(view.container.querySelector('.thumbnail-file-fallback')).toBeNull();
    });

    it('keeps the pending indicator mounted through retries and status query failures', async () => {
        vi.spyOn(console, 'warn').mockImplementation(() => {});
        outcome('waiting', 'pending');
        const view = render(() => (
            <Thumbnail id="waiting" src="/file.raw" alt="file.raw" thumbnail={null} />
        ));
        await vi.advanceTimersByTimeAsync(35);
        const placeholder = view.container.querySelector('.asset-placeholder');
        expect(placeholder).not.toBeNull();
        const expectWaiting = () => {
            expect(view.container.querySelector('.asset-placeholder')).toBe(placeholder);
            expect(view.container.querySelector('.thumbnail-file-fallback')).toBeNull();
        };
        for (const state of ['retry_wait', 'pending', 'retry_wait'] as const) {
            outcome('waiting', state);
            await vi.advanceTimersByTimeAsync(2000);
            expectWaiting();
        }
        api.unavailable = true;
        await vi.advanceTimersByTimeAsync(4000);
        expectWaiting();
        api.unavailable = false;
        outcome('waiting', 'pending');
        await vi.advanceTimersByTimeAsync(2000);
        expectWaiting();
        outcome('waiting', 'failed');
        await vi.advanceTimersByTimeAsync(2000);
        expect(view.container.querySelector('.asset-placeholder')).toBeNull();
        expect(view.container.querySelector('.thumbnail-file-fallback')).not.toBeNull();
    });

    it('preserves the generation cache key while a ready event refreshes the status', async () => {
        outcome('versioned', 'succeeded', 'versioned.webp', 4);
        const view = render(() => (
            <Thumbnail id="versioned" src="/file.png" alt="file.png" thumbnail={null} />
        ));
        await vi.advanceTimersByTimeAsync(35);
        expect(view.container.querySelector('img')?.getAttribute('src')).toContain('generation=4');
        api.ready?.({ payload: { id: 'versioned', path: 'versioned.webp' } });
        expect(view.container.querySelector('img')?.getAttribute('src')).toContain('generation=4');
        await vi.advanceTimersByTimeAsync(35);
        expect(view.container.querySelector('img')?.getAttribute('src')).toContain('generation=4');
    });
});
