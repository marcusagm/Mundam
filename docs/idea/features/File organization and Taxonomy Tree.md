# 🏷️ Fase 1: Organização e Taxonomia (Curto Prazo)

**Objetivo Central:** Implementar o ciclo completo de gerenciamento de etiquetas (criação, atribuição e navegação), permitindo que o usuário organize coleções massivas de imagens através de uma estrutura lógica e hierárquica.

## 1. Infraestrutura de Backend (Rust & SQLite)

A base de dados já possui as tabelas `tags` e `image_tags`, mas o backend em Rust precisa das funções de manipulação:

* [x] **Gerenciamento de Tags**: Implementar comandos Tauri para criar, renomear e excluir tags, suportando a coluna `parent_id` para permitir hierarquia (tags "pai e filho").
* [x] **Vínculo de Ativos**: Criar lógica de associação em lote (`batch tagging`) no `database.rs` para permitir que centenas de imagens sejam marcadas simultaneamente com alta performance.
* [x] **Persistência de Cores**: Ativar o suporte à coluna `color` na tabela de tags para diferenciação visual na interface.

## 2. Interface de Gerenciamento (SolidJS)

Transformar os componentes estáticos em ferramentas interativas de organização:

* [x] **Árvore de Tags na Sidebar**: Evoluir o `LibrarySidebar.tsx` para incluir uma lista navegável de tags com estados de expansão/colapso para categorias hierárquicas.
* [x] **Inspetor de Arquivos Ativo**: Substituir o marcador "Add tags..." no `FileInspector.tsx` por um sistema de entrada com auto-complete e exibição de badges para tags já atribuídas ao item selecionado.
* [x] **Sistema de Seleção Múltipla**: Implementar a lógica de seleção (Shift/Ctrl + Clique) no `ImageGrid.tsx` para habilitar ações de organização em massa.

## 3. Navegação e Filtragem

* [x] **Filtro por Contexto**: Ao clicar em uma tag na barra lateral, o `appStore.ts` deve disparar uma consulta ao SQLite para filtrar instantaneamente o grid de imagens.
* [x] **Pasta "Uncategorized"**: Implementar a lógica para listar automaticamente todas as imagens que não possuem vínculos na tabela `image_tags`, facilitando a triagem de novos arquivos.

## 4. UX & Interação

* [x] **Drag-and-Drop Interno**: Permitir que o usuário arraste imagens selecionadas do grid diretamente para uma tag na sidebar para realizar a atribuição.
* [x] **Feedback Visual**: Atualizar badges e contadores na interface em tempo real usando o modelo de eventos do Tauri, mantendo o compromisso de "Zero Lag".

---

## ✅ Critérios de Conclusão (Done When)

1. [x] O usuário pode criar uma estrutura de tags (ex: `Anatomia > Mãos`) via interface.
2. [x] É possível selecionar múltiplas imagens e atribuir uma tag a todas de uma vez.
3. [x] O painel lateral (`FileInspector`) exibe corretamente as tags de qualquer imagem selecionada e permite removê-las.
4. [x] Clicar em uma tag na barra lateral filtra o grid para exibir apenas os ativos correspondentes.
5. [x] A performance de filtragem permanece fluida (60fps) mesmo com milhares de tags e imagens no banco.


Este é um detalhamento completo do sistema de **Tags** do projeto **Elleven-Library**, inspirado na robustez do *Eagle.cool* e na flexibilidade do *Allusion*. Em sistemas de gerenciamento de ativos digitais (DAM), as tags não são apenas rótulos, mas sim o motor de metadados que permite a recuperação instantânea de informações.

---

## 1. Visão Geral do Sistema de Tags

Diferente de sistemas de arquivos tradicionais, as tags no Elleven-Library são **multidimensionais**. Um único arquivo pode possuir dezenas de tags, permitindo que ele "exista" em múltiplas categorias simultaneamente sem duplicar o espaço em disco.

### Atributos Técnicos

* [x] **Hierarquia (Parent/Child):** Tags podem ser aninhadas (ex: `Personagem > Humano > Guerreiro`).
* [x] **Grupos de Tags:** Organização lógica por cores e temas (ex: Grupo "Estilo" em Azul, Grupo "Projeto" em Verde).
* [x] **Persistência:** Tags são armazenadas no banco de dados SQLite local e, opcionalmente, em arquivos `metadata.json` para portabilidade.

---

## 2. Interface e Telas (UI)

O sistema de tags está distribuído em três zonas principais da interface:

### A. O Editor de Tags (Detail Inspector)

Localizado no painel lateral direito, é onde o usuário interage com as tags de um arquivo selecionado.

* [ ] **Campo de Entrada Inteligente:** Suporta *Fuzzy Search* (busca difusa). Ao digitar, o sistema sugere tags existentes.
* [ ] **Tags Sugeridas:** Uma lista baseada em IA ou frequência de uso aparece abaixo do campo para inserção em um clique.
* [x] **Visualização de Tags:** Cada tag é exibida como um "token" removível com a cor do seu respectivo grupo.

### B. O Gerenciador Global de Tags

Uma tela ou modal dedicado para manutenção da taxonomia.

* [x] **Árvore de Tags:** Visualização completa da hierarquia.
* **Ações em Massa:** Mesclar duas tags semelhantes ou renomear uma tag em toda a biblioteca.
* [x] **Estatísticas:** Mostra quantos arquivos estão vinculados a cada tag.

### C. Filtros de Tags (Sidebar)

Localizado no painel esquerdo, permite filtrar o Viewport atual.

* [ ] **Seleção Múltipla:** Filtrar por `Tag A` **E** `Tag B` ou `Tag A` **OU** `Tag B`.
* [ ] **Filtro de Exclusão:** Mostrar tudo, exceto arquivos com a `Tag C`.

---

## 3. Fluxos de Trabalho (UX)

### [ ] Fluxo 1: Atribuição de Tags (Single & Batch)

1. O usuário seleciona um ou mais ativos no **Viewport**.
2. Pressiona a tecla de atalho `T`.
3. O foco vai para o **TagEditor** no painel direito.
4. O usuário digita as tags separadas por vírgula ou espaço.
5. O sistema valida e cria novas tags ou vincula às existentes em tempo real.

### [x] Fluxo 2: Organização de Hierarquia

1. No **Gerenciador de Tags**, o usuário clica em uma tag e a arrasta para cima de outra.
2. O sistema pergunta se deseja tornar a tag arrastada uma "filha" (child).
3. Ao confirmar, a estrutura é atualizada. Buscar pela tag "Pai" agora pode, opcionalmente, incluir todos os resultados das tags "Filhas".

### [ ] Fluxo 3: Uso em Pastas Inteligentes (Smart Folders)

Este é o fluxo mais avançado de automação:

1. O usuário cria uma nova **Smart Folder**.
2. Define a regra: `Tag` | `Contém` | `Conceito`.
3. Define uma segunda regra: `Cor` | `Próxima de` | `Vermelho`.
4. A pasta agora exibirá automaticamente qualquer novo arquivo que receba a tag "Conceito" e tenha tons vermelhos, sem intervenção manual futura.

---

## 4. Estrutura de Software Sugerida (Componentes)

Para implementar isso na nova arquitetura do **Elleven-Library**, utilizaremos os seguintes componentes:

* **`TagProvider` (Context):** Gerencia o estado global de todas as tags e a lógica de busca.
* **`TagInput`:** Componente de input com *autocomplete* e lógica de criação de novos tokens.
* **`TagToken`:** Representação visual da tag (ícone, nome, botão de excluir).
* **`TagCloud`:** Renderizador de grupos de tags para a barra lateral de filtros.

> **Regra de Negócio:** Tags nunca devem ser excluídas automaticamente se o último arquivo vinculado for deletado, a menos que o usuário execute uma "Limpeza de Tags" manual, para evitar perda de taxonomia planejada.

Gostaria que eu escrevesse o código Typescript para o componente **`TagInput`** integrando com os comandos Tauri do seu backend?

[Tagging workflow in digital asset management](https://www.youtube.com/watch?v=0I_nZ4gDufY)
Este vídeo demonstra o fluxo de trabalho de uma ferramenta similar, ilustrando como a organização por tags e pastas inteligentes pode otimizar drasticamente a gestão de referências visuais.