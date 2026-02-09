# Linear HLS Implementation Report - 2026-02-09

## Overview
Implemented a "Linear HLS" streaming solution to reliably play complex media formats (SWF, MPEG, MPG, M2V) that were previously causing UI freezes or failing with standard transcoding.

## Changes

### Backend (Rust)
1.  **LinearManager (`src/streaming/linear.rs`)**:
    *   Manages FFmpeg processes for linear transcoding.
    *   Uses a dedicated temporary directory for each stream session.
    *   Implements `cleanup` logic to remove stale sessions and temporary files.
    *   Uses `tokio::process::Command` for non-blocking asynchronous execution.

2.  **Streaming Server (`src/streaming/server.rs`)**:
    *   Added new route `/hls-live/*path`.
    *   Implemented `linear_hls_handler` to handle playlist (`index.m3u8`) and segment (`.ts`) requests.
    *   Delegates to `LinearManager` to start or retrieve active sessions.
    *   Increased playlist generation timeout to 10 seconds to accommodate slower startups for legacy formats.

3.  **App State**:
    *   Integrated `LinearManager` into `AppState`.
    *   Added a background task to periodically clean up inactive linear sessions (every 60s).

### Frontend (TypeScript/React)
1.  **Stream Utils (`src/lib/stream-utils.ts`)**:
    *   Added `LINEAR_VIDEO_EXTENSIONS` set: `swf`, `mpg`, `mpeg`, `m2v`.
    *   Updated `getVideoUrl` to generate `/hls-live/` URLs for these formats.

2.  **Video Player UI (`src/components/ui/VideoPlayer.tsx`)**:
    *   Added a visual **Streaming Mode Badge** to indicate the active method:
        *   **LIVE** (Red): Linear HLS (SWF, MPG, etc.)
        *   **HLS** (Blue): Standard Segmented HLS (MKV, AVI, etc.)
        *   **NATIVE** (Green): Direct file access (MP4, MOV)
    *   Implemented `forcedDuration` prop to handle "Infinity" duration reported by live HLS streams, ensuring the correct total time is displayed.

## Verification
*   **Performance**: UI no longer freezes when playing SWF/MPG files.
*   **Reliability**: 408 Request Timeout errors minimized by increased server timeout.
*   **UX**: Users can clearly see which streaming mode is active via the new badge.
*   **Cleanup**: Temporary files and processes are correctly cleaned up after inactivity.
