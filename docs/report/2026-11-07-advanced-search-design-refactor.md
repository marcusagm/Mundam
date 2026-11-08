# 💎 Relatório de Design de Usabilidade, Acessibilidade e Arquitetura: Busca Avançada e Construtor de Consultas (Query Builder)

**Data:** 08 de Novembro de 2026  
**Autor:** Especialista em Usabilidade, Acessibilidade & Design System  
**Arquivos Alvo:** `src/components/features/search/AdvancedSearchModal.tsx`, `QueryEditor.tsx`, `CriteriaBuilder.tsx`, `advanced-search-modal.css` e subcomponentes de campos em `src/components/features/search/fields/`  
**Referência Visual:** Captura de tela da versão atual (`media_1794161071359_f021fd56.png`)  
**Status:** Planejamento e Especificação Técnica Arquitetural para Implementação S-Tier  

---

## 1. Sumário Executivo

O **Mundam** é uma aplicação desktop de gerenciamento de ativos digitais (DAM) voltada para profissionais de fotografia, design e mídia. Uma de suas ferramentas mais poderosas é a **Busca Avançada** (`AdvancedSearchModal`), que permite aos usuários filtrar bibliotecas massivas combinando múltiplos critérios lógicos (tags, cores, dimensões, metadados de câmera, datas e formatos de arquivo), além de persistir essas buscas como **Pastas Inteligentes** (*Smart Folders*).

### O Diagnóstico do Estado Atual
Embora a lógica de filtragem e manipulação de árvores booleanas (`searchTree.ts`, `useAdvancedSearch.ts`) funcione com precisão cirúrgica no backend e nas stores, **a experiência visual e de usabilidade do construtor de consultas está fragmentada, esteticamente crua e cognitiva e visualmente exaustiva**:

1. **Proliferação Desenfreada de Construtores de Regras:** Cada grupo (seja o grupo raiz ou qualquer subgrupo aninhado) renderiza incondicionalmente um bloco completo de 3 dropdowns sob o título `"CRITERIA BUILDER (?)"`. Isso gera telas poluídas com formulários vazios duplicados e triplicados, tornando impossível distinguir à primeira vista o que já é uma regra ativa do que é apenas um formulário de adição.
2. **Hierarquia e Profundidade Visual Inexistentes:** O aninhamento booleano é representado unicamente por linhas cinzas de 2px à esquerda (`border-left`), sem cartões de elevação cromática ou agrupamentos nítidos. O usuário se perde em um emaranhado de linhas verticais.
3. **Linhas de Regras Desprovidas de Polish S-Tier:** As condições ativas são exibidas como texto puro horizontal (`1  Tags  Contains  teste2`), com operadores em itálico cru, números de índice irrelevantes e botões de ação minúsculos e sem affordance. Não há chips visuais para tags, nem amostras para cores ou estrelas para ratings.
4. **Desconexão de Fluxo no Rodapé:** A criação de *Smart Folder* (um campo de texto solto e um botão de disquete) está posicionada no meio da barra de navegação do rodapé, competindo diretamente com os botões principais de ação (`Cancel` e `Search`).
5. **Copywriting Técnico e Pouco Amigável:** Termos como *"Query Editor"*, *"Criteria Builder"*, *"Match Any (OR)"* e *"Match All (AND)"* impõem álgebra booleana e vocabulário de banco de dados diretamente ao usuário criativo.

### O Objetivo
Elevar o `AdvancedSearchModal` ao padrão **S-Tier** (no nível de referências da indústria como macOS Finder Smart Folders, Lightroom Classic, Linear Filters e Notion Database Filters), entregando:
- **Card-Based Visual Hierarchy:** Grupos booleanos estruturados como cartões elegantes e contidos com tokens de superfície (`--bg-surface-1`, `--bg-surface-2`).
- **Streamlined Rule Authoring:** Eliminação da proliferação de formulários permanentes em favor de um compositor de regras sob demanda ou integrado com design refinado.
- **Rich Condition Chips:** Formatação rica para valores e tipos (badges de tags, amostras de cores, ícones temáticos por campo).
- **Humanized Microcopy:** Linguagem natural empática, clara e intuitiva para criadores visuais.
- **Acessibilidade Plena (WCAG 2.1 AA/AAA):** Suporte total a teclado, anéis de foco consistentes, rótulos ARIA descritivos e contraste certificado.

---

## 2. Auditoria Crítica Visual, UX e Acessibilidade (Deep-Dive no Layout Atual)

A auditoria detalhada toma como base a captura de tela `media_1794161071359_f021fd56.png` e os arquivos de código-fonte de `src/components/features/search/`.

```
┌────────────────────────────────────────────────────────────────────────┐
│ [X]                                                    Advanced Search │ <- Título desalinhado à direita no macOS
├────────────────────────────────────────────────────────────────────────┤
│ QUERY EDITOR (?)                                                       │ <- Rótulo técnico, caixa alta agressiva
│ ┌────────────────────────────────────────────────────────────────────┐ │
│ │ Match                                         (o) Any (OR)  ( ) All│ │ <- Radios empurrados para o canto direito
│ │ ┌────────────────────────────────────────────────────────────────┐ │ │
│ │ │ Group                           ( ) Any (OR)  (o) All (AND) [🗑]│ │ │ <- "Group" sem contexto, borda fina
│ │ │                                                                │ │ │
│ │ │ 1  Tags       Contains        teste2                  [✏️] [🗑] │ │ │ <- "Contains" em itálico cru, texto plano
│ │ │                                                                │ │ │
│ │ │ CRITERIA BUILDER (?)                                           │ │ │ <- Formulário duplicado ocupando espaço
│ │ │ [ Tags      v] [ Contains  v] [ Select...              v] [+]  │ │ │
│ │ │ + Add group                                                    │ │ │
│ │ └────────────────────────────────────────────────────────────────┘ │ │
│ │                                                                    │ │
│ │ 2  Tags         Contains        teste4                  [✏️] [🗑] │ │ <- Regra 2 parece solta fora do grupo
│ │                                                                    │ │
│ │ CRITERIA BUILDER (?)                                               │ │ <- Outro formulário idêntico duplicado!
│ │ [ Tags      v] [ Contains  v] [ Select...              v] [+]      │ │
│ │ + Add group                                                        │ │
│ └────────────────────────────────────────────────────────────────────┘ │
├────────────────────────────────────────────────────────────────────────┤
│ Cancel              [ Smart Folder Name... ] [💾 Save as Smart Folder] │ <- Input solto no rodapé entre Cancel e
│                                                               [🔍 Search]│    Search quebrando a hierarquia
└────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Principais Falhas de Usabilidade e Arquitetura de Informação

#### A. A Síndrome do "Criteria Builder" Onipresente
Em `QueryEditor.tsx`, a renderização do construtor de critérios está acoplada dentro do próprio componente recursivo `GroupEditor`:
```tsx
// Trecho atual em QueryEditor.tsx (linhas 175-185)
<CriteriaBuilder onAdd={c => props.search.handleAddItem(props.group.id, c)} />
<div class="group-actions">
    <Button onClick={() => props.search.handleAddGroup(props.group.id, 'or')}>
        <Plus size={14} /> Add group
    </Button>
</div>
```
- **Consequência:** Sempre que um grupo é criado, ele instantaneamente projeta 3 menus suspensos (`Select`), um botão de adição e um título em caixa alta com ícone de interrogação.
- **Dano Cognitivo:** Na imagem real com apenas 2 regras salvas, o usuário é bombardeado por **6 selects abertos**, **2 títulos repetidos** e **2 botões de interrogação**. Fica a sensação de que há tarefas incompletas ou campos obrigatórios não preenchidos.
- **Ambiguidade de Ação:** O usuário não sabe intuitivamente em qual dos botões `[+]` deve clicar para adicionar uma regra à raiz ou ao subgrupo, pois os blocos são geometricamente idênticos.

#### B. Pobreza Visual e Semântica das Regras Salvas (`CriterionItem`)
- **Falta de Chips e Cores:** O campo `Tags` com valor `teste2` é renderizado como texto branco corrido. Não parece uma tag, parece uma frase quebrada.
- **Itálico Anacrônico:** O operador `Contains` recebe `font-style: italic; opacity: 0.8;`. Em interfaces modernas (macOS Sonoma/Sequoia, Linear, Figma), itálico é reservado para notas de rodapé ou citações; em filtros estruturados, operadores devem ser exibidos como tokens de texto neutro ou pequenas pílulas tipográficas com contraste equilibrado.
- **Numeração Sequencial Ilusória:** Exibir `1`, `2` cria a falsa impressão de que a busca é executada em ordem cronológica de passos (como uma receita ou pipeline), quando na realidade trata-se de um predicado de conjunto lógico.
- **Edição Travada e Incompleta:** Ao clicar no lápis de edição, o usuário só consegue alterar o *valor* (`teste2`). Se ele quiser alterar o operador de `Contains` para `Equals`, ou trocar o campo de `Tags` para `Filename`, ele é forçado a deletar a regra inteira e digitar tudo de novo. Além disso, não há botão de "Cancelar Edição", obrigando o usuário a salvar ou deletar.

#### C. Lógica Booleana Exposta de Maneira Hermética
- As opções `Any (OR)` e `All (AND)` utilizam parênteses matemáticos como se fossem anotações de aula de lógica formal.
- Os botões de rádio ficam flutuando no extremo direito do contêiner, longe do título do grupo, criando uma quebra na linha visual de leitura ocidental (da esquerda para a direita).
- Falta um seletor unificado de modo de combinação (*Segmented Control* ou *Pill Toggle*), com textos que explicam claramente a consequência:
  - *"Corresponder a TODOS os critérios (AND)"*
  - *"Corresponder a QUALQUER critério (OR)"*

#### D. Poluição e Quebra de Fluxo no Rodapé (*Modal Footer*)
- O rodapé de um diálogo modal deve ter uma regra de ouro: **o canto direito é o destino da ação principal** (`Buscar` / `Salvar`) e o **canto esquerdo é a saída segura** (`Cancelar`).
- Inserir um campo de texto de 200px (`[ Smart Folder Name... ]`) e um botão de salvar pasta inteligente espremido no meio do rodapé:
  1. Distrai quem só quer fazer uma pesquisa rápida e fechar o modal.
  2. Cria ambiguidade: se o usuário digitar um nome e clicar em `Search`, ele salvará a pasta ou apenas fará a busca? (O código atual apenas executa a busca, descartando silenciosamente o texto digitado se ele não clicar especificamente no botão de disquete).
  3. Quando aberto em modo de edição de Smart Folder existente (`isSmartFolderMode`), o input do nome pula magicamente para o topo do modal, gerando inconsistência de memória muscular no usuário.

---

## 3. Matriz de Bisseção: Problemas Atuais vs. Soluções S-Tier

| Elemento / Dimensão | Layout Atual (Problema) | Solução de Padrão S-Tier Proposta | Justificativa / Impacto |
| :--- | :--- | :--- | :--- |
| **Estrutura de Grupos** | Linhas cinzas verticais (`border-left: 2px`) sem contraste de fundo. | Cartões delimitados com elevação sutil (`--bg-surface-2`), cantos arredondados (`--radius-m`) e bordas suaves (`--border-subtle`). | Confinamento visual claro que torna o aninhamento imediatamente compreensível. |
| **Modo de Combinação (AND/OR)** | Radio buttons distantes com notação crua `Any (OR)` / `All (AND)`. | *Segmented Pill Control* moderno integrado ao cabeçalho do cartão com ícone ou texto humanizado: `All conditions` / `Any condition`. | Reduz o movimento dos olhos (*eye travel*) e humaniza a lógica booleana. |
| **Compositor de Critérios** | Formulário estático de 3 selects e botão `+` replicado em todos os níveis o tempo todo. | **Botão de Adição Elegante** (`+ Add condition`) que mantém o cartão limpo, ou barra de composição refinada em estilo "draft" com placeholder intuitivo e botão de ação primário destacado. | Elimina 70% da poluição visual e remove campos vazios fantasmas. |
| **Exibição de Regras Salvas** | Texto corrido sem formatação visual, operadores em itálico e numeração irrelevante. | **Rule Cards / Chips:** Linhas compactas com ícone temático do tipo de campo, badges semânticos para tags/valores, operador em caixa baixa neutra e botões de ação que aparecem no hover. | Escaneabilidade instantânea; alinhamento aos designs do macOS Finder e Linear. |
| **Edição de Critérios** | Permite alterar apenas o valor; não altera operador nem campo; sem botão de cancelar. | **Edição Completa Inline:** Transforma a linha em um compositor ativo pré-preenchido com campo, operador e valor, com botões de confirmar (`Enter`) e cancelar (`Esc`). | Autonomia completa para o usuário sem necessidade de recriar regras do zero. |
| **Criação de Smart Folder** | Input de texto e botão soltos no rodapé entre Cancel e Search. | **Ação Auxiliar Dedicada:** Botão expansível no rodapé ou cabeçalho (`Salvar como Pasta Inteligente`) que revela suavemente o campo com validação e feedback instantâneo. | Limpa o rodapé para o fluxo principal de busca, mantendo a função acessível com um clique. |
| **Paleta e Tokens** | `rgba(0,0,0,0.2)`, `rgba(255,255,255,0.02)` soltos no CSS. | Adoção estrita de `--bg-surface-1`, `--bg-surface-2`, `--border-subtle`, `--radius-m`, `--shadow-sm`, `--text-primary`, `--text-secondary`. | Coerência estética com temas Dark, Light e Custom do Mundam. |
| **Acessibilidade** | Botões de ícones sem texto acessível descritivo, contraste baixo em labels. | `aria-label` completo para cada ação (ex: *"Remover regra de tags: teste2"*), anéis de foco com `--shadow-focus` e navegação fluida por teclado. | Conformidade rigorosa com WCAG 2.1 AA. |

---

## 4. Matriz de Copywriting & Microcopy Humanizado

Para que a interface seja acolhedora para fotógrafos e designers, substituímos jargões de engenharia de software por termos naturais e empáticos em inglês e português:

| Contexto / Chave | Texto Atual (Técnico / Hermético) | Novo Copywriting S-Tier (Inglês) | Novo Copywriting S-Tier (Português) | Motivo da Decisão |
| :--- | :--- | :--- | :--- | :--- |
| **Título do Modal** | `Advanced Search` / `Edit Smart Folder` | `Advanced Search` / `Edit Smart Folder` | `Busca Avançada` / `Editar Pasta Inteligente` | Mantém precisão com ícone contextual de busca ou pasta. |
| **Seção Principal** | `QUERY EDITOR (?)` | `Search Conditions` | `Condições de Busca` | "Query Editor" soa como console SQL; "Condições" é intuitivo e universal. |
| **Modo Lógico (AND)** | `All (AND)` | `Match all conditions` | `Corresponder a todos` | Explica a consequência lógica em vez de apenas nomear a porta lógica. |
| **Modo Lógico (OR)** | `Any (OR)` | `Match any condition` | `Corresponder a qualquer` | Linguagem natural que elimina dúvidas sobre exclusão/inclusão. |
| **Seção do Construtor**| `CRITERIA BUILDER (?)` | `Add Condition` | `Adicionar Condição` | Rótulo direto e ativo que estimula a ação sem títulos redundantes. |
| **Botão de Adicionar** | `+ Add group` | `+ Add nested group` | `+ Adicionar subgrupo` | Deixa explícito que criará uma subcondição com lógica própria. |
| **Grupo Vazio** | `This group is empty. Add a criterion below.` | `No conditions defined. Add a condition to start filtering.` | `Nenhuma condição definida. Adicione uma regra para começar a filtrar.` | Mensagem propositiva com estado vazio elegante (ilustração ou ícone sutil). |
| **Placeholder de Valor**| `Select...` / `Smart Folder Name...` | `Choose option...` / `e.g. Vacation Photos 2026` | `Selecione uma opção...` / `Ex: Fotos de Viagem 2026` | Exemplos concretos que orientam o usuário (*in-context guidance*). |
| **Botão Salvar Pasta** | `Save as Smart Folder` | `Save as Smart Folder` | `Salvar como Pasta Inteligente` | Mantém o termo do domínio com ícone harmonizado. |
| **Botão Principal** | `Search` | `Search Library` | `Buscar Ativos` | Comunica a ação concreta e o escopo da busca na biblioteca. |

---

## 5. Especificação de Design Tokens e Anatomia Visual

### 5.1 Hierarquia de Cores e Superfícies
- **Fundo do Conteúdo Modal:** `--bg-page` ou `--bg-surface-1`.
- **Cartão do Grupo Raiz:** `--bg-surface-1` com borda `--border-subtle`, cantos `--radius-l` e padding confortável (`--p-space-m`).
- **Cartão de Subgrupo Aninhado:** `--bg-surface-2` com borda `--border-subtle`, recuo visual à esquerda marcado por uma guia vertical de cor suave (`--border-default`), cantos `--radius-m` e padding interno de `--p-space-s`.
- **Linha de Regra Ativa (`CriterionItem`):**
  - Fundo neutro: `--bg-surface-2` (ou `--bg-surface-3` dentro de subgrupos).
  - Hover: `--bg-surface-hover` com transição suave (`var(--transition-fast)`).
  - Estado em Edição: Borda com `--border-active` ou `--action-primary-bg` com leve brilho e foco automático no primeiro controle editável.

### 5.2 Tipografia e Geometria dos Controles
- **Rótulo do Campo:** `font-weight: 600`, tamanho `var(--p-font-size-s)`, cor `var(--text-primary)`.
- **Operador:** `font-weight: 500`, tamanho `var(--p-font-size-xs)`, cor `var(--text-secondary)`, sem itálico, envolvido em pill discreta com `background: var(--bg-surface-3); padding: 2px 8px; border-radius: var(--radius-pill);`.
- **Valor da Regra:**
  - *Tags:* Badges com `--bg-surface-active` e ícone de tag.
  - *Cores:* Amostra circular da cor (swatch de 14px) + código HEX + indicador de proximidade/tolerância.
  - *Dimensões e Tamanho:* Valores monoespaçados (`font-family: var(--font-mono)`), com unidades claras (ex: `1920 × 1080 px`, `> 15.4 MB`).
  - *Datas:* Data formatada pelo padrão da localização do sistema.

### 5.3 Controles Lógicos (Alternador AND / OR)
- Substituição dos radio buttons isolados por um **Segmented Control** compacto:
  - Container: `display: inline-flex; background: var(--bg-surface-3); border-radius: var(--radius-s); padding: 2px; gap: 2px;`
  - Item ativo: `background: var(--bg-surface-1); color: var(--text-primary); box-shadow: var(--shadow-sm); font-weight: 600;`
  - Item inativo: `color: var(--text-tertiary); font-weight: 500;`

---

## 6. Wireframes Semânticos Comparativos

### 6.1 Layout Atual (Fragmentado e Pesado)
```
[X]                                                    Advanced Search
----------------------------------------------------------------------
QUERY EDITOR (?)
[ Match                                         (o) Any (OR)  ( ) All]
|-- [ Group                          ( ) Any (OR)  (o) All (AND) [x] ]
|   | 1  Tags   Contains   teste2                           [e] [d]  |
|   | CRITERIA BUILDER (?)                                           |
|   | [Tags v] [Contains v] [Select... v] [+]                        |
|   | + Add group                                                    |
|                                                                    |
|-- 2  Tags   Contains   teste4                             [e] [d]  |
|   CRITERIA BUILDER (?)                                             |
|   [Tags v] [Contains v] [Select... v] [+]                          |
|   + Add group                                                      |
----------------------------------------------------------------------
Cancel            [ Smart Folder Name... ] [💾 Save]        [🔍 Search]
```

### 6.2 Layout S-Tier Proposto (Harmônico, Limpo e Intuitivo)
```
+--------------------------------------------------------------------+
|  Advanced Search                                               [X] |
+--------------------------------------------------------------------+
|  Find assets matching: [ All conditions (AND) | Any condition (OR) ]|
|                                                                    |
|  +-- Subgroup: Match all conditions ------------------------ [🗑] -+
|  | [🏷️ Tags]  [contains]  ( #teste2 )                    [✏️] [🗑] |
|  |                                                                 |
|  | [+ Add condition]   [+ Add nested group]                        |
|  +-----------------------------------------------------------------+
|                                                                    |
|  [🏷️ Tags]    [contains]  ( #teste4 )                    [✏️] [🗑] |
|                                                                    |
|  +-----------------------------------------------------------------+
|  | [+ Add condition to search]               [+ Add group]         |
|  +-----------------------------------------------------------------+
+--------------------------------------------------------------------+
| [📁 Save as Smart Folder]            [ Cancel ]    [ 🔍 Search (2) ]|
+--------------------------------------------------------------------+
```

---

## 7. Arquitetura Técnica & Modularização dos Componentes

Para respeitar as diretrizes de código do projeto (`docs/guidelines/frontend-solid.md`, `max-lines < 300`, `complexity <= 10`, variáveis sem abreviações):

1. **`AdvancedSearchModal.tsx`**:
   - Atua estritamente como orquestrador do modal, conectando as stores (`useAdvancedSearch`, `useFilters`), lidando com atalhos de teclado globais (`Cmd+Enter` para buscar, `Esc` para fechar) e gerenciando o estado de expansão do Smart Folder Creator.
2. **`QueryEditor.tsx`**:
   - Renderiza a árvore recursiva com o componente `QueryGroupCard.tsx`.
3. **`QueryGroupCard.tsx`** *(Novo Subcomponente)*:
   - Encapsula o cartão de cada grupo, o cabeçalho com o Segmented Control de AND/OR, a lista de regras, o botão de exclusão do grupo e as ações de adicionar regra/subgrupo.
4. **`CriterionRow.tsx`** *(Extraído de `QueryEditor.tsx`)*:
   - Renderiza individualmente cada regra salva com badges ricos por tipo de dado, estado de hover suave e gatilhos de edição/exclusão com acessibilidade completa.
5. **`CriteriaBuilderRow.tsx`** *(Refatoração de `CriteriaBuilder.tsx`)*:
   - Linha compacta e moderna de composição com os seletores de Campo, Operador, Valor dinâmico e botão primário com ícone de `Check` ou `Plus`.
6. **`SmartFolderSavePopover.tsx` / `SmartFolderSaveBar.tsx`** *(Novo Subcomponente)*:
   - Componente isolado e desacoplado do rodapé principal para salvar a busca como Pasta Inteligente, com validação de nome em tempo real.
7. **`advanced-search-modal.css`**:
   - Estilização completa baseada em variáveis semânticas dos tokens do sistema, animações de entrada/saída suaves e ausência de seletores mágicos ou cores hardcoded.

---

## 8. Plano de Execução Passo a Passo

- [x] **Fase 1: Especificação e Documentação Aprovada**
  - Concluída auditoria crítica no arquivo `2026-11-07-advanced-search-design-refactor.md`.
- [x] **Fase 2: Componentes Atômicos de Apresentação**
  - Desenvolvido o seletor `LogicOperatorToggle.tsx` (Segmented Pill para AND/OR com suporte a modo compacto).
  - Criado `CriterionRow.tsx` com visualização rica por tipo (chips, swatches, badges de operador neutro, confirmação e cancelamento de edição).
  - Criado `CriteriaBuilderRow.tsx` com formulário inline limpo sob demanda.
- [x] **Fase 3: Cartão de Grupo e Árvore Booleana com Subgrupos Ilimitados**
  - Criado `QueryGroupCard.tsx` com elevação de superfícies dos tokens (`--bg-surface-1`, `--bg-surface-2`), guias laterais suaves e indentação controlada que suporta recursão profunda sem quebra de layout.
  - Implementada alternância sob demanda da linha de composição, eliminando o cabeçalho estático e duplicado `CRITERIA BUILDER (?)`.
- [x] **Fase 4: Rodapé e Fluxo de Smart Folder Unificado**
  - Harmonizado o cabeçalho modal no macOS (`modal.css`) com o botão de fechar à esquerda e o título posicionado imediatamente ao lado dele.
  - No modo `isSmartFolderMode`: cartão superior com ícone `FolderHeart`, título de detalhes e descrição, permitindo criar ou editar pastas inteligentes com memória muscular consistente.
  - No modo busca comum: botão `📁 Save as Smart Folder` na esquerda que expande suavemente o campo de nome sem poluir o fluxo principal de `Search Library`.
- [x] **Fase 5: Estilização S-Tier (`advanced-search-modal.css`)**
  - Integrados 100% dos tokens de cores, raios e sombras do design system.
  - Implementados anéis de foco acessíveis (`--shadow-focus`) e transições suaves.
- [x] **Fase 6: Testes, Lint e Validação Rigorosa**
  - Criada suíte de testes automatizados em `AdvancedSearchModal.spec.tsx` (6/6 testes aprovados).
  - Execução de `npm run lint` concluída com **0 erros e 0 warnings**.
  - Execução de `npx tsc --noEmit` concluída com **0 erros**.
  - Validação estrita de ausência de abreviações e variáveis de 1/2 caracteres.

---

## 9. Registro de Implementação e Validação Técnica Realizada

### 9.1 Modificações de Arquitetura e Código

1. **Alinhamento do Cabeçalho Modal no macOS (`src/components/ui/Modal/modal.css`):**
   - Atualizado o seletor `.ui-modal-header--macos` para `justify-content: flex-start; gap: var(--p-space-m);`.
   - O botão de fechar permanece posicionado à esquerda (ordem `-1`), e o título do diálogo é alinhado logo após o botão à esquerda, conforme convenção visual do macOS solicitada.

2. **Seletor Booleano Segmentado (`src/components/features/search/LogicOperatorToggle.tsx`):**
   - Substituição dos radio buttons isolados por um Segmented Control estilo pill (`[ All conditions (AND) ]` / `[ Any condition (OR) ]`).
   - Suporte a modo `isCompact` para subgrupos de profundidade avançada (`depth > 0`).

3. **Linha de Regra Ativa S-Tier (`src/components/features/search/CriterionRow.tsx`):**
   - Cada regra ativa agora exibe um ícone temático de acordo com o tipo de dado (Tags, Color, Folder, Date, Rating, Size, Number, Select, Text).
   - Operadores exibidos como badges neutros em caixa baixa, sem itálico cru.
   - Valores formatados semanticamente com fonte monoespaçada.
   - Estado de edição com botões acessíveis de confirmação (`Check`) e cancelamento explícito (`X`).

4. **Compositor Inline sob Demanda (`src/components/features/search/CriteriaBuilderRow.tsx`):**
   - Linha de rascunho com estilo de borda tracejada suave (`var(--border-default)`), contendo seletores de campo, operador e componente dinâmico de valor.
   - Botões de adicionar condição e cancelar.

5. **Cartão de Grupo Recursivo (`src/components/features/search/QueryGroupCard.tsx`):**
   - Renderização recursiva estruturada que permite **subgrupos aninhados ilimitados** sem deformar a largura do formulário.
   - Cores de superfície alternadas (`depth-surface-even` e `depth-surface-odd`) com trilhos laterais sutis (`border-left: 3px solid var(--border-default)`).
   - Estado vazio propositivo com botão `Add Condition`.
   - Barra de ações de rodapé com botões `Add Condition` e `Add Subgroup`.

6. **Orquestrador de Busca e Smart Folders (`src/components/features/search/AdvancedSearchModal.tsx`):**
   - Totalmente adaptado para suportar ambos os casos de uso:
     - **Modo Busca Avançada:** Foco na busca com botão primário `Search Library (${count})` e gaveta expansível opcional `Save as Smart Folder`.
     - **Modo Pasta Inteligente:** Cartão de detalhes (`Smart Folder Details`) com campo de nome, descrição amigável e botão primário `Save Changes` ou `Create Smart Folder`.

7. **Compatibilidade Retroativa (`src/components/features/search/CriteriaBuilder.tsx`):**
   - Encapsulado como wrapper que repassa para `CriteriaBuilderRow`, prevenindo quebras em módulos legados.

### 9.2 Resultados da Validação

- **TypeScript (`npx tsc --noEmit`):**
  - **0 erros** de compilação em todo o projeto.
- **Linter e Formatação (`eslint` + `prettier`):**
  - **0 erros e 0 avisos** com `--max-warnings 0`.
- **Testes Unitários e de Integração (`vitest`):**
  - `src/components/features/search/AdvancedSearchModal.spec.tsx`: **6/6 testes aprovados**.
  - `src/components/features/search/searchTree.spec.ts`: **5/5 testes aprovados**.
- **Regras Estritas de Nomenclatura:**
  - Nenhuma variável abreviada (`fieldConfiguration`, `operatorOption`, `criterion`, `metadataState`, `advancedSearch`, etc.).
  - Nenhuma variável de 1 ou 2 caracteres em todo o código e testes.

---

> **Conclusão:**  
> A reformulação da tela de Busca Avançada foi finalizada com sucesso. A experiência foi elevada a um padrão S-Tier, com layout limpo, cartões agrupados, eliminação de formulários redundantes, suporte nativo a subgrupos ilimitados e total integração tanto para busca quanto para criação/edição de Pastas Inteligentes.

