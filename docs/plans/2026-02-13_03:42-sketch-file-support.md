# Implementation Plan: Sketch File Support (.sketch)

**Created at:** 2026-02-13 03:42
**Status:** ✅ Completed
**Context:** Addition of support for Sketch files (.sketch) in Mundam, focusing on thumbnail generation and high-quality previews.

---

## 📖 Overview
Sketch files are modern design documents structured as renamed ZIP archives. This plan follows a modular approach to extract pre-rendered previews from these archives without requiring the Sketch application.

## 🏗️ Technical Information
- **Format:** ZIP (Renamed to .sketch)
- **Preview Path:** `previews/preview.png` (Standard)
- **Metadata Paths:** 
  - `meta.json`: General document info (version, fonts, app version).
  - `document.json`: Full document structure (pages, layers, artboards).
  - `user.json`: User-specific settings (canvas position, zoom).

---

## 🛠️ Step-by-Step Implementation

### Phase 1: Dedicated Extrator (Completed)
Created a specialized extractor to handle the specific structure of Sketch files.

1.  **File Creation**: Created `src-tauri/src/thumbnails/extractors/sketch.rs`.
    - Implemented `extract_sketch_preview` using the `zip` crate.
    - Added high-priority check for `previews/preview.png`.
    - Added a fallback scanner that searches for any file ending in `preview.png` to avoid issues with non-standard internal structures.
    - Followed **Backend Guidelines**: Descriptive variable names, standard error return types (`Result<(Vec<u8>, String), Box<dyn std::error::Error>>`), and full documentation.

2.  **Module Registration**: Updated `src-tauri/src/thumbnails/extractors/mod.rs`.
    - Registered `pub mod sketch;`.
    - Added a match arm for `"sketch"` in the `NativeExtractor` routing logic.

3.  **Format Registry Update**: Updated `src-tauri/src/formats/definitions.rs`.
    - Changed `strategy` from `ThumbnailStrategy::Icon` to `ThumbnailStrategy::NativeExtractor`.
    - Changed `preview_strategy` from `PreviewStrategy::None` to `PreviewStrategy::NativeExtractor`.

### Phase 2: Verification (Completed)
1.  **Compiler Check**: Ran `cargo check` to ensure type compatibility.
    - Fixed initial type mismatch (standardized to `Box<dyn std::error::Error>`).
2.  **Linting**: Removed unused imports as requested by Clippy warnings.

---

## 📈 Future Enhancements (Phase 3 & 4)

### 1. Rich Metadata Extraction (Inspector)
- **Objective**: Parse `meta.json` during the indexing phase.
- **Fields to Extract**:
  - `appVersion`: Minimum Sketch version required.
  - `fonts`: List of fonts used in the document (useful for reporting missing fonts).
  - `created`: Creation timestamp.
- **Implementation**: Create a `meta.json` parser in `sketch.rs` using `serde_json`.

### 2. Artboard Navigation
- **Objective**: Allow users to see all Artboards without opening Sketch.
- **Approach**: 
  - Parse `document.json` to list pages and artboards.
  - Sketch stores artboard previews if requested, but usually, only the global preview is reliable. If we want artboard-level previews, we might need a more complex rendering engine or check if the user enabled "Export Artboards as Images" in Sketch settings.

---

## 🔗 Related Files
- `src-tauri/src/thumbnails/extractors/sketch.rs`: Main implementation.
- `src-tauri/src/thumbnails/extractors/mod.rs`: Registry.
- `src-tauri/src/formats/definitions.rs`: Format metadata.
