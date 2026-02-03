# Item View Refactor & Font Preview Enhancement

## 🎯 Goal
Refactor the `ItemView` component into a dedicated feature module (`src/components/features/itemview`) to support a more scalable architecture. Simultaneously, enhance the `FontPreview` renderer to provide a professional-grade font inspection tool with tabs (Preview, Waterfall, Glyphs, Info) and a customizable toolbar, adhering to the project's design system.

---

## 📅 Phases

### Phase 1: Structural Refactor (The Move)
**Objective:** Organize files into the new feature directory and rename the context to reflect its scoped nature.

1.  **Create Directory Structure**
    - `src/components/features/itemview/`
    - `src/components/features/itemview/renderers/`
        - `image/` (ImageViewer, ImageToolbar)
        - `video/` (VideoPlayer)
        - `model/` (ModelViewer)
        - `font/` (FontView, FontToolbar, tabs/)
    - `src/components/features/itemview/common/`

2.  **Move & Rename Core Components**
    - Moved `src/components/features/viewport/ItemView.tsx` → `src/components/features/itemview/ItemView.tsx`.
    - Moved `src/components/features/viewport/ViewportContext.tsx` → `src/components/features/itemview/ItemViewContext.tsx`.
    - *Action:* Renamed `ViewportProvider` to `ItemViewProvider` and hook to `useItemViewContext`.

3.  **Move Renderers**
    - Moved all renderers to their specific subdirectories within `src/components/features/itemview/renderers/`.

4.  **Update Imports**
    - Updated all references in `src/components/layout/Viewport.tsx` and internal files.
    - Updated imports for moved renderers.

---

### Phase 2: Toolbar Architecture
**Objective:** Decouple the monolithic toolbar and allow renderers to define their own specific controls.

1.  **Create Generic Toolbar**
    - `src/components/features/itemview/common/BaseToolbar.tsx`: Handles common actions (Close, Prev/Next) and provides a slot for renderer-specific content.

2.  **Create Renderer-Specific Toolbars**
    - `ImageToolbar.tsx`: Contains Zoom, Pan, Rotate, Flip, Slideshow controls.
    - `FontToolbar.tsx`: Contains Font Size, Line Height, Letter Spacing, Color controls.
    - `ItemView.tsx`: Logic added to dynamic switch toolbars based on `mediaType`.

---

### Phase 3: Font Preview Enhancement
**Objective:** Build the new `FontView` with "Pro" features.

1.  **New Component Structure**
    - `src/components/features/itemview/renderers/font/`
        - `FontView.tsx`: Main container. Manages Tabs and Font Loading.
        - `FontToolbar.tsx`: Controls using `Slider`, `ColorPicker`, `Popover`.
        - `font-view.css`: Dedicated CSS file, no Tailwind.
        - `tabs/`
            - `PreviewTab.tsx`: Interactive textarea for custom testing.
            - `WaterfallTab.tsx`: Displays text at standard sizes (72px - 12px).
            - `GlyphsTab.tsx`: Grid view of character codes.
            - `InfoTab.tsx`: Metadata display.

2.  **Implementation Details**
    - **State**: `FontSettings` add to `ItemViewContext` (fontSize, lineHeight, letterSpacing, color, backgroundColor).
    - **UI**: Reused `src/components/ui/` components. Fixed `Popover` API usage (trigger prop).
    - **Styles**: Implemented strict CSS class usage.

---

### Phase 4: CSS & Polish
**Objective:** Ensure visual consistency and remove inline styles.

1.  **CSS Modules / Files**
    - Created `src/components/features/itemview/item-view.css` for the overlay and basics.
    - Created `src/components/features/itemview/renderers/renderers.css` for common renderer styles.
    - Created `src/components/features/itemview/renderers/font/font-view.css` for FontView specific styles.

2.  **Clean Up**
    - Removed old `ItemViewToolbar.tsx`.
    - Fixed `z-index` stacking issue where Popovers appeared behind the overlay.

---

## 📝 Checklist

- [x] Create `src/components/features/itemview` folder.
- [x] Move and Rename Context (`Viewport` -> `ItemView`).
- [x] Refactor imports project-wide.
- [x] Implement `FontView` structure.
- [x] Build `WaterfallTab`.
- [x] Build `GlyphsTab`.
- [x] Build `FontToolbar` using existing UI components.
- [x] Apply CSS styling (No Tailwind, dedicated .css files).
- [x] Verify functionality (Image viewer still works).
- [x] Organize renderers into subfolders (`image`, `video`, `model`, `font`).
- [x] Fix Popover Z-Index issues.

