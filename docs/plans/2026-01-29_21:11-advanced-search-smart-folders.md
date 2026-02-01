# Implementation Plan: Advanced Search and Smart Folders

## Overview
This plan documents the implementation of an advanced search system for Elleven Library, featuring a complex query builder with nested criteria and "Smart Folders" for persisting and quickly accessing saved searches.

## Project Type
**WEB** (SolidJS Frontend + Tauri/Rust Backend)

## Success Criteria
- [x] Functional Advanced Search Modal with field, operator, and value selection.
- [x] Support for logical operators (AND/OR) at the group level.
- [x] Smart Folder persistence in SQLite database.
- [x] Smart Folders listed and manageable in the Library Sidebar.
- [x] Active filters indicator with popover to view and remove individual filters.
- [x] Successful cross-layer integration (Frontend -> Tauri IPC -> Rust Logic -> SQL).
- [x] Robust Form Validation for criteria entry.
- [x] In-line Editing of existing criteria in the Query Editor.

## Tech Stack
- **Frontend**: SolidJS, Lucide Icons, Vanilla CSS (Variables).
- **State Management**: SolidJS Stores (`filterStore`, `metadataStore`).
- **Backend**: Rust, Tauri, SQLx (SQLite).
- **UI Components**: Custom `Modal`, `Select`, `Popover`, `MaskedInput`, `Tooltip`.

## File Structure
### Frontend
- `src/components/features/search/`
    - `AdvancedSearchModal.tsx` (Core query builder UI)
    - `SearchToolbar.tsx` (Integrated search bar + active filters)
    - `SmartFoldersSidebarPanel.tsx` (Sidebar management for smart folders)
    - `advanced-search-modal.css`, `search-toolbar.css`, `smart-folders.css`
- `src/components/ui/`
    - `Popover.tsx` (New multi-purpose popover component)
    - `MaskedInput.tsx` (Masked text input for dates/formatted fields)
- `src/core/store/`
    - `filterStore.ts` (Updated for `advancedSearch` state)
    - `metadataStore.ts` (Updated for `smartFolders` sync)
- `src/core/hooks/`
    - `useFilters.ts`, `useMetadata.ts` (Exposing new states and actions)

### Backend (Rust)
- `src-tauri/src/`
    - `search_logic.rs` (Translates JSON query to SQL WHERE clause)
    - `db_smart_folders.rs` (DB operations for smart folders)
    - `smart_folder_commands.rs` (Tauri IPC commands)
    - `db_tags.rs` (Enhanced `get_images_filtered` with dynamic WHERE)
    - `tag_commands.rs` (IPC bridge for enhanced search)
    - `schema.sql` (Added `smart_folders` table)

## Task Breakdown

### Phase 1: Foundation & Data Structures
- [x] **Database Schema**: Add `smart_folders` table to `schema.sql`.
- [x] **Frontend State**: Extend `filterStore` with `SearchGroup` and `SearchCriterion` types.
- [x] **Metadata Store**: Add smart folder loading/saving actions and state.

### Phase 2: Core UI Components
- [x] **MaskedInput**: Implement generic masking logic for date fields.
- [x] **Popover**: Create a generic popover for active filters list.
- [x] **TreeView Enhancement**: Modify `TreeView.tsx` to support `draggable={false}` for simplified list views.

### Phase 3: Advanced Search UI
- [x] **AdvancedSearchModal**: Build the criteria builder row by row.
- [x] **Select Components**: Integrate tags and folders with hierarchical indentation.
- [x] **Query Editor**: List added criteria and allow removal.
- [x] **Search Action**: Connect modal to `filterStore` and refresh the library.

### Phase 4: Smart Folders & Sidebar
- [x] **Smart Folders Feature**: Add "Save Smart Folder" flow within the search modal.
- [x] **SmartFoldersSidebarPanel**: Create the sidebar panel with delete and select functionality.
- [x] **Sidebar Integration**: Add the new panel to the main `LibrarySidebar` layout.

### Phase 5: Backend Implementation (Rust)
- [x] **SQL Translation**: Implement `build_where_clause` in `search_logic.rs` to handle nested criteria.
- [x] **DB Connectivity**: Implement CRUD for smart folders in `db_smart_folders.rs`.
- [x] **IPC Layer**: Register new commands and update `get_images_filtered` signature.

### Phase 6: Refinement
- [x] **Build Fixes**: Resolve Rust compilation errors in `search_logic.rs` (lifetime and match arm fixes).
- [x] **Validation**: Ensure dynamic SQL generation correctly integrates with the main filtering pipeline.
- [x] **UI Polish**: Verify active filters popover interaction and search execution.


## Extra-Planned Improvements & UX Refinement
Beyond the initial scope, the following enhancements were implemented to provide a premium experience:

### 1. Advanced Validation System
- **Real-time Feedback**: Implemented a `validationErrors` signal to track and display errors for each input field.
- **Context-Aware Rules**: Required field checks, date format validation (using Regex), and "between" range completeness.
- **UI Integration**: Updated `Input`, `Select`, and `MaskedInput` components to support `error` and `errorMessage` props with dedicated CSS styling.

### 2. In-line Criterion Editing
- **Functional Workflow**: Added the ability to edit existing criteria directly in the list (Query Editor) without removing them.
- **Dynamic Field Injection**: The editing mode automatically injects the correct input type (Date, Number, Select, etc.) based on the field type.
- **Auto-Conversion**: Handles MB-to-Bytes conversion automatically during editing for file size fields.

### 3. Progressive Disclosure & Help System
- **Tooltips**: Integrated a `Tooltip` component to explain the "Criteria Builder", "Query Editor", and "Match Mode" sections.
- **Logical Clarity**: Updated Match Mode labels to "Any (OR)" and "All (AND)" to assist users unfamiliar with boolean logic.

### 4. Layout Reorganization
- **Flow Optimization**: Moved "Match Mode" from the modal footer to the main content area (below the criteria list) for better cognitive flow.
- **Flexible UI**: Refactored CSS to use Flexbox instead of rigid Grids for criteria items, preventing layout breaks with long values.
- **Destructive Actions**: Implemented `ghost-destructive` button variants for deletion tasks to prevent accidental clicks while maintaining a clean look.

## Pending & Next Steps

### 🛠️ Functionality
- [ ] **Inline Editing Validation**: Add validation logic to the confirm-edit action (currently it trusts the input).
- [ ] **Complex Nesting**: While the model supports it, the UI doesn't yet allow creating groups within groups.
- [ ] **Recursive Filter Toggle**: Integrate the backend's recursive folder search with the Advanced Search "Folder" criterion.

### 🎨 UI/UX
- [ ] **Empty State Guidance**: Improve the "Empty Query" illustration with a call-to-action button or arrow.
- [ ] **Drag and Drop**: Allow reordering criteria in the Query Editor (Low Priority).
- [ ] **Feedback Animations**: Add subtle transitions when adding/removing criteria from the list.

### 🧪 Quality Assurance
- [x] **Lint & Type Check**: `npm run lint && npx tsc --noEmit` (Fixed initial reference errors).
- [x] **Backend Compilation**: `cargo check` (Success).
- [ ] **Playwright E2E**: Create a test suite for complex query combinations.

## Phase X: Verification
- [x] **Lint & Type Check**: Resolved `cn` reference errors and `Select` prop mismatches.
- [x] **Backend Compilation**: `cargo check` (Success)
- [ ] **Security Scan**: `python .agent/skills/vulnerability-scanner/scripts/security_scan.py .`
- [ ] **UI Audit**: `python .agent/skills/frontend-design/scripts/ux_audit.py .`
- [ ] **Final Build**: `npm run tauri build -- --debug`
