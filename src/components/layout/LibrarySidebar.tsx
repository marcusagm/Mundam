import { Component, For, createMemo, createSignal, onMount } from "solid-js";
import { useAppStore, appActions } from "../../core/store/appStore";
import { Folder, Trash2, Tag, Layers, Plus, Pencil, Palette } from "lucide-solid";
import { Badge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { Separator } from "../ui/Separator";
import { TreeView, TreeNode } from "../ui/TreeView";
import { ContextMenu, ContextMenuItem } from "../ui/ContextMenu";
import { tagService } from "../../lib/tags";
import { ConfirmModal } from "../ui/Modal";
import { ColorPicker } from "../ui/ColorPicker";

export const LibrarySidebar: Component = () => {
  const { state } = useAppStore();
  
  // Context Menu State
  const [contextMenuOpen, setContextMenuOpen] = createSignal(false);
  const [contextMenuPos, setContextMenuPos] = createSignal({ x: 0, y: 0 });
  const [contextMenuItems, setContextMenuItems] = createSignal<ContextMenuItem[]>([]);

  // Editing State
  const [editingId, setEditingId] = createSignal<number | null>(null);

  // Expansion State (Persisted)
  const [expandedIds, setExpandedIds] = createSignal<Set<string | number>>(new Set());

  // Load expansion state
  onMount(() => {
    const saved = localStorage.getItem("elleven_tree_expanded");
    if (saved) {
        try {
            const parsed = JSON.parse(saved);
            if (Array.isArray(parsed)) {
                setExpandedIds(new Set(parsed));
            }
        } catch (e) {
            console.error("Failed to load tree expansion state", e);
        }
    }
  });

  const toggleExpansion = (id: string | number) => {
      const current = new Set(expandedIds());
      if (current.has(id)) {
          current.delete(id);
      } else {
          current.add(id);
      }
      setExpandedIds(current);
      localStorage.setItem("elleven_tree_expanded", JSON.stringify(Array.from(current)));
  };

  const expandNode = (id: string | number) => {
      const current = new Set(expandedIds());
      if (!current.has(id)) {
          current.add(id);
          setExpandedIds(current);
          localStorage.setItem("elleven_tree_expanded", JSON.stringify(Array.from(current)));
      }
  };

  // Confirm Modal State
  const [confirmOpen, setConfirmOpen] = createSignal(false);
  const [confirmConfig, setConfirmConfig] = createSignal({ title: "", message: "", onConfirm: () => {} });

  // Build Tag Tree Memo
  const tagTree = createMemo(() => {
      const tags = state.tags || [];
      const map = new Map<number, TreeNode>();
      const roots: TreeNode[] = [];
      
      tags.forEach(t => {
          map.set(t.id, { 
              id: t.id, 
              label: t.name, 
              children: [], 
              data: t,
              icon: Tag,
              iconColor: t.color || undefined
          });
      });
      
      tags.forEach(t => {
          if (t.parent_id && map.has(t.parent_id)) {
              map.get(t.parent_id)!.children!.push(map.get(t.id)!);
          } else {
              roots.push(map.get(t.id)!);
          }
      });
      
      const sortNodes = (nodes: TreeNode[]) => {
            nodes.sort((a,b) => a.label.localeCompare(b.label));
            nodes.forEach(n => {
                if (n.children && n.children.length > 0) sortNodes(n.children);
            });
      };
      
      sortNodes(roots);
      return roots;
  });

  const getUniqueName = (baseStats: string) => {
      const tags = state.tags || [];
      let name = baseStats;
      let counter = 1;
      while (tags.some(t => t.name === name)) {
          name = `${baseStats} (${counter})`;
          counter++;
      }
      return name;
  };

  const handleCreateTag = async () => {
      if (editingId() !== null) return; 
      try {
          const name = getUniqueName("New Tag");
          const id = await tagService.createTag(name);
          await appActions.loadTags();
          setEditingId(id);
      } catch (err) {
          console.error(err);
      }
  };

  const handleCreateChildTag = async (parentId: number) => {
      try {
          expandNode(parentId);
          const name = getUniqueName("New Tag");
          const id = await tagService.createTag(name, parentId);
          await appActions.loadTags();
          setEditingId(id);
      } catch (err) {
          console.error(err);
      }
  };

  const handleRename = async (node: TreeNode, newName: string) => {
      if (!newName || !newName.trim()) {
          setEditingId(null);
          return;
      }

      if (newName === node.label) {
          setEditingId(null);
          return;
      }
      
      try {
          await tagService.updateTag(Number(node.id), newName);
          await appActions.loadTags();
      } catch (err) {
          console.error(err);
          // Optional: Show toast error if unique constraint failed during rename
      }
      setEditingId(null);
  };

  // Helper to get all descendant IDs
  const getAllDescendants = (node: TreeNode): number[] => {
      let ids: number[] = [];
      if (node.children) {
          node.children.forEach(child => {
              ids.push(Number(child.id));
              ids = [...ids, ...getAllDescendants(child)];
          });
      }
      return ids;
  };

  const handleContextMenu = (e: MouseEvent, node: TreeNode) => {
    e.preventDefault();
    setContextMenuPos({ x: e.clientX, y: e.clientY });
    
    // Pre-calculate recursive delete info
    const descendantIds = getAllDescendants(node);
    const count = descendantIds.length;

    setContextMenuItems([
        {
            type: 'item',
            label: 'Add Child Tag',
            icon: Plus,
            action: () => handleCreateChildTag(Number(node.id))
        },
        {
            type: 'item',
            label: 'Rename',
            icon: Pencil,
            action: () => setEditingId(Number(node.id))
        },
        {
            type: 'submenu',
            label: 'Change Color',
            icon: Palette,
            items: [
                {
                    type: 'custom',
                    content: (
                        <ColorPicker 
                            color={node.iconColor || "#cccccc"} 
                            onChange={async (newColor) => {
                                await tagService.updateTag(Number(node.id), undefined, newColor);
                                await appActions.loadTags();
                            }} 
                        />
                    )
                }
            ]
        },
        { type: 'separator' },
        {
            type: 'item',
            label: 'Delete',
            danger: true,
            icon: Trash2,
            action: () => {
                setConfirmConfig({
                    title: "Delete Tag",
                    message: count > 0 
                        ? `Are you sure you want to delete tag "${node.label}" and its ${count} children? This action cannot be undone.`
                        : `Are you sure you want to delete tag "${node.label}"?`,
                    onConfirm: async () => {
                        try {
                            // Recursively delete children first (manual cascade)
                            for (const childId of descendantIds) {
                                await tagService.deleteTag(childId);
                            }
                            // Delete the node itself
                            await tagService.deleteTag(Number(node.id));
                            await appActions.loadTags();
                        } catch (err) {
                            console.error("Delete failed:", err);
                        }
                    }
                });
                setConfirmOpen(true);
            }
        }
    ]);
    setContextMenuOpen(true);
  };

  return (
    <div style={{ padding: "8px", height: "100%", "overflow-y": "auto" }}>
        {/* Section Header */}
        <div style={{ 
            padding: "16px 8px 8px 8px", 
            "font-size": "11px", 
            "font-weight": "600", 
            color: "var(--text-muted)", 
            "text-transform": "uppercase",
            "letter-spacing": "0.5px"
        }}>
            Library
        </div>
        
        {/* Core Filters */}
        <div class="nav-item active">
            <Layers size={16} />
            <span style={{ flex: 1 }}>All Items</span>
            <Badge variant="secondary">{state.items.length}</Badge>
        </div>
        <div class="nav-item">
            <Tag size={16} />
            <span>Uncategorized</span>
        </div>
        <div class="nav-item">
            <Trash2 size={16} />
            <span>Trash</span>
        </div>

        <Separator class="my-3" style={{ margin: "12px 8px" }} />

        {/* Folders Section */}
        <div style={{ 
            padding: "8px", 
            "font-size": "11px", 
            "font-weight": "600", 
            color: "var(--text-muted)", 
            "text-transform": "uppercase",
            "letter-spacing": "0.5px",
            display: "flex", "align-items": "center", "justify-content": "space-between"
        }}>
            <span>Folders</span>
            <Button variant="ghost" size="icon-sm" title="Create Folder">
                <Plus size={14} />
            </Button>
        </div>
        
        <div style={{ "display": "flex", "flex-direction": "column", gap: "1px" }}>
            <For each={state.locations}>
                {(loc) => (
                    <div class="nav-item" title={loc.path}>
                        <Folder size={16} fill="var(--text-muted)" stroke="none" /> 
                        <span style={{ "white-space": "nowrap", "overflow": "hidden", "text-overflow": "ellipsis" }}>
                            {loc.name}
                        </span>
                    </div>
                )}
            </For>
            
            {state.locations.length === 0 && (
                <div style={{ padding: "12px", "text-align": "center", color: "var(--text-muted)", "font-size": "11px", "font-style": "italic" }}>
                    No folders linked
                </div>
            )}
        </div>
        
        <Separator class="my-3" style={{ margin: "12px 8px" }} />
        
        {/* Tags Section */}
        <div style={{ 
            padding: "8px", 
            "font-size": "11px", 
            "font-weight": "600", 
            color: "var(--text-muted)", 
            "text-transform": "uppercase",
            "letter-spacing": "0.5px",
            display: "flex", "align-items": "center", "justify-content": "space-between"
        }}>
            <span>Tags</span>
            <Button variant="ghost" size="icon-sm" title="Create Tag" onClick={handleCreateTag}>
                <Plus size={14} />
            </Button>
        </div>
        
        <TreeView 
            items={tagTree()} 
            onSelect={node => appActions.toggleTagSelection(Number(node.id))}
            selectedId={state.selectedTags[0] ? String(state.selectedTags[0]) : undefined} 
            onContextMenu={handleContextMenu}
            editingId={editingId()}
            onRename={handleRename}
            onEditCancel={() => setEditingId(null)}
            expandedIds={expandedIds()}
            onToggle={toggleExpansion}
        />
        
        <ContextMenu 
            x={contextMenuPos().x} 
            y={contextMenuPos().y} 
            items={contextMenuItems()} 
            isOpen={contextMenuOpen()} 
            onClose={() => setContextMenuOpen(false)}
        />
        

        <ConfirmModal 
            isOpen={confirmOpen()}
            onClose={() => setConfirmOpen(false)}
            onConfirm={confirmConfig().onConfirm}
            title={confirmConfig().title}
            message={confirmConfig().message}
            kind="danger"
            confirmText="Delete"
        />
    </div>
  );
};
