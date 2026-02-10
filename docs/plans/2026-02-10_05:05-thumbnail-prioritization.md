# Implementação de Priorização de Thumbnails

**Data:** 2026-02-10 05:05
**Status:** Concluído

## Visão Geral

Este documento descreve as alterações realizadas para implementar o sistema de priorização de geração de thumbnails. O objetivo era garantir que as imagens atualmente visíveis na viewport (Grid, Masonry ou List) tivessem suas thumbnails geradas antes das demais na fila de processamento em background.

## Alterações Realizadas

### 1. Backend (Rust)

#### Novo Módulo de Estado (`src-tauri/src/thumbnail_priority.rs`)
Criamos uma estrutura thread-safe para armazenar os IDs prioritários.
- **Struct `ThumbnailPriorityState`**: Contém um `Mutex<HashSet<i64>>`.
- **Métodos**: `set_priority` (sobrescreve a lista atual), `get_priority`, `clear`.

#### Workers (`src-tauri/src/thumbnail_worker.rs`)
O worker de geração de thumbnails foi alterado para consultar a lista de prioridade antes da fila normal.
- **Lógica de Loop**:
    1. Verifica se existem IDs na lista de prioridade.
    2. Se sim, busca no banco de imagens apenas esses IDs que ainda não têm thumbnail (`get_images_needing_thumbnails_by_ids`).
    3. Se encontrar tarefas prioritárias, processa-as imediatamente.
    4. Se não houver prioridade (ou se já estiverem processadas), cai para o processamento sequencial padrão (`get_images_needing_thumbnails`).

#### Banco de Dados (`src-tauri/src/database.rs`)
- **Novo Método**: `get_images_needing_thumbnails_by_ids(ids: &[i64])`.
- Gera uma query dinâmica com `IN (...)` para buscar eficientemente apenas os itens visíveis que precisam de processamento.

#### Comandos e Registro (`src-tauri/src/thumbnail_commands.rs`, `src-tauri/src/lib.rs`)
- **Command**: `set_thumbnail_priority(ids: Vec<i64>)`.
- Registro do comando e inicialização do `ThumbnailPriorityState` na `lib.rs`.

#### Permissões (`src-tauri/permissions/main.toml`, `src-tauri/capabilities/default.json`)
- Adicionada permissão explícita `allow-set-thumbnail-priority` para permitir que o frontend invoque o comando.

### 2. Frontend (TypeScript/Solid)

#### Store (`src/core/store/libraryStore.ts`)
- **Action**: `setThumbnailPriority(ids: number[])`.
- Envia a lista de IDs para o backend via IPC.

#### Hook de Virtualização (`src/core/hooks/useVirtualViewport.ts`)
Este hook é utilizado pelas views baseadas em worker customizado (`VirtualGridView`, `VirtualMasonry`).
- **Effect**: Monitora alterações em `controller.visibleItems()`.
- **Debounce**: Implementado delay de 150ms para evitar chamadas excessivas durante scroll rápido.
- Chamada automática para `lib.setThumbnailPriority` com os IDs visíveis.

#### Componente Table (`src/components/ui/Table.tsx`)
A `VirtualListView` usa um componente `Table` separado que não usava o `viewportController` do worker.
- **Prop**: Adicionado `onVisibleItemsChange` à interface `TableProps`.
- **Lógica**: Implementado `createEffect` interno para monitorar o range visível e notificar o pai (com debounce).

#### Integração em Lista (`src/components/features/viewport/VirtualListView.tsx`)
- Conectado o evento `onVisibleItemsChange` da tabela à action `setThumbnailPriority` da library.

## Fluxo de Funcionamento

1. O usuário rola a página (Grid, Lista ou Masonry).
2. O frontend detecta quais itens estão visíveis na tela.
3. Após 150ms de inatividade (fim do scroll), o frontend envia os IDs visíveis para o backend: `set_thumbnail_priority`.
4. O backend atualiza o `HashSet` de prioridade.
5. O `ThumbnailWorker` (em sua próxima iteração do loop) detecta a lista de prioridade.
6. O worker processa esses itens primeiro.
7. O frontend recebe eventos `thumbnail:ready` e atualiza a interface instantaneamente.

## Benefícios

- **UX Aprimorada**: O usuário vê as imagens carregarem muito mais rápido enquanto navega, sem precisar esperar o processamento de todo o diretório anterior.
- **Eficiência**: O sistema não desperdiça recursos processando imagens que não estão sendo vistas no momento se houver demanda prioritária.
