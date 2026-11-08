import { For, Show, type Component } from 'solid-js';
import { Button } from '../../../ui/Button';
import { Badge } from '../../../ui/Badge';
import { Wrench } from 'lucide-solid';
import type { ExtractionToolStatus } from '../../../../core/store/processing/types';

interface ProcessingToolsProperties {
    tools: ExtractionToolStatus[];
    checking: boolean;
    onCheck?: () => void;
}

const toolDisplayNames: Record<string, string> = {
    ffmpeg: 'FFmpeg',
    ffprobe: 'FFprobe',
    assimp: 'Assimp'
};

const toolStateLabels: Record<string, string> = {
    unknown: 'Not checked',
    available: 'Available',
    unavailable: 'Unavailable'
};

const toolBadgeVariant = (state: string): 'success' | 'warning' | 'secondary' => {
    if (state === 'available') return 'success';
    if (state === 'unavailable') return 'warning';
    return 'secondary';
};

/** Shows optional extraction dependencies without implying that every file needs them. */
export const ProcessingTools: Component<ProcessingToolsProperties> = properties => (
    <Show when={properties.tools.length > 0}>
        <div class="processing-tools-card processing-tools" aria-label="Extraction tools">
            <div class="processing-tools-header">
                <div class="processing-tools-title-group">
                    <Wrench size={14} class="processing-tools-icon" />
                    <strong>Extraction tools</strong>
                </div>
                <Show when={properties.onCheck}>
                    <Button
                        variant="outline"
                        size="sm"
                        disabled={properties.checking}
                        aria-busy={properties.checking}
                        onClick={() => properties.onCheck?.()}
                    >
                        {properties.checking
                            ? 'Checking extraction tools…'
                            : 'Check extraction tools'}
                    </Button>
                </Show>
            </div>

            <dl class="processing-worker-counts processing-tools-grid">
                <For each={properties.tools}>
                    {tool => (
                        <div class="processing-tool-item">
                            <dt>{toolDisplayNames[tool.tool] || tool.tool}</dt>
                            <dd>
                                <Badge variant={toolBadgeVariant(tool.state)} size="sm">
                                    {toolStateLabels[tool.state] || tool.state}
                                </Badge>
                            </dd>
                        </div>
                    )}
                </For>
            </dl>

            <p class="processing-help">
                Only files that need an unavailable tool wait. Tools are checked automatically every
                minute. Rechecking preserves pauses and retry limits.
            </p>
        </div>
    </Show>
);
