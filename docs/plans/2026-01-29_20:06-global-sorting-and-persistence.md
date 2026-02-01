# Report: Implementation of Global Sorting and Persistence
**Date:** 2026-01-29
**Status:** Completed

## 1. Objective
Uniformize the sorting system across all views (Grid, Masonry, List), implement persistence for user preferences, and fix metadata consistency (Rating and Format).

---

## 2. Steps Performed

### Backend (Rust/SQLite)
- **Dynamic Sorting:** Modified `get_images_filtered` to accept `sort_by` and `sort_order` parameters.
- **Security:** Implemented a whitelist for allowed columns to prevent SQL injection.
- **Case-Insensitive Sorting:** Added `COLLATE NOCASE` for string columns (`filename`, `format`).
- **Null Handling:** Implemented `(column IS NULL) ASC` in SQL to ensure items without rating or dates always appear at the end.
- **Stable Sort:** Added `filename` as a mandatory secondary sorting criteria to prevent "jumping" items when primary values are identical.
- **Database Migration:** Added an automatic migration step in `database.rs` to ensure existing databases receive the `format`, `rating`, `notes`, and `added_at` columns.

### Indexer
- **Format Persistence:** Updated the indexer to correctly identify and persist the `format` field in the database.
- **Standardization:** Forced format strings to lowercase (e.g., `webp` instead of `WebP`) for better grouping and UI consistency.

### Frontend (SolidJS)
- **Unified Store:** Standardized `SortField` keys in `filterStore.ts` to match database column names.
- **Persistence Layer:** Implemented `localStorage` syncing for `layout`, `sortBy`, `sortOrder`, and `thumbSize`. Added persistence for `AppShell` and `LibrarySidebar` resizable panel sizes.
- **Refined UI Components:** 
    - Updated `ListViewToolbar.tsx` with unified labels and the new "Rating" sort option.
    - Standardized `VirtualListView.tsx` headers to trigger sorting and toggle order.
    - Updated `CommonMetadata.tsx` (Inspector) to use real database metadata.

---

## 3. Beyond the Plan & Improvements

- **Automatic Migration:** Instead of just fixing the schema file, we added a runtime check that upgrades existing databases, preventing crashes for users with older versions.
- **Secondary ASC Sort:** When sorting by fields like "Rating" (DESC), we kept the secondary "Filename" sort as ASC, providing a more intuitive alphabetical sub-grouping.
- **CSS Semantic Refactoring:** Removed all "Tailwind-like" utility classes and inline styles from `VirtualListView` and `CommonMetadata`, moving them to semantic classes in `.css` files.
- **Safe UI Access:** Implemented optional chaining and fallback logic in UI components to handle null metadata gracefully during the transition period.
- **Dynamic Layout Memory:** The application now remembers the exact width of the sidebar/inspector and the height of sidebar sections (Folders, Tags) between sessions.

---

## 4. Pending Items & Next Steps

### Pending
- **Bulk Metadata Update:** Ensure that when multiple items are selected, changing the rating in the inspector updates all items (currently works for single selection).
- **Format Backfill:** Existing images in the DB might still have an empty `format` field until they are re-scanned or updated. A manual "Refresh Metadata" command could be useful.

### Next Steps
1. **Context Menu Extension:** Add sorting options directly to the main viewport context menu for easier access without the toolbar.
2. **Search Integration:** Ensure the search query results also respect the global sorting and persistence.
3. **Performance Profiling:** Monitor the performance of `ORDER BY` and `COLLATE NOCASE` with libraries of 10,000+ items (indexing may be required for specific columns).

---

## 5. Files Modified
- `src-tauri/src/db_tags.rs` (Sorting logic & SQL)
- `src-tauri/src/database.rs` (Migrations & Save logic)
- `src-tauri/src/indexer/metadata.rs` (Metadata extraction)
- `src/core/store/filterStore.ts` (State & Persistence)
- `src/core/store/libraryStore.ts` (API Refresh logic)
- `src/components/features/viewport/VirtualListView.tsx` (UI & Clean CSS)
- `src/components/features/viewport/list-view.css` (New semantic styles)
- `src/components/features/inspector/CommonMetadata.tsx` (Inspector UI)
- `src/components/features/inspector/inspector.css` (New semantic styles)
- `src/layouts/AppShell.tsx` (Layout persistence)
- `src/components/layout/LibrarySidebar.tsx` (Layout persistence)
