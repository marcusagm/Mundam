import { Component, createMemo, Switch, Match } from "solid-js";
import { useAppStore } from "../../core/store/appStore";
import { Info } from "lucide-solid";
import { ImageInspector } from "../features/inspector/ImageInspector";
import { AudioInspector } from "../features/inspector/AudioInspector";
import { VideoInspector } from "../features/inspector/VideoInspector";
import { MultiInspector } from "../features/inspector/MultiInspector";
import "../features/inspector/inspector.css";

export const FileInspector: Component = () => {
  const { state } = useAppStore();

  const selectionCount = createMemo(() => state.selection.length);
  
  const activeItem = createMemo(() => {
    if (selectionCount() === 0) return null;
    const id = state.selection[state.selection.length - 1]; // Last selected
    return state.items.find((i) => i.id === id) || null;
  });

  const selectedItems = createMemo(() => {
      return state.items.filter(i => state.selection.includes(i.id));
  });

  const fileType = createMemo(() => {
      const item = activeItem();
      if (!item) return "unknown";
      
      const ext = item.filename.split('.').pop()?.toLowerCase();
      if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp'].includes(ext || "")) return "image";
      if (['mp3', 'wav', 'ogg', 'flac'].includes(ext || "")) return "audio";
      if (['mp4', 'mov', 'webm', 'avi'].includes(ext || "")) return "video";
      
      return "image"; // Default to image/generic for now
  });

  return (
    <div class="inspector-container">
        <div class="inspector-header">
            Inspector
        </div>

        <Switch fallback={
             // Empty State
             <div class="inspector-loading-overlay">
                <Info size={32} />
                <span>No item selected</span>
            </div>
        }>
            <Match when={selectionCount() > 1}>
                <MultiInspector items={selectedItems()} />
            </Match>
            
            <Match when={selectionCount() === 1 && activeItem()}>
                <Switch fallback={<ImageInspector item={activeItem()!} />}>
                    <Match when={fileType() === "audio"}>
                        <AudioInspector item={activeItem()!} />
                    </Match>
                    <Match when={fileType() === "video"}>
                        <VideoInspector item={activeItem()!} />
                    </Match>
                    <Match when={fileType() === "image"}>
                        <ImageInspector item={activeItem()!} />
                    </Match>
                </Switch>
            </Match>
        </Switch>
    </div>
  );
};
