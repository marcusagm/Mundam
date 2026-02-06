# Inspector and Protocol Stabilization Refactoring
**Date:** 2026-02-06 01:09 (Local Time)
**Documentation Type:** Session Report / Implementation Log

## 1. Executive Summary
This session focused on stabilizing the Mundam application's custom protocol handlers (`orig://` and `thumb://`) and ensuring consistent URI encoding across the frontend. The primary goal was to eliminate runtime panics in the Rust backend and improve the robustness of file serving, especially for large files and paths with special characters.

## 2. Core Problem Analysis
The application was experiencing intermittent crashes (panics) and hanging during resource loading. The root causes identified were:
- **Unsafe Path Handling:** The backend used `.unwrap()` on URI parsing, which failed when encountering malformed or unencoded paths from the browser.
- **Resource Exhaustion:** Attempting to serve very large raw files or high-bitrate video without proper chunking led to Out-Of-Memory (OOM) risks.
- **URI Encoding Mismatch:** The frontend was passing raw file paths to custom schemes, which WebKit/Safari (Tauri's engine on macOS) interpreted incorrectly if they contained spaces or symbols.

## 3. Implementation Details

### A. Backend: Protocol Handler Refactor (`src-tauri/src/protocols.rs`)
The entire protocol handling logic was rewritten to be "panic-free" and more performant.

1.  **Safe `serve_file` Implementation:**
    *   Removed all `.unwrap()` calls.
    *   Introduced `error_response` helper for consistent HTTP error reporting (`404`, `500`, `413`).
    *   **Memory Safety:** Added a **100MB maximum chunk size** for Range requests.
    *   **OOM Protection:** Implemented a **150MB file size limit** for non-range requests. Files exceeding this must be streamed via Range headers.
    *   **MIME Fallbacks:** Added manual detection for common professional formats (MXF, MKV, HEIC, PSD, AI, OPUS) when `mime_guess` fails.

2.  **Standardized Path Extraction:**
    *   Implemented `extract_path_part` to handle different URI variations (e.g., `orig://localhost/path` vs `orig://path`).
    *   Correctly handles leading slash normalization for absolute paths on Unix systems.

3.  **Optimization for Native Extractors:**
    *   Re-enabled and stabilized the `extract_preview` logic which allows rendering formats like Affinity Suite, PSD, and RAW photos on-the-fly via the `orig://` protocol.

### B. Frontend: URI Encoding Synchronization
Ensured that all paths sent to the custom protocols are URL-safe.

1.  **Centralized Encoding:**
    *   Updated `ItemView.tsx` to wrap all `assetUrl` and `thumbUrl` generations with `encodeURIComponent()`.
2.  **Component-Level Updates:**
    *   **Audio/Video:** Updated `AudioInspector` and `VideoInspector` to encode paths before passing them to the players.
    *   **Previews:** Updated `FontInspector`, `ModelInspector`, and `MultiInspector` to ensure thumbnail thumbnails are correctly formationed.
    *   **Logic:** Fixed rendering logic in `MultiInspector` to correctly iterate and display the "deck" of selected previews.

### C. Developer Experience & Debugging
*   **Indexer Improvements:** Added detailed debug logs to `src-tauri/src/indexer/mod.rs` to track folder hierarchy verification and scanning progress.
*   **Log Monitoring:** Monitored the background indexing process, observing successful thumbnail generation for multiple formats (ICO, OGG, FLAC, AI, PSD, SVG, etc.).
*   **Error Handling in Logs:** Correctly identified remaining "Format error decoding Jpeg" logs as source-file corruptions in sample sets, which are now handled gracefully by the fallback logic.

## 4. Modified Files
| File Path | Change Summary |
| :--- | :--- |
| `src-tauri/src/protocols.rs` | Major refactor: Removal of unwraps, addition of size limits and safety checks. |
| `src-tauri/src/indexer/mod.rs` | Added debug logging for boot sequence and scanning. |
| `src/components/features/itemview/ItemView.tsx` | Standardized `encodeURIComponent` for resource URLs. |
| `src/components/features/audio/AudioInspector.tsx` | Fixed URI formation for audio player. |
| `src/components/features/video/VideoInspector.tsx` | Fixed URI formation for video player. |
| `src/components/features/font/FontInspector.tsx` | Encoded thumbnail paths for font previews. |
| `src/components/features/model/ModelInspector.tsx` | Encoded thumbnail paths for 3D model previews. |
| `src/components/features/multi/MultiInspector.tsx` | Encoded thumbnail paths for multi-select deck. |
| `src/components/features/inspector/base/InspectorTags.tsx` | Type-safety fixes for resource keys. |

## 5. Next Steps / Recommendations
1.  **Corrupt Asset Handling:** Continue monitoring "Not enough bytes" errors in Jpeg decoding (seen in some RAW thumbnails); investigate if `binary_jpeg` needs a более restrictive footer check.
2.  **Performance Profiling:** Monitor memory usage when multiple large videos are loaded simultaneously via the new 100MB chunk limit.
3.  **PDF/AI Thumbnails:** The `NativeExtractor` for AI (PDF-based) is working, but further refinement on PDF-to-Image rendering in Rust would improve the grid view.

---
**Report generated by Antigravity AI.**
