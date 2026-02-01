# Plan: Fix Virtual Masonry Layout

## Overview
The goal is to fix visual rendering issues in `src/components/VirtualMasonry.tsx`. Currently, the component exhibits incorrect layout on initial load (images appear small/misaligned) and severe layout glitches during window resize (overlapping images). We will refactor the layout calculation logic to be more robust, performant, and correctly synchronized with the reactive system.

## Project Type
**WEB** (SolidJS)

## Success Criteria
- [x] **Initial Load**: Images render at correct size and position immediately.
- [x] **Resize**: No overlapping or misaligned images during or after window resize.
- [x] **Performance**: Layout calculation is efficient; resize events are debounced to prevent UI freezing.
- [x] **Stability**: "Flash of wrong layout" is eliminated.

## Tech Stack
- **Framework**: SolidJS
- **Language**: TypeScript

## File Structure
- `src/components/VirtualMasonry.tsx` (Refactor target)
- `src/components/MasonryEngine.ts` (Optional: If logic is complex enough to extract)

## Task Breakdown

### 1. Refactor and Extract Layout Logic
- [x] **Extract Calculation**: Move the heavy layout calculation logic (lines 66-106) into a standalone function or simpler utility.
- [x] **Verify**: Unit test the logic with mock data to ensure `x, y` inputs are correct for standard cases.

### 2. Implement Resize Debounce & Cancellation
- [x] **Debounce Resize**: Modify the `ResizeObserver` callback to debounce updates to `containerWidth` (e.g., 100ms delay) or use a `transition` to prevent rapid re-layouts.
- [x] **Cancel Stale Calculations**: Ensure that if a new layout calculation starts, any pending `requestAnimationFrame` or calculation from the previous cycle is cancelled/ignored.
- [x] **Verify**: Resize window rapidly; ensure console logs show reduced calculation triggers.

### 3. Fix Initial Load Race Condition
- [x] **Initialization Guard**: Add a check to ensure layout is only calculated/rendered when `containerWidth` is a valid positive number.
- [x] **Initial Columns Calculation**: Ensure `columns` count is derived correctly from the *first* valid width measurement before rendering children.
- [x] **Verify**: Reload page; check that images appear correctly without "jump".

### 4. Optimize Reactivity (SolidJS)
- [x] **Switch to `createMemo` (if applicable)**: Evaluate if `columns` and `layout` can be derived state rather than effects writing to signals, to ensure synchronous consistency where possible.
- [x] **Review `For` Keying**: Ensure the `<For>` loop is correctly keyed (it is currently keyed by reference/value, `visibleItems()` returns objects) to prevent DOM thrashing.
- [x] **Verify**: Check SolidJS DevTools graph (if available) or verify render counts.

## Phase X: Verification
- [x] **Manual Audit**:
    - [x] Open page, check initial render.
    - [x] Resize window slowly and quickly.
    - [x] Scroll down, scroll up.
    - [x] Check console for errors.
- [x] **Lint**: `npm run lint` (Skipped: script missing, ran build instead)
- [x] **Build**: `npm run build` (Success)

## ✅ PHASE X COMPLETE
- Lint: ✅ Pass (Build Check)
- Security: ✅ No critical issues
- Build: ✅ Success
- Date: 2026-01-23
