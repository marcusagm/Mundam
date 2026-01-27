# Task: Refining UI Layouts

## Objective
Refactor and improve the design, accessibility, and component structure of the Elleven Library application. Move inline styles to dedicated CSS files, ensure design token usage, and improve component composition.

## Completed Tasks

### 1. CSS Refactoring & Design Tokens
- [x] **AppShell**: Converted to `app-shell.css` using grid layout classes.
- [x] **PrimaryHeader**: Refactored to `primary-header.css`.
- [x] **GlobalStatusbar**: Refactored to `global-statusbar.css`.
- [x] **LibrarySidebar**: Refactored to `library-sidebar.css` and sub-panels.
- [x] **Sidebar Panels**: Updated `LibrarySidebarPanel`, `FoldersSidebarPanel`, `TagTreeSidebarPanel` to use shared CSS utility classes (`panel-fixed`, `panel-fluid`, etc.).
- [x] **Accordion**: Improved icon styling with `.accordion-icon-wrapper`.
- [x] **Viewport**: Created `viewport.css` for `VirtualMasonry` and `AssetCard`.

### 2. Accessibility Improvements
- [x] **TreeView**: Added ARIA roles (`tree`, `treeitem`, `group`) and `aria-selected`/`aria-expanded` attributes.
- [x] **TagInput**: Fixed dropdown clipping using `@floating-ui/dom` and Portal.
- [x] **ContextMenu**: Improved markup and styling.

### 3. Code Cleanup & Bug Fixes
- [x] **ImageGrid**: Removed unused `ImageGrid.tsx` component.
- [x] **Tauri Permissions**: Added `allow-get-image-exif` to `permissions/main.toml`.
- [x] **AppStore**: Fixed `setRootLocation` locations update issue.
- [x] **ColorPicker**: Fixed React-style `style={{ alignItems }}` syntax error.
- [x] **Linting**: Fixed unused imports and variables in `ContextMenu.tsx` and `ColorPicker.tsx`.

## Verification
- Code builds successfully (`tsc --noEmit`).
- Components use design tokens from `tokens.css`.
- UI is responsive and layout issues (clipping, scrolling) are resolved.

## Status
**COMPLETED** - The UI codebase has been significantly improved for maintainability and consistency.
