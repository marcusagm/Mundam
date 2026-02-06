# Protocol Refactoring & Architectural Excellence
**Date:** 2026-02-06 03:13
**Author:** Antigravity AI Assistant

## 1. Objective
Refactor the generic `orig://` and `thumb://` Tauri custom protocols into a fragmented, modular, and high-performance architecture. The goal was to improve maintainability, establish a scalable foundation for multiple media types, and optimize asset streaming.

---

## 2. Backend Implementation (Rust)

### A. Modular Directory Structure
Created a dedicated directory `src-tauri/src/protocols/` to encapsulate all protocol logic.

- **`common.rs`**: Core utility layer.
    - Centralized `serve_file` logic with robust error handling.
    - Integrated with `formats.rs` for central MIME-type detection.
    - Implemented **Intelligent Chunking**: Large Range requests are now capped at 100MB and served as `206 Partial Content` instead of rejected.
- **`thumb.rs`**: Specialized handler for the `thumb://` protocol.
    - Made generic over `tauri::Runtime` to ensure compatibility with different Tauri backends.
- **`image.rs`, `audio.rs`, `video.rs`, `font.rs`, `model.rs`**: Dedicated handlers for each media type.
    - `image.rs` includes support for native preview extractors (RAW, PSD, etc.).
- **`placeholders.rs`**: Registered `document://`, `ebook://`, and `code://` for future expansion.
- **`mod.rs`**: Central registration helper `register_all` that keeps `lib.rs` clean.

### B. SQLite Optimization
Enabled **WAL (Write-Ahead Logging)** mode in `database.rs`.
- **Impact**: Prevents UI/IPC hangs during heavy indexer or thumbnail worker activity by allowing concurrent reads and writes.

---

## 3. Frontend Implementation (SolidJS)

### A. Protocol Switching
Updated `src/components/features/itemview/ItemView.tsx` to dynamically switch protocols based on the file extension and detected media type.

| Media Type | Protocol | Usage |
|------------|----------|-------|
| Images | `image://`| Standard image rendering + RAW previews |
| Videos | `video://`| Streaming chunks via native video element |
| Audio | `audio://`| Progressive loading for audio playback |
| Fonts | `font://` | CSS FontFace injection |
| 3D Models | `model://`| serving assets to the 3D renderer |

### B. Bug Fixes & Refinements
- **`FontView.tsx`**: Removed redundant `encodeURI` call that caused double-encoding (transforming `%2F` into `%252F`), fixing font load failures.
- **Inspectors**: Updated `AudioInspector.tsx` and `VideoInspector.tsx` to use the new typed protocols.

---

## 4. Stability & Performance Gains

1. **Streaming Fix**: Large media files (videos) no longer fail on initialization. By serving partial content, we avoid 413 (Payload Too Large) errors while keeping server memory usage low.
2. **Concurrency**: Enabling WAL mode in SQLite ensures that the `Inspector` can fetch tags and metadata instantly even while the `Indexer` is scanning the disk.
3. **Double-Encoding Resolution**: Assets with spaces or special characters in their paths now load reliably across all renderers.

---

## 5. Detailed Step-by-Step

1. **Phase 1: Foundation**
   - Created `src-tauri/src/protocols/` directory.
   - Built `common.rs` with `serve_file` abstraction.
2. **Phase 2: Backend Modules**
   - Implemented separate files for `image`, `audio`, `video`, `font`, `model`, and `thumb`.
   - Connected `common::serve_file` to use `crate::formats::FileFormat::detect`.
   - Cleaned up `lib.rs` by moving logic to `protocols::register_all`.
3. **Phase 3: Frontend Sync**
   - Refactored `ItemView.tsx` to use specific protocol templates.
   - Updated `AudioInspector` and `VideoInspector` templates.
4. **Phase 4: Debugging & Optimization**
   - Fixed `tauri::Runtime` generic type mismatch in `thumb.rs`.
   - Resolved Font 404 error by removing double-encoding in `FontView.tsx`.
   - Optimized `serve_file` to handle Range requests gracefully (capping chunks at 100MB).
   - Applied `PRAGMA journal_mode = WAL` in `database.rs` to fix IPC stalls.

---

## 6. Affected Files
- `src-tauri/src/lib.rs`
- `src-tauri/src/database.rs`
- `src-tauri/src/protocols.rs` (Deleted)
- `src-tauri/src/protocols/mod.rs` (New)
- `src-tauri/src/protocols/common.rs` (New)
- `src-tauri/src/protocols/thumb.rs` (New)
- `src-tauri/src/protocols/image.rs` (New)
- `src-tauri/src/protocols/audio.rs` (New)
- `src-tauri/src/protocols/video.rs` (New)
- `src-tauri/src/protocols/font.rs` (New)
- `src-tauri/src/protocols/model.rs` (New)
- `src-tauri/src/protocols/placeholders.rs` (New)
- `src/components/features/itemview/ItemView.tsx`
- `src/components/features/itemview/renderers/font/FontView.tsx`
- `src/components/features/inspector/audio/AudioInspector.tsx`
- `src/components/features/inspector/video/VideoInspector.tsx`
