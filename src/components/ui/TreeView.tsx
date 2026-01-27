import { Component, For, createMemo, createSignal, Show, createEffect, JSX } from "solid-js";
import { ChevronRight, ChevronDown } from "lucide-solid";
import { Dynamic } from "solid-js/web";
import { dndRegistry, setDragItem, currentDragItem } from "../../core/dnd";
import { useAppStore } from "../../core/store/appStore";
import "./tree-view.css";

export interface TreeNode {
  id: string | number;
  label: string;
  children?: TreeNode[];
  data?: any;
  icon?: Component<{ size?: number | string; color?: string; fill?: string; stroke?: string }>;
  iconColor?: string;
  badge?: JSX.Element;
}

interface TreeViewProps {
  items: TreeNode[];
  onSelect?: (node: TreeNode) => void;
  onContextMenu?: (e: MouseEvent, node: TreeNode) => void;
  selectedIds?: (string | number)[];
  editingId?: string | number | null;
  onRename?: (node: TreeNode, newName: string) => void;
  onEditCancel?: () => void;
  defaultIcon?: Component<{ size?: number | string; color?: string; fill?: string; stroke?: string }>;
  expandedIds?: Set<string | number>;
  onToggle?: (id: string | number) => void;
  onMove?: (draggedId: string | number, targetId: string | number) => void;
}

interface TreeViewItemProps {
  node: TreeNode;
  depth: number;
  onSelect?: (node: TreeNode) => void;
  onContextMenu?: (e: MouseEvent, node: TreeNode) => void;
  selectedIds?: (string | number)[];
  editingId?: string | number | null;
  onRename?: (node: TreeNode, newName: string) => void;
  onEditCancel?: () => void;
  defaultIcon?: Component<{ size?: number | string; color?: string; fill?: string; stroke?: string }>;
  expandedIds?: Set<string | number>;
  onToggle?: (id: string | number) => void;
  onMove?: (draggedId: string | number, targetId: string | number) => void;
}

export const TreeViewItem: Component<TreeViewItemProps> = (props) => {
  const { state } = useAppStore();
  const [localExpanded, setLocalExpanded] = createSignal(false);
  const isExpanded = () => props.expandedIds ? props.expandedIds.has(props.node.id) : localExpanded();
  const isEditing = () => props.editingId === props.node.id;
  
  let inputRef: HTMLInputElement | undefined;

  // DnD State
  const [dropPosition, setDropPosition] = createSignal<"before" | "inside" | "after" | null>(null);
  
  // Validation Logic
  const validationState = createMemo(() => {
      const dragItem = currentDragItem();
      if (!dragItem || dragItem.type !== "TAG") return { valid: true };
      
      const draggedId = Number(dragItem.payload.id);
      const targetId = Number(props.node.id);
      
      if (draggedId === targetId) return { valid: false }; // Self
      
      // Helper to check if child is descendant of parent
      const isDescendant = (parentId: number, childId: number): boolean => {
          let current = state.tags.find(t => t.id === childId);
          while (current && current.parent_id) {
              if (current.parent_id === parentId) return true;
              const nextParentId = current.parent_id;
              current = state.tags.find(t => t.id === nextParentId);
          }
          return false;
      };
      
      if (isDescendant(draggedId, targetId)) return { valid: false };
      
      return { valid: true };
  });

  const handleDragStart = (e: DragEvent) => {
      e.stopPropagation();
      if (!isEditing() && e.dataTransfer) {
         const data = { type: "TAG", payload: { id: props.node.id } };
         setDragItem(data as any);
         
         e.dataTransfer.effectAllowed = "move";
         e.dataTransfer.setData("application/json", JSON.stringify(data));
      }
  };

  const handleDragEnd = () => {
      setDragItem(null);
      setDropPosition(null);
  };

  const handleDragEnter = (e: DragEvent) => {
      e.preventDefault(); 
  };

  const handleDragOver = (e: DragEvent) => {
      e.preventDefault();
      
      const validation = validationState();
      
      // Calculate Drop Position
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      const relY = e.clientY - rect.top;
      const height = rect.height;
      const threshold = height * 0.25; // Top/Bottom 25%
      
      let pos: "before" | "inside" | "after" = "inside";
      
      if (relY < threshold) pos = "before";
      else if (relY > height - threshold) pos = "after";
      
      if (!validation.valid) {
          e.dataTransfer!.dropEffect = "none";
          setDropPosition(null);
          return;
      }
      
      const dragItem = currentDragItem();
      if (dragItem?.type === "TAG") {
           setDropPosition(pos);
           e.dataTransfer!.dropEffect = "move"; 
           return;
      }
      
      if (dragItem?.type === "IMAGE") {
          setDropPosition("inside");
          e.dataTransfer!.dropEffect = "copy";
          return;
      }
  };

  const handleDragLeave = (_e: DragEvent) => {
      setDropPosition(null);
  };

  const handleDrop = async (e: DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const pos = dropPosition();
      setDropPosition(null);
      
      if (!validationState().valid) return;
      
      try {
          const json = e.dataTransfer?.getData("application/json");
          if (json) {
              const item = JSON.parse(json);
              const strategy = dndRegistry.get("TAG");
              if (strategy && strategy.accepts(item)) {
                  // We cast to any to pass 3rd argument (position)
                  await (strategy as any).onDrop(item, props.node.id, pos || "inside");
              }
          }
      } catch (err) {
          console.error("Drop failed", err);
      }
  };

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
          (e.currentTarget as HTMLInputElement).blur(); 
      } else if (e.key === "Escape") {
          props.onEditCancel?.();
      }
  };

  const handleBlur = () => {
       if (inputRef) {
            const val = inputRef.value.trim();
            if (val) {
                props.onRename?.(props.node, val);
            } else {
                 props.onEditCancel?.();
            }
       }
  };
  
  // Derived state for styling
  const isDraggingSource = () => currentDragItem()?.type === "TAG" && Number(currentDragItem()?.payload.id) === Number(props.node.id);
  const isInvalid = () => !validationState().valid && currentDragItem()?.type === "TAG";

  return (
    <div 
        class="tree-item-container"
        role="treeitem"
        aria-selected={props.selectedIds?.some(id => String(id) === String(props.node.id))}
        aria-expanded={hasChildren() ? isExpanded() : undefined}
    >
        <div 
            class={`tree-item-content ${props.selectedIds?.some(id => String(id) === String(props.node.id)) ? 'selected' : ''} ${dropPosition() === 'inside' ? 'drop-target' : ''} ${isInvalid() ? 'drop-disabled' : ''} ${isDraggingSource() ? 'dragging-source' : ''}`}
            style={{ 
                "padding-left": `${props.depth * 16}px`
            }}
            draggable={!isEditing()}
            onDragStart={handleDragStart}
            onDragEnd={handleDragEnd}
            onDragEnter={handleDragEnter}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
            onClick={() => !isEditing() && props.onSelect?.(props.node)}
            onContextMenu={(e) => {
                e.preventDefault();
                props.onContextMenu?.(e, props.node);
            }}
        >
             {/* Lines for Before/After */}
             <Show when={dropPosition() === 'before'}>
                <div class="drop-line before" style={{ left: `${props.depth * 16 + 4}px` }}></div>
             </Show>
             <Show when={dropPosition() === 'after'}>
                 <div class="drop-line after" style={{ left: `${props.depth * 16 + 4}px` }}></div>
             </Show>

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

            <Show when={isEditing()} fallback={
                <>
                    <span class="tree-label">{props.node.label}</span>
                    <Show when={props.node.badge}>
                        <div style={{ "margin-left": "auto" }}>
                            {props.node.badge}
                        </div>
                    </Show>
                </>
            }>
                <input 
                    ref={inputRef}
                    type="text" 
                    class="tree-edit-input"
                    value={props.node.label}
                    onClick={(e) => e.stopPropagation()}
                    onKeyDown={handleKeyDown}
                    onBlur={handleBlur}
                    onInput={() => {}}
                />
            </Show>
        </div>
        
        <Show when={isExpanded() && hasChildren()}>
            <div role="group">
                <For each={props.node.children}>
                    {(child) => (
                        <TreeViewItem 
                            node={child} 
                            depth={props.depth + 1} 
                            onSelect={props.onSelect}
                            onContextMenu={props.onContextMenu}
                            selectedIds={props.selectedIds}
                            editingId={props.editingId}
                            onRename={props.onRename}
                            onEditCancel={props.onEditCancel}
                            defaultIcon={props.defaultIcon}
                            expandedIds={props.expandedIds}
                            onToggle={props.onToggle}
                            onMove={props.onMove}
                        />
                    )}
                </For>
            </div>
        </Show>
    </div>
  );
};

export const TreeView: Component<TreeViewProps> = (props) => {
    const [isDragOver, setIsDragOver] = createSignal(false);
    
    return (
        <div 
            class={`tree-view ${isDragOver() ? 'root-drop-active' : ''}`}
            role="tree"
            onDragEnter={(e) => e.preventDefault()}
            onDragOver={(e) => {
                e.preventDefault();
                const dragItem = currentDragItem();
                if (e.target === e.currentTarget && dragItem && dragItem.type === "TAG") {
                     setIsDragOver(true);
                     e.dataTransfer!.dropEffect = "move";
                }
            }}
            onDragLeave={(e) => {
                if (e.target === e.currentTarget) setIsDragOver(false);
            }}
            onDrop={async (e) => {
                if (e.target !== e.currentTarget) return;
                e.preventDefault();
                setIsDragOver(false);
                
                try {
                    const json = e.dataTransfer?.getData("application/json");
                    if (json) {
                         const item = JSON.parse(json);
                         const strategy = dndRegistry.get("TAG");
                         if (strategy && item.type === "TAG") {
                             await strategy.onDrop(item, "root");
                         }
                    }
                } catch (err) {}
            }}
        >
            <For each={props.items}>
                {(node) => (
                    <TreeViewItem 
                        node={node} 
                        depth={0} 
                        onSelect={props.onSelect} 
                        onContextMenu={props.onContextMenu}
                        selectedIds={props.selectedIds} 
                        editingId={props.editingId}
                        onRename={props.onRename}
                        onEditCancel={props.onEditCancel}
                        defaultIcon={props.defaultIcon}
                        expandedIds={props.expandedIds}
                        onToggle={props.onToggle}
                        onMove={props.onMove}
                    />
                )}
            </For>
            
        </div>
    );
};
