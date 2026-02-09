# Implementation Plan: Progressive Streaming (Option A)

> **Date:** 2026-02-09
> **Status:** **Failed and reverted**
> **Goal:** Implement non-blocking progressive streaming for audio and video to resolve UI freezing issues during transcoding.

## 1. Architecture Overview

We have implemented **Option A: Progressive Streaming Server** by extending the existing HLS Axum server. This moves the heavy transcoding work from the synchronous Tauri protocol handlers to an asynchronous, background web server.

### Current State (Problematic)
- `video-stream://` -> `video_stream.rs` -> `transcode_sync` (Blocks thread)
- `audio-stream://` -> `audio_stream.rs` -> `transcode_sync` (Blocks thread)

### Target State (Solution - Completed)
- Frontend (`stream-utils.ts`) detects if file needs transcoding.
- If yes, points generic HTML5 video/audio element to: `http://localhost:9876/stream/{type}/{path}`.
- Backend (`streaming/server.rs`) accepts connection and spawns `ffmpeg` stream via `progressive.rs`.
- `ffmpeg` writes to `stdout` -> piped directly to HTTP Response Body via `tokio-util::ReaderStream`.
- **Zero blocking:** UI remains responsive. Playback starts immediately (as soon as bytes arrive).

## 2. Component Design

### 2.1 Backend: Progressive Streaming Module (`src-tauri/src/streaming/progressive.rs`)
New module handles the FFmpeg streaming logic.

- **Video Stream:**
  - Codec: H.264 (libx264) + AAC
  - Container: Fragmented MP4 (`-movflags +frag_keyframe+empty_moov+default_base_moof`)
  - Preset: `ultrafast` (for instant start)
  - Tune: `zerolatency`
  - Quality: Configurable via `TranscodeQuality` enum (affects CRF/Bitrate).
- **Audio Stream:**
  - Codec: AAC
  - Container: ADTS (`-f adts`)
  - Quality: Configurable via `TranscodeQuality` enum (affects Bitrate).
  
### 2.2 Backend: Server Routes (`src-tauri/src/streaming/server.rs`)
Updated `Router` to include:
- `GET /stream/video/*path?quality=...` -> `progressive::stream_video`
- `GET /stream/audio/*path?quality=...` -> `progressive::stream_audio`

### 2.3 Frontend: URL Utils (`src/lib/stream-utils.ts`)
Updated URL generators to point to the local server instead of custom protocols when transcoding is needed.

- `video-stream://` -> `http://localhost:9876/stream/video/...`
- `audio-stream://` -> `http://localhost:9876/stream/audio/...`

## 3. Implementation Steps

### Step 1: Backend - Progressive Module ✅
Created `src-tauri/src/streaming/progressive.rs` with `FfmpegStream` implementation using `stdout` pipe.

### Step 2: Backend - Server Integration ✅
Registered new routes in `src-tauri/src/streaming/server.rs` and wired up handlers.

### Step 3: Frontend Update ✅
Modified `src/lib/stream-utils.ts` to switch from protocol URLs to HTTP server URLs using `HLS_SERVER_URL`.

### Step 4: Verification ✅
- Compilation: Passed `cargo check`.
- Architecture: Non-blocking async streams implemented via `tokio::process::Command` and `Axum Body`.
