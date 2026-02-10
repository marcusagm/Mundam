# Database Layer Refactoring - Execution Plan

**Date:** 2026-02-10
**Time:** 15:31
**Status:** ✅ Completed

## 1. Objective
Refactor the database layer to improve maintainability, resolve circular dependencies, and standardize data access patterns. The goal was to move from a flat structure of unrelated files to a cohesive `db` module with domain-specific submodules.

## 2. Execution Steps

### Phase 1: Infrastructure & Models
1.  **Module Creation:** Established the `src-tauri/src/db/` directory.
2.  **Core Initialization (`db/mod.rs`):**
    *   Moved the `Db` struct and its `new` (connection/migration logic) and `run_maintenance` functions.
    *   Initialized the module tree (`pub mod models`, `pub mod images`, etc.).
3.  **Centralized Models (`db/models.rs`):**
    *   Consolidated `ImageMetadata`, `Tag`, `LibraryStats`, `SmartFolder`, `TagCount`, and `FolderCount` into a single source of truth.
    *   Adjusted field types (e.g., `created_at` as `DateTime<Utc>`) to match database outputs.

### Phase 2: Domain Logic Migration
1.  **Folders (`db/folders.rs`):**
    *   Migrated hierarchy management, recursive counts, and renaming logic.
    *   Implemented non-null aliases (`id as "id!"`) for SQLx compile-time safety.
2.  **Images (`db/images.rs`):**
    *   Refactored image CRUD and metadata updates.
    *   Integrated with the new `ImageMetadata` model.
3.  **Tags (`db/tags.rs`):**
    *   Moved tag creation, association, and library statistics aggregation.
4.  **Search Logic (`db/search.rs`):**
    *   Centralized dynamic search query building using `sqlx::QueryBuilder`.
    *   Fixed lifetime issues in advanced search criteria parsing.
5.  **Settings & Smart Folders:**
    *   Created `db/settings.rs` and `db/smart_folders.rs` for dedicated persistence logic.

### Phase 3: Integration & Cleanup
1.  **Indexer Update:** Modified `src-tauri/src/indexer/mod.rs` and `metadata.rs` to use the new models and module paths.
2.  **Command Handlers:** Updated imports in:
    *   `tag_commands.rs`, `location_commands.rs`, `smart_folder_commands.rs`, `settings_commands.rs`, `thumbnail_commands.rs`.
3.  **File Cleanup:** Physically removed legacy files:
    *   `database.rs`, `db_tags.rs`, `db_smart_folders.rs`, `db_settings.rs`, `search_logic.rs`, `database_extension.rs`.

### Phase 4: Bug Fixing & Stabilization
1.  **Type Safety:** Resolved several mismatched type errors in SQLx macros by adding explicit nullability indicators (`!`) and type overrides (`"created_at!: DateTime<Utc>"`).
2.  **Serialization Correction:**
    *   **Issue:** `#[serde(rename_all = "camelCase")]` was causing frontend `TypeError` because it couldn't find `tag_counts` (received `tagCounts`).
    *   **Fix:** Removed global camelCase renaming in `db/models.rs` to maintain compatibility with existing snake_case frontend expectations.
3.  **Chrono Mapping:** Ensured `chrono` types are correctly mapped for `sqlx` 0.8 compatibility.

## 3. Results
*   **Modular Architecture:** Each database domain is isolated and clear.
*   **Zero Duplication:** Models are shared across the system without circular imports.
*   **Compile-time Safety:** Queries are now more strictly checked by SQLx.
*   **Stabilized Frontend:** Library statistics and search results load correctly following case-sensitivity fixes.

## 4. Next Steps
*   Continue monitoring for any missing command updates.
*   Update documentation in `/docs/guidelines/backend-rust.md` to reflect the new `db` module structure as the new project standard.
