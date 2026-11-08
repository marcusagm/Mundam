import { cleanup, fireEvent, render } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createSignal } from 'solid-js';
import { ProcessingActivityPanel } from './ProcessingActivityPanel';
import { processingIndicator, type ProcessingIndicator } from './processingPresentation';
import type {
    ProcessingClientState,
    WorkerSnapshot
} from '../../../../core/store/processing/types';
import {
    createWorkerRow,
    createInitialState,
    createMatchingPayload
} from './ProcessingActivityPanelTestUtils';

afterEach(cleanup);

describe('processing statusbar controls', () => {
    it('separates unavailable optional tools from blocked work and acknowledges checks inline', () => {
        const initial = createInitialState();
        initial.snapshot!.tools = [{ tool: 'ffmpeg', state: 'unavailable', checked_at: 1 }];
        expect(processingIndicator(initial, false)).toBe('idle');
        initial.snapshot!.workers[0].blocked = 2;
        expect(processingIndicator(initial, false)).toBe('warning');
        const [current, setCurrent] = createSignal(initial);
        const check = vi.fn();
        const view = render(() => (
            <ProcessingActivityPanel
                state={current()}
                indexing={null}
                onPause={vi.fn()}
                onCheckTools={check}
            />
        ));
        expect(view.getByText('Waiting for extraction tools')).toBeTruthy();
        const button = view.getByRole('button', {
            name: 'Check extraction tools'
        }) as HTMLButtonElement;
        fireEvent.click(button);
        expect(check).toHaveBeenCalledTimes(1);
        button.focus();
        setCurrent(previous => ({ ...previous, checkingTools: true }));
        expect(button.disabled).toBe(true);
        expect(button.getAttribute('aria-busy')).toBe('true');
        setCurrent(previous => ({
            ...previous,
            checkingTools: false,
            snapshot: {
                ...previous.snapshot!,
                sequence: 2,
                tools: [{ tool: 'ffmpeg', state: 'available', checked_at: 2 }]
            }
        }));
        expect(button.disabled).toBe(false);
        expect(view.getByLabelText('Extraction tools').textContent).toContain('Available');
        expect(view.getByRole('button', { name: 'Check extraction tools' })).toBe(button);
    });

    it('reactively reports effective capacity while a reduced limit drains active operations', () => {
        const initial = createInitialState();
        initial.snapshot!.capacity = {
            limit: 1,
            active: 3,
            waiting: 2,
            thumbnail_limit: 1
        };
        const [current, setCurrent] = createSignal(initial);
        const view = render(() => (
            <ProcessingActivityPanel state={current()} indexing={null} onPause={vi.fn()} />
        ));
        expect(view.getByLabelText('Processing capacity').textContent).toContain('3 / 1');
        expect(view.getByLabelText('Processing capacity').textContent).toContain('2 waiting');
        setCurrent(previous => ({
            ...previous,
            snapshot: {
                ...previous.snapshot!,
                capacity: {
                    limit: 4,
                    active: 2,
                    waiting: 0,
                    thumbnail_limit: 2
                }
            }
        }));
        expect(view.getByLabelText('Processing capacity').textContent).toContain('2 / 4');
        expect(view.getByLabelText('Processing capacity').textContent).toContain(
            'Thumbnail limit: 2'
        );
    });

    it('queues explicit retries and shows safe grouped causes without exposing arbitrary backend text', () => {
        const initial = createInitialState();
        initial.snapshot!.workers[0].failed = 2;
        initial.snapshot!.workers[0].skipped = 1;
        initial.snapshot!.workers[0].diagnostics = [
            { issue: 'ffmpeg_unavailable', state: 'failed', count: 2 },
            { issue: 'C:/private/SECRET', state: 'skipped', count: 1 }
        ];
        const [current, setCurrent] = createSignal(initial);
        const retry = vi.fn();
        const view = render(() => (
            <ProcessingActivityPanel
                state={current()}
                indexing={null}
                onPause={vi.fn()}
                onRetry={retry}
            />
        ));
        expect(view.getByText(/FFmpeg is unavailable/)).toBeTruthy();
        expect(view.container.textContent).not.toContain('SECRET');
        fireEvent.click(view.getByRole('button', { name: 'Retry failures' }));
        expect(retry).toHaveBeenLastCalledWith('thumbnails', 'failed');
        fireEvent.click(view.getByRole('button', { name: 'Recheck unsupported' }));
        expect(retry).toHaveBeenLastCalledWith('thumbnails', 'unsupported');
        setCurrent(previous => ({
            ...previous,
            pending: new Set(['thumbnails'])
        }));
        expect(
            (
                view.getByRole('button', {
                    name: 'Retry failures'
                }) as HTMLButtonElement
            ).disabled
        ).toBe(true);
        expect(
            (
                view.getByRole('button', {
                    name: 'Pause Thumbnails and metadata'
                }) as HTMLButtonElement
            ).disabled
        ).toBe(true);
    });

    it('offers duplicate comparison retries even when no individual asset failed', () => {
        const current = createInitialState();
        current.snapshot!.matching = createMatchingPayload(1);
        const retry = vi.fn();
        const view = render(() => (
            <ProcessingActivityPanel
                state={current}
                indexing={null}
                onPause={vi.fn()}
                onRetry={retry}
            />
        ));
        fireEvent.click(view.getByRole('button', { name: 'Retry failures' }));
        expect(retry).toHaveBeenCalledWith('duplicates', 'failed');
    });

    it('reports durable comparison failures separately and presents paused recovery without a busy indicator', () => {
        const current = createInitialState();
        current.snapshot!.matching = createMatchingPayload(2);
        const view = render(() => (
            <ProcessingActivityPanel state={current} indexing={null} onPause={vi.fn()} />
        ));
        expect(view.getByText('Comparisons failed')).toBeTruthy();
        expect(processingIndicator(current, false)).toBe('warning');
        const paused = createInitialState();
        paused.snapshot!.workers[2].paused = true;
        paused.snapshot!.manual_scan = {
            is_running: true,
            phase: 'paused',
            processed: 0,
            total: 0
        };
        expect(processingIndicator(paused, false)).toBe('paused');
    });

    it('shows cancellation and matching phases without a misleading hashing percentage', () => {
        const active = createInitialState();
        active.snapshot!.manual_scan = {
            is_running: true,
            phase: 'cancel_requested',
            processed: 10,
            total: 10
        };
        const [current, setCurrent] = createSignal(active);
        const view = render(() => (
            <ProcessingActivityPanel state={current()} indexing={null} onPause={vi.fn()} />
        ));
        expect(view.getByRole('status').textContent).toContain(
            'Stopping — finishing the current operation'
        );
        expect(view.queryByText('10 / 10 files analyzed')).toBeNull();
        setCurrent(previous => ({
            ...previous,
            snapshot: {
                ...previous.snapshot!,
                manual_scan: {
                    is_running: true,
                    phase: 'exact_matching',
                    processed: 10,
                    total: 10
                }
            }
        }));
        expect(view.getByRole('status').textContent).toContain('Comparing identical files');
        expect(view.queryByText('10 / 10 files analyzed')).toBeNull();
    });

    it('preserves focused controls when polling replaces the worker snapshots', () => {
        const [current, setCurrent] = createSignal(createInitialState());
        const view = render(() => (
            <ProcessingActivityPanel state={current()} indexing={null} onPause={vi.fn()} />
        ));
        const button = view.getByRole('button', {
            name: 'Pause Thumbnails and metadata'
        });
        button.focus();
        setCurrent(previous => ({
            ...previous,
            snapshot: {
                ...previous.snapshot!,
                sequence: 2,
                workers: previous.snapshot!.workers.map(worker => ({
                    ...worker,
                    pending: 42
                }))
            }
        }));
        expect(view.getByRole('button', { name: 'Pause Thumbnails and metadata' })).toBe(button);
        expect(document.activeElement).toBe(button);
        expect(view.getAllByText('42')).toHaveLength(3);
    });

    it('retains the acknowledged state until a control response and disables duplicate requests', () => {
        const [current, setCurrent] = createSignal(createInitialState());
        const action = vi.fn();
        const view = render(() => (
            <ProcessingActivityPanel state={current()} indexing={null} onPause={action} />
        ));
        const button = view.getByRole('button', {
            name: 'Pause Thumbnails and metadata'
        });
        fireEvent.click(button);
        expect(action).toHaveBeenCalledWith('thumbnails', true);
        expect(view.getByRole('button', { name: 'Pause Thumbnails and metadata' })).toBe(button);
        setCurrent(previous => ({
            ...previous,
            pending: new Set(['thumbnails'])
        }));
        expect(button.hasAttribute('disabled')).toBe(true);
        setCurrent(previous => ({
            ...previous,
            pending: new Set(),
            snapshot: {
                ...previous.snapshot!,
                workers: [
                    {
                        ...createWorkerRow('thumbnails'),
                        paused: true,
                        active: 1,
                        phase: 'thumbnail'
                    },
                    createWorkerRow('colors'),
                    createWorkerRow('duplicates')
                ]
            }
        }));
        expect(view.getByText('Pausing — finishing current operation')).toBeTruthy();
        fireEvent.click(view.getByRole('button', { name: 'Resume Thumbnails and metadata' }));
        expect(action).toHaveBeenLastCalledWith('thumbnails', false);
    });

    it('shows query errors without claiming Ready or showing an endless loading indicator', () => {
        const current: ProcessingClientState = {
            snapshot: null,
            error: 'Status unavailable',
            pending: new Set()
        };
        const view = render(() => (
            <ProcessingActivityPanel state={current} indexing={null} onPause={vi.fn()} />
        ));
        expect(view.getByRole('status').textContent).toBe('Status unavailable');
        expect(view.queryByText('Loading processing status…')).toBeNull();
        expect(processingIndicator(current, false)).toBe('unknown');
    });

    it('distinguishes pending activity, deferred retry, failed results, disabled automation and pause', () => {
        const current = createInitialState();
        expect(processingIndicator(current, false)).toBe('idle');
        const worker = current.snapshot!.workers[0];
        const scenarioMatrix: [Partial<WorkerSnapshot>, boolean, ProcessingIndicator][] = [
            [{ scheduled: true }, false, 'busy'],
            [{ scheduled: false, pending: 5 }, false, 'busy'],
            [{ paused: true }, false, 'paused'],
            [{ active: 1 }, false, 'busy'],
            [{ active: 0, paused: false, automatic_enabled: false }, false, 'idle'],
            [{ retry_wait: 1 }, false, 'warning'],
            [{ retry_wait: 0, failed: 1 }, false, 'warning'],
            [{}, true, 'busy']
        ];
        for (const [scenarioPatch, indexingProgress, expectedIndicator] of scenarioMatrix) {
            Object.assign(worker, scenarioPatch);
            expect(processingIndicator(current, indexingProgress)).toBe(expectedIndicator);
        }
    });
});
