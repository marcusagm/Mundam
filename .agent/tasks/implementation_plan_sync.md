
# Task: Robust File Synchronization & Recursive Folder Events

## 🎯 Objective
Implement a robust synchronization system between the file system, database, and frontend. Ensure that file additions and removals (detected via watcher) update the UI (Grid, Sidebar Counts, Tags) in real-time without full reloads, maintaining total data consistency.

## 🛡️ Principles
1.  **Consistency First:** Database and UI must perfectly reflect the file system state.
2.  **Granular Updates:** Avoid full re-fetches for single file changes.
3.  **Context-Aware Deletion:** When a file is removed, know its tags/folder *before* DB deletion to update UI counters correctly.
4.  **Safe "Remove Folder":** UI action to remove a root folder only removes it from the database (un-monitor), never deletes original files.

## 📋 Implementation Plan

### Phase 1: Backend Infrastructure (Contextual Events)
- [x] **Database Helpers (`database.rs`)**:
    - Implement `get_image_context(id)`: Returns tags, folder_id, and is_untagged status before deletion.
    - Implement `delete_image_by_path_returning_context(path)`: Wraps the above + deletion in a transaction.
- [x] **Watcher Logic (`indexer/mod.rs`)**:
    - Add `Remove` event handling.
    - Implement a `Debouncer` struct to aggregate rapid file system events (500ms window).
    - Batch events into `library:batch-change` payload containing added/removed items metadata.
- [x] **Thumbnail Handling**:
    - Ensure thumbnails are deleted from the `thumbnails` Application local data folder when their source image is removed from DB.

### Phase 2: Location Management
- [x] **Explicit Location Removal (`location_commands.rs`)**:
    - Verify `remove_location` deletes the Folder + Children Images + Metadata from DB.
    - Ensure it cleans up generated thumbnails.
    - **Crucial:** Ensure it does NOT touch source files.

### Phase 3: Frontend State Synchronization
- [x] **Store Updates (`systemStore.ts` / `libraryStore.ts`)**:
    - Listen for `library:batch-change`.
    - **Add:** Check if new item fits current filter (Folder/Recursive/Tags) -> Add to `items` array.
    - **Remove:** Remove from `items` array by ID.
    - **Counters:** Update `metadataStore` stats (Folder Counts, Tag Counts, Total, Untagged) based on the "diff" payload.
- [x] **Recursive Count Logic**:
    - When a file is added/removed in Folder C (A/B/C), recursively update counts for C, B, A, and Root.

### Phase 4: UI Integration
- [x] **Context Menu (`FolderContextMenu.tsx`)**:
    - "Remove from Library": Only available for Root folders. Calls `remove_location`.
    - (Optional) "Delete from Disk": Distinct action for subfolders, blocked for now to strictly follow "Synchronization" request scope.

## 🧪 Verification
1.  **Add File:** Drop an image into a watched folder -> Appears in Grid + Counts increment.
2.  **Remove File:** Delete image from finder -> Disappears from Grid + Counts decrement (Tags & Folders).
3.  **Remove Root:** Click "Remove" on Sidebar Root -> Entire tree disappears from Sidebar + Images cleared from View.

## 🚀 Improvements & Unplanned Fixes
During implementation, several critical robustness issues were identified and resolved:

1.  **Robust Rename Handling (Split Events)**:
    - Implemented a 3-layer defense for file renames:
        1.  Atomic `RenameMode::Both` (Standard).
        2.  Tracker-based correlation for Split `From`/`To` events.
        3.  **Heuristic Correlation**: Matches `Remove` + `Add` events based on File Size and Creation Time when the OS fails to provide trackers.

2.  **Idempotency & Toggle Count Bug**:
    - Fixed a bug where file modification events were treated as "Additions", causing folder counts to incorrectly increment (e.g. 1 -> 3).
    - Backend now distinguishes `Insert` vs `Update`. Updates only modify metadata, preserving IDs and Counts.

3.  **Recursive Folder Hierarchy Repair**:
    - Implemented `ensure_folder_hierarchy` in Database.
    - Automatically reconstructs the database folder tree (Parent/Child links) when a deep folder structure is added or modified, preventing new subfolders from appearing as "Floating Roots".

## ✅ Resolved Issues (2025-01-29)

### 1. Subfolder Visual Glitch & Floating Roots
- **Cause:** The `Rename` logic in the Watcher was using `db.upsert_folder` incorrectly (passing full path as name) when encountering a new folder. This created orphaned folders with wrong names.
- **Fix:** Replaced with `db.ensure_folder_hierarchy`, which correctly derives the name and recursively ensures parent existence.

### 2. Empty Folders & Hierarchy Sync
- **Cause:** 
    1. `get_folder_counts_recursive` used `JOIN` instead of `LEFT JOIN`, excluding empty folders from stats.
    2. Startup Scan and Watcher ignored Directory-only events (filtered by `is_image_file`).
- **Fix:** 
    1. Updated SQL to `LEFT JOIN`.
    2. Updated Watcher to listen for Directory Create/Modify events and trigger `needs_refresh`.
    3. Updated Startup Scan (`WalkDir`) to explicitly collect and index keys for all directories.

### 3. Frontend Sync
- **Fix:** Added `needs_refresh` flag to `BatchChangePayload`. Frontend `metadataStore` now re-fetches locations when this flag is true (e.g., when an empty folder is added).

### 4. Folder Rename/Move Handling
- **Cause:** 
    1. Watcher was ignoring `Rename`/`Remove` for Directories.
    2. Race conditions in `upsert_folder` caused Unique Constraint violations.
    3. MacOS Case-Insensitive filesystem vs SQLite Case-Sensitive equality caused `rename_folder` to fail finding the old folder (e.g. "Teste 5" vs "teste 5"), treating it as a new folder creation (duplication).
- **Fix:** 
    - Updated Watcher to handle Directory events.
    - Implemented `db.rename_folder` with **Merge Strategy**.
    - **Race Condition Fix:** `upsert_folder` now handles `SQLITE_CONSTRAINT_UNIQUE` (2067) by re-fetching the existing ID.
    - **Case-Insensitivity:** `get_folder_by_path` now attempts an exact match, and falls back to `COLLATE NOCASE` if not found, ensuring compatibility with MacOS filesystem behavior.

### 5. Startup Sanity Check (Orphaned Folders)
- **Cause:** Folders deleted or renamed while the app was closed (or due to bugs) remained in the DB, causing "ghost" folders (e.g. "Pasta Sem Título") to persist after restarts.
- **Fix:** 
    - Implemented a **Pruning Step** at the end of `start_scan`.
    - After scanning the disk and building a map of *verified existing* folders, the system queries the DB for all folders under the current root.
    - Any folder in the DB that is NOT in the verified map is deleted. This automatically "cleans up" any phantom data on every startup.

### 6. Robust Rename Detection
- **Cause:** Renames often come as split events (Remove "From" + Add "To") or generic events. The previous buffer loop only treated renames as **Image** renames, ignoring buffered Folder renames.
- **Fix:** 
    - Updated `buffer_renamed.drain()` loop to check `is_dir()` on the target path.
    - If it is a directory, it now correctly calls `db.rename_folder` (triggering the Merge/Recursive logic) instead of trying to process it as an image.
    - Added heuristic matching (Size/Time) to pair orphan Remove+Add events for files, correcting split-event rename detection on MacOS.

## ⏳ Pending
- None. System should now be robust.
