import { Component, Show, createMemo, createResource } from "solid-js";
import { useAppStore, appActions } from "../../core/store/appStore";
import { Info, Tag as TagIcon, Hash, Calendar, Maximize2 } from "lucide-solid";
import { Input } from "../ui/Input";
import { Badge } from "../ui/Badge";
import { Separator } from "../ui/Separator";
import { TagInput, TagOption } from "../ui/TagInput";
import { tagService } from "../../lib/tags";

export const FileInspector: Component = () => {
  const { state } = useAppStore();

  // Derived state for the active item
  const activeItem = createMemo(() => {
    if (state.selection.length === 0) return null;
    const id = state.selection[state.selection.length - 1];
    return state.items.find((i) => i.id === id);
  });

  // Fetch tags for active item
  const [itemTags, { refetch: refetchTags }] = createResource(
      () => activeItem()?.id,
      async (id) => {
          if (!id) return [];
          return await tagService.getTagsForImage(id);
      }
  );
  
  // Tag Options for Input (mapped from itemTags)
  const tagValue = createMemo<TagOption[]>(() => {
      return (itemTags() || []).map(t => ({
          id: t.id,
          label: t.name,
          color: t.color || undefined
      }));
  });
  
  // All tags for suggestions
  const allTagsOptions = createMemo<TagOption[]>(() => {
      return state.tags.map(t => ({
          id: t.id,
          label: t.name,
          color: t.color || undefined
      }));
  });

  const handleTagsChange = async (newTags: TagOption[]) => {
      const current = itemTags() || [];
      const currentIds = new Set(current.map(t => t.id));
      const newIds = new Set(newTags.map(t => Number(t.id)));
      
      const itemId = activeItem()?.id;
      if (!itemId) return;

      // Added items
      const added = newTags.filter(t => !currentIds.has(Number(t.id)));
      if (added.length > 0) {
          await tagService.addTagsToImagesBatch([itemId], added.map(t => Number(t.id)));
      }

      // Removed items
      const removed = current.filter(t => !newIds.has(t.id));
      for (const t of removed) {
          await tagService.removeTagFromImage(itemId, t.id);
      }
      
      refetchTags(); // Sync
  };

  const handleCreateTag = async (name: string) => {
      const itemId = activeItem()?.id;
      if (!itemId) return;
      
      const newTagId = await tagService.createTag(name);
      await appActions.loadTags(); // Refresh global list
      
      // Add to image
      await tagService.addTagsToImagesBatch([itemId], [newTagId]);
      refetchTags();
  };

  return (
    <div style={{ height: "100%", display: "flex", "flex-direction": "column" }}>
        {/* Header */}
        <div style={{ 
             padding: "16px 12px 12px", 
             "font-size": "11px", 
             "font-weight": "600", 
             color: "var(--text-muted)", 
             "text-transform": "uppercase",
             "letter-spacing": "0.5px",
             "border-bottom": "1px solid var(--border-subtle)"
        }}>
            Inspector
        </div>

        <Show when={activeItem()} fallback={
            <div style={{ 
                flex: 1, 
                display: "flex", 
                "flex-direction": "column",
                "align-items": "center", 
                "justify-content": "center", 
                color: "var(--text-muted)", 
                gap: "12px",
                opacity: 0.5
            }}>
                <Info size={32} />
                <span style={{ "font-size": "12px" }}>No item selected</span>
            </div>
        }>
            <div style={{ padding: "16px", "overflow-y": "auto" }}>
                {/* Preview */}
                <div style={{ "margin-bottom": "20px" }}>
                    <div class="reference-image-container" style={{ 
                        "aspect-ratio": "1/1", 
                        "border-radius": "var(--radius-md)", 
                        "background": "var(--bg-color)",
                        "border": "1px solid var(--border-subtle)",
                        "display": "flex",
                        "align-items": "center",
                        "justify-content": "center",
                        "overflow": "hidden"
                    }}>
                       <img 
                            src={activeItem()?.thumbnail_path ? `thumb://localhost/${activeItem()?.thumbnail_path?.split(/[\\/]/).pop()}` : ""} 
                            style={{ "max-width": "100%", "max-height": "100%", "object-fit": "contain" }} 
                       />
                    </div>
                </div>

                {/* Fields */}
                <div class="field-group" style={{ "margin-bottom": "16px" }}>
                    <label style={{ display: "block", "font-size": "11px", color: "var(--text-secondary)", "margin-bottom": "6px", "font-weight": "500" }}>Name</label>
                    <Input value={activeItem()?.filename} disabled />
                </div>

                <div class="field-group" style={{ "margin-bottom": "16px" }}>
                    <label style={{ display: "flex", "align-items": "center", gap: "6px", "font-size": "11px", color: "var(--text-secondary)", "margin-bottom": "6px", "font-weight": "500" }}>
                        <TagIcon size={12} /> Tags
                    </label>
                    
                    <TagInput 
                        value={tagValue()} 
                        onChange={handleTagsChange} 
                        suggestions={allTagsOptions()}
                        onCreate={handleCreateTag}
                        placeholder="Add tags..."
                    />
                </div>

                {/* Metadata Grid */}
                <div style={{ 
                    "margin-top": "24px", 
                    "padding-top": "16px",
                    display: "grid",
                    "grid-template-columns": "1fr 1fr",
                    gap: "12px"
                }}>
                    <Separator class="col-span-2" style={{ "grid-column": "span 2", "margin-bottom": "8px" }} />
                    
                    <div>
                        <span style={{ display: "block", "font-size": "10px", color: "var(--text-muted)", "margin-bottom": "2px" }}>DIMENSIONS</span>
                        <div style={{ display: "flex", "align-items": "center", gap: "6px", "font-size": "12px", color: "var(--text-secondary)" }}>
                            <Maximize2 size={12} />
                            {activeItem()?.width} x {activeItem()?.height}
                        </div>
                    </div>
                    <div>
                        <span style={{ display: "block", "font-size": "10px", color: "var(--text-muted)", "margin-bottom": "2px" }}>TYPE</span>
                         <div style={{ display: "flex", "align-items": "center", gap: "6px", "font-size": "12px", color: "var(--text-secondary)" }}>
                            <Badge variant="secondary" style={{ "border-radius": "2px", "padding": "0 4px", "font-size": "10px" }}>
                                {activeItem()?.filename.split('.').pop()}
                            </Badge>
                        </div>
                    </div>
                     <div>
                        <span style={{ display: "block", "font-size": "10px", color: "var(--text-muted)", "margin-bottom": "2px" }}>CREATED</span>
                         <div style={{ display: "flex", "align-items": "center", gap: "6px", "font-size": "12px", color: "var(--text-secondary)" }}>
                            <Calendar size={12} />
                            <span>-</span>
                        </div>
                    </div>
                    <div>
                        <span style={{ display: "block", "font-size": "10px", color: "var(--text-muted)", "margin-bottom": "2px" }}>ID</span>
                         <div style={{ display: "flex", "align-items": "center", gap: "6px", "font-size": "12px", "font-family": "var(--font-mono)", color: "var(--text-muted)" }}>
                            <Hash size={12} />
                            {activeItem()?.id}
                        </div>
                    </div>
                </div>
            </div>
        </Show>
    </div>
  );
};
