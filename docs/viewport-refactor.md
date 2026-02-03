# Viewport Refactor & Modularization

## Overview
Refactor the `ItemView` component into a modular "Orchestrator" that delegates rendering to specialized sub-components (`ImageViewer`, `VideoPlayer`, `ModelViewer`, `FontPreview`) based on the file type. The goal is to support precise zoom handling, advanced transformations (free rotation, flip), and diverse media formats without polluting a single monolithic file.

## Project Type
**WEB** (Tauri/SolidJS)

## Success Criteria
- [ ] `ItemView` acts as a container/switcher, not a monolithic renderer.
- [ ] Zoom displays exact percentage (0% to 200%+) and starts at "Fit" logic correctly.
- [ ] Rotation tool allows free rotation (mouse drag) and resets correctly.
- [ ] New "Flip" tool (Horizontal/Vertical) implemented for images.
- [ ] Support for **Video/Audio** playback with native controls.
- [ ] Support for **Font** preview (custom text rendering).
- [ ] Support for **3D Models** (GLB/GLTF) using `<model-viewer>` or similar.
- [ ] UI Controls in `ItemViewToolbar` adapt or disable based on the active media type.

## Tech Stack
- **SolidJS**: Core framework.
- **Tauri**: File system access (`orig://` protocol).
- **CSS Variables**: Theme styling.
- **HTML5**: `<video>`, `<audio>` for media.
- **FontFace API**: For dynamic font loading.
- **Leaflet / Canvas / CSS Transform**: For Image Viewer manipulation (CSS Transforms preferred for lightweight approach).
- **Google <model-viewer>**: For 3D rendering.

## File Structure
```
src/components/features/viewport/
├── ItemView.tsx                  # Orchestrator (Cleaner)
├── ItemViewToolbar.tsx           # Universal Toolbar (Context-aware)
├── ViewportContext.tsx           # State management (Zoom, Rotate, Tool, Media Info)
└── renderers/
    ├── ImageViewer.tsx           # Handles Image Logic (Zoom, Pan, Rotate, Flip)
    ├── VideoPlayer.tsx           # Handles Video/Audio
    ├── ModelViewer.tsx           # Handles 3D Models
    └── FontPreview.tsx           # Handles Fonts
```

## Task Breakdown

### Phase 1: Foundation & Refactor
- [x] **Task 1: Create ViewportContext**  
  **Input:** Create `src/components/features/viewport/ViewportContext.tsx`.  
  **Output:** specific context provider handling `zoom`, `rotation`, `flip`, `tool` mode, and `reset()` logic.  
  **Verify:** Wrap `ItemView` with provider; dev tools show state updates.

- [x] **Task 2: Extract ImageViewer**  
  **Input:** `ItemView.tsx`, `ImageViewer.tsx`.  
  **Output:** Move existing image rendering, pan, and zoom logic to `ImageViewer.tsx`.  
  **Verify:** Images load and pan as before, but isolated in new component.

- [x] **Task 3: Implement Advanced Image Tools**  
  **Input:** `ImageViewer.tsx`.  
  **Output:** 
  - "Fit" logic calls `(container / natural) * 100`.
  - Rotation tool uses `Math.atan2` for free drag rotation.
  - Flip toggle adds `scaleX(-1)` / `scaleY(-1)`.
  **Verify:** User can rotate freely, flip image, and "Fit" button calculates correct %.

- [x] **Task 4: Update Toolbar**  
  **Input:** `ItemViewToolbar.tsx`.  
  **Output:** Connect buttons to `ViewportContext`. Display Zoom %. Add Flip button.  
  **Verify:** Toolbar buttons control the isolated `ImageViewer`.

### Phase 2: New Formats
- [x] **Task 5: Implement VideoPlayer**  
  **Input:** `renderers/VideoPlayer.tsx`.  
  **Output:** Component using `<video>` tag.  
  **Verify:** `.mp4`, `.webm` files play with sound. Toolbar zoom/rotate disabled or adapted.

- [x] **Task 6: Implement FontPreview**  
  **Input:** `renderers/FontPreview.tsx`.  
  **Output:** Component that loads `FontFace` from path and displays editable sample text.  
  **Verify:** `.ttf`/`.otf` files show "The quick brown fox" in the correct font.

- [x] **Task 7: Implement ModelViewer**  
  **Input:** `renderers/ModelViewer.tsx`.  
  **Output:** Integration of `<model-viewer>` or basic placeholder for 3D files.  
  **Verify:** `.glb`/`.gltf` files render in 3D space.

- [x] **Task 8: Orchestrator Switch**  
  **Input:** `ItemView.tsx`.  
  **Output:** `Switch/Match` logic to render appropriate component based on file extension/mime-type.  
  **Verify:** Opening different file types authenticates the correct renderer.

## Phase X: Verification
- [ ] **Lint Check:** `npm run lint` matches standards.
- [ ] **Type Check:** No `any` types in new Context/Renderers.
- [ ] **Manual UX:**
    - [ ] Zoom slider shows real %.
    - [ ] Rotate is smooth and resets.
    - [ ] Video plays.
    - [ ] Font loads.
    - [ ] 3D model loads (if sample available).
