/** Names finite duplicate-scan phases without exposing internal identifiers. */
export function duplicateScanPhaseLabel(phase: string): string {
    const labels: Record<string, string> = {
        preparing: 'Preparing scan',
        recovering: 'Resuming duplicate comparison',
        paused: 'Duplicate comparison paused',
        retry_wait: 'Waiting to retry duplicate comparison',
        hashing: 'Analyzing files',
        exact_matching: 'Comparing identical files',
        visual_matching: 'Comparing similar images',
        cancel_requested: 'Stopping — finishing the current operation',
        idle: 'No manual scan running'
    };
    return labels[phase] || 'Scanning library';
}

/** Describes queue admission only, keeping eventual execution and completion distinct. */
export function formatProcessingRetry(
    counts: { assets: number; comparisons: number },
    scope: 'failed' | 'unsupported'
): string {
    const parts: string[] = [];
    if (counts.assets > 0) parts.push(`${counts.assets} ${counts.assets === 1 ? 'file' : 'files'}`);
    if (counts.comparisons > 0)
        parts.push(
            `${counts.comparisons} ${counts.comparisons === 1 ? 'comparison' : 'comparisons'}`
        );
    if (parts.length === 0) return 'No eligible outcomes to queue.';
    return `${parts.join(' and ')} queued for ${scope === 'failed' ? 'retry' : 'recheck'}. Pause and automation settings still apply.`;
}
