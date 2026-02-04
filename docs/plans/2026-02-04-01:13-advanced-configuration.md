# Plan: Advanced Configuration & Optimization

> **Goal**: Implement high-value optimizations and architectural standardizations identified in the Analysis Report, focusing on performance (threading, recursion) and code quality (constants, date parsing).

## 1. Context & Objectives
Based on the `analysis-report.md` and user request, we will address four key areas:
1.  **Dynamic Threading (Backend)**: Automatically detect logical cores to optimize thumbnail generation on high-end machines.
2.  **Date Standardization (Full Stack)**: Remove fragile custom date parsing in Rust by enforcing ISO-8601 strings from the frontend.
3.  **Recursion Optimization (Frontend)**: Improve `isChildOf` performance in `libraryStore.ts` using a flat map lookup (O(1)) instead of tree traversal (O(N)).
4.  **Constants Integration (Frontend)**: Enforce usage of `APP_CONFIG` constants (`SEARCH_DEBOUNCE_MS`, `THUMBNAIL_SIZE`, `MAX_FOLDER_DEPTH`) throughout the specific components.

## 2. Proposed Changes

### 2.1 Backend: Dynamic Threading
-   **File**: `src-tauri/src/config.rs`
    -   Update `load_config` to check if `thumbnail_threads` is `0` (or missing).
    -   Use `std::thread::available_parallelism` to detect CPU cores.
    -   Set default to `max(1, available_cores / 2)` if auto-detect is enabled.
-   **File**: `src-tauri/src/thumbnail_worker.rs`
    -   Ensure the worker accepts the resolved thread count.

### 2.2 Backend: Date Parsing Simplification
-   **File**: `src-tauri/src/search_logic.rs`
    -   Refactor date parsing to expect standard RFC3339/ISO-8601 strings (e.g., `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM:SS`).
    -   Remove custom manual string filtering for `DD/MM/YYYY`.

### 2.3 Frontend: Constants Integration
-   **File**: `src/config/constants.ts` (Reference)
-   **Target Files**:
    -   `src/components/features/search/SearchBar.tsx` (Use `SEARCH_DEBOUNCE_MS`)
    -   `src/core/services/thumbnailService.ts` (or equivalent) (Use `THUMBNAIL_SIZE`)
    -   `src/core/functions/file-operations.ts` (or where recursion happens) (Use `MAX_FOLDER_DEPTH`)

### 2.4 Frontend: Performance Optimization
-   **File**: `src/core/store/libraryStore.ts`
    -   Analyze usage of `isChildOf`.
    -   Introduce a `folderPathMap` (ID -> Path) or flattened structure to allow checking ancestry without iterating the tree array.

## 3. Implementation Steps

### Phase 1: Backend Improvements
1.  [x] Modify `src-tauri/src/config.rs` to import `std::thread`.
2.  [x] Implement auto-detection logic in `load_config`.
3.  [x] Refactor `src-tauri/src/search_logic.rs` to strict ISO date parsing.

### Phase 2: Frontend Integration
4.  [x] Audit codebase for magic numbers matching `APP_CONFIG` values.
5.  [x] Replace occurrences with `APP_CONFIG` imports.
6.  [x] Verify Search Bar debounce implementation.

### Phase 3: Hot Path Optimization
7.  [x] Refactor `libraryStore.ts` to maintain a lookup map for folder paths/IDs.
8.  [x] Rewrite `isChildOf` to use the lookup map.

## 4. Verification Plan

### Automated
-   **Build Check**: `cargo check` and `npm run type-check`.

### Manual
1.  **Threading**: Start app with `thumbnail_threads` set to 0 (or default). Inspect logs to see calculated thread count.
2.  **Date Search**: Perform a date range search in the UI. Verify backend logs/results are correct with ISO format.
3.  **UI Responsiveness**: Test deep folder structures to ensure no UI freeze during updates (verifying `isChildOf` optimization).
