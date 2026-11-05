import type { Component } from 'solid-js';
import { Checkbox } from '../Checkbox';

/** Shared checkbox row for context-menu preferences and toggles. */
export const ContextMenuCheckboxItem: Component<{
    label: string;
    checked: boolean;
    onCheckedChange: (checked: boolean) => void;
    onClose: () => void;
}> = props => (
    <div class="ui-context-menu-item">
        <Checkbox
            class="ui-context-menu-checkbox"
            label={props.label}
            checked={props.checked}
            onCheckedChange={checked => {
                props.onCheckedChange(checked);
                props.onClose();
            }}
        />
    </div>
);
