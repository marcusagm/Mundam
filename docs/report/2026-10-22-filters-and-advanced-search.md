# Relatório: filtros da sidebar e busca avançada

Data: 2026-10-22

Este relatório consolida as alterações de filtros e busca avançada em três frentes:

1. Seleção nos filtros da sidebar (tags, pastas, filtros rápidos).
2. Smart Folders como busca avançada salva e pré-preenchimento do modal.
3. Grupos AND/OR aninhados no `AdvancedSearchModal`.

## 1. Seleção nos filtros da sidebar

### 1.1 Problema

Clicar em uma tag ou pasta acumulava a seleção. Para remover era preciso clicar de novo no item. Esse comportamento acumulativo deveria ser o de Ctrl/Meta+clique, e o clique simples deveria alternar entre itens.

### 1.2 Comportamento final

| Ação                               | Resultado                                                      |
| ---------------------------------- | -------------------------------------------------------------- |
| Clique simples em tag/pasta        | Substitui a seleção pelo item clicado                          |
| Clique simples no único item ativo | Limpa a seleção                                                |
| Ctrl/Meta+clique em tag/pasta      | Adiciona ou remove o item, mantendo os demais                  |
| Clique em filtro rápido (Library)  | Sempre exclusivo entre si (ex.: favoritos e lixeira não somam) |

O modo aditivo dos filtros rápidos foi removido, pois combinações como "has tag + untagged" ou "favorites + trash" não fazem sentido.

### 1.3 Frontend

- `src/core/store/filter/filterActions.ts`
  - `selectTag(id, additive)` e `selectFolder(id, additive)` implementam substituição, limpeza e alternância aditiva.
  - `toggleQuickFilter` é sempre exclusivo.
- `TreeView`: `onSelect(node, isAdditive)` repassa o estado de Ctrl/Meta do clique.
- Painéis de tags e pastas passam o modificador para as ações.
- O estado de pastas passou de uma única pasta para múltiplas: `selectedFolderIds` e `folderIds` no filtro.

### 1.4 Backend (multi-pasta)

- `AssetFilter.folder_ids: Option<Vec<String>>` substitui o filtro de pasta única.
- Nova função `push_folder_filter` em `asset_queries.rs`, incluindo o caso recursivo com várias raízes.

### 1.5 Testes e documentação

- `filterActions.spec.ts` atualizado para o novo comportamento.
- `docs/features/shortcuts.md` documenta clique simples versus Ctrl/Meta+clique e a exclusividade dos filtros rápidos.

## 2. Smart Folders como busca avançada salva

### 2.1 Decisão

Uma smart folder é uma busca avançada salva, editável no `AdvancedSearchModal`. Combinar buscas avançadas com os demais filtros é complexo. A estratégia adotada foi que ativar uma smart folder (ou busca avançada) limpa os outros filtros, e o inverso também vale. Assim, clicar em uma tag com uma smart folder ativa sempre produz um resultado visível.

### 2.2 Implementação

- `filterActions.ts`
  - `setAdvancedSearch(search)` limpa tags, pastas e filtros rápidos antes de aplicar a busca.
  - Selecionar tag, pasta ou filtro rápido chama `clearAdvancedSearch()`.
  - `clearAdvancedSearch()` é uma nova ação.
- `LibrarySidebarPanel`: `isAllItemsActive` agora exige `!filters.advancedSearch`, voltando a "All items" quando nada está filtrado.
- `filterActions.spec.ts` ganhou casos para essas transições (10 testes no arquivo).

### 2.3 Pré-preenchimento do modal a partir dos filtros ativos

Ao abrir a busca avançada pelo `SearchToolbar`, os itens de `activeFilterList` viram critérios:

- Grupo OR com as tags selecionadas.
- Grupo OR com as pastas selecionadas, usando o operador `in` se a visão recursiva estiver ativa e `is` caso contrário.
- Os dois grupos são combinados por AND na raiz.
- Se já existir `filters.advancedSearch`, ele tem prioridade sobre os filtros da sidebar.

A lógica está em `sidebarFiltersQuery` no `SearchToolbar.tsx`, passada como `initialQuery`.

### 2.4 Renderização de grupos aninhados no editor

Como o pré-preenchimento gera grupos, o `QueryEditor` passou a renderizá-los (CSS `.criterion-group` em `advanced-search-modal.css`). Esse passo preparou a refatoração da seção 3.

## 3. Grupos AND/OR aninhados no modal de busca avançada

### 3.1 Requisito

Antes só existia um "Match Mode" global (AND ou OR para todos os critérios). Agora cada grupo tem o seu operador, há um grupo raiz que não pode ser removido, cada grupo tem um `CriteriaBuilder` próprio, e a estrutura permite grupos dentro de grupos.

### 3.2 Backend

Sem alterações. `search_builder.rs` (`SearchItem::Group` e `Criterion`) já gerava o `WHERE` recursivo. `libraryActions.fetchLibraryBatch` já enviava o grupo raiz via `mapToSearchGroup`.

### 3.3 Arquivos novos

- `src/components/features/search/searchTree.ts`: funções puras e imutáveis sobre a árvore.
  - `createEmptyGroup(op)`, com id `group-<uuid>`.
  - `addItemToGroup`, `setGroupOperator`.
  - `removeGroup`, que nunca remove a raiz.
  - `removeCriterion`, `updateCriterion`, `findCriterion`.
  - `countCriteria`, `pruneEmptyGroups`, `isCriterion`.
- `src/components/features/search/searchTree.spec.ts`: 5 testes dessas funções.
- `src/components/features/search/useCriterionBuilder.ts`: hook do formulário "adicionar critério", com uma instância por grupo.
  - Estado de chave, operador, valores, unidade e erros de validação.
  - `buildCriterion()` valida, processa, formata o `displayValue`, limpa o formulário e devolve `SearchCriterion | null`.
  - Ids de critério usam `criterion-${crypto.randomUUID()}`.

### 3.4 Arquivos alterados

- `useAdvancedSearch.ts` (reescrito)
  - Nova assinatura: `useAdvancedSearch({ isOpen, initialQuery })`, sem o parâmetro `metadata`.
  - Estado: `rootGroup` (reiniciado ao abrir o modal com `initialQuery` ou um grupo vazio) e edição inline única (`editingId`, valores e erros).
  - Ações: `handleAddItem(groupId, item)`, `handleAddGroup(parentId, op)`, `handleRemoveGroup`, `handleSetGroupOperator`, `handleRemoveCriteria`, `handleStartEdit`, `handleConfirmEdit`, `handleReset`.
  - Derivados: `criteriaCount()` e `buildQuery()` (raiz sem grupos vazios).
- `QueryEditor.tsx`
  - Novo `GroupEditor` recursivo: cabeçalho com `RadioGroup` Any (OR) / All (AND), botão de remover oculto na raiz, aviso de grupo vazio, itens (`CriterionItem` ou subgrupo), `CriteriaBuilder` e botão "Add group".
  - O seletor global "Match Mode" foi removido.
- `CriteriaBuilder.tsx`: recebe `onAdd(criterion)`, usa `useCriterionBuilder` com `useMetadata()` e não depende mais do estado do modal.
- `AdvancedSearchModal.tsx`
  - Usa `search.buildQuery()` para buscar e salvar smart folders, e `search.criteriaCount()` para desabilitar botões.
  - Removidos o `CriteriaBuilder` de nível superior, `createId` e `useMetadata`, que ficaram sem uso.
- `advanced-search-modal.css`: estilos do cabeçalho do grupo e das ações do grupo.

### 3.5 Decisões técnicas

- Ids novos usam UUID. O `createId` é um contador reiniciado a cada sessão e poderia colidir com ids de smart folders já salvas.
- Grupos vazios são removidos antes de enviar, para não gerar `1=1` no SQL. A raiz vazia continua válida, mas desabilita os botões de busca e de salvar.
- Itens são discriminados com `'key' in item`.
- O formulário de critério por grupo isola o estado, permitindo vários builders ao mesmo tempo.

## 4. Validação

- `npm run typecheck`: sem erros.
- `npx eslint src/components/features/search`: sem avisos.
- `npm test`: 9 arquivos, 42 testes passando (inclui `searchTree.spec.ts`).
- `cargo check --lib` passou na etapa do multi-pasta (seção 1.4).
- `npm run build` passou na etapa da seção 2. Não foi reexecutado após a seção 3.
- O modal com grupos não foi testado manualmente na interface.

## 5. Pendências e observações

- Testar manualmente: criar subgrupos, aninhar mais de um nível, editar critérios inline e salvar/reabrir uma smart folder com grupos.
- Smart folders antigas (um nível, com `matchMode`) continuam compatíveis, pois o grupo raiz salvo é carregado como `initialQuery`.
- `Meta+1` conflita com "Original Size" no visualizador (pendência antiga).
- `defaults.ts` excede `max-lines` e há um aviso `solid/reactivity` em `createFocusScope.ts` (pendências antigas, fora do escopo).
- Assets antigos com caminho NFD no banco não foram migrados (pendência da frente da lixeira).
- Documentar a busca avançada com grupos em `docs/features/` ainda não foi feito, pois não há doc específica de busca.
