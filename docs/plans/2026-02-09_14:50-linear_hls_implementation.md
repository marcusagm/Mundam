# Implementation Plan: Linear HLS Streaming (Live-Transcoding)

> **Date:** 2026-02-09
> **Goal:** implement non-blocking "Linear HLS" streaming for legacy/complex formats (SWF, MPEG, MPG) that cause UI freezing or seeking failures with the current architecture.

## 1. Context & Problem
Currently, we have two streaming modes:
1. **Native/Direct:** Works for MP4, WebM, etc.
2. **On-Demand HLS:** Works for most formats (MKV, AVI) but relies on seeking (`-ss`) which fails or is slow for formats like SWF, MPEG-PS, and M2V.
3. **Synchronous Transcoding:** The fallback for problematic formats blocks the main thread, freezing the UI.

**The Solution:** Implement a "Linear HLS" mode. Instead of generating segments on-demand (jumping around the file), we treat the file as a live stream. We spawn a single FFmpeg process that converts the file linearly from start to finish into an HLS playlist.

## 2. Architecture Changes

### 2.1 Backend: New `linear.rs` Module
A new `src-tauri/src/streaming/linear.rs` module will manage the linear transcoding sessions.

**Responsibilities:**
- Manage a temporary directory for each active stream.
- Spawn FFmpeg with `-f hls` to generate `index.m3u8` and `segment_%d.ts` files continuously.
- Monitor the process.
- Clean up resources when the stream is idle/stopped.

**FFmpeg Command Strategy:**
```bash
ffmpeg -i input.swf \
  -c:v libx264 -preset ultrafast -tune zerolatency \
  -c:a aac -b:a 128k \
  -f hls \
  -hls_time 6 \
  -hls_list_size 0 \
  -hls_segment_filename "segment_%03d.ts" \
  index.m3u8
```

### 2.2 Backend: `ProcessManager` Updates
Update `process_manager.rs` (or create a new `LinearManager`) to track these "Live" sessions.
- Map `file_path` -> `Session { temp_dir, process_id, last_access }`.
- Auto-kill processes that haven't been accessed in X seconds.

### 2.3 Backend: Server Routing Updates (`server.rs`)
Update `playlist_handler` and `segment_handler` to delegate to `linear.rs` for specific formats.

**Logic Flow `GET /playlist/{path}`:**
1. Check file extension/type.
2. If `SWF`, `MPEG`, `MPG`, `M2V`:
   - Call `linear::get_or_start_playlist(path)`.
   - Return dynamic M3U8 content (reading from the generated file).
3. Else:
   - Use existing On-Demand logic (`playlist::generate_m3u8`).

**Logic Flow `GET /segment/{path}/{file}`:**
1. Check file extension/type.
2. If `linear`:
   - Serve file directly from the temp directory managed by `linear.rs`.
3. Else:
   - Use existing On-Demand logic.

## 3. Implementation Steps

### Step 1: Create `linear.rs`
- Struct `LinearSession` to hold temp dir path and process handle.
- Function `start_transcode(file, quality) -> Result<Session>`.

### Step 2: Integrate with Server
- Modify `server.rs` to detect target formats.
- Wire up the endpoints to use `linear.rs`.

### Step 3: Frontend
- No major changes expected if the API remains compatible (HLS is HLS).
- Verify `stream-utils.ts` points these formats to the HLS server logic.

## 4. targeted Formats
- .swf
- .mpg
- .mpeg
- .m2v

## 5. Verification
- Play an `.swf` file.
- Confirm UI does NOT freeze.
- Confirm playback starts reasonably fast (~3-5s).
- Confirm "live" scrubbing (can go back, but only forward as far as transcoding has reached).
