import { For, Show, type Component } from 'solid-js';
import type { ProcessingDiagnostic } from '../../../../core/store/processing/types';

/** Presents only safe causes, never server error strings or asset paths. */
const safeDiagnosticMessages: Record<string, string> = {
    provider_missing:
        'No extractor recognizes this format. Update extractors and restart, then recheck.',
    capability_missing:
        'The format has no supported extraction capability. Update extractors and restart before trying again.',
    tool_unavailable: 'A required extraction tool is unavailable. Restore it, then retry.',
    ffmpeg_unavailable: 'FFmpeg is unavailable. Restore the tool, then check extraction tools.',
    ffprobe_unavailable: 'FFprobe is unavailable. Restore the tool, then check extraction tools.',
    assimp_unavailable: 'Assimp is unavailable. Restore the tool, then check extraction tools.',
    input_unavailable:
        'An input file or cached thumbnail is unavailable. Check its location and access, then retry.',
    invalid_output:
        'The extractor returned an unusable image. Check the input file or update the extractor.',
    timeout: 'Extraction timed out. Check the file and processing settings before retrying.',
    storage_failure: 'The result could not be saved. Check storage availability before retrying.',
    extraction_failed:
        'Extraction failed. Check the input file and extraction tools before retrying.',
    unknown_failure: 'An earlier extraction failed. Retry to obtain a current diagnosis.',
    unknown_unsupported:
        'An earlier attempt reported no support. Recheck after updating extractors.'
};

const outcomeStatusLabels: Record<ProcessingDiagnostic['state'], string> = {
    failed: 'Failed',
    retry_wait: 'Waiting to retry',
    skipped: 'Unsupported',
    blocked: 'Waiting for tools'
};

/** Collapsed diagnostics keep the activity panel compact and polling quiet. */
export const ProcessingDiagnostics: Component<{
    diagnostics: ProcessingDiagnostic[];
}> = properties => (
    <Show when={properties.diagnostics.length > 0}>
        <details class="processing-diagnostics">
            <summary>Processing details</summary>
            <ul>
                <For each={properties.diagnostics}>
                    {diagnosticItem => (
                        <li>
                            <strong>
                                {diagnosticItem.count} —{' '}
                                {outcomeStatusLabels[diagnosticItem.state] || 'Processing issue'}
                                :{' '}
                            </strong>
                            {safeDiagnosticMessages[diagnosticItem.issue] ||
                                safeDiagnosticMessages.unknown_failure}
                        </li>
                    )}
                </For>
            </ul>
        </details>
    </Show>
);
