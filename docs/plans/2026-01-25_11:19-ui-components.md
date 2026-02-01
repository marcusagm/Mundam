# Plan: UI Component Internalization & Standardization

## Overview
Extract reusable UI patterns currently embedded in layouts and feature components into a dedicated, accessible, and standardized Design System within `src/components/ui`. This ensures consistency, simplifies maintenance, and enforces accessibility best practices (WCAG 2.1 AA+).

## Project Type
**WEB** (SolidJS + Vanilla CSS)

## Core Principles
1.  **Accessibility First**: Native HTML5 semantics, proper ARIA labels, keyboard navigation (focus rings), and contrast compliance.
2.  **Variant-Driven**: Components must support visual variants (e.g., Button: `default`, `ghost`, `outline`, `destructive`) via props.
3.  **Composition**: Components should be composable (e.g., Input with slots for icons).
4.  **Zero-Runtime CSS**: Use CSS variables (`tokens.css`) and standardized class naming (BEM-ish or utility-based) without heavy CSS-in-JS runtimes.

## Proposed Components (`src/components/ui`)

| Component | Variants | Usage Location (Rewrite Target) |
|-----------|----------|---------------------------------|
| `Button` | Primary, Secondary, Ghost, Icon | Header, sidebar actions, inspector |
| `Input` | Default, Search (w/ icon slot) | PrimaryHeader, FileInspector |
| `Badge` | Default, Outline, Secondary | Sidebar (counts), Inspector (tags) |
| `Separator` | Horizontal, Vertical | AppShell, FileInspector |
| `Skeleton` | - | Loading states (Masonry, Sidebar) |
| `ScrollArea` | - | Replace native overflow for styled scrollbars |
| `Tooltip` | - | Sidebar icons (collapsed mode preparation) |
| `Table` | Virtualized, ARIA Grid | ListView, Metadata management |

## Tech Stack Strategy
- **Framework**: SolidJS
- **Styling**: Standard CSS with `clsx` or custom `cn` utility for class merging.
- **Icons**: continue using `lucide-solid`.

## Task Breakdown

### Phase 1: Infrastructure
- [x] **Utils**: Create `src/lib/utils.ts` (or `src/utils/classnames.ts`) with a class merging function (`cn`).
- [x] **Focus Styles**: Define a global focus ring token in `tokens.css` for consistent keyboard navigation.

### Phase 2: Core Primitives
- [x] **Implement Button**:
    - Support `variant` (primary, ghost, destructive).
    - Support `size` (sm, md, icon).
    - Auto-handle `disabled` and `loading` states.
- [x] **Implement Input**:
    - Wrapper for `input` with consistent styling.
    - Support `leftIcon` and `rightIcon` props (for Search).
- [x] **Implement Badge**:
    - For tags and counts (Sidebar/Inspector).
- [x] **Implement Separator**:
    - Simple semantic `<hr>` or `div` with styling.

### Phase 3: Advanced Components
- [x] **Implement Tooltip** (Optional but recommended for accessible icon-only buttons).
- [x] **Implement Dialog/Modal** (Base for future settings/confirmations).
- [x] **Implement Table**:
    - High-performance virtualization for thousands of items.
    - ARIA Grid Pattern (roles, indices, absolute-semantic hybrid).
    - Advanced Keyboard Navigation (Arrows, Home/End, Scroll-into-view).
    - Support for custom cells, sorting, and hidden columns.

### Phase 4: Refactoring Integration
- [x] **Refactor PrimaryHeader**: Replace raw `<button>` and `<input>` with `Button` and `Input`.
- [x] **Refactor LibrarySidebar**: Replace badges and buttons.
- [x] **Refactor FileInspector**: Use `Input` for readonly fields, `Badge` for tags.
- [x] **Refactor AssetCard**: Ensure focus states match the new system.

## Phase X: Verification
- [x] **A11y Audit**: Test all new components with keyboard (Tab/Enter/Space).
- [x] **Visual Consistency**: Ensure all inputs/buttons share height/padding metrics.
- [x] **Build Check**: `npm run build`.
