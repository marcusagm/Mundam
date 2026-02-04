# Initial Configuration Refactor Report
**Date:** 2026-02-04 00:50

## Overview
This report documents the initial refactoring of the application configuration and the Settings UI. The goal was to centralize configuration constants, enable dynamic performance settings for the backend, and standardizing the UI components in the settings panels.

## Completed Tasks

### 1. Configuration Centralization
*   **Frontend**: Created `src/config/constants.ts` to hold global constants like `BATCH_SIZE`. Refactored `src/core/store/libraryStore.ts` to use these constants.
*   **Backend**: Created `src-tauri/src/config.rs` to handle loading configuration from the database (`app_settings` table). Updated `src-tauri/src/lib.rs` and `src-tauri/src/thumbnail_worker.rs` to initialize and use this configuration, specifically allowing dynamic adjustment of thumbnail generation threads.

### 2. Settings UI Refactoring
*   **Component Standardization**: created a new reusable component `SectionGroup` (`src/components/ui/SectionGroup.tsx`) to standardize the layout of settings sections (Title + Content), ensuring consistency across panels.
*   **Keyboard Shortcuts Panel**: Refactored `KeyboardShortcutsPanel.tsx` to use `SectionGroup`. Moved specific styles out of `keyboard-shortcuts-panel.css` that were related to the section structure, now handled by `section-group.css`.
*   **General Panel**: 
    *   Refactored `GeneralPanel.tsx` to remove all inline styles and Tailwind-like classes.
    *   Created `src/components/features/settings/general-panel.css` for dedicated styling.
    *   Integrated `SectionGroup` for layout.
    *   Replaced the native HTML `<select>` with the project's standard `Select` component from `src/components/ui/Select.tsx`.
    *   Added a "Performance" section to allow users to configure the number of thumbnail worker threads.

### 3. Artifacts Updated
*   `implementation_plan.md` -> Saved as `implementation_plan.md.resolved`
*   `task.md` -> Saved and updated as `task.md.resolved`

## Pending Validation
*   Restart the application to ensure settings are correctly saved to the SQLite database and loaded by the Rust backend on startup.
*   Verify visual consistency of the new `SectionGroup` across both `GeneralPanel` and `KeyboardShortcutsPanel`.



---


# Implementação: Centralização de Configurações e UI de Preferências

## Objetivo
Centralizar configurações "hardcoded" espalhadas pelo código (ex: `BATCH_SIZE`, limites de threads) em módulos dedicados e expor opções relevantes para o usuário no painel de configurações (`GeneralPanel.tsx`).

## Hardcoded Configs Identificados
1.  **Frontend (`src/core/store/libraryStore.ts`)**:
    *   `BATCH_SIZE = 100`: Tamanho do lote de carregamento de imagens.
2.  **Backend (`src-tauri/src/thumbnail_worker.rs`)**:
    *   `num_threads(2)`: Número de threads para geração de thumbnails.
    *   `get_images_needing_thumbnails(6)`: Tamanho do lote de processamento do worker.
    *   `sleep(Duration::from_secs(2))`: Tempo de espera quando ocioso.
3.  **Backend (`src-tauri/src/search_logic.rs`)**:
    *   Limites de recursividade (ex: `depth < 50`).

## Arquitetura Proposta

### 1. Backend: `Startup Config` não persistente
Para configurações de baixo nível que afetam a inicialização (como threads), usaremos variáveis de ambiente ou um arquivo `config.rs` com valores padrão, mas permitindo override via banco de dados (`app_settings`) na inicialização.

*   **Novo Módulo**: `src-tauri/src/config.rs`
    *   Struct `AppConfig` com campos como `thumbnail_threads`, `indexer_batch_size`.
    *   Carregamento: Padrão -> Override do DB.

### 2. Frontend: `Constants` e `SettingsStore`
*   **Arquivo de Constantes**: `src/config/constants.ts` para valores imutáveis de desenvolvimento (ex: timeouts de API, endpoints).
*   **Persistent Settings**: Usar a tabela `app_settings` via `db_settings.rs` para configurações do usuário.
*   **Store**: Atualizar `systemStore.ts` ou criar `settingsStore.ts` para cachear essas preferências no frontend.

## Passo a Passo de Implementação

### Passo 1: Centralização Frontend
1.  Criar `src/config/constants.ts`.
2.  Mover `BATCH_SIZE` de `libraryStore.ts` para o novo arquivo.
3.  Refatorar `libraryStore.ts` para importar de `constants.ts`.

### Passo 2: Backend Config & Threads Dinâmicas
1.  Criar `src-tauri/src/config.rs` com defaults.
2.  Ler configurações do DB na inicialização do `main.rs` ou `lib.rs` e armazenar no `State`.
3.  Atualizar `thumbnail_worker.rs` para ler `num_threads` do `State` ou de uma configuração passada no construtor.
    *   Isso permitirá que o usuário altere a performance no futuro sem recompilar.

### Passo 3: UI de Configurações (`GeneralPanel.tsx`)
Adicionar controles para:
1.  **Performance de Indexação**: Slider ou Select para "Uso de CPU" (Baixo, Médio, Alto) -> controla threads do Worker.
    *   *Nota*: Alterar threads do worker pode exigir reinicialização do app ou do worker. Para simplificar nesta iteração, salvaremos no DB e pediremos reinício.
2.  **Visualização**: Checkbox para "Mostrar animações de carregamento" ou similar (se houver).

## Plano de Verificação
1.  **Build**: Verificar se frontend e backend compilam.
2.  **Runtime**: Alterar `BATCH_SIZE` no código e verificar se a rolagem infinita adapta a frequência de requests.
3.  **UI Settings**: Alterar configuração simulada e verificar persistência no SQLite (`select * from app_settings`).



---

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
    - [x] Restart app and verify Settings persistence.
    - [x] Verify UI visual consistency.
