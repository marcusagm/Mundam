/** Names finite duplicate-scan phases without exposing internal identifiers. */
export function duplicateScanPhaseLabel(phase: string): string {
    const labels: Record<string, string> = {
        preparing: 'Preparing scan',
        hashing: 'Analyzing files',
        exact_matching: 'Comparing identical files',
        visual_matching: 'Comparing similar images',
        cancel_requested: 'Stopping — finishing the current operation',
        idle: 'No manual scan running'
    };
    return labels[phase] || 'Scanning library';
}
