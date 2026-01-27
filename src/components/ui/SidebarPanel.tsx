import { Component, JSX, Show, splitProps } from "solid-js";
import { cn } from "../../lib/utils";
import "./sidebar-panel.css";

export interface SidebarPanelProps extends JSX.HTMLAttributes<HTMLElement> {
  /** Panel title */
  title: string;
  /** Panel content */
  children: JSX.Element;
  /** Actions to display in the header (e.g., buttons) */
  actions?: JSX.Element;
  /** Footer content */
  footer?: JSX.Element;
  /** Additional class for content area */
  contentClass?: string;
  /** Whether the panel is collapsible */
  collapsible?: boolean;
}

/**
 * SidebarPanel component for sidebar sections with header and optional footer.
 * 
 * @example
 * <SidebarPanel 
 *   title="Tags" 
 *   actions={<Button size="icon-sm"><Plus size={14} /></Button>}
 * >
 *   <TagList />
 * </SidebarPanel>
 */
export const SidebarPanel: Component<SidebarPanelProps> = (props) => {
  const [local, others] = splitProps(props, [
    "title",
    "children",
    "actions",
    "footer",
    "class",
    "contentClass",
    "collapsible",
  ]);

  return (
    <section
      class={cn("ui-sidebar-panel", local.class)}
      aria-label={local.title}
      {...others}
    >
      <header class="ui-sidebar-panel-header">
        <h3 class="ui-sidebar-panel-title">{local.title}</h3>
        <Show when={local.actions}>
          <div class="ui-sidebar-panel-actions" role="group">
            {local.actions}
          </div>
        </Show>
      </header>

      <div class={cn("ui-sidebar-panel-content", local.contentClass)}>
        {local.children}
      </div>

      <Show when={local.footer}>
        <footer class="ui-sidebar-panel-footer">
          {local.footer}
        </footer>
      </Show>
    </section>
  );
};
