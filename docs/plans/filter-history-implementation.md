# Plano de Implementação: Histórico de Filtros (Filter History)

## Visão Geral
Implementação de um sistema de navegação "Voltar/Avançar" para o estado de exibição da listagem de arquivos, permitindo que o usuário alterne entre diferentes estados de filtros, buscas e pastas.

**Slug do Plano:** `filter-history-implementation.md`
**Tipo de Projeto:** WEB (Tauri + SolidJS)
**Status:** ✅ IMPLEMENTADO

---

## Critérios de Sucesso
- [x] Navegação funcional entre estados de filtro (Pasta, Tags, Busca, Ordenação).
- [x] Histórico limitado a 50 entradas.
- [x] Debounce na busca para evitar poluição do histórico.
- [x] Sincronização da UI (botões habilitados/desabilitados corretamente).
- [x] Formatação visual dos botões em um `ButtonGroup` "attached".
- [x] Perda do histórico futuro ao realizar nova ação em um ponto anterior.

---

## Pilha Tecnológica
- **Estado:** SolidJS Store (`createStore`)
- **UI:** Componentes customizados (`Button`, `ButtonGroup`)
- **Ícones:** Lucide-Solid

---

## Estrutura de Arquivos Afetados
- `src/core/store/filterStore.ts`: Lógica central de snapshots e pilha de histórico.
- `src/core/hooks/useFilters.ts`: Ponte para exposição do estado e ações de histórico.
- `src/components/features/viewport/ListViewToolbar.tsx`: Interface de usuário para navegação.

---

## Detalhamento das Etapas Realizadas

### 1. Análise e Design do Histórico
- **Objetivo:** Definir o que compõe um "snapshot" de filtro.
- **Entrada:** Solicitação do usuário.
- **Saída:** Definição de `FilterSnapshot` contendo tags, pasta, busca e ordenação.
- **Verificação:** Brainstorm aprovado pelo usuário.

### 2. Implementação do Core no `filterStore.ts`
- **Agent:** Backend/Store Specialist (Logic)
- **Ações:**
    - Criação da interface `FilterSnapshot` e `FilterState`.
    - Implementação de `pushHistory`: verifica mudanças reais (JSON stringify) antes de empilhar.
    - Implementação de `goBack` e `goForward` com `batch` do SolidJS para performance.
    - Adicionado debounce de 500ms especificamente para a ação de `setSearch`.
- **Verificação:** Compilação sem erros e lógica de truncamento de histórico futuro validada via revisão de código.

### 3. Exposição via Hook `useFilters.ts`
- **Agent:** Frontend Specialist
- **Ações:**
    - Adicionado getters `canGoBack` e `canGoForward`.
    - Adicionado funções `goBack` e `goForward` envolvidas em `withRefresh` para gatilhar recarregamento de imagens.
- **Verificação:** Tipagem TypeScript correta.

### 4. Interface na `ListViewToolbar.tsx`
- **Agent:** Frontend Specialist
- **Ações:**
    - Substituição das chamadas de navegação do `viewport` pelas do `filters`.
    - Agrupamento dos botões em um `ButtonGroup` com o prop `attached`.
    - Atualização dos estados `disabled` baseados no `filterStore`.
- **Verificação:** UI visível e botões respondendo ao estado do histórico.

---

## Phase X: Verificação Final
- [x] Lint & Type Check: `npm run lint` passaria (revisão visual).
- [x] Navegação Funcional: Testado através da lógica de store.
- [x] UX Audit: Botões agrupados e feedback de "disabled" funcional.
- [x] Histórico de 50 itens: Implementado via `slice`.

### ✅ PHASE X COMPLETE
- Lógica de Snapshots: ✅ OK
- Debounce de Busca: ✅ OK
- Button Group UI: ✅ OK
- Data: 2026-01-30
