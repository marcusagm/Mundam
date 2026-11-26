/**
 * Dropdown Menu Item List
 *
 * Internal component for rendering the recursive list of items within a dropdown menu.
 * Integrates with the custom keyboard navigation system.
 */

import { Component, For, createEffect, createSignal, Show, Switch, Match } from 'solid-js';
import { ChevronRight } from 'lucide-solid';
import { Dynamic } from 'solid-js/web';
import { cn } from '../../../../lib/utils';
import {
    DropdownMenuItem,
    DropdownContextValue,
    SubmenuMenuItem,
    ActionMenuItem,
    CheckboxMenuItem,
    RadioMenuItem,
    LabelMenuItem
} from '../types';
import { MenuItem } from './MenuItem';
import { MenuCheckboxItem, MenuRadioItem } from './MenuStateItems';
import { useMenuNavigation } from '../useMenuNavigation';

/**
 * Properties for the internal menu list component.
 */
interface MenuListProps {
    /** Array of items to be displayed. */
    items: DropdownMenuItem[];
    /** Internal context for menu state. */
    context?: DropdownContextValue;
}

/**
 * Internal recursive menu list that handles keyboard navigation and item rendering.
 * Uses discriminated unions to ensure type safety without 'as unknown' casts.
 */
export const MenuList: Component<MenuListProps> = props => {
    /** Index of the currently hovered or keyboard-focused submenu. */
    const [activeSubmenuIndex, setActiveSubmenuIndex] = createSignal<number | null>(null);

    /** Accessible keyboard navigation hook for the current list level. */
    const {
        focusedItemIndex,
        setFocusedItemIndex,
        isKeyboardFocusActive,
        setIsKeyboardFocusActive
    } = useMenuNavigation(
        () => props.items,
        () => props.context?.close()
    );

    return (
        <div
            class="ui-dropdown-list"
            role="menu"
            tabIndex={-1} // Allow the container to receive focus for keyboard events
        >
            <For each={props.items}>
                {(item, index) => (
                    <div
                        class="ui-dropdown-item-wrapper"
                        ref={element => {
                            createEffect(() => {
                                if (isKeyboardFocusActive() && focusedItemIndex() === index()) {
                                    element.scrollIntoView?.({
                                        block: 'nearest',
                                        inline: 'nearest'
                                    });
                                }
                            });
                        }}
                        onMouseEnter={() => {
                            setIsKeyboardFocusActive(false);
                            setFocusedItemIndex(index());
                            if (item.type === 'submenu') setActiveSubmenuIndex(index());
                        }}
                        onMouseLeave={() => {
                            if (item.type === 'submenu') setActiveSubmenuIndex(null);
                        }}
                    >
                        <Switch>
                            <Match when={item.type === 'separator'}>
                                <div class="ui-dropdown-separator" role="separator" />
                            </Match>

                            <Match when={item.type === 'label'}>
                                <div class="ui-dropdown-label">{(item as LabelMenuItem).label}</div>
                            </Match>

                            <Match when={item.type === 'item'}>
                                <MenuItem
                                    item={item as ActionMenuItem}
                                    isFocused={focusedItemIndex() === index()}
                                    context={props.context}
                                />
                            </Match>

                            <Match when={item.type === 'checkbox'}>
                                <MenuCheckboxItem
                                    item={item as CheckboxMenuItem}
                                    isFocused={focusedItemIndex() === index()}
                                />
                            </Match>

                            <Match when={item.type === 'radio'}>
                                <MenuRadioItem
                                    item={item as RadioMenuItem}
                                    isFocused={focusedItemIndex() === index()}
                                    context={props.context}
                                />
                            </Match>

                            <Match when={item.type === 'submenu'}>
                                <div
                                    class={cn(
                                        'ui-dropdown-item ui-dropdown-submenu-trigger',
                                        focusedItemIndex() === index() && 'ui-dropdown-item-focused'
                                    )}
                                    role="menuitem"
                                    aria-haspopup="true"
                                    aria-expanded={activeSubmenuIndex() === index()}
                                >
                                    <Show when={(item as SubmenuMenuItem).icon}>
                                        <Dynamic
                                            component={(item as SubmenuMenuItem).icon}
                                            size={14}
                                            class="ui-dropdown-item-icon"
                                        />
                                    </Show>

                                    <span class="ui-dropdown-item-label">
                                        {(item as SubmenuMenuItem).label}
                                    </span>

                                    <ChevronRight size={14} class="ui-dropdown-chevron" />

                                    <Show when={activeSubmenuIndex() === index()}>
                                        <div class="ui-dropdown-submenu">
                                            <MenuList
                                                items={(item as SubmenuMenuItem).items}
                                                context={props.context}
                                            />
                                        </div>
                                    </Show>
                                </div>
                            </Match>
                        </Switch>
                    </div>
                )}
            </For>
        </div>
    );
};
