import { createSignal, Show, type Component } from 'solid-js';
import { Button, ConfirmModal, ProgressBar, SectionGroup } from '../../ui';
import { useDuplicateScan } from '../duplicates/hooks/useDuplicateScan';
import { useNotification } from '../../../core/hooks';
import { duplicateScanPhaseLabel } from '../../../utils/processing';
import './duplicate-scan-controls.css';

/** Shares manual scan admission and live progress with the finder and statusbar. */
export const DuplicateScanControls: Component = () => {
    const scan = useDuplicateScan();
    const notification = useNotification();
    const [isConfirming, setIsConfirming] = createSignal(false);
    const handleRebuild = async (): Promise<void> => {
        setIsConfirming(false);
        if (await scan.rebuildIndex()) {
            notification.info(
                'Rebuild Finished',
                'Review the duplicate groups and processing status.'
            );
        }
    };
    return (
        <SectionGroup
            title="Scan and Maintenance"
            description="Manual scans analyze eligible files and compare existing fingerprints, even when automatic detection is off."
        >
            <Show when={scan.isScanning()}>
                <p role="status">{duplicateScanPhaseLabel(scan.phase())}</p>
                <Show when={scan.progress() !== undefined}>
                    <p>
                        {scan.status()?.processed} / {scan.status()?.total} files analyzed
                    </p>
                </Show>
                <Show when={!scan.isWaiting()}>
                    <ProgressBar
                        value={scan.progress() ?? 0}
                        isIndeterminate={scan.progress() === undefined}
                    />
                </Show>
            </Show>
            <Show when={!scan.isScanning() && scan.unavailableReason()}>
                <p role="status">{scan.unavailableReason()}</p>
            </Show>
            <div class="setting-action-row duplicate-scan-actions">
                <Show when={scan.isPaused()}>
                    <Button onClick={scan.resumeProcessing} disabled={scan.isControlPending()}>
                        Resume Duplicate Processing
                    </Button>
                </Show>
                <Show
                    when={scan.isScanning()}
                    fallback={
                        <Button onClick={scan.startScan} disabled={!scan.canStart()}>
                            Scan Now
                        </Button>
                    }
                >
                    <Button onClick={scan.cancelScan} disabled={scan.isStopping()}>
                        {scan.isStopping() ? 'Stopping…' : 'Cancel Scan'}
                    </Button>
                </Show>
                <Button
                    variant="outline"
                    aria-label="Rebuild Index"
                    onClick={() => setIsConfirming(true)}
                    disabled={!scan.canStart()}
                >
                    Rebuild Index
                </Button>
            </div>
            <div class="duplicate-scan-note-row">
                <p class="duplicates-setting-description">
                    File analysis respects scheduled retries and exhausted attempts. Scan Now
                    requests fresh comparisons. Rebuild also resets file analysis. Files, thumbnails
                    and colors are preserved.
                </p>
            </div>
            <ConfirmModal
                isOpen={isConfirming()}
                onClose={() => setIsConfirming(false)}
                onConfirm={handleRebuild}
                title="Rebuild Duplicate Index?"
                message="This clears duplicate fingerprints, groups and resolution history, then analyzes the library again. Original files, thumbnails and colors are preserved."
                confirmText="Rebuild Index"
                kind="warning"
            />
        </SectionGroup>
    );
};
