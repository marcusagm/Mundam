import { Component, For, Show, createSignal, onCleanup, createEffect } from "solid-js";
import { Portal, Dynamic } from "solid-js/web";
import { ChevronRight } from "lucide-solid";
import "./context-menu.css";

// Re-export type so it can be used elsewhere
export type ContextMenuItem =
  | {
      type: "item";
      label: string;
      icon?: Component<{ size?: number | string }>;
      action: () => void;
      danger?: boolean;
    }
  | { type: "separator" }
  | {
      type: "submenu";
      label: string;
      icon?: Component<{ size?: number | string }>;
      items: ContextMenuItem[];
    }
  | {
      type: "custom";
      content: JSX.Element;
    };

export interface ContextMenuProps {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
  isOpen: boolean;
}

// Submenu Wrapper for positioning
const SubmenuWrapper: Component<{ children: JSX.Element; parentRect: DOMRect }> = (props) => {
    let ref: HTMLDivElement | undefined;
    
    createEffect(() => {
        if (ref) {
           // Reset styles first to measure naturally
           ref.style.left = "100%";
           ref.style.top = "0";
           ref.style.right = "auto";
           ref.style.bottom = "auto";
           
           requestAnimationFrame(() => {
               if (!ref) return;
               const rect = ref.getBoundingClientRect();
               const viewportWidth = window.innerWidth;
               const viewportHeight = window.innerHeight;
               
               // Horizontal Check
               if (rect.right > viewportWidth) {
                   ref.style.left = "auto";
                   ref.style.right = "100%"; // Flip to left
                   ref.style.marginLeft = "0";
                   ref.style.marginRight = "4px";
               }

               // Vertical Check
               if (rect.bottom > viewportHeight) {
                   // Flip upwards. 
                   // Align bottom of submenu with bottom of parent item? 
                   // Or just shift it up so it fits?
                   // If we set bottom: 0, it aligns bottom-to-bottom with parent wrapper.
                   ref.style.top = "auto";
                   ref.style.bottom = "0";
               }
           });
        }
    });

    return (
        <div 
            ref={ref}
            class="context-submenu"
            style={{
                position: "absolute",
                left: "100%",
                top: "0",
                "margin-left": "4px"
            }}
        >
            {props.children}
        </div>
    );
};

// Recursive Menu List Component
const MenuList: Component<{ items: ContextMenuItem[]; onCloseRoot: () => void }> = (props) => {
    const [activeSubMenu, setActiveSubMenu] = createSignal<number | null>(null);

    return (
        <div class="context-menu-list">
            <For each={props.items}>
                {(item, index) => (
                    <div 
                        class="context-menu-wrapper"
                        onMouseEnter={(e) => {
                            setActiveSubMenu(index());
                        }}
                        onMouseLeave={() => setActiveSubMenu(null)}
                    >
                        <Show when={item.type === "separator"}>
                            <div class="context-menu-separator" />
                        </Show>
                        
                        <Show when={item.type === "custom"}>
                            <div class="context-menu-custom">
                                {(item as any).content}
                            </div>
                        </Show>

                        <Show when={item.type === "item"}>
                            <div
                                class={`context-menu-item ${(item as any).danger ? "danger" : ""}`}
                                onClick={() => {
                                    (item as any).action();
                                    props.onCloseRoot();
                                }}
                            >
                                <Show when={(item as any).icon}>
                                    <Dynamic component={(item as any).icon} size={14} />
                                </Show>
                                <span>{(item as any).label}</span>
                            </div>
                        </Show>

                        <Show when={item.type === "submenu"}>
                            <div class="context-menu-item submenu-trigger">
                                <span style={{ display: "flex", gap: "8px", "align-items": "center", flex: 1 }}>
                                    <Show when={(item as any).icon}>
                                        <Dynamic component={(item as any).icon} size={14} />
                                    </Show>
                                    {(item as any).label}
                                </span>
                                <ChevronRight size={14} />
                                
                                <Show when={activeSubMenu() === index()}>
                                   {/* We pass a dummy rect, or simply handle logic in SubmenuWrapper by measuring itself against viewport */}
                                    <SubmenuWrapper parentRect={new DOMRect()}>
                                        <MenuList 
                                            items={(item as any).items} 
                                            onCloseRoot={props.onCloseRoot} 
                                        />
                                    </SubmenuWrapper>
                                </Show>
                            </div>
                        </Show>
                    </div>
                )}
            </For>
        </div>
    );
};

export const ContextMenu: Component<ContextMenuProps> = (props) => {
  let containerRef: HTMLDivElement | undefined;

  createEffect(() => {
    if (props.isOpen && containerRef) {
      // Reset first to measure true size/pos
      containerRef.style.opacity = "0"; 
      containerRef.style.top = `${props.y}px`;
      containerRef.style.left = `${props.x}px`;

      requestAnimationFrame(() => {
        if (containerRef) {
            const rect = containerRef.getBoundingClientRect();
            const viewportWidth = window.innerWidth;
            const viewportHeight = window.innerHeight;

            let top = props.y;
            let left = props.x;

            if (left + rect.width > viewportWidth) left -= rect.width;
            if (top + rect.height > viewportHeight) top -= rect.height;

            containerRef.style.top = `${top}px`;
            containerRef.style.left = `${left}px`;
            containerRef.style.opacity = "1";
        }
      });
    }
  });

  return (
    <Show when={props.isOpen}>
      <Portal>
        <div
          style={{ position: "fixed", inset: 0, "z-index": 9998 }}
          onContextMenu={(e) => {
            e.preventDefault();
            props.onClose();
          }}
          onClick={props.onClose}
        />
        <div
          ref={containerRef}
          class="context-menu-container"
          style={{ 
              top: `${props.y}px`, 
              left: `${props.x}px`, 
              position: "fixed", 
              "z-index": 9999,
              opacity: 0
          }}
          onContextMenu={(e) => e.preventDefault()}
        >
            <MenuList items={props.items} onCloseRoot={props.onClose} />
        </div>
      </Portal>
    </Show>
  );
};
