import { 
  Component, 
  For, 
  Show, 
  createSignal, 
  createEffect, 
  onCleanup,
  JSX 
} from "solid-js";
import { Portal, Dynamic } from "solid-js/web";
import { ChevronRight } from "lucide-solid";
import { cn } from "../../lib/utils";
import { createClickOutside } from "../../lib/primitives";
import "./context-menu.css";

// Menu item types
export type ContextMenuItem =
  | {
      type: "item";
      label: string;
      icon?: Component<{ size?: number | string }>;
      action: () => void;
      shortcut?: string;
      danger?: boolean;
      disabled?: boolean;
    }
  | { type: "separator" }
  | {
      type: "submenu";
      label: string;
      icon?: Component<{ size?: number | string }>;
      items: ContextMenuItem[];
      disabled?: boolean;
    }
  | {
      type: "custom";
      content: JSX.Element;
    };

export interface ContextMenuProps {
  /** X coordinate to display the menu */
  x: number;
  /** Y coordinate to display the menu */
  y: number;
  /** Menu items */
  items: ContextMenuItem[];
  /** Callback when menu should close */
  onClose: () => void;
  /** Whether the menu is open */
  isOpen: boolean;
}

// Submenu wrapper for positioning
const SubmenuWrapper: Component<{ children: JSX.Element }> = (props) => {
  let ref: HTMLDivElement | undefined;

  createEffect(() => {
    if (!ref) return;

    requestAnimationFrame(() => {
      if (!ref) return;
      
      const rect = ref.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;

      // Horizontal: flip to left if overflowing right
      if (rect.right > viewportWidth) {
        ref.style.left = "auto";
        ref.style.right = "100%";
        ref.style.marginLeft = "0";
        ref.style.marginRight = "4px";
      }

      // Vertical: shift up if overflowing bottom
      if (rect.bottom > viewportHeight) {
        ref.style.top = "auto";
        ref.style.bottom = "0";
      }
    });
  });

  return (
    <div
      ref={ref}
      class="ui-context-submenu"
      role="menu"
    >
      {props.children}
    </div>
  );
};

// Recursive menu list
const MenuList: Component<{
  items: ContextMenuItem[];
  onClose: () => void;
  level?: number;
}> = (props) => {
  const [activeSubmenu, setActiveSubmenu] = createSignal<number | null>(null);
  const [focusedIndex, setFocusedIndex] = createSignal(-1);

  const level = () => props.level ?? 0;

  const handleKeyDown = (e: KeyboardEvent, index: number, item: ContextMenuItem) => {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setFocusedIndex((prev) =>
          prev < props.items.length - 1 ? prev + 1 : 0
        );
        break;
      case "ArrowUp":
        e.preventDefault();
        setFocusedIndex((prev) =>
          prev > 0 ? prev - 1 : props.items.length - 1
        );
        break;
      case "ArrowRight":
        if (item.type === "submenu" && !item.disabled) {
          e.preventDefault();
          setActiveSubmenu(index);
        }
        break;
      case "ArrowLeft":
        if (level() > 0) {
          e.preventDefault();
          // Close this submenu (parent will handle)
        }
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        if (item.type === "item" && !item.disabled) {
          item.action();
          props.onClose();
        } else if (item.type === "submenu" && !item.disabled) {
          setActiveSubmenu(index);
        }
        break;
      case "Escape":
        e.preventDefault();
        props.onClose();
        break;
    }
  };

  return (
    <div class="ui-context-menu-list" role="menu">
      <For each={props.items}>
        {(item, index) => (
          <div
            class="ui-context-menu-item-wrapper"
            onMouseEnter={() => setActiveSubmenu(index())}
            onMouseLeave={() => setActiveSubmenu(null)}
          >
            <Show when={item.type === "separator"}>
              <div class="ui-context-menu-separator" role="separator" />
            </Show>

            <Show when={item.type === "custom"}>
              <div class="ui-context-menu-custom">
                {(item as any).content}
              </div>
            </Show>

            <Show when={item.type === "item"}>
              {(() => {
                const menuItem = item as Extract<ContextMenuItem, { type: "item" }>;
                return (
                  <button
                    type="button"
                    class={cn(
                      "ui-context-menu-item",
                      menuItem.danger && "ui-context-menu-item-danger",
                      menuItem.disabled && "ui-context-menu-item-disabled",
                      focusedIndex() === index() && "ui-context-menu-item-focused"
                    )}
                    role="menuitem"
                    disabled={menuItem.disabled}
                    onClick={() => {
                      menuItem.action();
                      props.onClose();
                    }}
                    onKeyDown={(e) => handleKeyDown(e, index(), item)}
                    onFocus={() => setFocusedIndex(index())}
                  >
                    <span class="ui-context-menu-item-content">
                      <Show when={menuItem.icon}>
                        <Dynamic component={menuItem.icon} size={14} />
                      </Show>
                      <span>{menuItem.label}</span>
                    </span>
                    <Show when={menuItem.shortcut}>
                      <span class="ui-context-menu-shortcut">
                        {menuItem.shortcut}
                      </span>
                    </Show>
                  </button>
                );
              })()}
            </Show>

            <Show when={item.type === "submenu"}>
              {(() => {
                const submenuItem = item as Extract<ContextMenuItem, { type: "submenu" }>;
                return (
                  <div
                    class={cn(
                      "ui-context-menu-item ui-context-menu-submenu-trigger",
                      submenuItem.disabled && "ui-context-menu-item-disabled"
                    )}
                    role="menuitem"
                    aria-haspopup="menu"
                    aria-expanded={activeSubmenu() === index()}
                    tabindex={submenuItem.disabled ? -1 : 0}
                    onKeyDown={(e) => handleKeyDown(e, index(), item)}
                  >
                    <span class="ui-context-menu-item-content">
                      <Show when={submenuItem.icon}>
                        <Dynamic component={submenuItem.icon} size={14} />
                      </Show>
                      <span>{submenuItem.label}</span>
                    </span>
                    <ChevronRight size={14} class="ui-context-menu-chevron" />

                    <Show when={activeSubmenu() === index()}>
                      <SubmenuWrapper>
                        <MenuList
                          items={submenuItem.items}
                          onClose={props.onClose}
                          level={level() + 1}
                        />
                      </SubmenuWrapper>
                    </Show>
                  </div>
                );
              })()}
            </Show>
          </div>
        )}
      </For>
    </div>
  );
};

/**
 * ContextMenu component for right-click menus.
 * Supports items, separators, submenus, and custom content.
 * 
 * @example
 * const [menu, setMenu] = createSignal({ isOpen: false, x: 0, y: 0 });
 * 
 * <div onContextMenu={(e) => {
 *   e.preventDefault();
 *   setMenu({ isOpen: true, x: e.clientX, y: e.clientY });
 * }}>
 *   Right-click here
 * </div>
 * 
 * <ContextMenu
 *   isOpen={menu().isOpen}
 *   x={menu().x}
 *   y={menu().y}
 *   items={[
 *     { type: "item", label: "Edit", action: () => {} },
 *     { type: "separator" },
 *     { type: "item", label: "Delete", action: () => {}, danger: true },
 *   ]}
 *   onClose={() => setMenu({ ...menu(), isOpen: false })}
 * />
 */
export const ContextMenu: Component<ContextMenuProps> = (props) => {
  let containerRef: HTMLDivElement | undefined;

  // Adjust position on open
  createEffect(() => {
    if (!props.isOpen || !containerRef) return;

    containerRef.style.opacity = "0";
    containerRef.style.top = `${props.y}px`;
    containerRef.style.left = `${props.x}px`;

    requestAnimationFrame(() => {
      if (!containerRef) return;

      const rect = containerRef.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;

      let top = props.y;
      let left = props.x;

      if (left + rect.width > viewportWidth) {
        left = Math.max(0, viewportWidth - rect.width - 8);
      }
      if (top + rect.height > viewportHeight) {
        top = Math.max(0, viewportHeight - rect.height - 8);
      }

      containerRef.style.top = `${top}px`;
      containerRef.style.left = `${left}px`;
      containerRef.style.opacity = "1";
    });
  });

  // Click outside to close
  createClickOutside(
    () => containerRef,
    () => props.onClose()
  );

  // Close on Escape
  createEffect(() => {
    if (!props.isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        props.onClose();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    onCleanup(() => document.removeEventListener("keydown", handleKeyDown));
  });

  return (
    <Show when={props.isOpen}>
      <Portal>
        {/* Invisible overlay to capture clicks */}
        <div
          class="ui-context-menu-backdrop"
          onContextMenu={(e) => {
            e.preventDefault();
            props.onClose();
          }}
          onClick={props.onClose}
        />
        <div
          ref={containerRef}
          class="ui-context-menu-container"
          onContextMenu={(e) => e.preventDefault()}
        >
          <MenuList items={props.items} onClose={props.onClose} />
        </div>
      </Portal>
    </Show>
  );
};
