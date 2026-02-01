import { Component, Show, createSignal, onMount, onCleanup } from "solid-js";
import { useSystem } from "../../../core/hooks";
import { Loader } from "../../ui/Loader";
import { CircleCheck, CircleAlert, Settings, List } from "lucide-solid";
import { Button } from "../../ui/Button";

export const StatusSystem: Component = () => {
    const system = useSystem();
    const [thumbnailQueue, setThumbnailQueue] = createSignal(0);
    const [isPopoverOpen, setIsPopoverOpen] = createSignal(false);

    // TODO: Wire up to real backend event "thumbnail:queue-status"
    // Mocking or listening to existing events
    
    return (
        <div class="statusbar-section statusbar-system">
             {/* Background Processes Indicator */}
             <div class="system-indicator">
                <Switch fallback={
                    <Button 
                        variant="ghost" 
                        size="icon-sm" 
                        class="status-btn success-text" 
                        title="All Systems Operational"
                        onClick={() => setIsPopoverOpen(!isPopoverOpen())}
                    >
                        <CircleCheck size={12} />
                    </Button>
                }>
                    <Match when={system.progress() || thumbnailQueue() > 0}>
                         <Button 
                            variant="ghost" 
                            size="icon-sm" 
                            class="status-btn"
                            onClick={() => setIsPopoverOpen(!isPopoverOpen())}
                         >
                            <Loader size="sm" />
                         </Button>
                    </Match>
                </Switch>
             </div>

             {/* Popover Logic (Simplistic Inline for now, ideal to be a real Popover) */}
             <Show when={isPopoverOpen()}>
                <div class="system-popover">
                    <div class="popover-header">System Activity</div>
                    <div class="popover-content">
                        <Show when={!system.progress() && thumbnailQueue() === 0}>
                            <div class="empty-state">No background tasks running.</div>
                        </Show>
                        
                        <Show when={system.progress()}>
                             <div class="task-row">
                                <Loader size="sm" />
                                <div>
                                    <div class="task-name">Indexing Library</div>
                                    <div class="task-status">
                                        {system.progress()?.processed} / {system.progress()?.total}
                                    </div>
                                </div>
                             </div>
                        </Show>
                        
                        {/* Add Thumbnail Logic Here Later */}
                    </div>
                </div>
             </Show>

            <div class="statusbar-divider" />

             {/* Settings Trigger */}
             <Button 
                variant="ghost" 
                size="icon-sm"
                title="Settings (Cmd+,)"
                onClick={() => window.dispatchEvent(new CustomEvent('app:open-settings'))}
            >
                <Settings size={12} />
            </Button>
        </div>
    );
};

// Helper for Switch/Match imports
import { Switch, Match } from "solid-js";
