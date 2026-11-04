import { cleanup, fireEvent, render, screen, waitFor, within } from '@solidjs/testing-library';
import { createRoot, createSignal } from 'solid-js';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ProcessingClientState } from '../../../core/store/processing/types';
import { useDuplicateScan } from '../duplicates/hooks/useDuplicateScan';
import { DuplicateScanControls } from './DuplicateScanControls';
import { Switch } from '../../ui/Switch';

const mocks = vi.hoisted(() => ({
    state: vi.fn(),
    refresh: vi.fn(),
    setPaused: vi.fn(),
    start: vi.fn(),
    rebuild: vi.fn(),
    cancel: vi.fn(),
    error: vi.fn(),
    info: vi.fn()
}));
vi.mock('../../../core/hooks/useProcessing', () => ({
    useProcessing: () => ({
        state: mocks.state,
        refresh: mocks.refresh,
        setPaused: mocks.setPaused
    })
}));
vi.mock('../../../core/hooks', () => ({
    useNotification: () => ({ error: mocks.error, info: mocks.info })
}));
vi.mock('../../../lib/duplicates', () => ({
    duplicatesApi: {
        startDuplicateScan: mocks.start,
        rebuildDuplicateIndex: mocks.rebuild,
        cancelDuplicateScan: mocks.cancel
    }
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue(null) }));

function snapshot(): ProcessingClientState {
    return {
        error: null,
        pending: new Set(),
        snapshot: {
            instance_id: 'session',
            sequence: 1,
            workers: [
                {
                    worker: 'duplicates',
                    online: true,
                    paused: false,
                    active: 0,
                    phase: null,
                    scheduled: false,
                    pending: 0,
                    retry_wait: 0,
                    failed: 0,
                    skipped: 0,
                    automatic_enabled: false
                }
            ],
            manual_scan: { is_running: false, phase: 'idle', processed: 0, total: 0 }
        }
    };
}

function deferred() {
    let resolve!: () => void;
    const promise = new Promise<void>(complete => {
        resolve = complete;
    });
    return { promise, resolve };
}

let update: (state: ProcessingClientState) => void;
const disposers: (() => void)[] = [];
function controls() {
    return createRoot(dispose => {
        disposers.push(dispose);
        return useDuplicateScan();
    });
}
beforeEach(() => {
    vi.clearAllMocks();
    mocks.refresh.mockResolvedValue(undefined);
    mocks.start.mockResolvedValue(undefined);
    mocks.rebuild.mockResolvedValue(undefined);
    mocks.cancel.mockResolvedValue(undefined);
    const [state, setState] = createSignal(snapshot());
    // The mock invokes this getter inside the consumers' tracked memos.
    // eslint-disable-next-line solid/reactivity
    mocks.state.mockImplementation(() => state());
    update = setState;
});
afterEach(() => {
    cleanup();
    disposers.splice(0).forEach(dispose => dispose());
    vi.restoreAllMocks();
});

describe('duplicate processing integration', () => {
    it('keeps recovered comparisons visible while paused without animating progress', () => {
        const current = snapshot();
        current.snapshot!.workers[0].paused = true;
        current.snapshot!.manual_scan = {
            is_running: true,
            phase: 'paused',
            processed: 0,
            total: 0
        };
        update(current);
        const view = render(() => <DuplicateScanControls />);
        expect(view.getByRole('status').textContent).toBe('Duplicate comparison paused');
        expect(view.queryByRole('progressbar')).toBeNull();
        expect(view.getByRole('button', { name: 'Cancel Scan' })).toBeTruthy();
        fireEvent.click(view.getByRole('button', { name: 'Resume Duplicate Processing' }));
        expect(mocks.setPaused).toHaveBeenCalledWith('duplicates', false);
    });
    it('shares admission across views and recovers phases and completion from snapshots', async () => {
        const pending = deferred();
        mocks.start.mockReturnValue(pending.promise);
        const finder = controls();
        const settings = controls();
        const running = finder.startScan();
        expect(settings.isScanning()).toBe(true);
        expect(await settings.rebuildIndex()).toBe(false);
        expect(mocks.rebuild).not.toHaveBeenCalled();
        const active = snapshot();
        active.snapshot!.manual_scan = {
            is_running: true,
            phase: 'hashing',
            processed: 7,
            total: 10
        };
        update(active);
        expect(settings.progress()).toBe(70);
        update({
            ...active,
            snapshot: {
                ...active.snapshot!,
                manual_scan: { ...active.snapshot!.manual_scan, phase: 'visual_matching' }
            }
        });
        expect(finder.progress()).toBeUndefined();
        expect(finder.phase()).toBe('visual_matching');
        update(snapshot());
        pending.resolve();
        expect(await running).toBe(true);
        expect(settings.isScanning()).toBe(false);
    });

    it('blocks stale, paused and offline starts while allowing manual scans with automation off', async () => {
        const scan = controls();
        for (const kind of ['paused', 'offline', 'error', 'unknown']) {
            const current = snapshot();
            if (kind === 'paused') current.snapshot!.workers[0].paused = true;
            if (kind === 'offline') current.snapshot!.workers[0].online = false;
            if (kind === 'error') current.error = 'Unavailable';
            if (kind === 'unknown') current.snapshot = null;
            update(current);
            expect(await scan.startScan()).toBe(false);
        }
        expect(mocks.start).not.toHaveBeenCalled();
        update(snapshot());
        expect(await scan.startScan()).toBe(true);
        expect(mocks.start).toHaveBeenCalledTimes(1);
    });

    it('keeps stopping distinct from stopped and does not report a canceled rebuild as completed', async () => {
        const pending = deferred();
        mocks.rebuild.mockReturnValue(pending.promise);
        const scan = controls();
        const running = scan.rebuildIndex();
        await scan.cancelScan();
        expect(scan.isStopping()).toBe(true);
        expect(scan.isScanning()).toBe(true);
        await scan.cancelScan();
        expect(mocks.cancel).toHaveBeenCalledTimes(1);
        pending.resolve();
        expect(await running).toBe(false);
        expect(scan.isScanning()).toBe(false);
    });

    it('retains a remote scan after cancellation fails and shows one actionable error', async () => {
        vi.spyOn(console, 'error').mockImplementation(() => {});
        const current = snapshot();
        current.snapshot!.manual_scan = {
            is_running: true,
            phase: 'hashing',
            processed: 3,
            total: 10
        };
        update(current);
        mocks.cancel.mockRejectedValueOnce(new Error('Disconnected'));
        const scan = controls();
        await scan.cancelScan();
        expect(scan.isScanning()).toBe(true);
        expect(scan.isStopping()).toBe(false);
        expect(mocks.error).toHaveBeenCalledTimes(1);
        update(snapshot());
        expect(scan.isScanning()).toBe(false);
    });

    it('retains an acknowledged remote cancellation until a snapshot confirms exit', async () => {
        const current = snapshot();
        current.snapshot!.manual_scan = {
            is_running: true,
            phase: 'hashing',
            processed: 3,
            total: 10
        };
        update(current);
        const scan = controls();
        await scan.cancelScan();
        expect(scan.isStopping()).toBe(true);
        expect(scan.isScanning()).toBe(true);
        await scan.cancelScan();
        expect(mocks.cancel).toHaveBeenCalledTimes(1);
        update(snapshot());
        expect(scan.isScanning()).toBe(false);
        expect(scan.isStopping()).toBe(false);
    });

    it('requires rebuild confirmation and rechecks pause before erasing the index', async () => {
        const view = render(() => <DuplicateScanControls />);
        fireEvent.click(view.getByRole('button', { name: 'Rebuild Index' }));
        expect(mocks.rebuild).not.toHaveBeenCalled();
        const dialog = await screen.findByRole('alertdialog');
        expect(dialog.textContent).toContain('Original files, thumbnails and colors are preserved');
        const paused = snapshot();
        paused.snapshot!.workers[0].paused = true;
        update(paused);
        fireEvent.click(within(dialog).getByRole('button', { name: 'Rebuild Index' }));
        await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
        expect(mocks.rebuild).not.toHaveBeenCalled();
        fireEvent.click(view.getByRole('button', { name: 'Resume Duplicate Processing' }));
        expect(mocks.setPaused).toHaveBeenCalledWith('duplicates', false);
    });

    it('presents matching as a distinct phase without stale hashing counts', () => {
        const current = snapshot();
        current.snapshot!.manual_scan = {
            is_running: true,
            phase: 'exact_matching',
            processed: 10,
            total: 10
        };
        update(current);
        const view = render(() => <DuplicateScanControls />);
        expect(view.getByRole('status').textContent).toBe('Comparing identical files');
        expect(view.queryByText('10 / 10 files analyzed')).toBeNull();
    });

    it('places the automation switch accessible name on the interactive control', () => {
        const view = render(() => <Switch aria-label="Automatic Duplicate Detection" />);
        expect(view.getByRole('switch', { name: 'Automatic Duplicate Detection' })).toBeTruthy();
    });
});
