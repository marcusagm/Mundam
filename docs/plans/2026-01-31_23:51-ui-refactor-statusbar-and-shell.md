# Refactor Status Bar and App Shell Enhancements

## Overview
This plan documents the completed efforts to refactor the application's global status bar, improve asset management (icons/logos), implement backend support for filtered counts, and enhance the `AppShell` with collapsible panels and smooth animations.

## Project Type
**WEB** (Tauri + SolidJS)

## Success Criteria
- [x] `GlobalStatusbar` refactored into micro-components (`StatusCounts`, `StatusMessages`, `StatusSystem`).
- [x] Application Logo added to the Sidebar.
- [x] "Filtered Total" count displayed and accurate in the status bar (backed by Rust).
- [x] Application Icons refreshed across all platforms.
- [x] `AppShell` supports toggling Sidebar and Inspector via status bar buttons.
- [x] Sidebar and Inspector toggles persist across reloads.
- [x] Central Viewport fills available space dynamically when panels are toggled.
- [x] Panel collapse/expand animations are smooth.

## Tech Stack
- **Frontend:** SolidJS, CSS Modules/Variables
- **Backend:** Rust (Tauri Commands, SQLx)
- **State Management:** `libraryStore` (SolidJS Store)
- **Styling:** CSS variables, Flexbox/Grid

## File Structure
```
src/
├── components/
│   ├── layout/
│   │   ├── GlobalStatusbar.tsx      # Main container
│   │   ├── global-statusbar.css     # Styles
│   │   └── LibrarySidebar.tsx       # Added Logo here
│   └── features/
│       └── statusbar/               # New directory
│           ├── StatusCounts.tsx     # Filter/Load counts
│           ├── StatusMessages.tsx   # Transient messages
│           └── StatusSystem.tsx     # System status/settings
├── layouts/
│   ├── AppShell.tsx                 # Added toggles & persistence
│   └── app-shell.css                # Added animations
src-tauri/
├── src/
│   ├── db_tags.rs                   # Added get_image_count_filtered
│   └── tag_commands.rs              # Exposed new command
└── permissions/
    └── tags.toml                    # Permission definition
```

## Task Breakdown

### Phase 1: Status Bar Refactoring
- [x] **Task 1.1**: Split `GlobalStatusbar` into three distinct components: `StatusCounts`, `StatusMessages`, and `StatusSystem`.
    - *Input:* `src/components/layout/GlobalStatusbar.tsx`
    - *Output:* New files in `src/components/features/statusbar/`
- [x] **Task 1.2**: Update CSS for the new status bar layout, ensuring the center section takes up remaining space (`flex-grow: 1`).
    - *Input:* `src/components/layout/global-statusbar.css`
    - *Output:* Flexbox layout suitable for the 3-section design.

### Phase 2: Backend & Data
- [x] **Task 2.1**: Implement `get_image_count_filtered` in `src-tauri/src/db_tags.rs` to efficiently count items matching filters.
- [x] **Task 2.2**: Expose command in `src-tauri/src/tag_commands.rs`.
- [x] **Task 2.3**: Update `libraryStore.ts` and `tags.ts` to fetch and store this count.
- [x] **Task 2.4**: Define missing permission `allow-get-image-count-filtered` in `src-tauri/permissions/tags.toml` and add to default capabilities.

### Phase 3: Assets & Branding
- [x] **Task 3.1**: Add `logo-color.svg` to `LibrarySidebar`.
- [x] **Task 3.2**: Regenerate Tauri icons from `docs/idea/logo/icon.png`.
- [x] **Task 3.3**: Clean build bundles to force icon refresh.

### Phase 4: AppShell Enhancements
- [x] **Task 4.1**: Update `AppShell` to manage `isSidebarOpen` and `isInspectorOpen` state.
- [x] **Task 4.2**: Implement `AppShellContext` to expose toggles to children (Statusbar).
- [x] **Task 4.3**: Add persistence for panel states using `localStorage`.
- [x] **Task 4.4**: Add toggle buttons to `GlobalStatusbar` (Left/Right ends).
- [x] **Task 4.5**: Implement "Clear Selection" button in `StatusCounts`.

### Phase 5: Layout & Animation
- [x] **Task 5.1**: Update `ResizablePanel` to support `flexGrow` and read sizes from DOM (fixing drag-after-toggle bugs).
- [x] **Task 5.2**: Update `AppShell` central panel to use `flexGrow={1}`.
- [x] **Task 5.3**: Implement CSS-based collapse animations (transition width/opacity).
- [x] **Task 5.4**: Add `.is-resizing` class to disable transitions during drag for performance.

## Phase X: Verification
- [x] **Build:** `npm run tauri dev` runs without permission errors.
- [x] **Functionality:** 
    - [x] Filtered counts update correctly.
    - [x] Sidebar/Inspector toggle buttons work.
    - [x] Selection can be cleared via status bar.
    - [x] Layout persists after refresh.
- [x] **UI:**
    - [x] Animations are smooth.
    - [x] Resizing works correctly after toggling panels.
    - [x] Icons are sharp and correct.

## ✅ PHASE X COMPLETE
- Lint: ✅ Pass (Fixed variable shadowing in AppShell)
- Security: ✅ Permissions properly scoped
- Build: ✅ Success
- Date: 2026-01-31
