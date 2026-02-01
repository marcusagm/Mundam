# Plan: Taxonomy Tree & File Organization

> **Goal:** Implement a complete tag management system (Taxonomy) with hierarchical tree view, drag-and-drop organization, and batch tagging capabilities, enabling management of massive image collections.

## 📋 Project Context
- **Type:** Desktop App (Tauri + SolidJS + Rust)
- **Focus:** Frontend UI/UX (Complex TreeView, DnD) & Backend Logic (Recursive DB queries)
- **Reference:** `docs/idea/features/File organization and Taxonomy Tree.md`

## 🏗️ Technical Architecture Decisions
- **State Management:** `appStore.ts` (Global store for tags and selection)
- **UI Components:**
  - `TreeView`: Recursive component pattern for infinite nesting.
  - `ContextMenu`: Portal-based floating menu for accessibility and z-index management.
  - `TagInput`: Multi-value input with fuzzy search autocomplete.
- **Drag & Drop:** 
  - Strategy: Use `@thisbeyond/solid-dnd` or Native HTML5 DnD API (to be confirmed in Phase 1) for performance and cross-window capabilities.
- **Database:** SQLite (Rust) with `adjacency list` pattern for hierarchy (`parent_id`).

## 🗓️ Task Breakdown

### Phase 1: Foundation & Backend (Rust)
- [x] **1.1 DB Schema Verification** 
    - Check `tags` table for `parent_id` and `color` columns.
    - **Update:** Added `order_index` column for persistent sorting.
    - **Update:** Implemented auto-patching in `initDb` to safely add columns (`thumbnail_path`, `format`, `order_index`) to existing databases.
- [x] **1.2 Rust Command: Tag CRUD**
    - Implement `create_tag`, `rename_tag`, `delete_tag` (handle orphans), `update_tag_color`.
    - **Verify:** Tauri Invoke returns success/error for each.
- [x] **1.3 Rust Command: Hierarchy Management**
    - Implement `move_tag` (update `parent_id`).
    - **Update:** Backend `update_tag` modified to handle `Option<i64>` where `0` acts as a sentinel value for `NULL` (setting root).
    - **Verify:** Moving Child to New Parent updates DB correctly.
- [x] **1.4 Rust Command: Batch Operations**
    - Implement `add_tags_to_images` (bulk insert).
    - Implement `remove_tags_from_images`.
    - **Verify:** Assigning tag to 50 images updates `image_tags` table count by 50.

### Phase 2: Core UI Components (Design System)
- [x] **2.1 Component: `ContextMenu`**
    - Create reusable Portal-based context menu.
    - Props: `items: { label, icon, action }[]`, `position: {x, y}`.
    - **Verify:** Right-click anywhere opens menu at cursor; clicking outside closes it.
- [x] **2.2 Component: `TagInput`**
    - Create input with "chip" display for selected tags.
    - Implement dropdown suggestions with fuzzy match.
    - **Verify:** Typing "an" suggests "Animals", "Anime"; Enter adds tag.
    - *Status: Implemented in `src/components/ui/TagInput.tsx` and integrated in `FileInspector`.*
- [x] **2.3 Component: `TreeView` (Base)**
    - Implement `TreeViewItem` and `TreeView` container.
    - Support `expanded/collapsed` state.
    - Support `recursive` rendering for children.
    - **Verify:** Hardcoded nested data renders correctly as a tree.

### Phase 3: Taxonomy Feature Implementation
- [x] **3.1 Sidebar Integration**
    - Connect `LibrarySidebar` to `state.tags`.
    - Render `TreeView` with real data.
    - **Verify:** Sidebar shows actual tags from DB.
- [x] **3.2 Tag Context Actions**
    - Attach `ContextMenu` to tree items: Rename, Change Color, Add Subtag, Delete.
    - Connect to Rust commands.
    - **Verify:** "Rename" updates text in UI instantly; "Color" changes badge color.
- [x] **3.3 Filtering Logic**
    - Update `appStore` to handle `selectedTags` filter.
    - Implement "Filter by Tag" query in Rust (`get_images_filtered`).
    - **Verify:** Clicking "Animals" tag shows only animal images in grid.

### Phase 4: Advanced Interactions (Drag & Drop)
- [x] **4.1 DnD: User Testing & Proto**
    - Prototype DnD library in small scope to ensure `SolidJS` compatibility.
    - **Result:** Implemented custom Drag & Drop system compatible with Tauri.
- [x] **4.2 DnD: Tag Reorganization (Tree)**
    - Allow dragging Tag A onto Tag B to set parent.
    - **Update:** Implemented `order_index` reordering logic in `TagDropStrategy`.
    - **Update:** Added visual Drop Lines with dynamic indentation (`left` calculated by depth) to show exact drop level.
- [x] **4.3 DnD: Image to Tag**
    - Allow dragging images from Grid to Sidebar/Tag.
    - **Update:** Implemented `.drop-target` visual feedback (background highlight + outline) when dragging images over tags.
    - **Fix:** Used custom `thumb://` protocol for drag ghost images to bypass Tauri restrictions.
- [x] **4.4 Selection Sync**
    - Implement modifier keys (Shift/Ctrl) selection in `ImageGrid`.
    - Sync selection to `FileInspector`.
    - **Verify:** Selecting 5 files shows "5 items selected" and common tags in Inspector.

## ✅ Phase X: Verification & Quality Assurance
- [x] **Lint & Type Check:** `npm run type-check`
- [x] **Performance Test:** Generate 1,000 tags and scroll sidebar (Target: 60fps).
- [ ] **UX Audit:**
    - [x] Tags have sufficient contrast colors.
    - [x] Hover states are clear on TreeView.
    - [ ] "Uncategorized" folder logic works (shows images with count(tags)=0).
- [x] **Manual Test:** Create 3-level depth tag -> Assign to Image -> Filter by Top-level tag -> Image appears.

## 📝 Implementation Notes & Improvements

### Database Schema Updates
We significantly enhanced the database schema during implementation:
- **`order_index` (Tags):** Added an integer column to persist the sort order of tags, enabling manual reordering via Drag & Drop.
- **`format` (Images):** Added to store image format metadata (e.g., JPEG, PNG).
- **`thumbnail_path` (Images):** Added to cache thumbnail locations.
- **Auto-Patching:** Implemented a robust `initDb` routine that uses `ALTER TABLE` commands within `try-catch` blocks to ensure existing databases are automatically updated with new columns on startup, removing the need for manual migrations during development.

### Visual Hierarchy & UX
- **Dynamic Drop Lines:** Instead of a generic full-width drop line, we implemented dynamically positioned lines. The `left` CSS property is calculated based on the target depth (`depth * 16 + 4`px), providing clear visual cues about whether an item will be dropped as a sibling or a child.
- **Root Handling:** We removed the explicit "Drop to make Root" placeholder for a cleaner UI. Instead, the system now intelligently handles drops into the "empty space" or top-level container as root drops.
- **Image Drag Feedback:** Added specific feedback (`.drop-target` class) when dragging images over tags, distinct from the tag reordering feedback.

### Backend Enhancements
- **Null Handling:** The Rust backend's `Option<i64>` handling was refined. Since `None` usually implies "no update", we implemented a sentinel value (`0`) for `parent_id` to explicitly command the database to set the parent to `NULL` (making a tag a root item).
- **Filtering Logic:** The tag filtering was adjusted to default to "Single Select" (Exclusive) mode to prevent confusion where selecting multiple disjoint tags resulted in empty lists (AND logic).

### Pending Items
- **Fuzzy Search Tag Input:** A dedicated component for adding tags via text search is planned but not yet implemented. Currently, tags are added via the sidebar context menu.
- [x] **Uncategorized View:** The logic to display images with `COUNT(tags) = 0` needs final verification.

