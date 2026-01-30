import { Component, createMemo, createSignal, onMount } from "solid-js";
import { Tag, Plus } from "lucide-solid";
import { useMetadata, useFilters, useNotification } from "../../../core/hooks";
import { TreeView, TreeNode } from "../../ui/TreeView";
import { SidebarPanel } from "../../ui/SidebarPanel";
import { Button } from "../../ui/Button";
import { CountBadge } from "../../ui/CountBadge";
import { dndRegistry, setDragItem } from "../../../core/dnd";
import { tagService } from "../../../lib/tags";
import { TagContextMenu } from "./TagContextMenu";
import { TagDeleteModal } from "./TagDeleteModal";

export const TagTreeSidebarPanel: Component = () => {
  const metadata = useMetadata();
  const filters = useFilters();
  const notification = useNotification();
  const [isTagHeaderDragOver, setIsTagHeaderDragOver] = createSignal(false);
  
  // Context Menu State
  const [contextMenuOpen, setContextMenuOpen] = createSignal(false);
  const [contextMenuPos, setContextMenuPos] = createSignal({ x: 0, y: 0 });
  const [contextMenuNode, setContextMenuNode] = createSignal<TreeNode | null>(null);

  // Selection & Editing State
  const [editingId, setEditingId] = createSignal<number | null>(null);
  const [expandedIds, setExpandedIds] = createSignal<Set<string | number>>(new Set());

  // Delete Modal State
  const [deleteModalOpen, setDeleteModalOpen] = createSignal(false);
  const [nodeToDelete, setNodeToDelete] = createSignal<TreeNode | null>(null);

  // Load/Save expansion state
  onMount(() => {
    const saved = localStorage.getItem("elleven_tree_expanded");
    if (saved) {
        try {
            const parsed = JSON.parse(saved);
            if (Array.isArray(parsed)) setExpandedIds(new Set(parsed));
        } catch (e) { console.error(e); }
    }
  });

  const toggleExpansion = (id: string | number) => {
    setExpandedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      localStorage.setItem("elleven_tree_expanded", JSON.stringify(Array.from(next)));
      return next;
    });
  };

  const expandNode = (id: string | number) => {
    setExpandedIds(prev => {
      if (prev.has(id)) return prev;
      const next = new Set(prev);
      next.add(id);
      localStorage.setItem("elleven_tree_expanded", JSON.stringify(Array.from(next)));
      return next;
    });
  };

  // Tree building
  const tagTree = createMemo(() => {
      const tags = metadata.tags || [];
      const map = new Map<number, TreeNode>();
      const roots: TreeNode[] = [];
      
      tags.forEach(t => {
          map.set(t.id, { 
              id: t.id, 
              label: t.name, 
              children: [], 
              data: t,
              icon: Tag,
              iconColor: t.color || undefined,
              badge: <CountBadge showZero={true} count={metadata.stats.tag_counts.get(t.id) || 0} />
          });
      });
      
      tags.forEach(t => {
          if (t.parent_id && map.has(t.parent_id)) {
              map.get(t.parent_id)!.children!.push(map.get(t.id)!);
          } else {
              roots.push(map.get(t.id)!);
          }
      });
      return roots;
  });

  const getUniqueName = (baseStats: string) => {
      const tags = metadata.tags || [];
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
          await metadata.loadTags();
          setEditingId(id);
      } catch (err) { 
          console.error(err);
          notification.error("Failed to Create Tag");
      }
  };

  const handleCreateChildTag = async (parentId: number) => {
      try {
          expandNode(parentId);
          const name = getUniqueName("New Tag");
          const id = await tagService.createTag(name, parentId);
          await metadata.loadTags();
          setEditingId(id);
      } catch (err) { 
          console.error(err);
          notification.error("Failed to Create Child Tag");
      }
  };

  const handleRename = async (node: TreeNode, newName: string) => {
      if (!newName || !newName.trim() || newName === node.label) {
          setEditingId(null);
          return;
      }
      const oldName = node.label;
      const isPlaceholder = /^New Tag( \(\d+\))?$/.test(oldName);
      try {
          await tagService.updateTag(Number(node.id), newName);
          await metadata.loadTags();
          
          if (isPlaceholder) {
              notification.success("Tag Created", `Created tag "${newName}"`);
          } else {
              notification.success("Tag Renamed", `Changed "${oldName}" to "${newName}"`);
          }
      } catch (err) { 
          console.error(err);
          notification.error("Failed to Rename Tag");
      }
      setEditingId(null);
  };

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
    setContextMenuNode(node);
    setContextMenuOpen(true);
  };

  const handleOpenDeleteModal = (node: TreeNode) => {
    setNodeToDelete(node);
    setDeleteModalOpen(true);
  };

  const handleMoveTag = async (draggedIdStr: string | number, targetIdStr: string | number) => {
      const draggedId = Number(draggedIdStr);
      const targetId = Number(targetIdStr);
      if (draggedId === targetId) return;

      const findNodeLocal = (nodes: TreeNode[], id: number): TreeNode | null => {
        for (const node of nodes) {
            if (Number(node.id) === id) return node;
            if (node.children) {
                const found = findNodeLocal(node.children, id);
                if (found) return found;
            }
        }
        return null;
      };

      const draggedNode = findNodeLocal(tagTree(), draggedId);
      if (draggedNode && getAllDescendants(draggedNode).includes(targetId)) return;
      
      try {
          await tagService.updateTag(draggedId, undefined, undefined, Number(targetId) || null);
          await metadata.loadTags();
          expandNode(targetId);
      } catch (err) { console.error("Failed to move tag:", err); }
  };

  return (
      <SidebarPanel 
        title="Tags" 
        class="panel-fluid"
        actions={
            <Button variant="ghost" size="icon-xs" title="Create Tag" onClick={handleCreateTag}>
                <Plus size={14} />
            </Button>
        }
        contentClass={isTagHeaderDragOver() ? "drag-over" : ""}
        onDragOver={(e: DragEvent) => {
            const strategy = dndRegistry.get("TAG");
            if (strategy && strategy.onDragOver) {
                e.preventDefault();
                (e.dataTransfer as DataTransfer).dropEffect = "move";
                setIsTagHeaderDragOver(true);
            }
        }}
        onDragLeave={() => setIsTagHeaderDragOver(false)}
        onDrop={async (e: DragEvent) => {
            e.preventDefault();
            setIsTagHeaderDragOver(false);
            try {
                const json = e.dataTransfer?.getData("application/json");
                if (json) {
                    const item = JSON.parse(json);
                    const strategy = dndRegistry.get("TAG");
                    if (strategy && item.type === "TAG") await strategy.onDrop(item, "root");
                }
            } catch(err) { console.error(err); }
            setDragItem(null);
        }}
      >
          <TreeView 
              items={tagTree()} 
              onSelect={node => filters.toggleTag(Number(node.id))}
              selectedIds={filters.selectedTags} 
              onContextMenu={handleContextMenu}
              editingId={editingId()}
              onRename={handleRename}
              onEditCancel={() => setEditingId(null)}
              expandedIds={expandedIds()}
              onToggle={toggleExpansion}
              onMove={handleMoveTag}
          />
          
          <TagContextMenu 
              x={contextMenuPos().x} 
              y={contextMenuPos().y} 
              isOpen={contextMenuOpen()} 
              node={contextMenuNode()}
              onClose={() => setContextMenuOpen(false)}
              onAddChild={handleCreateChildTag}
              onRename={id => setEditingId(id)}
              onDelete={handleOpenDeleteModal}
          />

          <TagDeleteModal 
              isOpen={deleteModalOpen()}
              onClose={() => setDeleteModalOpen(false)}
              node={nodeToDelete()}
          />
      </SidebarPanel>
  );
};
