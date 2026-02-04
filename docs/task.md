# Project Analysis Task List

- [x] **Initial Exploration**
    - [x] Analyze project root and configuration (`package.json`, `Cargo.toml`, `README.md`).
    - [x] Analyze Backend Architecture (`src-tauri`).
        - [x] Database schema and interaction.
        - [x] Command handlers and concurrency.
        - [x] File system operations and performance.
    - [x] Analyze Frontend Architecture (`src`).
        - [x] Component structure and modularity.
        - [x] State management (Context, Stores).
        - [x] Performance (Virtualization, re-renders).
        - [x] CSS/Styling approach.
- [x] **Detailed Code Review**
    - [x] Identify code duplication.
    - [x] Check for hardcoded values vs tokens.
    - [x] Review error handling and logging.
- [x] **Feature Gap Analysis**
    - [x] Compare current implementation with inferred "Library" requirements.
    - [x] Identify missing standard features (e.g., specific file support, batch actions).
- [x] **Report Generation**
    - [x] Compile findings into a comprehensive report.
    - [x] Suggest architectural improvements.
    - [x] Recommend performance optimizations.

# Configuration Refactoring Task List

- [x] **Frontend Configuration**
    - [x] Create `src/config/constants.ts`.
    - [x] Refactor `libraryStore.ts` to use centralized constants.
- [x] **Backend Configuration**
    - [x] Create `src-tauri/src/config.rs`.
    - [x] Integrate config loading in `lib.rs`.
    - [x] Update `thumbnail_worker.rs` to use dynamic thread count.
- [x] **Settings UI Refactoring**
    - [x] Create `SectionGroup` reusable component (`src/components/ui/SectionGroup.tsx`).
    - [x] Remove inline styles from `GeneralPanel.tsx` and create `general-panel.css`.
    - [x] Update `GeneralPanel.tsx` to use `SectionGroup` and standard UI components (`Select`).
    - [x] Refactor `KeyboardShortcutsPanel.tsx` to use `SectionGroup`.
    - [x] Clean up `keyboard-shortcuts-panel.css`.
- [ ] **Validation**
    - [ ] Restart app and verify Settings persistence.
    - [ ] Verify UI visual consistency.
