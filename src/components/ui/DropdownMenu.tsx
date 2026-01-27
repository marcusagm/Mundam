import {
  Component,
  JSX,
  splitProps,
  createSignal,
  createEffect,
  onCleanup,
  Show,
  For,
  createContext,
  useContext,
  createMemo,
  Accessor,
} from "solid-js";
import { Portal, Dynamic } from "solid-js/web";
import { ChevronRight, Check } from "lucide-solid";
import { cn } from "../../lib/utils";
import { createClickOutside } from "../../lib/primitives";
import "./dropdown-menu.css";

// Types
export type DropdownMenuItem =
  | { type: "item"; label: string; icon?: Component<{ size?: number }>; action: () => void; disabled?: boolean; shortcut?: string }
  | { type: "checkbox"; label: string; checked: boolean; onCheckedChange: (checked: boolean) => void; disabled?: boolean }
  | { type: "radio"; label: string; value: string }
  | { type: "separator" }
  | { type: "label"; label: string }
  | { type: "submenu"; label: string; icon?: Component<{ size?: number }>; items: DropdownMenuItem[] };

// Context
interface DropdownContextValue {
  close: () => void;
  radioValue?: Accessor<string>;
  onRadioChange?: (value: string) => void;
}

const DropdownContext = createContext<DropdownContextValue>();

// Dropdown Root
export interface DropdownMenuProps {
  trigger: JSX.Element;
  items: DropdownMenuItem[];
  align?: "start" | "center" | "end";
  side?: "top" | "bottom" | "left" | "right";
  radioValue?: string;
  onRadioChange?: (value: string) => void;
  class?: string;
}

/**
 * DropdownMenu component with items, checkboxes, radios, and submenus.
 * 
 * @example
 * <DropdownMenu
 *   trigger={<Button>Options</Button>}
 *   items={[
 *     { type: "item", label: "Edit", action: () => {} },
 *     { type: "separator" },
 *     { type: "item", label: "Delete", action: () => {} },
 *   ]}
 * />
 */
export const DropdownMenu: Component<DropdownMenuProps> = (props) => {
  const [local] = splitProps(props, [
    "trigger",
    "items",
    "align",
    "side",
    "radioValue",
    "onRadioChange",
    "class",
  ]);

  const [isOpen, setIsOpen] = createSignal(false);
  const [position, setPosition] = createSignal({ top: 0, left: 0 });

  let triggerRef: HTMLDivElement | undefined;
  let contentRef: HTMLDivElement | undefined;

  const close = () => setIsOpen(false);

  // Calculate position
  createEffect(() => {
    if (isOpen() && triggerRef && contentRef) {
      const triggerRect = triggerRef.getBoundingClientRect();
      const contentRect = contentRef.getBoundingClientRect();
      const viewport = { width: window.innerWidth, height: window.innerHeight };

      let top = 0;
      let left = 0;

      const side = local.side || "bottom";
      const align = local.align || "start";

      // Side positioning
      if (side === "bottom") {
        top = triggerRect.bottom + 4;
      } else if (side === "top") {
        top = triggerRect.top - contentRect.height - 4;
      } else if (side === "left") {
        left = triggerRect.left - contentRect.width - 4;
        top = triggerRect.top;
      } else if (side === "right") {
        left = triggerRect.right + 4;
        top = triggerRect.top;
      }

      // Align positioning (for top/bottom)
      if (side === "top" || side === "bottom") {
        if (align === "start") {
          left = triggerRect.left;
        } else if (align === "center") {
          left = triggerRect.left + (triggerRect.width - contentRect.width) / 2;
        } else if (align === "end") {
          left = triggerRect.right - contentRect.width;
        }
      }

      // Viewport bounds check
      if (left + contentRect.width > viewport.width) {
        left = viewport.width - contentRect.width - 8;
      }
      if (left < 8) left = 8;
      if (top + contentRect.height > viewport.height) {
        top = triggerRect.top - contentRect.height - 4;
      }
      if (top < 8) top = 8;

      setPosition({ top, left });
    }
  });

  // Click outside
  createClickOutside(
    () => contentRef,
    () => {
      if (isOpen()) close();
    }
  );

  // Escape key
  createEffect(() => {
    if (!isOpen()) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        close();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    onCleanup(() => window.removeEventListener("keydown", handleKeyDown));
  });

  const contextValue: DropdownContextValue = {
    close,
    radioValue: () => local.radioValue || "",
    onRadioChange: local.onRadioChange,
  };

  return (
    <DropdownContext.Provider value={contextValue}>
      <div class={cn("ui-dropdown", local.class)}>
        <div
          ref={triggerRef}
          class="ui-dropdown-trigger"
          onClick={() => setIsOpen(!isOpen())}
        >
          {local.trigger}
        </div>

        <Show when={isOpen()}>
          <Portal>
            <div
              ref={contentRef}
              class="ui-dropdown-content"
              style={{
                position: "fixed",
                top: `${position().top}px`,
                left: `${position().left}px`,
                "z-index": 9999,
              }}
            >
              <MenuList items={local.items} />
            </div>
          </Portal>
        </Show>
      </div>
    </DropdownContext.Provider>
  );
};

// Internal Menu List (handles submenu recursion)
const MenuList: Component<{ items: DropdownMenuItem[] }> = (props) => {
  const [activeSubmenu, setActiveSubmenu] = createSignal<number | null>(null);

  return (
    <div class="ui-dropdown-list" role="menu">
      <For each={props.items}>
        {(item, index) => (
          <div
            class="ui-dropdown-item-wrapper"
            onMouseEnter={() => item.type === "submenu" && setActiveSubmenu(index())}
            onMouseLeave={() => setActiveSubmenu(null)}
          >
            <Show when={item.type === "separator"}>
              <div class="ui-dropdown-separator" role="separator" />
            </Show>

            <Show when={item.type === "label"}>
              <div class="ui-dropdown-label">{(item as any).label}</div>
            </Show>

            <Show when={item.type === "item"}>
              <MenuItem item={item as any} />
            </Show>

            <Show when={item.type === "checkbox"}>
              <MenuCheckboxItem item={item as any} />
            </Show>

            <Show when={item.type === "radio"}>
              <MenuRadioItem item={item as any} />
            </Show>

            <Show when={item.type === "submenu"}>
              <div class="ui-dropdown-item ui-dropdown-submenu-trigger">
                <Show when={(item as any).icon}>
                  <Dynamic component={(item as any).icon} size={14} />
                </Show>
                <span class="ui-dropdown-item-label">{(item as any).label}</span>
                <ChevronRight size={14} class="ui-dropdown-chevron" />
                
                <Show when={activeSubmenu() === index()}>
                  <div class="ui-dropdown-submenu">
                    <MenuList items={(item as any).items} />
                  </div>
                </Show>
              </div>
            </Show>
          </div>
        )}
      </For>
    </div>
  );
};

// Menu Item
const MenuItem: Component<{ item: Extract<DropdownMenuItem, { type: "item" }> }> = (props) => {
  const context = useContext(DropdownContext);

  const handleClick = () => {
    if (props.item.disabled) return;
    props.item.action();
    context?.close();
  };

  return (
    <div
      class={cn("ui-dropdown-item", props.item.disabled && "ui-dropdown-item-disabled")}
      role="menuitem"
      aria-disabled={props.item.disabled}
      onClick={handleClick}
    >
      <Show when={props.item.icon}>
        <Dynamic component={props.item.icon} size={14} />
      </Show>
      <span class="ui-dropdown-item-label">{props.item.label}</span>
      <Show when={props.item.shortcut}>
        <span class="ui-dropdown-shortcut">{props.item.shortcut}</span>
      </Show>
    </div>
  );
};

// Checkbox Item
const MenuCheckboxItem: Component<{ item: Extract<DropdownMenuItem, { type: "checkbox" }> }> = (props) => {
  const handleClick = () => {
    if (props.item.disabled) return;
    props.item.onCheckedChange(!props.item.checked);
  };

  return (
    <div
      class={cn("ui-dropdown-item ui-dropdown-checkbox", props.item.disabled && "ui-dropdown-item-disabled")}
      role="menuitemcheckbox"
      aria-checked={props.item.checked}
      aria-disabled={props.item.disabled}
      onClick={handleClick}
    >
      <span class="ui-dropdown-indicator">
        <Show when={props.item.checked}>
          <Check size={12} />
        </Show>
      </span>
      <span class="ui-dropdown-item-label">{props.item.label}</span>
    </div>
  );
};

// Radio Item
const MenuRadioItem: Component<{ item: Extract<DropdownMenuItem, { type: "radio" }> }> = (props) => {
  const context = useContext(DropdownContext);

  const isSelected = createMemo(() => context?.radioValue?.() === props.item.value);

  const handleClick = () => {
    context?.onRadioChange?.(props.item.value);
  };

  return (
    <div
      class="ui-dropdown-item ui-dropdown-radio"
      role="menuitemradio"
      aria-checked={isSelected()}
      onClick={handleClick}
    >
      <span class="ui-dropdown-indicator">
        <Show when={isSelected()}>
          <div class="ui-dropdown-radio-dot" />
        </Show>
      </span>
      <span class="ui-dropdown-item-label">{props.item.label}</span>
    </div>
  );
};
