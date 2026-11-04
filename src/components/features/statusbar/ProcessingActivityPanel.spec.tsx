import { cleanup, fireEvent, render } from '@solidjs/testing-library';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createSignal } from 'solid-js';
import { ProcessingActivityPanel } from './ProcessingActivityPanel';
import { processingIndicator } from './processingPresentation';
import type { ProcessingClientState, WorkerSnapshot } from '../../../core/store/processingTypes';

const row = (worker: WorkerSnapshot['worker']): WorkerSnapshot => ({
    worker,
    online: true,
    paused: false,
    active: 0,
    phase: null,
    scheduled: false,
    pending: 0,
    retry_wait: 0,
    failed: 0,
    skipped: 0,
    automatic_enabled: true
});
const state = (): ProcessingClientState => ({
    snapshot: {
        instance_id: 'session',
        sequence: 1,
        workers: [row('thumbnails'), row('colors'), row('duplicates')],
        manual_scan: { is_running: false, phase: 'idle', processed: 0, total: 0 }
    },
    error: null,
    pending: new Set()
});

afterEach(cleanup);

describe('processing statusbar controls', () => {
    it('shows cancellation and matching phases without a misleading hashing percentage', () => {
        const [current, setCurrent] = createSignal(state());
        const active = state();
        active.snapshot!.manual_scan = {
            is_running: true,
            phase: 'cancel_requested',
            processed: 10,
            total: 10
        };
        setCurrent(active);
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
                manual_scan: { is_running: true, phase: 'exact_matching', processed: 10, total: 10 }
            }
        }));
        expect(view.getByRole('status').textContent).toContain('Comparing identical files');
        expect(view.queryByText('10 / 10 files analyzed')).toBeNull();
    });
    it('preserves focused controls when polling replaces the worker snapshots', () => {
        const [current, setCurrent] = createSignal(state());
        const view = render(() => (
            <ProcessingActivityPanel state={current()} indexing={null} onPause={vi.fn()} />
        ));
        const button = view.getByRole('button', { name: 'Pause Thumbnails and metadata' });
        button.focus();
        setCurrent(previous => ({
            ...previous,
            snapshot: {
                ...previous.snapshot!,
                sequence: 2,
                workers: previous.snapshot!.workers.map(worker => ({ ...worker, pending: 42 }))
            }
        }));
        expect(view.getByRole('button', { name: 'Pause Thumbnails and metadata' })).toBe(button);
        expect(document.activeElement).toBe(button);
        expect(view.getAllByText('42')).toHaveLength(3);
    });
    it('retains the acknowledged state until a control response and disables duplicate requests', () => {
        const [current, setCurrent] = createSignal(state());
        const action = vi.fn();
        const view = render(() => (
            <ProcessingActivityPanel state={current()} indexing={null} onPause={action} />
        ));
        const button = view.getByRole('button', { name: 'Pause Thumbnails and metadata' });
        fireEvent.click(button);
        expect(action).toHaveBeenCalledWith('thumbnails', true);
        expect(view.getByRole('button', { name: 'Pause Thumbnails and metadata' })).toBe(button);
        setCurrent(previous => ({ ...previous, pending: new Set(['thumbnails']) }));
        expect(button.hasAttribute('disabled')).toBe(true);
        setCurrent(previous => ({
            ...previous,
            pending: new Set(),
            snapshot: {
                ...previous.snapshot!,
                workers: [
                    { ...row('thumbnails'), paused: true, active: 1, phase: 'thumbnail' },
                    row('colors'),
                    row('duplicates')
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
        const current = state();
        expect(processingIndicator(current, false)).toBe('idle');
        const worker = current.snapshot!.workers[0];
        worker.scheduled = true;
        expect(processingIndicator(current, false)).toBe('busy');
        worker.scheduled = false;
        worker.pending = 5;
        expect(processingIndicator(current, false)).toBe('busy');
        worker.paused = true;
        expect(processingIndicator(current, false)).toBe('paused');
        worker.active = 1;
        expect(processingIndicator(current, false)).toBe('busy');
        worker.active = 0;
        worker.paused = false;
        worker.automatic_enabled = false;
        expect(processingIndicator(current, false)).toBe('idle');
        worker.retry_wait = 1;
        expect(processingIndicator(current, false)).toBe('warning');
        worker.retry_wait = 0;
        worker.failed = 1;
        expect(processingIndicator(current, false)).toBe('warning');
        expect(processingIndicator(current, true)).toBe('busy');
    });
});
