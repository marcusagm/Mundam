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
import "./reference-image.css";

// ============================================================================
// Global Image Cache
// ============================================================================
// Tracks which thumbnail URLs have been successfully loaded.
// This persists across component recycling during virtualization.
const loadedThumbnails = new Set<string>();

// Add a loaded thumbnail to cache
function markThumbnailLoaded(url: string) {
  loadedThumbnails.add(url);
}

// Check if thumbnail was previously loaded
function isThumbnailLoaded(url: string | undefined): boolean {
  return url ? loadedThumbnails.has(url) : false;
}

// ============================================================================
// Component
// ============================================================================

interface ReferenceImageProps {
  id: number;
  src: string;
  thumbnail: string | null;
  alt: string;
  width?: number | null;
  height?: number | null;
}

export function ReferenceImage(props: ReferenceImageProps) {
  const [localError, setLocalError] = createSignal(false);
  const [localThumbnail, setLocalThumbnail] = createSignal<string | null>(null);
  
  let unsubscribe: (() => void) | null = null;
  
  // Subscribe to thumbnail ready events for this specific image
  onMount(() => {
    unsubscribe = subscribeThumbnailReady(props.id, (_id, path) => {
      console.log(`Thumbnail ready for image ID: ${props.id}, path: ${path}`);
      setLocalThumbnail(path);
      setLocalError(false);
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
    
    // Validate path is not absolute (sanity check)
    // Normalize backslashes to forward slashes for URL
    const normalizedPath = path.replace(/\\/g, '/');
    return `thumb://localhost/${normalizedPath}`;
  });

  // Se não houver thumbnail, ou erro, ou pendente, não mostramos a imagem
  const displaySrc = createMemo((): string | undefined => {
    if (localError()) return undefined;
    if (!shouldShowImage()) return undefined;
    return thumbUrl();
  });

  // Check if this thumbnail was already loaded (from cache)
  const isAlreadyLoaded = createMemo(() => isThumbnailLoaded(thumbUrl()));
  
  // Track loaded state - initialize from cache
  const [loaded, setLoaded] = createSignal(false);
  
  // Calculate aspect ratio for stability
  const aspectRatio = createMemo(() => {
    if (props.width && props.height) {
      return `${props.width} / ${props.height}`;
    }
    return undefined;
  });

  const handleLoad = () => {
    const url = thumbUrl();
    if (url) {
      markThumbnailLoaded(url);
    }
    setLoaded(true);
  };

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

  // Determine if we should show placeholder
  // Don't show if image was already loaded from cache
  const showPlaceholder = createMemo(() => {
    // No source to display
    if (!displaySrc()) return true;
    // Has error
    if (localError()) return true;
    // Already loaded from cache - don't show placeholder
    if (isAlreadyLoaded()) return false;
    // Not yet loaded
    if (!loaded()) return true;
    return false;
  });

  return (
    <div 
      class="reference-image-container" 
      style={{ "aspect-ratio": aspectRatio() }}
      data-id={props.id}
    >
      <Show when={showPlaceholder()}>
        <div class="image-placeholder">
           <Loader size="sm" />
        </div>
      </Show>
      
      <Show when={displaySrc() && !localError()}>
        <img
          src={displaySrc()}
          alt={props.alt}
          draggable={false}
          onLoad={handleLoad}
          onError={handleError}
          class={loaded() || isAlreadyLoaded() ? "loaded" : "loading"}
        />
      </Show>
    </div>
  );
}
