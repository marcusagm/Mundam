import { createSignal, createMemo, Show, onMount, onCleanup } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Loader } from "../../ui/Loader";
import {
  isPendingRegeneration,
  markPendingRegeneration,
  getCompletedThumbnail,
  clearCompleted,
  subscribeThumbnailReady,
} from "../../../core/store/thumbnailStore";

interface ReferenceImageProps {
  id: number;
  src: string;
  thumbnail: string | null;
  alt: string;
  width?: number | null;
  height?: number | null;
}

export function ReferenceImage(props: ReferenceImageProps) {
  const [loaded, setLoaded] = createSignal(false);
  const [localError, setLocalError] = createSignal(false);
  const [localThumbnail, setLocalThumbnail] = createSignal<string | null>(null);
  
  let unsubscribe: (() => void) | null = null;
  
  // Subscribe to thumbnail ready events for this specific image
  onMount(() => {
    unsubscribe = subscribeThumbnailReady(props.id, (_id, path) => {
      console.log(`Thumbnail ready for image ID: ${props.id}, path: ${path}`);
      setLocalThumbnail(path);
      setLocalError(false);
      setLoaded(false);
    });
  });
  
  onCleanup(() => {
    if (unsubscribe) unsubscribe();
  });
  
  // Check if this image has a completed regeneration from store
  const effectiveThumbnail = createMemo(() => {
    // First check if we have a locally updated thumbnail from event
    const local = localThumbnail();
    if (local) return local;
    
    // Check if regeneration completed while unmounted
    const completed = getCompletedThumbnail(props.id);
    if (completed) {
      // Clear it from store since we're using it now
      clearCompleted(props.id);
      return completed;
    }
    
    // Use props thumbnail
    return props.thumbnail;
  });
  
  // Should we show the image? Not if pending regeneration with no completed thumbnail
  const shouldShowImage = createMemo(() => {
    if (isPendingRegeneration(props.id) && !localThumbnail() && !getCompletedThumbnail(props.id)) {
      return false;
    }
    return true;
  });
  
  // URL da thumbnail usando o protocolo customizado
  const thumbUrl = createMemo(() => {
    const path = effectiveThumbnail();
    if (!path || path === "") return undefined;
    const filename = path.split(/[\\/]/).pop();
    return `thumb://localhost/${filename}`;
  });

  // Se não houver thumbnail, ou erro, ou pendente, não mostramos a imagem
  const displaySrc = createMemo((): string | undefined => {
    if (localError()) return undefined;
    if (!shouldShowImage()) return undefined;
    return thumbUrl();
  });

  // Calculate aspect ratio for stability
  const aspectRatio = createMemo(() => {
    if (props.width && props.height) {
      return `${props.width} / ${props.height}`;
    }
    return undefined;
  });

  const handleError = () => {
    const thumb = thumbUrl();
    if (!thumb) return;
    
    // Already handling this error
    if (localError()) return;
    
    // Already pending regeneration (from store)
    if (isPendingRegeneration(props.id)) {
      setLocalError(true);
      return;
    }
    
    setLocalError(true);
    
    // Mark as pending in centralized store (persists across unmount)
    markPendingRegeneration(props.id);
    
    console.log(`Requesting thumbnail regeneration for image ID: ${props.id}`);
    invoke("request_thumbnail_regenerate", { imageId: props.id })
      .catch(err => console.error(`Failed to request regeneration:`, err));
  };

  return (
    <div 
      class="reference-image-container" 
      style={{ "aspect-ratio": aspectRatio() }}
      data-id={props.id}
    >
      <Show when={!displaySrc() || !loaded() || localError()}>
        <div class="image-placeholder">
           <Loader size="sm" />
        </div>
      </Show>
      
      <Show when={displaySrc() && !localError()}>
        <img
          src={displaySrc()}
          alt={props.alt}
          onLoad={() => setLoaded(true)}
          onError={handleError}
          class={loaded() ? "loaded" : "loading"}
        />
      </Show>
      
      <Show when={thumbUrl() && loaded() && !localError()}>
         <div class="badge-thumbnail">HD</div>
      </Show>
    </div>
  );
}
