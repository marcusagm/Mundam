import type { Component } from 'solid-js';
import { Checkbox } from '../Checkbox';

import type { ContextMenuCheckboxItemProps } from './types';

/** Shared checkbox row for menu preferences.
 * @param props - Controlled preference and menu callbacks.
 * @returns Accessible checkbox using the shared UI control.
 * @example <ContextMenuCheckboxItem label="Recursive" checked={true} onCheckedChange={() => {}} onClose={() => {}} />
 */
export const ContextMenuCheckboxItem: Component<ContextMenuCheckboxItemProps> = props => (
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
