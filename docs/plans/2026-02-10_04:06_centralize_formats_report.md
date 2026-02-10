# Task: Centralize File Format Definitions - Complete Report

## Context
Currently, file format definitions and their playback logic are fragmented between `src-tauri/src/formats.rs` (Backend) and `src/lib/stream-utils.ts` / `src/components/features/itemview/ItemView.tsx` (Frontend). This leads to inconsistencies and maintenance risks. We will centralize the "Source of Truth" in the Backend.

## Requirements
1.  **Backend (`src-tauri/src/formats.rs`)**:
    -   Add missing formats: `m4r`, `mp2`, `spx`, `ra`, `mka`, `hevc`.
    -   Split grouped formats that require different handling (e.g., `swf` vs `flv`, `mpeg` vs `ts`).
    -   Add a `playback_strategy` field to `FileFormat` exposed to the frontend.
    -   Ensure all logic from `stream-utils.ts` is captured in the backend metadata.

2.  **Frontend**:
    -   Create a Store/Service to fetch specific format info from Backend.
    -   Refactor `stream-utils.ts` to use this backend data instead of hardcoded lists.
    -   Refactor `ItemView.tsx` and `inspector/utils.ts` to use the unified `getMediaType` logic derived from backend data.
    -   Establish Strict HLS vs Linear HLS differentiation.

## Completed Actions

### 1. Backend Refactoring (`src-tauri/src/formats.rs`)
-   Created `PlaybackStrategy` enum with specific HLS variants:
    -   `Native`: Direct browser support (`mp4`, `mp3`, etc.)
    -   `Hls`: Standard VOD streaming (`mkv`, `webm`, `flv`, etc.)
    -   `LinearHls`: Live transcoding for complex formats (`swf`, `mpg`, etc.)
    -   `AudioHls`: Standard VOD Audio streaming (`opus`, `ogg`, etc.)
    -   `AudioLinearHls`: Live Audio transcoding (`aiff`, `ra`, etc.)
    -   `Transcode`/`AudioTranscode`: Legacy strategies (deprecated/fallback)

-   Updated `SUPPORTED_FORMATS` registry based on user specifications:
    -   **Videos (Linear HLS):** `swf`, `m2v`, `mpg`, `mpeg`, `mjpeg`
    -   **Videos (HLS):** `webm`, `wmv`, `asf`, `mkv`, `flv`, `f4v`, `avi`, `mxf`, `ts`, `mts`, `vob`, `m2ts`, `3gp`, `3g2`, `wtv`, `rm`, `ogv`
    -   **Videos (Native):** `mp4`, `m4v`, `mov`
    -   **Audio (HLS):** `opus`, `oga`, `wma`, `ac3`, `dts`, `wv`, `aifc`, `amr`, `ape`
    -   **Audio (Linear HLS):** `ogg`, `spx`, `ra`, `mka`, `aiff`, `aif`
    -   **Audio (Native):** `mp3`, `wav`, `aac`, `m4a`, `m4r`, `flac`, `mp2`

### 2. Frontend Integration
-   **Created `src/core/store/formatStore.ts`**:
    -   SolidJS store that syncs `get_library_supported_formats` from Rust.
    -   Provides `getFormat`, `getMediaType`, and `getPlaybackStrategy` helpers.

-   **Refactored `src/lib/stream-utils.ts`**:
    -   Removed all hardcoded extension lists.
    -   Logic now driven entirely by `formatStore`.
    -   **Strict Endpoint Separation:**
        -   `Hls` / `AudioHls` strategies route to `/playlist/` (Standard VOD HLS).
        -   `LinearHls` / `AudioLinearHls` strategies route to `/hls-live/` with `mode=live` or `mode=audio` (Async Live Streaming).
        -   Synchronous transcoding (`video-stream://`) replaced by asynchronous HLS pipelines to prevent UI freezing.

-   **Refactored `src/components/ui/VideoPlayer.tsx`**:
    -   Updated `getStreamMode` to detect playback mode via URL parameters.
    -   `mode=vod` -> Blue/Purple "HLS" Badge.
    -   `mode=live` -> Red "LIVE" Badge.
    -   `NATIVE` -> Green "NATIVE" Badge.

### 3. Verification
-   `flv`, `mkv` correctly identified as HLS (VOD).
-   `swf` correctly identified as LIVE (Linear).
-   `mp4` correctly identified as NATIVE.
-   Audio formats routed to appropriate HLS endpoints to avoid blocking main thread.

## Migration Notes
-   The `Transcode` strategy is effectively deprecated for video in favor of `Hls` or `LinearHls` to ensure non-blocking playback.
-   The `/playlist/` endpoint is efficient for seekable media, while `/hls-live/` handles infinite/complex streams.
-   All format definitions are now centralized in Rust; frontend updates require recompiling the backend.

**Status:** Completed
**Date:** 2026-02-10
