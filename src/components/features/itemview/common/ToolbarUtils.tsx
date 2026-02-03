import { Component, Show, For } from "solid-js";
import { Kbd } from "../../../ui/Kbd";
import { shortcutStore } from "../../../../core/input";
import { getShortcutDisplayParts } from "../../../../core/input/normalizer";

export const ShortcutHint: Component<{ name: string; scope?: string }> = (props) => {
    const shortcut = () => shortcutStore.getByNameAndScope(props.name, props.scope || 'image-viewer');
    const parts = () => {
        const s = shortcut();
        if (!s) return [];
        const keys = Array.isArray(s.keys) ? s.keys[0] : s.keys;
        return getShortcutDisplayParts(keys);
    };
    
    return (
        <div class="flex items-center gap-3">
            <span>{props.name}</span>
            <Show when={parts().length > 0}>
                <div class="flex gap-1">
                    <For each={parts()}>
                        {part => <Kbd class="text-[10px] h-5 px-1.5 min-w-[20px]">{part}</Kbd>}
                    </For>
                </div>
            </Show>
        </div>
    );
};
