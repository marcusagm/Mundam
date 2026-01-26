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
- [ ] **1.1 DB Schema Verification** 
    - Check `tags` table for `parent_id` and `color` columns.
    - Run migration if missing.
    - **Verify:** `sqlite3 .data.db ".schema tags"` shows columns.
- [ ] **1.2 Rust Command: Tag CRUD**
    - Implement `create_tag`, `rename_tag`, `delete_tag` (handle orphans), `update_tag_color`.
    - **Verify:** Tauri Invoke returns success/error for each.
- [ ] **1.3 Rust Command: Hierarchy Management**
    - Implement `move_tag` (update `parent_id`).
    - Check for circular dependencies (A -> B -> A).
    - **Verify:** Moving Child to New Parent updates DB correctly.
- [ ] **1.4 Rust Command: Batch Operations**
    - Implement `add_tags_to_images` (bulk insert).
    - Implement `remove_tags_from_images`.
    - **Verify:** Assigning tag to 50 images updates `image_tags` table count by 50.

### Phase 2: Core UI Components (Design System)
- [ ] **2.1 Component: `ContextMenu`**
    - Create reusable Portal-based context menu.
    - Props: `items: { label, icon, action }[]`, `position: {x, y}`.
    - **Verify:** Right-click anywhere opens menu at cursor; clicking outside closes it.
- [ ] **2.2 Component: `TagInput`**
    - Create input with "chip" display for selected tags.
    - Implement dropdown suggestions with fuzzy match.
    - **Verify:** Typing "an" suggests "Animals", "Anime"; Enter adds tag.
- [ ] **2.3 Component: `TreeView` (Base)**
    - Implement `TreeViewItem` and `TreeView` container.
    - Support `expanded/collapsed` state.
    - Support `recursive` rendering for children.
    - **Verify:** Hardcoded nested data renders correctly as a tree.

### Phase 3: Taxonomy Feature Implementation
- [ ] **3.1 Sidebar Integration**
    - Connect `LibrarySidebar` to `state.tags`.
    - Render `TreeView` with real data.
    - **Verify:** Sidebar shows actual tags from DB.
- [ ] **3.2 Tag Context Actions**
    - Attach `ContextMenu` to tree items: Rename, Change Color, Add Subtag, Delete.
    - Connect to Rust commands.
    - **Verify:** "Rename" updates text in UI instantly; "Color" changes badge color.
- [ ] **3.3 Filtering Logic**
    - Update `appStore` to handle `selectedTags` filter.
    - Implement "Filter by Tag" query in Rust.
    - **Verify:** Clicking "Animals" tag shows only animal images in grid.

### Phase 4: Advanced Interactions (Drag & Drop)
- [ ] **4.1 DnD: User Testing & Proto**
    - Prototype DnD library in small scope to ensure `SolidJS` compatibility.
- [ ] **4.2 DnD: Tag Reorganization (Tree)**
    - Allow dragging Tag A onto Tag B to set parent.
    - **Verify:** Dragging "Dog" onto "Animals" nests it; DB updates `parent_id`.
- [ ] **4.3 DnD: Image to Tag**
    - Allow dragging images from Grid to Sidebar/Tag.
    - **Verify:** Dragging image to "Favorites" adds tag; Badge appears on image.
- [ ] **4.4 Selection Sync**
    - Implement modifier keys (Shift/Ctrl) selection in `ImageGrid`.
    - Sync selection to `FileInspector`.
    - **Verify:** Selecting 5 files shows "5 items selected" and common tags in Inspector.

## ✅ Phase X: Verification & Quality Assurance
- [ ] **Lint & Type Check:** `npm run type-check`
- [ ] **Performance Test:** Generate 1,000 tags and scroll sidebar (Target: 60fps).
- [ ] **UX Audit:**
    - [ ] Tags have sufficient contrast colors.
    - [ ] Hover states are clear on TreeView.
    - [ ] "Uncategorized" folder logic works (shows images with count(tags)=0).
- [ ] **Manual Test:** Create 3-level depth tag -> Assign to Image -> Filter by Top-level tag -> Image appears.

