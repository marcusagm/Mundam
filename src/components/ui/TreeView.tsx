import { Component, For, createSignal, Show, createEffect } from "solid-js";
import { ChevronRight, ChevronDown } from "lucide-solid";
import { Dynamic } from "solid-js/web";
import "./tree-view.css";

export interface TreeNode {
  id: string | number;
  label: string;
  children?: TreeNode[];
  data?: any;
  icon?: Component<{ size?: number | string; color?: string; fill?: string; stroke?: string }>;
  iconColor?: string;
}

interface TreeViewProps {
  items: TreeNode[];
  onSelect?: (node: TreeNode) => void;
  onContextMenu?: (e: MouseEvent, node: TreeNode) => void;
  selectedId?: string | number;
  editingId?: string | number | null;
  onRename?: (node: TreeNode, newName: string) => void;
  onEditCancel?: () => void;
  defaultIcon?: Component<{ size?: number | string; color?: string; fill?: string; stroke?: string }>;
  // Expansion State
  expandedIds?: Set<string | number>;
  onToggle?: (id: string | number) => void;
}

const TreeViewItem: Component<{ 
  node: TreeNode, 
  depth: number,
  onSelect?: (node: TreeNode) => void,
  onContextMenu?: (e: MouseEvent, node: TreeNode) => void,
  selectedId?: string | number,
  editingId?: string | number | null,
  onRename?: (node: TreeNode, newName: string) => void,
  onEditCancel?: () => void,
  defaultIcon?: Component<{ size?: number | string; color?: string; fill?: string; stroke?: string }>,
  expandedIds?: Set<string | number>,
  onToggle?: (id: string | number) => void
}> = (props) => {
  // Fallback to local state if no external state provided (though we will provide it)
  const [localExpanded, setLocalExpanded] = createSignal(false);
  const isExpanded = () => props.expandedIds ? props.expandedIds.has(props.node.id) : localExpanded();
  
  const isEditing = () => props.editingId === props.node.id;
  
  let inputRef: HTMLInputElement | undefined;

  createEffect(() => {
      if (isEditing() && inputRef) {
          inputRef.focus();
      }
  });

  const hasChildren = () => props.node.children && props.node.children.length > 0;

  const handleToggle = (e: MouseEvent) => {
    e.stopPropagation();
    if (hasChildren()) {
        if (props.onToggle) {
            props.onToggle(props.node.id);
        } else {
            setLocalExpanded(!localExpanded());
        }
    }
  };
  
  const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Enter") {
          e.currentTarget.blur(); // Trigger blur to save
      } else if (e.key === "Escape") {
          props.onEditCancel?.();
      }
  };

  const handleBlur = () => {
       if (inputRef) {
            // Trim whitespace?
            const val = inputRef.value.trim();
            if (val) {
                props.onRename?.(props.node, val);
            } else {
                // If empty, maybe cancel or keep old name?
                // Let's keep old name to be safe
                 props.onEditCancel?.();
            }
       }
  };

  return (
    <div class="tree-item-container">
        <div 
            class={`tree-item-content ${props.selectedId === props.node.id ? 'selected' : ''}`}
            style={{ "padding-left": `${props.depth * 16}px` }}
            onClick={() => !isEditing() && props.onSelect?.(props.node)}
            onContextMenu={(e) => {
                e.preventDefault();
                props.onContextMenu?.(e, props.node);
            }}
        >
            <span 
                class={`tree-toggle ${hasChildren() ? 'visible' : ''}`} 
                onClick={handleToggle}
            >
                <Show when={isExpanded()} fallback={<ChevronRight size={14} />}>
                    <ChevronDown size={14} />
                </Show>
            </span>
            
            {/* Icon */}
            <Show when={props.node.icon || props.defaultIcon}>
                <div style={{ 
                    "margin-right": "6px", 
                    display: "flex", 
                    color: props.node.iconColor || "var(--text-secondary)" 
                }}>
                    <Dynamic 
                        component={props.node.icon || props.defaultIcon} 
                        size={14} 
                    />
                </div>
            </Show>

            {/* Label or Input */}
            <Show when={isEditing()} fallback={<span class="tree-label">{props.node.label}</span>}>
                <input 
                    ref={inputRef}
                    type="text" 
                    class="tree-edit-input"
                    value={props.node.label}
                    onClick={(e) => e.stopPropagation()}
                    onKeyDown={handleKeyDown}
                    onBlur={handleBlur}
                    onInput={(e) => {
                        // Optional: update local model if needed, but we rely on ref
                    }}
                />
            </Show>
        </div>
        
        <Show when={isExpanded() && hasChildren()}>
            <For each={props.node.children}>
                {(child) => (
                    <TreeViewItem 
                        node={child} 
                        depth={props.depth + 1} 
                        onSelect={props.onSelect}
                        onContextMenu={props.onContextMenu}
                        selectedId={props.selectedId}
                        editingId={props.editingId}
                        onRename={props.onRename}
                        onEditCancel={props.onEditCancel}
                        defaultIcon={props.defaultIcon}
                        expandedIds={props.expandedIds}
                        onToggle={props.onToggle}
                    />
                )}
            </For>
        </Show>
    </div>
  );
};

export const TreeView: Component<TreeViewProps> = (props) => {
    return (
        <div class="tree-view">
            <For each={props.items}>
                {(node) => (
                    <TreeViewItem 
                        node={node} 
                        depth={0} 
                        onSelect={props.onSelect} 
                        onContextMenu={props.onContextMenu}
                        selectedId={props.selectedId}
                        editingId={props.editingId}
                        onRename={props.onRename}
                        onEditCancel={props.onEditCancel}
                        defaultIcon={props.defaultIcon}
                        expandedIds={props.expandedIds}
                        onToggle={props.onToggle}
                    />
                )}
            </For>
        </div>
    );
};
