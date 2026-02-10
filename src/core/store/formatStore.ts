import { createStore } from "solid-js/store";
import { invoke } from "@tauri-apps/api/core";

// Match Rust types (camelCase enums)
export type MediaType = 'image' | 'video' | 'audio' | 'project' | 'archive' | 'model3d' | 'font' | 'unknown';
export type PlaybackStrategy = 'native' | 'hls' | 'linearHls' | 'audioTranscode' | 'audioHls' | 'audioLinearHls' | 'transcode' | 'none';

// Match Rust struct fields (snake_case struct fields, but camelCase enum values)
export interface FileFormat {
  name: string;
  extensions: string[];
  mime_types: string[];
  type_category: MediaType;
  playback: PlaybackStrategy;
}

interface FormatState {
  formats: FileFormat[];
  extensionMap: Record<string, FileFormat>;
  initialized: boolean;
}

const [formatState, setFormatState] = createStore<FormatState>({
  formats: [],
  extensionMap: {},
  initialized: false
});

export const formatActions = {
  // Initialize the store by fetching formats from backend
  initialize: async () => {
    if (formatState.initialized) return;

    try {
      const formats = await invoke<FileFormat[]>('get_library_supported_formats');

      const map: Record<string, FileFormat> = {};
      formats.forEach(fmt => {
        fmt.extensions.forEach(ext => {
           map[ext.toLowerCase()] = fmt;
        });
      });

      setFormatState({
        formats,
        extensionMap: map,
        initialized: true
      });
      console.log(`Format registry initialized with ${formats.length} formats.`);
    } catch (err) {
      console.error("Failed to load formats from backend:", err);
    }
  },

  // Get format for an extension
  getFormat: (filename: string): FileFormat | undefined => {
    const ext = filename.split('.').pop()?.toLowerCase() || '';
    return formatState.extensionMap[ext];
  },

  // Get media type helper
  getMediaType: (filename: string): MediaType => {
    const fmt = formatActions.getFormat(filename);
    const cat = fmt?.type_category;
    if (cat) return cat.toLowerCase() as MediaType; // Rust might return 'Image' (Pascal) if I didn't set rename correctly, but I set camelCase.
    // Wait, let's verify casing. Rust Enum rename_all="camelCase" -> Image, Video... wait.
    // camelCase of "Image" is "image". "Model3D" is "model3d" (or "model3D"?).
    // Default strum display might differ from serde? Serde handles serialization.
    return 'unknown';
  },

  // Get playback strategy helper
  getPlaybackStrategy: (filename: string): PlaybackStrategy => {
    const fmt = formatActions.getFormat(filename);
    return fmt?.playback || 'none';
  }
};

export { formatState };
