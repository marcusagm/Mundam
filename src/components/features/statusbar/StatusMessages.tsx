import { createMemo, type Component } from 'solid-js';
import { Info } from 'lucide-solid';
import { useProcessing } from '../../../core/hooks/useProcessing';
import { useSystem } from '../../../core/hooks';
import { processingIndicator, processingLabels } from './processingPresentation';

/** Ready is shown only after worker health is known. */
export const StatusMessages: Component = () => {
    const processing = useProcessing();
    const system = useSystem();
    const label = createMemo(
        () => processingLabels[processingIndicator(processing.state(), Boolean(system.progress()))]
    );
    return (
        <div class="statusbar-section statusbar-messages">
            <Info size={12} class="text-muted" />
            <span class="text-muted text-xs">{label()}</span>
        </div>
    );
};
