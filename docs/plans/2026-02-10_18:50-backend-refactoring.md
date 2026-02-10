# Backend Refactoring and Organization

This document details the refactoring process undertaken to organize the `src-tauri` directory, improving modularity, maintainability, and architectural consistency as per `docs/guidelines/backend-rust.md`.

## 1. Overview
The goal of this refactoring was to clean up the `src-tauri/src` directory by moving scattered files into their respective domain modules and establishing a centralized registry for file format definitions.

## 2. Refactoring Steps

### A. Consolidation of Thumbnails & Settings
**Objective**: Move thumbnail and configuration logic into dedicated modules.

1.  **Thumbnails Module**:
    *   Moved `src-tauri/src/thumbnail_worker.rs` -> `src-tauri/src/thumbnails/worker.rs`.
    *   Moved `src-tauri/src/thumbnail_priority.rs` -> `src-tauri/src/thumbnails/priority.rs`.
    *   Updated `src-tauri/src/thumbnails/mod.rs` to export `worker` and `priority`.

2.  **Settings Module**:
    *   Moved `src-tauri/src/config.rs` -> `src-tauri/src/settings/config.rs`.
    *   Updated `src-tauri/src/settings/mod.rs` to export `config`.

3.  **Media Module**:
    *   Moved `src-tauri/src/ffmpeg.rs` -> `src-tauri/src/media/ffmpeg.rs`.
    *   Moved `src-tauri/src/metadata_reader.rs` -> `src-tauri/src/media/metadata_reader.rs`.
    *   Updated `src-tauri/src/media/mod.rs` to export `ffmpeg` and `metadata_reader`.

### B. Indexer Decomposition
**Objective**: Break down the monolithic `indexer` module into manageable components.

1.  **Created Sub-modules**:
    *   `src-tauri/src/indexer/types.rs`: Shared types (`ProgressPayload`, `BatchChangePayload`, `WatcherRegistry`).
    *   `src-tauri/src/indexer/watcher.rs`: File system watcher logic (`notify` crate integration).
    *   `src-tauri/src/indexer/scan.rs`: Initial scanning logic using `WalkDir`.
    *   `src-tauri/src/indexer/mod.rs`: Main entry point and re-exports.

2.  **Logic Separation**:
    *   Moved pure type definitions to `types.rs`.
    *   Isolated the long-running watcher task in `watcher.rs`.
    *   Encapsulated the initial scan and hierarchy ensuring logic in `scan.rs`.

### C. Modularization of Format Definitions
**Objective**: Create a single source of truth for supported file formats.

1.  **Created `formats` Module**:
    *   `src-tauri/src/formats/types.rs`: Enums for `MediaType`, `ThumbnailStrategy`, and `PlaybackStrategy`.
    *   `src-tauri/src/formats/definitions.rs`: The registry `SUPPORTED_FORMATS` containing all file definitions.
    *   `src-tauri/src/formats/mod.rs`: `FileFormat` struct implementation with detection logic.

2.  **Refactored `detector.rs`**:
    *   Updated `src-tauri/src/transcoding/detector.rs` to use `crate::formats::FileFormat` for detection instead of hardcoded string arrays.
    *   Removed redundant `TRANSCODE_VIDEO`, `NATIVE_VIDEO`, etc. constants.

### D. Updates to `lib.rs` and Imports
**Objective**: Ensure the application compiles and uses the new paths.

1.  **Module Declarations**:
    *   Updated `src-tauri/src/lib.rs` to use the new module structure.
    *   Updated all `use crate::...` statements across the codebase to point to the new locations (e.g., `crate::media::ffmpeg`).
    *   Corrected visibility of structs like `WatcherRegistry`.

## 3. Verification & Tests

### Integration Checks
*   Ran `cargo check` to resolve compiler errors and unused imports.
*   Verified that `start_indexing` command correctly initializes `Indexer` with the decomposed modules.

### Unit Tests
*   Added unit tests in `src-tauri/src/formats/mod.rs` to verify:
    *   Format detection by extension.
    *   Correct assignment of `MediaType` and `PlaybackStrategy`.

## 4. Documentation Updates
*   Updated `docs/guidelines/backend-rust.md`:
    *   Added a section on **Supported File Formats**.
    *   Updated the module organization section.
    *   Added examples for adding new formats and frontend usage.

## 5. Summary
The backend is now structured significantly better. Adding new features or formats will be more intuitive, and the separation of concerns in the indexer will make debugging file system events much easier.
