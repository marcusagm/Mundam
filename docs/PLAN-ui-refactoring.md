# Plan: UI/UX Refactoring & Architecture Restructure

## Overview
Refactor the entire frontend codebase to match the "Eagle.cool" inspired 3-pane architecture defined in `docs/idea/ui-ux-structure.md`. This involves moving away from the monolithic `App.tsx` pattern to feature-based modules, separating logic (Core) from presentation (UI), and implementing a robust `AppShell`.

## Project Type
**WEB** (SolidJS + Tauri)

## Success Criteria
- [x] Project structure matches the proposed tree (src/core, src/layouts, src/features).
- [x] `App.tsx` is clean and only mounts providers and `AppShell`.
- [x] New 3-pane layout (Sidebar, Viewport, Inspector) is implemented using CSS Grid.
- [x] State management is decoupled from UI components (using Stores).
- [x] Virtual Masonry and Grid are fully migrated to the new Viewport module.

## Tech Stack
- **Framework**: SolidJS
- **State Management**: SolidJS Signals / Stores
- **Styling**: Vanilla CSS (CSS Modules or Global)
- **Host**: Tauri (Rust)

## File Structure Target
```text
src/
├── core/                   # Logic (store, hooks, API)
├── layouts/                # AppShell
├── components/
│   ├── layout/             # Header, Sidebar, Inspector
│   ├── features/           # Viewport, Search, Organizer
│   └── ui/                 # Atomic components
└── App.tsx                 # Entry point
```

## Task Breakdown

### Phase 1: Infrastructure & Core
- [x] **Create Directory Structure**: Create `src/core` (hooks, store, tauri), `src/layouts`, `src/components/layout`, `src/components/features`, `src/components/ui`.
- [x] **Isolate Tauri API**: Move `invoke` string commands to typed functions in `src/core/tauri/services.ts`.
- [x] **Extract Global State**: Refactor state currently in `App.tsx` (e.g., `fileList`, `currentPath`) to a dedicated Store (e.g., `src/core/store/appStore.ts`).

### Phase 2: AppShell Implementation
- [x] **Create AppShell**: Build `src/layouts/AppShell.tsx` implementing the 3-pane CSS Grid layout (Header, Nav, Main, Inspector, Footer).
- [x] **Create Layout Skeletons**: Create placeholder components:
    - `src/components/layout/PrimaryHeader.tsx`
    - `src/components/layout/LibrarySidebar.tsx`
    - `src/components/layout/FileInspector.tsx`
    - `src/components/layout/GlobalStatusbar.tsx`
- [x] **Update Entry Point**: Replace `App.tsx` content to render `AppShell` wrapped in necessary Providers.

### Phase 3: Viewport Migration (The Grid)
- [x] **Migrate Masonry**: Move `VirtualMasonry.tsx` and `ReferenceImage.tsx` to `src/components/features/viewport/`.
- [x] **Refactor AssetCard**: Extract the individual image card logic into `src/components/features/viewport/AssetCard.tsx` (handle selection/hover states).
- [x] **Connect Viewport to Store**: Ensure the migrated Viewport component consumes data from `appStore.ts` instead of props drilling.

### Phase 4: Panel Logic Implementation
- [x] **Implement Header**: Move Search and Navigation logic to `PrimaryHeader.tsx`.
- [x] **Implement Sidebar**: Move Folder Tree logic to `LibrarySidebar.tsx` (using the new store).
- [x] **Implement Inspector**: Create basic metadata view in `FileInspector.tsx` that reacts to selection state.

### Phase 5: Polish & UX
- [x] **Global Shortcuts**: Create `src/core/hooks/useKeyboardShortcuts.ts` for app-wide hotkeys (e.g., Search, Select All).
- [x] **Cleanup**: Remove legacy components and unused code from `App.tsx` and root `src` folder.

## Phase X: Verification
- [x] **Structure Check**: Verify file tree matches the plan.
- [x] **Build Check**: `npm run build` passes without circular dependencies.
- [x] **Functionality Check**: App launches, grid loads images, sidebar shows folders, selection works.
- [x] **Lint**: `npm run lint` (if available) or manual check.

## ✅ Completion Report
- **Date**: 2026-01-25
- **Status**: Successfully refactored to 3-pane architecture.
- **Notes**: 
    - `App.tsx` is now just a bootstrap point.
    - Global state is handled in `appStore.ts` using Solid Stores.
    - Layout is controlled by `AppShell.tsx` using CSS Grid.
    - Core features (Masonry, Inspection) are modularized.
    - Added keyboard shortcuts support (Ctrl+A, Esc, Ctrl+K).
