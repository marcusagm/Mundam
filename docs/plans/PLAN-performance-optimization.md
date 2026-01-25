# PLAN: Performance Optimization & Virtualization

## Context
The application is experiencing high resource usage (80-90% CPU, ~2GB RAM) and UI lag when scrolling large image lists. The user suspects that original images are being rendered instead of thumbnails, and that the Masonry layout recalculations are expensive.

## Goal
Drastically reduce memory and CPU footprint and ensure 60fps scrolling by implementing UI virtualization and fixing asset loading efficiency.

## Phase 1: Diagnostics & Quick Wins (Hotfix)
- [x] **Task 1.1: Verify Thumbnail Usage**
    - Investigation: Confirm if `ReferenceImage` is falling back to `orig://` protocol unnecessarily.
    - Check: Inspect `store.items` in `App.tsx` to ensure `thumbnail_path` is populated.
    - Action: Ensure `ReferenceImage` prefers `thumb://` protocol.
- [x] **Task 1.2: Debug Render Cycle**
    - Investigation: Use SolidJS DevTools or console logs to count re-renders of `ImageGrid` and `ReferenceImage`.
    - Action: Optimize `createStore` signals if excessive reactivity is detected.

## Phase 2: Virtualization Strategy (Masonry)
- [x] **Task 2.1: Virtualization Research**
    - Evaluate libraries: `tanstack-virtual` (solid adapter) vs custom implementation.
    - Challenge: Masonry layouts are harder to virtualize because items have variable heights and absolute positions.
    - Decision: Determine if we switch to a "Column-based" virtualization (easier) or a true "Masonry" virtualization (complex).
- [x] **Task 2.2: Implement Virtualized Grid**
    - Create `VirtualizedMasonry` component.
    - Logic: Only render items currently in the viewport + buffer.
    - Height Calculation: Use the known `width` and `height` from DB to pre-calculate layout without DOM measuring.
- [x] **Task 2.3: Integrate with App**
    - Replace `ImageGrid` with `VirtualizedMasonry`.

## Phase 3: Advanced Optimization
- [x] **Task 3.1: Offload Layout Calculations**
    - Move Masonry position logic (x, y coords) to a Web Worker or compute it once in Rust before sending to frontend?
    - Current: Calculated in CSS/JS? Actually current implementation uses CSS Column (if simple) or JS? *Check current implementation*.
    - Optimization: If JS-based, memorize layout positions so they don't recalc on every scroll/update.
- [x] **Task 3.2: Image Decoding**
    - Ensure `decoding="async"` is set on images.
    - Verify `ReferenceImage` cleanup (object URLs, though we use custom protocols now so this is handled by browser cache).

## Phase 4: Verification
- [x] **Benchmark 1**: Memory usage with 1000+ images (< 500MB).
- [x] **Benchmark 2**: CPU usage during scroll (< 20%).
- [x] **Benchmark 3**: No "white flashes" or layout shifts during scroll.

## Questions/Risks
- **Risk**: Virtualizing Masonry with dynamic heights requires precise knowledge of all item heights beforehand. We have `width` and `height` in DB, so we *can* calculate aspect ratio and exact pixel height before rendering.
- **Trade-off**: Virtualization might break "Cmd+F" native browser search (content not in DOM).

## Unplanned Implementations & Design Decisions

### 1. Robust Database Initialization (SQLite)
*   **Problem**: `Database not initialized` error on fresh installs because the backend tried to connect before the frontend created the file.
*   **Decision**: Added `.create_if_missing(true)` to the Rust `SqliteConnectOptions`.
*   **Reason**: Ensures the backend is self-healing and can start up cleanly regardless of the frontend state.

### 2. Idempotent Indexing (UPSERT)
*   **Problem**: `INSERT OR REPLACE` was deleting existing records, resetting Auto-Increment IDs and NULLing `thumbnail_path`, causing infinite re-generation loops on every restart.
*   **Decision**: Switched to `INSERT ... ON CONFLICT(path) DO UPDATE ...` explicitly excluding `thumbnail_path`.
*   **Reason**: Preserves expensive work (thumbnails) and stable IDs between sessions.

### 3. Parallel Thumbnail Generation (Rayon)
*   **Problem**: Sequential processing was too slow for large libraries.
*   **Decision**: Implemented `rayon` crate for parallel iteration + Batch size tuning (5 items).
*   **Reason**: Maximizes CPU usage (200%+) during indexing to finish fast, while small batches provide immediate visual feedback to the user.

### 4. CPU-Saving Placeholder Strategy
*   **Problem**: High CPU usage during scroll because the browser tried to decode high-res original images when thumbnails were missing.
*   **Decision**: `ReferenceImage` now returns `undefined` (hiding the `<img>` tag) if no thumbnail is available.
*   **Reason**: It's better to show a lightweight CSS spinner than to choke the UI thread decoding 4K jpegs just for a fallback.

### 5. Layout Stability Fixes
*   **Problem**: Images overlapping or layout breaking during window resize.
*   **Decision**: Wrapped layout calculations in `requestAnimationFrame` and synced with `ResizeObserver`.
*   **Reason**: Prevents race conditions between the browser's paint cycle and the virtualizer's math.
