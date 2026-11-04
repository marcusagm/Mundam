import { createEffect, createMemo, createSignal } from 'solid-js';
import { useProcessing } from '../../../../core/hooks/useProcessing';
import { useNotification } from '../../../../core/hooks';
import { duplicatesApi } from '../../../../lib/duplicates';

interface ScanRequest {
    mode: 'scan' | 'rebuild';
    cancelRequested: boolean;
}

const [request, setRequest] = createSignal<ScanRequest | null>(null, { equals: false });
const [isCancelling, setIsCancelling] = createSignal(false);
const [cancelAcknowledged, setCancelAcknowledged] = createSignal(false);

/** Shares command admission across settings and duplicate views; snapshots own execution state. */
export function useDuplicateScan() {
    const processing = useProcessing();
    const notification = useNotification();
    const status = createMemo(() => processing.state().snapshot?.manual_scan);
    const worker = createMemo(() =>
        processing.state().snapshot?.workers.find(worker => worker.worker === 'duplicates')
    );
    const isScanning = createMemo(() => Boolean(request() || status()?.is_running));
    createEffect(() => {
        if (!status()?.is_running && !request()) setCancelAcknowledged(false);
    });
    const isStopping = createMemo(() =>
        Boolean(
            isCancelling() ||
            cancelAcknowledged() ||
            request()?.cancelRequested ||
            status()?.phase === 'cancel_requested'
        )
    );
    const unavailableReason = createMemo(() => {
        if (processing.state().error || !worker()) return 'Waiting for current processing status.';
        if (!worker()?.online) return 'Duplicate processing is unavailable.';
        if (isCancelling()) return 'Waiting for cancellation acknowledgement.';
        if (isScanning()) return 'A manual scan is already running.';
        if (worker()?.paused)
            return 'Resume duplicate processing in System Activity to start a scan.';
        if (processing.state().pending.has('duplicates')) return 'Waiting for processing control.';
        return '';
    });
    const phase = createMemo(() => {
        if (isStopping()) return 'cancel_requested';
        return status()?.is_running ? status()!.phase : 'preparing';
    });
    const progress = createMemo(() => {
        if (isStopping()) return undefined;
        const current = status();
        if (!current?.is_running || current.phase !== 'hashing' || current.total <= 0)
            return undefined;
        return Math.min(100, Math.max(0, (current.processed / current.total) * 100));
    });

    const run = async (mode: ScanRequest['mode']): Promise<boolean> => {
        if (unavailableReason()) return false;
        setCancelAcknowledged(false);
        const current: ScanRequest = { mode, cancelRequested: false };
        setRequest(current);
        try {
            const operation =
                mode === 'rebuild'
                    ? duplicatesApi.rebuildDuplicateIndex()
                    : duplicatesApi.startDuplicateScan();
            void processing.refresh();
            await operation;
            return !current.cancelRequested;
        } catch (error) {
            console.error('Duplicate scan failed:', error);
            notification.error('Scan Failed', 'The duplicate scan could not be completed.');
            return false;
        } finally {
            await processing.refresh();
            setRequest(null);
        }
    };

    const cancelScan = async (): Promise<void> => {
        if (!isScanning() || isStopping()) return;
        const current = request();
        setIsCancelling(true);
        if (current) {
            current.cancelRequested = true;
            setRequest(current);
        }
        try {
            await duplicatesApi.cancelDuplicateScan();
            if (request() || status()?.is_running) setCancelAcknowledged(true);
        } catch (error) {
            if (current && request() === current) {
                current.cancelRequested = false;
                setRequest(current);
            }
            console.error('Duplicate scan cancellation failed:', error);
            notification.error('Cancellation Failed', 'The scan is still running. Try again.');
        } finally {
            await processing.refresh();
            setIsCancelling(false);
        }
    };

    return {
        isScanning,
        isStopping,
        phase,
        status,
        progress,
        unavailableReason,
        canStart: () => !unavailableReason(),
        isPaused: () => Boolean(worker()?.paused),
        resumeProcessing: () => processing.setPaused('duplicates', false),
        isControlPending: () => processing.state().pending.has('duplicates'),
        startScan: () => run('scan'),
        rebuildIndex: () => run('rebuild'),
        cancelScan
    };
}
