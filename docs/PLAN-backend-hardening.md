# Plan: Backend Hardening & Refactoring
> Based on `docs/report/consolidated_status.md` (Items 1.1 - 1.6) and `docs/code-review.md`.

## 1. Context & Objectives
The goal is to consolidate the "Unified Media Detection System (UMDS)" and ensure the backend is robust against failures (corrupted files) and performant.

**Primary Objectives:**
1.  **Enforce Single Source of Truth:** Ensure `indexer` uses the same detection logic as `thumbnails`.
2.  **Fault Tolerance:** Prevent the thumbnail worker from getting stuck in loops (Poison Pill).
3.  **Format Expansion:** Proper support for SVG (rendering) and AI/EPS (preview).
4.  **Optimization:** Reduce I/O overhead and maintain DB health.

---

## 2. Technical Strategy

### A. Indexer Synchronization (UMDS)
- **Current State:** `indexer/metadata.rs` uses extension/imagesize.
- **Target State:** It must call `crate::formats::FileFormat::detect(path)` first.
- **Challenge:** `FileFormat::detect` reads bytes. We need to ensure performant seeking.

### B. Worker Resilience (Poison Pill)
- **Concept:** Add `thumbnail_errors` (int) and `thumbnail_status` (enum) columns to `images` table.
- **Logic:**
    - If processing fails -> increment error count.
    - If count > 3 -> Mark as `Failed`. Stop trying.
    - Worker query: `SELECT ... WHERE thumbnail_status != 'Failed'`.

### C. SVG & Design Rendering
- **SVG:** Since we are in Tauri, we can't easily "spawn a hidden webview" purely from Rust backend threads without a window context unless we use a headless browser or a Rust library.
    - *Decision:* Investigate `resvg` (Rust library) for high-performance SVG to Image conversion without Webview overhead. It's lighter and faster.
- **AI/EPS:** These are often PDF-compatible.
    - *Strategy:* Use `ffmpeg -i file.ai` (it often works) or `mutool` if present. We will stick to `ffmpeg` first as we already have the wrapper.

---

## 3. Implementation Phases

### Phase 1: Robustness Core (The "Poison Pill")
*Focus: Prevent crashes and infinite loops.*

*   [x] **DB Schema Migration:**
    *   Add columns to `images`: `thumbnail_attempts` (int, default 0), `thumbnail_last_error` (text, nullable).
*   [x] **Update `ThumbnailWorker`:**
    *   Modify selection query to exclude `thumbnail_attempts >= 3`.
    *   Implement error catching block: On Error -> Increment count, Save error message.
    *   On Success -> Reset count.

### Phase 2: Unified Media Detection (UMDS)
*Focus: Alignment between Indexer and Worker.*

*   [x] **Refactor `indexer/metadata.rs`:**
    *   Remove legacy detection logic.
    *   Import and use `crate::formats::FileFormat::detect`.
    *   Map `MediaType` to string for DB storage if necessary.

### Phase 3: Advanced Formats (SVG & Design)
*Focus: Visual fidelity.*

*   [x] **SVG Strategy Implementation:**
    *   Research/Add `resvg` crate or similar Rust SVG renderer.
    *   Implement `thumbnails/webview.rs` (rename to `thumbnails/svg.rs` maybe?) to render SVG to byte buffer -> save as WebP.
*   [x] **AI/EPS Strategy:**
    *   Update `formats.rs` to map `.ai` and `.eps` to `ThumbnailStrategy::Ffmpeg` (or a specific `PdfBased` strategy).
    *   Test FFmpeg extraction on sample `.ai` files.

### Phase 4: Performance Polish
*Focus: Speed and Maintenance.*

*   [x] **Header Optimization (Implemented):**
    *   Review `FileFormat::detect`. If possible, pass the open `File` handle to the decoder to avoid re-opening (requires significant refactoring of traits).
    *   *Alternative:* Just ensure `detect` reads the absolute minimum (header only).
*   [x] **DB Maintenance Task:**
    *   Create a backend command `run_db_maintenance()` that executes `VACUUM` and `ANALYZE`.
    *   Expose to Frontend Settings (so user can run "Optimize Library").

---

## 4. Verification Checklist

- [x] **Resilience Test:** Corrupt a file (zero bytes or random garbage) renamed to `.jpg`. Ensure Worker tries 3 times and then ignores it.
- [x] **UMDS Test:** Rename a `.png` file to `.jpg`. Indexer should detect it as "PNG Image" (via magic bytes) despite the extension.
- [x] **SVG Test:** Import a complex `.svg`. Verify a real thumbnail is generated (not an icon).
- [x] **Performance Monitor:** Check logs to ensure files aren't being double-read unnecessarily (or accept strict separation for now if refactor is too deep).

---

## 5. Agent Assignments

- **Backend Specialist:** All Phases.
- **Frontend Specialist:** Phase 4 (Expose "Optimize Library" button in Settings).
