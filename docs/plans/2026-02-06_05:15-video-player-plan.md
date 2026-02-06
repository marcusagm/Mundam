# Implementation Plan - Robust & Premium Video Player

Implement a complete, robust, and aesthetically premium Video Player component in `src/components/ui`, supporting both full-featured and compact modes, with streaming support via Tauri protocols.

## 🛠️ Architecture

### 1. UI Component: `src/components/ui/VideoPlayer.tsx`
- **Props**:
  - `src`: URL of the video (using `video://` protocol).
  - `variant`: `'full'` | `'compact'`.
  - `autoPlay`: boolean.
  - `title`: string (optional, display in controls).
- **Internal Hooks/Signals**:
  - Accurate buffer tracking using `video.buffered`.
  - Controls visibility signal (`showControls`) with 2.5s fade-out.
  - Center pulse indicator signal (`lastAction`) for play/pause feedback.

### 2. Styling: `src/components/ui/video-player.css`
- **Glassmorphism**: Advanced blur (12px) + dark semi-transparent backgrounds.
- **Animations**:
  - Smooth fade in/out for controls.
  - Pulse animation for center indicators.
  - Adaptive volume slider expansion in compact mode.
- **Design Tokens**: Standardized with Mundam's token system.

### 3. Backend (Protocol): `src-tauri/src/protocols/common.rs`
- **MPEG-TS Support**: Overriding MIME for `.m2ts` to `video/mp2t`.
- **Large File Handling**: Automatic Range fallthrough for 500MB+ files to prevent 414 errors and enable initial prodding.
- **OOM Protection**: Increased buffer limits to 1GB.

## 📋 Task List

### Phase 1: Foundation & UI
- [x] Create `src/components/ui/video-player.css` with core design tokens.
- [x] Create `src/components/ui/VideoPlayer.tsx` with basic `<video>` element and custom controls layout.
- [x] Integrate existing UI components (`Button`, `Slider`, `Tooltip`).

### Phase 2: Logic & Interactions
- [x] Implement Play/Pause, Seek, and Volume logic.
- [x] Add Formatted Time display (00:00 / 00:00).
- [x] Implement Control visibility logic (fade-out on inactivity).
- [x] Add Keyboard shortcuts support.

### Phase 3: Premium Features & Optimization
- [x] Add Fullscreen support (Webkit + Standard).
- [x] Implement Compact Mode layout (Adaptive for Inspector).
- [x] Add "Seek Preview" (time tooltip on hover).
- [x] Add Center Pulse Action indicators.
- [ ] Ensure ARIA compliance and accessibility.

### Phase 4: Integration
- [x] Update `src/components/features/itemview/renderers/video/VideoPlayer.tsx` to use the new component.
- [x] Update `src/components/features/inspector/video/VideoInspector.tsx`.

## 🧪 Verification Status
- [x] Test MP4 / WebM / MOV.
- [x] Fix M2TS playback issues.
- [x] Verify Range requests/streaming via Network tab.
- [x] Verify no OOM on 1GB+ files.
- [x] Test 'full' and 'compact' variants.
- [x] Verify keyboard navigation (Space, J/L, Arrows, F, M).
