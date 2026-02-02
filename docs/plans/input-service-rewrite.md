# Input Service - Full SolidJS Rewrite

## Goal
Reescrever a biblioteca de input/shortcuts (`docs/Shortcuts`) usando arquitetura 100% SolidJS com TypeScript, signals e directives. Inclui integração com todos os componentes e UI de configuração.

---

## Architecture Overview

```
src/core/input/
├── index.ts                    # Public exports
├── types.ts                    # TypeScript types/interfaces
├── store/
│   ├── inputStore.ts           # Main reactive store (createStore)
│   └── shortcutStore.ts        # Shortcuts registry store
├── providers/
│   ├── KeyboardProvider.ts     # Keyboard events listener
│   ├── PointerProvider.ts      # Mouse/touch basic events
│   └── GestureProvider.ts      # Multi-touch gesture recognition
├── primitives/
│   ├── createShortcut.ts       # Main primitive for shortcuts
│   ├── createKeyState.ts       # Reactive key pressed state
│   ├── createGesture.ts        # Gesture detection primitive
│   └── createInputContext.ts   # Context provider
├── normalizer.ts               # Token normalization (pure utility)
├── dispatcher.ts               # Match logic (reactive)
└── context.tsx                 # InputProvider component

src/components/features/settings/
├── SettingsModal.tsx           # Main settings modal with sidebar
├── SettingsModal.css
├── KeyboardShortcutsPanel.tsx  # Shortcuts configuration panel
├── KeyboardShortcutsPanel.css
└── index.ts                    # Exports

src-tauri/src/
├── db_settings.rs              # Backend persistence logic
├── settings_commands.rs        # Tauri commands for settings
├── schema.sql                  # Database schema (added app_settings)
└── lib.rs                      # Module registration
```

**Scopes:**
- `global` (Interface geral: sidebars, header, etc.) - Priority: 0
- `image-viewer` (Viewer de imagens: zoom, navigation) - Priority: 50 [BLOCKING]
- `search` (Barra de busca e filtros) - Priority: 25
- `modal` (Modais ativos) - Priority: 100 [BLOCKING]

**Priority System:** Maior número = maior prioridade.
**Scope Blocking:** Scopes marcados como `BLOCKING` impedem que eventos cheguem a scopes de menor prioridade.

---

## Technical Decisions (Updated)

### 1. Scope Conflict Resolution & Blocking
- **Priority:** Scopes são organizados por prioridade.
- **Blocking:** Funcionalidade `blockLowerScopes` implementada.
  - Se um scope ativo tem `blockLowerScopes: true`, todos os atalhos de scopes com prioridade menor são ignorados.
  - Exemplo: Modal aberto (priority 100, blocking) impede atalhos globais (priority 0).
  - Exemplo: Viewer aberto (priority 50, blocking) impede atalhos globais.

### 2. Input Focus Handling
- Flag `ignoreInputs: boolean` configurável por shortcut (default: `true`).
- Se `ignoreInputs: true` e foco em `<input>`/`<textarea>`, shortcut é ignorado.
- Exceção: `Escape` sempre funciona para blur.

### 3. Persistence (Backend)
- Armazenamento em SQLite tabela `app_settings` (key-value store).
- Comandos Tauri: `get_setting`, `set_setting`.
- Carregamento assíncrono na inicialização do `shortcutStore`.
- Persistência automática ao editar atalhos.
- Chave de persistência composta: `NomeDoAtalho::Escopo` para estabilidade.

### 4. Settings & UI
- **SettingsModal:** Integrado ao sistema de input (escopo `modal`, blocking, focus trap).
- **Recorder:** Permite gravar qualquer tecla, incluindo `Escape`.
- **Conflicts:** Detecção inteligente de conflitos considerando escopo. Atalhos em escopos diferentes (especialmente se isolados/bloqueantes) não conflitam.
- **Layout:** Avisos de conflito posicionados abaixo do campo de input para melhor visualização.

- Botão "Reset to Defaults" restaura todos os shortcuts para defaults da aplicação
- Confirmação modal antes de resetar
- Custom shortcuts são removidos, não sobrescritos

### 5. Default Shortcuts Reset
- Suporte a reset individual e global.
- "Clear Search / Blur" adicionado aos defaults para permitir restauração.


### 6. Image Viewer Shortcuts (Scope: `image-viewer`)

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Escape` | Close viewer | Fecha o visualizador |
| `+` / `=` | Zoom in | Aumenta zoom 10% |
| `-` | Zoom out | Diminui zoom 10% |
| `Meta+0` | Fit to screen | Ajusta à tela |
| `Meta+1` | Original size | 100% zoom |
| `H` | Pan tool | Ativa ferramenta de pan |
| `R` | Rotate tool | Ativa ferramenta de rotate |
| `ArrowLeft` | Previous | Imagem anterior |
| `ArrowRight` | Next | Próxima imagem |

---

## Status Check

### Phase 1: Core Types & Normalization (Pure utilities) ✅

- [x] **1.1** Create `src/core/input/types.ts` with all TypeScript interfaces
  - `ShortcutDefinition`, `InputToken`, `InputScope`, `ShortcutMatch`, `GestureType`, etc.
  - Verify: File compiles without errors

- [x] **1.2** Create `src/core/input/normalizer.ts` - Pure token normalization
  - Port logic from `TokenNormalizer.js` to TypeScript
  - Add platform detection (Mac uses `Meta`, Windows uses `Ctrl`)
  - Verify: Unit logic works in console

### Phase 2: Reactive Store & Dispatcher ✅

- [x] **2.1** Create `src/core/input/store/inputStore.ts` - Main input state
  - Pressed keys (Set), active scopes (Stack), enabled status
  - Actions: `pushScope`, `popScope`, `setEnabled`, `keyDown`, `keyUp`
  - Verify: Store reactive updates work

- [x] **2.2** Create `src/core/input/store/shortcutStore.ts` - Shortcuts registry
  - Store registered shortcuts with `createStore`
  - Actions: `register`, `unregister`, `edit`, `list`, `getByScope`
  - Verify: CRUD operations work

- [x] **2.3** Create `src/core/input/dispatcher.ts` - Reactive matching
  - Match tokens against registry
  - Priority-based selection
  - Sequence buffer for key combos (e.g., `g g` for go)
  - Chord detection (simultaneous keys)
  - Verify: Correct shortcut fires

### Phase 3: Input Providers (Event Listeners) ✅

- [x] **3.1** Create `src/core/input/providers/KeyboardProvider.ts`
  - `createKeyboardProvider()` - attaches keydown/keyup to document
  - Normalizes events to `InputToken`
  - Calls dispatcher on key events
  - Handles visibility change (clear pressed keys)
  - Verify: Keyboard events logged correctly

- [x] **3.2** Create `src/core/input/providers/PointerProvider.ts`
  - `createPointerProvider()` - pointerdown, wheel
  - Mouse buttons + modifiers (Ctrl+Click, etc.)
  - Wheel up/down with modifiers
  - Verify: Mouse events normalized

- [x] **3.3** Create `src/core/input/providers/GestureProvider.ts`
  - `createGestureProvider()` - touch gestures
  - Port `GestureRecognizer.js` logic: swipe, pinch, rotate
  - Throttle support
  - Verify: Touch gestures on trackpad work

### Phase 4: Primitives (Hooks/Composables) ✅

- [x] **4.1** Create `src/core/input/primitives/createShortcut.ts`
  - Main API: `createShortcut({ keys, action, scope?, priority?, enabled? })`
  - Auto-register on mount, auto-cleanup on cleanup
  - Returns accessor with `{ isActive, disable, enable }`
  - Verify: Shortcut registers and fires

- [x] **4.2** Create `src/core/input/primitives/createKeyState.ts`
  - `createKeyState(key?)` - returns reactive pressed state
  - If no key, returns Set of all pressed keys
  - Verify: `isPressed('Shift')` reactive

- [x] **4.3** Create `src/core/input/primitives/createGesture.ts`
  - `createGesture({ type, handler })` for specific gestures
  - Verify: Gesture callback fires

- [x] **4.4** Create `src/core/input/primitives/createInputScope.ts`
  - `createInputScope(name, priority?)` - pushes scope on mount
  - Verify: Scope stack works

### Phase 5: Context Provider & Integration ✅

- [x] **5.1** Create `src/core/input/context.tsx`
  - `InputProvider` component - initializes all providers
  - `useInput()` hook for accessing store
  - Verify: Provider wraps app

- [x] **5.2** Create `src/core/input/index.ts` - Public API
  - Export all primitives, types, context
  - Clean external API
  - Verify: Imports work from `@/core/input`

- [x] **5.3** Integrate in `App.tsx`
  - Wrap app with `InputProvider`
  - Remove old `useKeyboardShortcuts` hook
  - Verify: App loads without errors

### Phase 6: Settings UI Components ✅

- [x] **6.1** Create `src/components/features/settings/SettingsModal.tsx`
  - Modal with header (title) + sidebar navigation
  - Sidebar items: "General", "Appearance", "Keyboard Shortcuts", etc.
  - Uses existing `Modal` component
  - Verify: Modal opens with sidebar

- [x] **6.2** Create `src/components/features/settings/SettingsModal.css`
  - Sidebar layout (200px fixed)
  - Content area scrollable
  - Uses design tokens
  - Verify: Matches app design

- [x] **6.3** Create `src/components/features/settings/KeyboardShortcutsPanel.tsx`
  - Lists all shortcuts grouped by scope
  - Each shortcut: name, description, key combo (Kbd component)
  - Edit button → inline editing mode
  - Recording mode for key capture
  - Conflict detection (warning badge)
  - Verify: Panel displays shortcuts

- [x] **6.4** Create `src/components/features/settings/KeyboardShortcutsPanel.css`
  - Shortcut list styling
  - Recording state styling
  - Conflict warning styling
  - Verify: Matches app design

- [x] **6.5** Create `src/components/features/settings/index.ts`
  - Export all settings components
  - Verify: Imports work

### Phase 7: Component Integration ✅

- [x] **7.1** Update `src/core/hooks/useKeyboardShortcuts.ts`
  - Replace imperative logic with `createShortcut` calls
  - Or deprecate if no longer needed (moved to App.tsx)
  - Verify: Existing shortcuts work

- [x] **7.2** Add shortcuts to viewport components (`src/components/features/viewport/`)
  - Navigation: Arrow keys, Page Up/Down
  - Zoom: Ctrl+0 (reset), Ctrl++ (zoom in), Ctrl+- (zoom out)
  - Selection: Ctrl+A, Escape
  - Verify: Viewport responds to keys

- [x] **7.3** Add shortcuts to search components (`src/components/features/search/`)
  - Ctrl+K/Cmd+K: Focus search
  - Escape: Clear/close search
  - Verify: Search responds to keys

- [x] **7.4** Add modal scope to `Modal.tsx`
  - Push `modal` scope when open
  - Baseline shortcuts: Escape to close
  - Verify: Modal shortcuts work

### Phase 8: Backend Persistence (Tauri Commands) ✅
- [x] 8.1 Add Rust commands for shortcut persistence (`db_settings`, `settings_commands`)
- [x] 8.2 Integrate persistence in `shortcutStore.ts` (Load/Save using `invoke`)
- [x] 8.3 Configure Permissions (`capabilities/default.json`, `permissions/main.toml`)

### Phase 9: Verification & Polish

- [x] **9.1** Test all default shortcuts work
  - Global: Ctrl+K (search), Ctrl+A (select all), Escape (deselect)
  - Image Viewer: Arrows, Zoom controls
  - Modals: Escape to close
  - Verify: All shortcuts functional

- [x] **9.2** Test shortcut editing flow
  - Open settings → Keyboard Shortcuts
  - Click edit on a shortcut
  - Press new key combo
  - Save and verify it works
  - Verify: Custom shortcuts work

- [x] **9.3** Test conflict detection
  - Try to assign same shortcut to two actions
  - Warning should appear
  - Verify: Conflicts detected

- [x] **9.4** Remove old code
  - Delete `docs/Shortcuts/` folder
  - Clean up unused imports
  - Verify: Build passes
  
- [x] **9.5** Layout improvements for conflict warning

---

## Done When

- [x] All shortcuts from `useKeyboardShortcuts` work with new system
- [x] Shortcuts persist to backend database
- [x] Settings modal with keyboard shortcuts panel is functional
- [x] No TypeScript errors, app builds successfully
- [x] Touch gestures work on trackpad (swipe, pinch-to-zoom)

---

## Notes

- **Platform Detection:** Mac uses `Cmd/Meta`, Windows/Linux uses `Ctrl`. Normalize in UI display.
- **Recording Mode:** When editing a shortcut, capture the next key combo (ignore modifier-only presses).
- **Conflict Resolution:** If same combo assigned twice, show warning. Allow override with confirmation.
- **Scope Priority:** modal (100) > image-viewer (50) > search (25) > global (0)
- **Design:** Follow existing design tokens. No Tailwind. Each component has its own .css file.


## Implemented Improvements

1.  **Scope Blocking (`blockLowerScopes`):**
    - Adicionado suporte para que escopos de alta prioridade (Modal, Image Viewer) bloqueiem completamente eventos de escopos inferiores (Global). Isso evita conflitos e comportamentos inesperados quando um modal está aberto.

2.  **Settings Modal Input Integration:**
    - O `SettingsModal` agora se comporta como um modal cidadão de primeira classe no sistema de input, bloqueando atalhos globais e capturando `Escape` para fechar. Uso de `FocusTrap`.

3.  **Robust Persistence:**
    - Uso de chaves compostas (`Name::Scope`) para salvar customizações, garantindo que mudanças de IDs internos não quebrem as preferências do usuário.
    - Tabela genérica `app_settings` no banco de dados para flexibilidade futura.

4.  **Recorder UX:**
    - UI ajustada para exibir erros de conflito sem quebrar o layout (abaixo do input).
    - Permissão para gravar a tecla `Escape` como atalho.

5.  **Settings UI Standardization:**
    - Refatoração do `SettingsModal` para utilizar o componente `Modal` compartilhado.
    - Herda gerenciamento robusto de foco, suporte a `Escape` e estilização consistente.

## Pending / Future Work

- [ ] **Strict Typing:** Refinar tipos de comandos para usar Enums/Unions em vez de strings livres.
- [ ] **A11y:** Testes mais profundos de acessibilidade na navegação por teclado dentro do Settings Modal.
- [ ] **I18n:** Melhorar formatação de exibição de teclas para teclados não-US.
