# Plan: UI/UX Polish & Design System Implementation

## Overview
Implement a professional, cohesive "Dark Modern" design system inspired by professional tools (Eagle.cool, VS Code). Fix alignment issues, improve spacing, and establish a high-quality visual hierarchy. Use `lucide-solid` for consistent iconography.

## Project Type
**WEB** (SolidJS)

## Design Pillars
1.  **Professional Density**: High information density but with sufficient whitespace (8pt grid).
2.  **Left Alignment**: Sidebar and lists must be left-aligned for readability (F-pattern).
3.  **Subtle Hierarchy**: Use font weights and colors (Grays) for hierarchy, reserving Accents (Blue/Teal) for active states.
4.  **Feedback**: Clear hover states for all interactive elements.

## Success Criteria
- [ ] **Icons**: `lucide-solid` installed and implemented in Sidebar/Header.
- [ ] **Sidebar**: Left-aligned, proper padding (12px), hover effects, distinct active state.
- [ ] **Header**: Integrated look (no floating box), unified background, unobtrusive search.
- [ ] **Inspector**: Clean empty state, labeled fields, aligned layout.
- [ ] **Status Bar**: Subtle typography (11px), distraction-free.
- [ ] **Global CSS**: Defined CSS Variables for specific semantic roles (surface-hover, border-subtle).

## Task Breakdown

### Phase 1: Foundation & Assets
- [x] **Install Icons**: `npm install lucide-solid`
- [x] **Update Design Tokens (`tokens.css`)**:
    - Define semantic palette: `--bg-sidebar`, `--bg-main`, `--item-hover`, `--item-active`, `--text-muted`.
    - Set Typography: `Inter` (if available) or system-ui with tight tracking.

### Phase 2: Component Polish
- [x] **LibrarySidebar**:
    - Align items left.
    - Add icons for "All Items", "Uncategorized", "Trash", "Folders".
    - Implement `nav-item` CSS class with hover/active states.
- [x] **PrimaryHeader**:
    - Remove heavy borders.
    - Style Search Input to blend in (darker bg, subtle border).
    - Fix Navigation buttons sizing.
- [x] **FileInspector**:
    - Create a "Empty State" illustration/icon.
    - Style form fields (Inputs) to be minimal (no massive focus rings).
    - Align labels and values.
- [x] **AssetCard**:
    - Improve selection ring (blue/primary color border).
    - Add subtle shadow on hover.

### Phase 3: Global Cleanup
- [x] **AppShell**: Ensure borders between panes are subtle (`1px solid var(--border-subtle)`).
- [x] **Global CSS**: Remove conflicting legacy styles.

## Phase X: Verification
- [x] **Visual Audit**: Verify alignment in Sidebar and Inspector.
- [x] **Build Check**: `npm run build`.

## ✅ Completion Report
- **Date**: 2026-01-25
- **Status**: Implemented Professional Theme.
- **Notes**: 
    - Migrated to "Pro Dark" palette (VS Code / Eagle style).
    - Implemented `lucide-solid` icons throughout.
    - Cleaned up centered alignment in Sidebar.
    - Added empty states and better field styling in Inspector.

