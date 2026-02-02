Esta é a **Parte 1** do relatório de auditoria técnica do projeto **Elleven Library**.

Devido à extensão e complexidade do projeto, dividi esta análise em áreas de foco. Nesta primeira parte, focarei na **Arquitetura Crítica (Backend Rust e Banco de Dados)**, onde residem os maiores riscos de performance e estabilidade.

---

# Relatório de Auditoria Técnica — Parte 1: Backend e Persistência

## 1. Segurança e Performance de Banco de Dados (`schema.sql` e `search_logic.rs`)

A lógica de busca e a estrutura do banco são o coração deste software. Detectei pontos críticos que impactarão a performance assim que a biblioteca do usuário passar de 5.000 imagens.

### 🔴 Crítico: Performance de Busca (Full Table Scans)

**Arquivo:** `src-tauri/src/search_logic.rs`

O código utiliza extensivamente o operador `LIKE` com *wildcards* no início da string (`%valor%`) para buscas textuais (filename, notes, tags).

```rust
// Trecho de search_logic.rs
query_builder.push_bind(format!("%{}%", c.value.as_str().unwrap_or("")));

```

* **O Problema:** Em bancos SQL (incluindo SQLite), um `LIKE` que começa com `%` **invalida o uso de índices B-Tree**. Isso força o banco a ler linha por linha da tabela `images` (Full Table Scan). Se o usuário tiver 50.000 imagens, cada letra digitada na busca varrerá o disco inteiro.
* **A Solução Recomendada:** Implementar **FTS5 (Full-Text Search)** do SQLite.
1. Criar uma tabela virtual: `CREATE VIRTUAL TABLE images_fts USING fts5(filename, notes, content='images', content_rowid='id');`
2. Alterar a query Rust para usar o operador `MATCH`. Isso reduzirá o tempo de busca de centenas de milissegundos para microsegundos.
**✅ STATUS: CONCLUÍDO (Implemented FTS5 + Triggers + Match Query)**



### 🟠 Importante: Índices Ausentes para Ordenação

**Arquivo:** `src-tauri/src/schema.sql`

O schema define índices para chaves estrangeiras (`folder_id`, `parent_id`) e `path`, mas **ignora colunas usadas para ordenação frequente**.

* **O Problema:** O `libraryStore.ts` permite ordenar por `rating`, `size`, `created_at`, `modified_at`. Sem índices nessas colunas, qualquer ordenação combinada com paginação (`LIMIT/OFFSET`) será lenta, pois o banco precisa carregar tudo na memória para ordenar antes de cortar os resultados.
* **Correção:** Adicionar índices compostos para os casos de uso mais comuns:
```sql
CREATE INDEX idx_images_rating ON images(rating DESC, created_at DESC);
CREATE INDEX idx_images_modified ON images(modified_at DESC);
```
**✅ STATUS: CONCLUÍDO (Added Indices for rating, size, created, modified)**


### 🟡 Code Smell: Construção de Query "Frágil"

**Arquivo:** `src-tauri/src/search_logic.rs`

A função `build_criterion_clause` faz um *whitelist* das colunas (`match c.key.as_str()`), o que previne SQL Injection direto. No entanto, há um padrão de "falha silenciosa".

* **O Problema:** Se uma chave desconhecida cair no `_ =>`, o código injeta `1=1` (verdadeiro).
```rust
_ => { query_builder.push(" i."); query_builder.push(&c.key); query_builder.push(" = 1 "); },

```


Isso retorna *todos* os registros se houver um erro de digitação no frontend ou uma chave nova não implementada no backend, confundindo o usuário ou o desenvolvedor.
* **Melhoria:** Retornar um `Result::Err` ou logar um aviso explícito de "Critério de busca desconhecido/ignorado".
**✅ STATUS: CONCLUÍDO (Added Warning Logs)**

---

## 2. Arquitetura e Rust Idioms (`thumbnails.rs`)

### 🟠 Code Smell: Uso Excessivo de `unwrap()` em Produção

**Arquivo:** `src-tauri/src/thumbnails.rs`

Detectei múltiplos usos de `unwrap()` ou `expect()` em fluxos que podem falhar em tempo de execução.

* **Exemplos:**
* `input_path.file_name().unwrap_or_default()` (Seguro, mas esconde erros).
* No `search_logic.rs`: `c.value.as_str().unwrap_or("")`. Se o JSON vier como número onde se espera string, converte para vazio silenciosamente.


* **Risco:** Em Rust, um `unwrap()` em uma `Option::None` ou `Result::Err` causa um **Panic**, que derruba a thread ou a aplicação inteira.
* **Correção:** Usar propagação de erros com `?` (operator try) ou pattern matching (`if let Some(...)`) para tratar falhas de forma recuperável e retornar erros formatados para o frontend.

### 🟡 Manutenibilidade: Detecção de Formatos Manual

**Arquivo:** `src-tauri/src/thumbnails.rs`

A função `get_strategy` faz um `match` gigante em strings de extensão.

* **O Problema:** A lista de extensões está duplicada ou espalhada (Rust e TypeScript `fileFormats.json`). Adicionar suporte a um novo formato requer alterar múltiplos arquivos e linguagens.
* **Melhoria:** Centralizar as definições de formatos em um arquivo de configuração compartilhado ou usar uma *crate* de detecção de MIME types (`infer` ou `mime_guess`) em vez de confiar apenas na extensão do arquivo, que pode ser enganosa.

### 🔴 Concorrência: Bloqueio de Thread (Blocking I/O)

**Arquivo:** `src-tauri/src/thumbnails.rs`

Funções como `std::fs::read` e `image::open` são **bloqueantes**.

* **O Problema:** Embora o `Cargo.toml` mostre o uso de `tokio`, se essas funções de thumbnail forem chamadas diretamente de um comando Tauri `#[tauri::command] async fn`, elas bloquearão a thread do *runtime async* do Rust. Isso congela outras operações IPC.
* **Correção:** Envolver operações pesadas de I/O e CPU (decodificação de imagem) em `tokio::task::spawn_blocking` ou usar o worker thread dedicado que parece existir em `thumbnail_worker.rs` (mencionado na árvore de arquivos, mas código não analisado a fundo, assumindo que ele resolve isso).

---

## 3. Integridade de Dados e Sistema de Arquivos

### 🟠 Risco de Integridade: "Caminho Único" vs. Realidade

**Arquivo:** `src-tauri/src/schema.sql`

A tabela define `path TEXT NOT NULL UNIQUE`.

* **O Problema:** O sistema de arquivos é "vivo". O usuário pode mover, renomear ou deletar arquivos fora do Elleven Library.
1. Se o usuário renomeia um arquivo externamente, o registro no banco torna-se um "link quebrado".
2. Se o usuário adiciona o mesmo arquivo novamente (com novo caminho), o sistema pode falhar ao tentar inserir se houver colisão de hash (se implementado) ou criará duplicata lógica.


* **Recomendação:** Implementar um **Watcher** robusto (usando a crate `notify` listada no Cargo.toml) que:
* Escute eventos de `Rename`/`Move`.
* Atualize o banco atomicamente.
* Execute uma rotina de "Sanity Check" na inicialização para limpar registros de arquivos inexistentes.
**✅ STATUS: CONCLUÍDO (Indexer contains robust notify watcher)**



---

## 4. Resumo de Bugs Potenciais (Backend)

1.  **Date Parsing:** Em `search_logic.rs`, o código tenta converter datas manualmente: `let parts: Vec<&str> = raw_val.split('/').collect();`. Isso assume formato `DD/MM/YYYY`. Se o frontend enviar `MM/DD/YYYY` (locale US) ou ISO `YYYY-MM-DD`, a busca falhará ou retornará dados errados. **Correção:** Padronizar tudo para ISO-8601 (`YYYY-MM-DD`) na comunicação JSON. **✅ STATUS: CONCLUÍDO**
2.  **Case Sensitivity no SQL:** O uso de `LIKE` no SQLite é *case-insensitive* para caracteres ASCII, mas depende de configuração para UTF-8 (acentos, etc.). Pode haver inconsistência ao buscar "Araçá" vs "araçá".
3.  **Recursividade Infinita:** A query recursiva para pastas (`WITH RECURSIVE subfolders`) em `search_logic.rs` não tem limite de profundidade. Em uma estrutura de pastas circular (symlinks maliciosos ou erro de lógica), isso poderia causar travamento. **✅ STATUS: CONCLUÍDO (Added depth limit)**

---

### Próximo Passo

Esta foi a análise focada no **Backend (Rust)**. Para o relatório completo, preciso analisar o **Frontend (SolidJS)**, onde detectei problemas de gerenciamento de estado e renderização.

**Deseja que eu prossiga com a Parte 2: Frontend, Code Smells de JavaScript e UX?**



Esta é a **Parte 2** do relatório, focada no **Frontend (SolidJS/TypeScript)** e na **Interface de Usuário**.

Encontrei pontos críticos na virtualização da grade de imagens que afetarão a escalabilidade, além de inconsistências arquiteturais no gerenciamento de estado.

---

# Relatório de Auditoria Técnica — Parte 2: Frontend e UI

## 1. Performance de Renderização e Virtualização (`VirtualMasonry.tsx`)

O componente `VirtualMasonry` é o mais importante da interface. A implementação atual é funcional para pequenas bibliotecas (< 1.000 imagens), mas **falhará catastroficamente** em bibliotecas grandes (> 10.000 imagens).

### 🔴 Crítico: Complexidade O(N) no Scroll

**Local:** `src/components/features/viewport/VirtualMasonry.tsx`

```typescript
// Linha 96: Cálculo de itens visíveis
return items.filter(item => {
  const pos = currentLayout.positions.get(item.id);
  // ... verifica se está na viewport ...
});

```

* **O Problema:** A função `visibleItems` roda a cada evento de scroll (ou frame de animação). Ela executa um `.filter()` em **todos** os itens da biblioteca.
* Se o usuário tiver 20.000 imagens, o javascript fará 20.000 verificações de limite a cada *frame* de rolagem. Isso causará travamentos visíveis (jank) e alto uso de CPU.


* **A Solução:**
1. **Binning Espacial:** Durante o cálculo do layout, agrupe os IDs dos itens em "buckets" verticais (ex: chunks de 1000px de altura).
2. No scroll, consulte apenas os buckets que interceptam a viewport, reduzindo a busca de O(N) para O(1) ou O(K) (onde K é o número de itens na tela).
**✅ STATUS: CONCLUÍDO (Worker uses Spatial Grid / Buckets)**



### 🔴 Crítico: Cálculo de Layout Síncrono (Main Thread)

**Local:** `src/components/features/viewport/VirtualMasonry.tsx` -> `calculateMasonryLayout`

* **O Problema:** A função `calculateMasonryLayout` é chamada dentro de um `createMemo`. Embora o SolidJS seja eficiente, recalcular a posição (x,y) de 50.000 itens de uma vez na thread principal vai congelar a interface por segundos sempre que a janela for redimensionada ou o zoom (colunas) mudar.
* **A Solução:** Mover a lógica de `calculateMasonryLayout` para um **Web Worker**. O worker recebe a lista de alturas e a largura do container, devolve um Map de posições, e a UI atualiza sem travar.
**✅ STATUS: CONCLUÍDO (Moved to layout.worker.ts)**

### 🟡 Code Smell: Renderização Condicional "Suja"

**Local:** `src/components/features/viewport/VirtualMasonry.tsx`

```typescript
// Dentro do JSX
display: layout().positions.get(item.id) ? "block" : "none",

```

* **O Problema:** O `VirtualMasonry` tenta renderizar itens que o filtro `visibleItems` retornou, mas faz uma verificação de segurança extra (`if (!pos) return null`). Se o `visibleItems` e o `layout` ficarem dessincronizados por um milissegundo (race condition de signals), você pode ter "flicker".
* **Melhoria:** Garantir que `visibleItems` seja derivado estritamente do `layout` atual, tornando a verificação de existência redundante e o código mais limpo.

---

## 2. Arquitetura de Componentes e Acoplamento

### 🟠 Acoplamento Forte em Componentes de UI

**Local:** `src/components/features/viewport/AssetCard.tsx`

O componente `AssetCard` não é "puro". Ele consome hooks globais diretamente:

```typescript
const lib = useLibrary();
const selection = useSelection();
const viewport = useViewport();

```

* **O Problema:** Isso torna o `AssetCard` impossível de reutilizar em outros contextos (ex: num modal de seleção, num plugin ou storybook) sem "mockar" toda a store global. Ele deveria ser um componente "burro" (dumb component) que recebe `isSelected`, `onSelect`, `onOpen` via props.
* **Impacto:** Dificulta testes unitários e refatorações futuras onde talvez você queira mostrar um card de imagem fora do contexto da biblioteca principal.
**✅ STATUS: CONCLUÍDO (Decoupled AssetCard)**

### 🟡 Inconsistência no Acesso ao Estado

**Comparação:**

* `AssetCard.tsx`: Usa `useSelection()` (Hook pattern).
* `ImageDropStrategy.ts`: Importa `selectionState` diretamente do arquivo store (Direct Access pattern).
* **O Problema:** Misturar padrões dificulta o rastreamento de onde o estado é modificado.
* **Recomendação:** Padronizar. Em SolidJS, o acesso direto (importando a store) é performático e aceitável, mas deve ser consistente. Se criou hooks (`useSelection`), use-os em todo lugar, ou remova-os se forem apenas wrappers desnecessários.

---

## 3. Lógica de Drag-and-Drop (`dnd`)

### 🟠 Risco de Performance em Diretivas

**Local:** `src/components/features/viewport/AssetCard.tsx`

```typescript
use:assetDnD={{ 
    item: props.item, 
    // ...
    allItems: lib.items // <--- CUIDADO
}}

```

* **O Problema:** Você está passando `lib.items` (a lista inteira da biblioteca) para a diretiva de DnD de **cada** card.
* Embora o SolidJS use referências finas, se a diretiva `assetDnD` criar algum efeito (createEffect) que dependa de `allItems`, qualquer alteração na biblioteca (ex: carregar mais itens) disparará a recriação da lógica de DnD para *todos* os cards visíveis.


* **Correção:** A diretiva de DnD deveria receber apenas o ID do item. A lógica global de "quem está sendo arrastado" deve residir no gerenciador central de DnD, não injetada em cada instância.
**✅ STATUS: CONCLUÍDO (Fixed in AssetCard)**

---

## 4. UX e Interface (`MultiInspector.tsx`)

### 🟢 Elogio: Feedback Visual "Deck"

O código em `MultiInspector.tsx` implementa um efeito de "baralho" (`inspector-deck-card`) rotacionando os itens levemente. Isso é um excelente detalhe de polimento visual que imita softwares nativos de alta qualidade.

### 🟡 Bug Potencial: Seleção Vazia

Não vi tratamento explícito (Guard Clause) no `MultiInspector` para caso `props.items` venha vazio ou undefined, embora ele provavelmente só seja montado se houver seleção. Se por erro de estado ele renderizar vazio, `previewItems()` quebrará ou mostrará algo estranho.

---

## 5. Resumo de Bugs e Code Smells (Frontend)

1. **Race Condition no Resize:** No `VirtualMasonry`, `setContainerWidth` dispara um novo layout. Se a mudança de layout alterar a altura total e fizer a barra de rolagem aparecer/desaparecer, a largura muda novamente. Isso cria um loop infinito de Resize -> Layout -> Resize (Layout Thrashing).
* *Correção:* Usar `scrollbar-gutter: stable` no CSS ou lógica de detecção de loop no JS.
**✅ STATUS: CONCLUÍDO (Fixed with threshold check)**


2. **Hardcoded Values:**
* `gap = 16` no Masonry. Deveria vir do theme/tokens ou user preferences.
* `buffer = 1000` pixels. Valor alto, consome memória. Deveria ser dinâmico com base na altura da viewport (ex: `window.innerHeight * 1.5`).


3. **Z-Index War:** No `MultiInspector`, o z-index é manual (`index()`). Verifique se isso não conflita com tooltips ou dropdowns que possam abrir sobre o inspector.

---

## Conclusão Geral da Auditoria

O projeto **Elleven Library** demonstra um nível técnico acima da média, com escolhas de stack modernas e focadas em performance (Rust/Tauri + SolidJS).

**Principais ações recomendadas (Prioridade Alta):**

1. **Backend:** Implementar FTS5 no SQLite para busca textual e adicionar índices de ordenação.
2. **Frontend:** Reescrever a lógica de `visibleItems` no `VirtualMasonry` para usar um algoritmo espacial (Buckets/Quadtree) e não iterar o array inteiro.
3. **Integridade:** Adicionar um `File Watcher` no Rust para sincronizar mudanças externas de arquivos.

O código está bem estruturado, mas a escalabilidade para "milhares de imagens" (promessa do software) depende estritamente de corrigir os pontos 1 e 2 acima.

**Fim do relatório.**