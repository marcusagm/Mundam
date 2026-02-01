import { Component } from "solid-js";
import { Info } from "lucide-solid";

export const StatusMessages: Component = () => {
    return (
        <div class="statusbar-section statusbar-messages">
             {/* Placeholder for future "Tip of the day" or "Action feedback" */}
             <Info size={12} class="text-muted" />
             <span class="text-muted text-xs">Ready</span>
        </div>
    );
};
