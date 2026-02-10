# Refactoring Media Players (Video & Audio)

**Date**: 2026-02-10  
**Status**: Completed  
**Objective**: Decouple monolithic media player components, centralize streaming logic, and improve maintainability through colocation and hooks.

---

## 1. Video Player Refactoring (`src/components/ui/VideoPlayer`)

The original `VideoPlayer.tsx` was a monolithic component (~730 lines) handling logic, state, and UI.

### 🏗️ Architectural Changes
- **`useVideoPlayer.ts`**: Extract all playback logic (HLS management, state signals, event handlers, keyboard shortcuts).
- **`VideoPlayerContext.tsx`**: Implementation of a Solid.js Context to provide state/actions to sub-components without prop-drilling.
- **Sub-components**: 
  - **`VideoControls.tsx`**: Interactive controls (Play/Pause, Skip, Volume, Quality, Speed).
  - **`VideoSeekbar.tsx`**: Seek bar with buffer progress and hover previews.
- **`index.tsx`**: Main entry point, now a lightweight layout component wrapping sub-components in an `AudioProvider`.
- **Colocation**: Moved `video-player.css` into the component directory.

---

## 2. Audio Player Refactoring (`src/components/ui/AudioPlayer`)

Followed the same pattern established for the video player to ensure consistency across the library.

### 🏗️ Architectural Changes
- **`useAudioPlayer.ts`**: Extract waveform fetching, HLS handling, and playback logic.
- **`AudioPlayerContext.tsx`**: State management via Context.
- **Sub-components**:
  - **`AudioControls.tsx`**: Playback controls (Sync with `videoActions` for active player logic).
  - **`AudioWaveform.tsx`**: Interactive waveform rendering and seek bar.
- **`index.tsx`**: Refactored entry point using sub-components.
- **Colocation**: Moved `audio-player.css` into the component directory.

---

## 3. Media Source Consolidation (Hooks)

Centralized the logic for determining streaming URLs (Native vs HLS vs Linear HLS) to prevent code duplication between renderers and inspectors.

### 🪝 New Hooks
- **`useVideoSource.ts`**: 
  - Handles `isHlsServerAvailable` checks.
  - Manages `probeVideo` execution.
  - Automatically generates the correct URL based on codec and path via `getVideoUrl`.
- **`useAudioSource.ts`**:
  - Manages audio URL generation via `getAudioUrl`.
  - Ensures reactivity to path and quality changes.

---

## 4. Feature Integration (Renderers & Inspectors)

Updated the consumer components to use the new hooks and consistent props.

### 📺 Renderers (`src/components/features/itemview/renderers`)
- **`VideoPlayer.tsx`**: Refactored to use `useVideoSource`.
- **`AudioRenderer.tsx`**: Refactored to use `useAudioSource` and transitioned from `src` to `path` prop for consistency.
- **`ItemView.tsx`**: Updated calling logic to pass `path` instead of manually generating URLs.

### 🔎 Inspectors (`src/components/features/inspector`)
- **`VideoInspector.tsx`**: Refactored to use `useVideoSource`, removing duplicated probing logic.
- **`AudioInspector.tsx`**: Refactored to use `useAudioSource`.

---

## 5. Final Polishing & Verification

- **Exports**: Updated `src/components/ui/index.ts` to export the new component structures.
- **Type Safety**: Ran `npx tsc --noEmit` and resolved all lint/type errors.
- **Consistency**: Ensured that both players correctly sync with the global `videoStore` and `audioStore` to enforce "Single Active Player" behavior.
