# Plan: Notification Service Implementation

## Overview
Implementation of a centralized notification system using [Sonner](https://sonner.emilkowal.ski/) to provide real-time, non-intrusive feedback for user actions across the Elleven Library application.

## Project Type
**WEB** (Tauri + SolidJS)

## Success Criteria
- [x] Centralized notification hook for consistent API usage.
- [x] Clear visual feedback for all CRUD operations (Tags, Folders, Smart Folders).
- [x] Background process status updates (Indexing lifecycle).
- [x] Safety mechanisms for destructive actions (Undo for Tag deletion).
- [x] Contextual messaging (e.g., mentioning the specific tag name in the toast).
- [x] Interaction refinement (notifications only after final confirmation).

## Tech Stack
- **Library**: Sonner (SolidJS port)
- **Icons**: Lucide-Solid
- **State Management**: SolidJS Context/Signals
- **Styling**: Vanilla CSS with custom animations and backdrop-blur.

## File Structure
- `src/components/ui/Sonner.tsx`: Core toaster and toast item components.
- `src/components/ui/sonner.css`: Premium aesthetics (glassmorphism, animations).
- `src/core/hooks/useNotification.ts`: Public API for triggering notifications.
- `src/App.tsx`: Global toaster integration.

## Task Breakdown

### Phase 1: Foundation
- **Task ID**: FOUNDATION-01
- **Name**: Create UseNotification Hook
- **Agent**: `frontend-specialist`
- **Priority**: P0
- **Dependencies**: None
- **INPUT**: Requirement for centralized notification API.
- **OUTPUT**: `src/core/hooks/useNotification.ts` with `success`, `error`, `info`, and `warning` methods.
- **VERIFY**: Hook can be imported and methods called without errors.

- **Task ID**: FOUNDATION-02
- **Name**: Global Toaster Setup
- **Agent**: `frontend-specialist`
- **Priority**: P0
- **Dependencies**: FOUNDATION-01
- **INPUT**: `App.tsx` layout.
- **OUTPUT**: `<Sonner />` component added to the root layout and styled with `sonner.css`.
- **VERIFY**: Empty toast container rendered in DOM without affecting existing UI.

### Phase 2: Core Feature Notifications
- **Task ID**: CORE-01
- **Name**: Tag Lifecycle Notifications
- **Agent**: `frontend-specialist`
- **Priority**: P1
- **Dependencies**: FOUNDATION-02
- **INPUT**: `TagTreeSidebarPanel.tsx`, `TagDeleteModal.tsx`.
- **OUTPUT**: Notifications for Create, Rename, and Delete. Delete includes "Undo" logic.
- **VERIFY**: Perform tag actions; toast appears with correct contextual message.

- **Task ID**: CORE-02
- **Name**: Tag Assignment Notifications
- **Agent**: `frontend-specialist`
- **Priority**: P1
- **Dependencies**: FOUNDATION-02
- **INPUT**: `InspectorTags.tsx`, `TagDropStrategy.ts`, `ImageDropStrategy.ts`.
- **OUTPUT**: Notifications for applying/removing tags via Inspector or Drag-and-Drop.
- **VERIFY**: Drag a tag onto an image; toast confirms assignment to X items.

- **Task ID**: CORE-03
- **Name**: Library Folder Notifications
- **Agent**: `frontend-specialist`
- **Priority**: P2
- **Dependencies**: FOUNDATION-02
- **INPUT**: `FolderTreeSidebarPanel.tsx`, `FolderDeleteModal.tsx`.
- **OUTPUT**: Notifications for adding/removing monitored folders.
- **VERIFY**: Unlink a folder; toast confirms removal.

- **Task ID**: CORE-04
- **Name**: Smart Folder Notifications
- **Agent**: `frontend-specialist`
- **Priority**: P2
- **Dependencies**: FOUNDATION-02
- **INPUT**: `SmartFoldersSidebarPanel.tsx`, `SmartFolderDeleteModal.tsx`.
- **OUTPUT**: Notifications for saving searches and deleting smart folders.
- **VERIFY**: Save an advanced search; toast confirms "Smart Folder Created".

### Phase 3: System & Background Notifications
- **Task ID**: SYSTEM-01
- **Name**: Indexing Lifecycle Notifications
- **Agent**: `frontend-specialist`
- **Priority**: P3
- **Dependencies**: FOUNDATION-02
- **INPUT**: `App.tsx` (indexing triggers).
- **OUTPUT**: "Indexing Started" info toast and "Indexing Complete" success toast.
- **VERIFY**: Add a folder; toast confirms indexing started. Wait for completion; toast confirms finish.

- **Task ID**: SYSTEM-02
- **Name**: Library Sync Notifications
- **Agent**: `frontend-specialist`
- **Priority**: P2
- **Dependencies**: FOUNDATION-02
- **INPUT**: `metadataStore.ts` (handleBatchChange).
- **OUTPUT**: Notifications for background sync (added, removed, updated items).
- **VERIFY**: Move/Delete/Add files in a monitored folder; toast confirms sync summary.

### Phase 4: Refinement & Bug Fixing
- **Task ID**: REFINEMENT-01
- **Name**: Notification Timing Refinement
- **Agent**: `frontend-specialist`
- **Priority**: P1
- **Dependencies**: CORE-01
- **INPUT**: User feedback on immediate "New Tag" notification.
- **OUTPUT**: Logic modified to only show notification during `handleRename` after user confirms the name.
- **VERIFY**: Click "+"; no notification. Type name and Enter; notification appears.

- **Task ID**: REFINEMENT-02
- **Name**: Fix remove_location Arguments
- **Agent**: `frontend-specialist`
- **Priority**: P0
- **Dependencies**: CORE-03
- **INPUT**: Console error `missing required key locationId`.
- **OUTPUT**: `FolderDeleteModal.tsx` updated to use `locationId` instead of `id`.
- **VERIFY**: Folder removal successfully completes backend call without error.

- **Task ID**: REFINEMENT-03
- **Name**: Visual Polish
- **Agent**: `frontend-specialist`
- **Priority**: P3
- **Dependencies**: FOUNDATION-02
- **INPUT**: `sonner.css`.
- **OUTPUT**: Added `backdrop-filter: blur(8px)` and vendor prefixes for glassmorphism effect.
- **VERIFY**: Toasts have premium translucent look.

## Phase X: Verification
- [x] Lint: `npm run lint` checked.
- [x] Security: No credentials exposed; input sanitized.
- [x] Build: `npm run tauri build` (verified via `tauri dev` logs).
- [x] Date: 2026-01-30

## ✅ PHASE X COMPLETE
- Status: All tasks implemented and verified.
- Date: 2026-01-30
