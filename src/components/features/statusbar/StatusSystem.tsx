import { createMemo, createSignal, Match, Switch, type Component } from 'solid-js';
import { CircleCheck, CircleHelp, PaintRoller, Pause, Settings, TriangleAlert } from 'lucide-solid';
import { Button } from '../../ui/Button';
import { PopoverContent, PopoverRoot, PopoverTrigger } from '../../ui/Popover';
import { Loader } from '../../ui/Loader';
import { useSystem } from '../../../core/hooks';
import { useProcessing } from '../../../core/hooks/useProcessing';
import { ProcessingActivityPanel } from './processing/ProcessingActivityPanel';
import { processingIndicator, processingLabels } from './processing/processingPresentation';
import { isMac } from '../../../core/input/utils/platform';

/** Displays authoritative activity and acknowledged session pause controls. */
export const StatusSystem: Component = () => {
    const system = useSystem();
    const processing = useProcessing();
    const [isOpen, setIsOpen] = createSignal(false);
    const indicator = createMemo(() =>
        processingIndicator(processing.state(), Boolean(system.progress()))
    );
    return (
        <div class="statusbar-section statusbar-system">
            <PopoverRoot placement="top-end" isOpen={isOpen()} onOpenChange={setIsOpen}>
                <PopoverTrigger>
                    <Button
                        variant="ghost"
                        size="icon-sm"
                        class="status-btn"
                        classList={{ 'success-text': indicator() === 'idle' }}
                        aria-label={'System activity: ' + processingLabels[indicator()]}
                        aria-haspopup="dialog"
                        aria-expanded={isOpen()}
                        title={processingLabels[indicator()]}
                    >
                        <Switch fallback={<CircleCheck size={12} />}>
                            <Match when={indicator() === 'unknown'}>
                                <CircleHelp size={12} />
                            </Match>
                            <Match when={indicator() === 'busy'}>
                                <Loader size="sm" />
                            </Match>
                            <Match when={indicator() === 'paused'}>
                                <Pause size={12} />
                            </Match>
                            <Match when={indicator() === 'warning'}>
                                <TriangleAlert size={12} />
                            </Match>
                        </Switch>
                    </Button>
                </PopoverTrigger>
                <PopoverContent>
                    <ProcessingActivityPanel
                        state={processing.state()}
                        indexing={system.progress()}
                        onPause={processing.setPaused}
                        onRetry={processing.retry}
                    />
                </PopoverContent>
            </PopoverRoot>
            <div class="statusbar-divider" />
            <Button
                variant="ghost"
                size="icon-sm"
                title={isMac() ? 'Settings (Cmd+,)' : 'Settings (Ctrl+,)'}
                aria-label="Open Settings"
                onClick={() => system.openSettings(true)}
            >
                <Settings size={12} />
            </Button>
            <Button
                variant="ghost"
                size="icon-sm"
                title="Design System"
                aria-label="Open Design System"
                onClick={() => system.openDesignSystem(true)}
            >
                <PaintRoller size={12} />
            </Button>
        </div>
    );
};
