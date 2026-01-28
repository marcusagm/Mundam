# Refactor Folders Architecture

## Context
Merging `locations` and `subfolders` into a single `folders` table to support recursive views, prevent hierarchy duplication, and simplify complex queries using Recursive CTEs.

## Plan

### Phase 1: Database Schema & Migration
- [x] Update `src-tauri/src/schema.sql` to replace `locations`/`subfolders` with `folders`.
- [x] Update `images` table schema in SQL file.
- [x] Update `Db::new` in `src-tauri/src/database.rs` to apply the NEW schema (drop old tables since we are in dev mode).

### Phase 2: Backend Logic (Rust)
- [x] Update `src-tauri/src/database.rs`:
    - [x] Remove `get_or_create_location`, `get_or_create_subfolder`.
    - [x] Add `upsert_folder(path, name, parent_id, is_root)`.
    - [x] Add `get_folder_by_path(path)`.
    - [x] Add `get_folder_hierarchy(root_id)`.
    - [x] Update `save_images_batch` to use `folder_id`.
- [x] Update `src-tauri/src/db_tags.rs` (Library Stats, Images Filtered).
    - [x] Implement `get_images_filtered` using Recursive CTEs for the `recursive` flag.
- [x] Update `src-tauri/src/indexer/mod.rs`:
    - [x] Refactor `start_scan` to generic "Ensure Folder Path" logic.
    - [x] Refactor `create_subfolder_hierarchy` to `ensure_folder_hierarchy` (Unified table).
- [x] Update Commands:
    - [x] `src-tauri/src/location_commands.rs` -> Update to use `folders`.
    - [x] `src-tauri/src/tag_commands.rs` -> Update `get_images_filtered` implementation.

### Phase 3: Frontend Logic
- [x] Update `src/lib/types.ts` (or similar) to match new `Folder` shape.
- [x] Update `src/components/features/library/FolderTreeSidebarPanel.tsx`.
    - [x] Consume unified `folders` tree.
    - [x] Implement "Recursive" checkbox using the new API.
- [x] Update `src/core/store/libraryStore.ts`:
    - [x] Pass `recursive` flag to backend.

## Technical Details

### Recursive Query (SQLite)
```sql
WITH RECURSIVE folder_tree AS (
    SELECT id FROM folders WHERE id = ?  -- Selected Folder
    UNION ALL
    SELECT f.id FROM folders f
    JOIN folder_tree ft ON f.parent_id = ft.id
)
SELECT * FROM images WHERE folder_id IN folder_tree;
```

### Folder Table
```sql
CREATE TABLE folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id INTEGER NULL REFERENCES folders(id) ON DELETE CASCADE,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    is_root BOOLEAN DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

## Implementation Summary (Post-Development)

### Beyond the Plan
- **Isolated Component Logic**: Extracted `FolderContextMenu` into its own component (`src/components/features/library/FolderContextMenu.tsx`) to decouple context menu logic from the main sidebar panel. This improves readability and maintainability.
- **Enhanced UI Components**:
    - Updated `Checkbox` component to handle interactive states within complex UI structures like context menus.
    - Refactored `ContextMenu` styling (`src/components/ui/context-menu.css`) to be more robust and visually consistent with the design system.
- **Recursive Count Logic**: Added `get_folder_counts_recursive` in the backend (`src-tauri/src/db_tags.rs`) to pre-calculate image counts for folders *including their children*. This allows the frontend to toggle counts dynamically without complex client-side math.
- **Direct Recursive View Toggle**:  Implemented the Recursive View toggle directly in the folder context menu. This UX pattern allows users to quickly switch modes for specific navigation contexts.

### Improvements
- **Simplified State Management**: The `metadataStore` was refactored to handle `folder_counts` and `folder_counts_recursive` separately but in parallel. This ensures that switching views is instant (O(1) lookup) after the initial load.
- **Optimized Database Queries**:
    - Used efficient Single-Query CTEs for fetching counts, avoiding N+1 query problems.
    - Indexed `parent_id` and `path` implicitly via foreign keys and unique constraints to ensure fast lookups during heavy indexing.
- **Code Cleanup**:
    - Removed legacy `location_commands.rs` functions that were specific to the old split `locations`/`subfolders` architecture.
    - Cleaned up frontend imports and removed unused dependencies (e.g., `lucide-solid` icons that were no longer needed in specific files).

### Pending / Next Steps
- [ ] **Drag & Drop Reordering**: While the folder structure supports hierarchy, we haven't implemented Drag & Drop to move folders in the database yet. This is a potential future feature.
- [ ] **Virtualization for Large Trees**: If the user has thousands of folders, the `FolderTreeSidebarPanel` might benefit from list virtualization (e.g., `solid-virtual`). currently it renders all nodes.
- [ ] **Watch Mode Robustness**: Verify that `notify` watcher events correctly trigger `ensure_folder_hierarchy` for deep changes in the new unified structure. Initial tests pass, but edge cases in deep nesting should be monitored.
