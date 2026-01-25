import { createSignal, createEffect, Show } from "solid-js";

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
  
  // URL da thumbnail usando o protocolo customizado
  const thumbUrl = () => {
    // Ensure we handle empty strings or nulls
    if (!props.thumbnail || props.thumbnail === "") return null;
    const filename = props.thumbnail.split(/[\\/]/).pop();
    // thumb://localhost/<filename>
    return `thumb://localhost/${filename}`;
  };

  // Se não houver thumbnail, usamos a imagem original via protocolo orig
  const displaySrc = () => {
    const thumb = thumbUrl();
    // BLOCKING OPTIMIZATION:
    // If we have a thumbnail, use it.
    // If NOT, return undefined so the <img> tag is not rendered or stays hidden,
    // maximizing CPU savings until the worker finishes.
    if (thumb) return thumb;
    
    // We do NOT return orig:// anymore to save CPU.
    // The user will see the placeholder until the thumbnail ready event fires.
    return undefined;
  };

  // Calculate aspect ratio for stability
  const aspectRatio = () => {
    if (props.width && props.height) {
      return `${props.width} / ${props.height}`;
    }
    return undefined;
  };

  // Reset loaded state when src changes (e.g. thumb ready)
  createEffect(() => {
    displaySrc();
    setLoaded(false);
  });

  return (
    <div 
      class="reference-image-container" 
      style={{ "aspect-ratio": aspectRatio() }}
      data-id={props.id}
    >
      <Show when={!displaySrc() || !loaded()}>
        <div class="image-placeholder animate-pulse">
           <div class="placeholder-icon"></div>
        </div>
      </Show>
      
      <Show when={displaySrc()}>
        <img
          src={displaySrc()}
          alt={props.alt}
          // loading="lazy"
          onLoad={() => setLoaded(true)}
          onError={(e) => {
             // Only log real errors, not initial undefined states
             if (displaySrc()) console.error("Image failed to load:", displaySrc(), e);
          }}
          class={loaded() ? "loaded" : "loading"}
        />
      </Show>
      
      <Show when={thumbUrl() && loaded()}>
         <div class="badge-thumbnail">HD</div>
      </Show>
    </div>
  );
}
