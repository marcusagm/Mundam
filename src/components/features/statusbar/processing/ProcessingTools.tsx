import { For, Show, type Component } from 'solid-js';
import { Button } from '../../../ui/Button';
import type { ExtractionToolStatus } from '../../../../core/store/processing/types';

interface ProcessingToolsProperties {
    tools: ExtractionToolStatus[];
    checking: boolean;
    onCheck?: () => void;
}

const names = { ffmpeg: 'FFmpeg', ffprobe: 'FFprobe', assimp: 'Assimp' };
const states = { unknown: 'Not checked', available: 'Available', unavailable: 'Unavailable' };

/** Shows optional extraction dependencies without implying that every file needs them. */
export const ProcessingTools: Component<ProcessingToolsProperties> = properties => (
    <Show when={properties.tools.length > 0}>
        <div class="processing-tools" aria-label="Extraction tools">
            <strong>Extraction tools</strong>
            <dl class="processing-worker-counts">
                <For each={properties.tools}>
                    {tool => (
                        <div>
                            <dt>{names[tool.tool]}</dt>
                            <dd>{states[tool.state]}</dd>
                        </div>
                    )}
                </For>
            </dl>
            <p class="processing-help">
                Only files that need an unavailable tool wait. Tools are checked automatically every
                minute. Rechecking preserves pauses and retry limits.
            </p>
            <Show when={properties.onCheck}>
                <Button
                    variant="ghost"
                    size="sm"
                    disabled={properties.checking}
                    aria-busy={properties.checking}
                    onClick={() => properties.onCheck?.()}
                >
                    {properties.checking ? 'Checking extraction tools…' : 'Check extraction tools'}
                </Button>
            </Show>
        </div>
    </Show>
);
