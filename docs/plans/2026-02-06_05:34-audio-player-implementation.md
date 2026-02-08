# Audio Player Implementation - Detailed Report

## 1. Overview
The goal was to implement a professional-grade audio player for Mundam, replacing the basic HTML5 `<audio>` tag with a custom solution supporting real-time waveform visualization, global state synchronization, and multiple UI variants for the main view and the inspector.

## 2. Architecture & Backend (Rust/Tauri)
### FFmpeg Waveform Extraction
- **File:** `src-tauri/src/ffmpeg.rs`
- **Function:** `get_audio_waveform`
- **Details:** 
    - Executes FFmpeg to resample audio at 100Hz (downmixing to mono).
    - Outputs raw 32-bit float PCM data (`f32le`).
    - Aggregates samples into blocks to find peaks.
    - Normalizes data to a format suitable for the frontend (0.0 to 1.0).
- **Optimization:** Processes only peaks to keep the data lightweight.

### Tauri Commands
- **File:** `src-tauri/src/audio_commands.rs`
- **Command:** `get_audio_waveform_data`
    - High-level command that handles path resolution and calls the FFmpeg utility.
- **Registration:** Registered in `src-tauri/src/lib.rs`.

### Security & Capabilities (Tauri v2)
- **Permissions:** Added `allow-get-audio-waveform-data` in `src-tauri/permissions/main.toml`.
- **Capabilities:** Enabled the permission in `src-tauri/capabilities/default.json` for the main window.

## 3. Global State Management
- **File:** `src/core/store/audioStore.ts`
- **Purpose:** Synchronize audio settings between the main player and the inspector.
- **States:** 
    - `volume`: Floating point (0 to 1). Persisted in `localStorage`.
    - `isMuted`: Boolean.
    - `isLooping`: Boolean.
- **Sync:** Changes in one player (e.g., volume slider in ItemView) are immediately reflected in the other (e.g., Inspector).

## 4. UI Components (SolidJS)
### AudioPlayer Component
- **File:** `src/components/ui/AudioPlayer.tsx`
- **Variants:**
    - `full`: Album art placeholder, metadata, complex controls.
    - `compact`: Slim one-row layout for the sidebar inspector.
- **Features:**
    - **Waveform Seekbar:** Visualization that "fills" with color as the track plays.
    - **Buffering:** Secondary color bar indicating the stream's buffer progress.
    - **Adaptive Density:** Logic specifically to downsample the waveform (250 points for Full, 80 for Compact) to ensure it fits perfectly in any container.
    - **Keyboard Shortcuts:** `Space` (Play), `M` (Mute), `Left/Right` (Seek 5s).
    - **Error Handling:** Robust error state with "Retry" functionality that forces an `audio.load()`.

### Styling
- **File:** `src/components/ui/audio-player.css`
- **Design Tokens:** Adheres strictly to the Mundam design system (Oklch colors, glassmorphism, spacing tokens).
- **Animations:** Subtle pulse animations for central actions (Play/Pause feedback).

## 5. Integration Points
### ItemView (Main Viewer)
- **File:** `src/components/features/itemview/renderers/audio/AudioRenderer.tsx`
- **Details:** A wrapper that connects the global viewport state to the `AudioPlayer`. Uses the `audio://` protocol for efficient file streaming.

### Inspector (Sidebar)
- **File:** `src/components/features/inspector/audio/AudioInspector.tsx`
- **Details:** Integrates the `compact` variant. Includes a transitional `Loader` when switching between different audio files.

## 6. UX Enhancements & Fixes
- **Persistent Volume:** Reimplemented to ensure the user's preferred volume is remembered across sessions.
- **Loop Toggle:** Added a global loop toggle with visual feedback.
- **Loading Overlay:** Unified loading state (`isActuallyLoading`) that waits for both the audio stream to be ready AND the waveform to be extracted before showing the content.
- **Adaptive Layout:** Fixed a bug where controls would disappear in the inspector by using a column-based layout for slim containers.

## 7. Verification Results
- **Formats:** Successfully tested with MP3, FLAC, WAV.
- **Protocol:** Correct handling of `audio://` with range requests (support for seeking in large files).
- **Aesthetics:** High-contrast waveform, smooth animations, and consistent design with the Mundam UI.
