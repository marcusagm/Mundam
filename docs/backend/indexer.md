# Library Indexer

O Indexer é o serviço central de sincronização entre o sistema de arquivos e o banco de dados interno do Mundam. Ele garante que toda alteração feita pelo usuário no Finder, Explorer ou gerenciador de arquivos do sistema operacional seja refletida com fidelidade no catálogo de assets da aplicação.

---

## Overview

O Mundam monitora pastas definidas pelo usuário (bibliotecas) e mantém um inventário indexado de todos os ativos digitais (imagens, vídeos, áudios, documentos, modelos 3D) contidos nessas pastas. O Indexer é o componente responsável por:

1. **Scan inicial** — Varredura paralela de todos os arquivos ao adicionar uma biblioteca.
2. **Monitoramento em tempo real** — Detecção e processamento de alterações individuais no filesystem via watcher.
3. **Reparo diferencial** — Correção de assets com formatos ou thumbnails ausentes.

O Indexer **não é o watcher**. O watcher (`notify` crate) captura eventos brutos do SO e os repassa ao **Debouncer**, que os agrega e classifica. O Debouncer então emite `DomainEvents` limpos, e o **Indexer** consome esses eventos para executar operações atômicas no banco de dados via Ledger.

---

## Arquitetura

```text
┌────────────────────┐
│   Sistema Operac.  │
│  (FSEvents/inotify │
│  /ReadDirChangesW) │
└────────┬───────────┘
         │ eventos brutos
         ▼
┌────────────────────┐
│   notify (crate)   │
│   Watcher Layer    │
└────────┬───────────┘
         │ notify::Event
         ▼
┌────────────────────┐
│  EventDebouncer    │
│  (debouncer.rs)    │
│                    │
│  • Debounce 1.5s   │
│  • Deletion guard  │
│  • Rename heurístic│
│  • File vs Dir     │
└────────┬───────────┘
         │ DomainEvent
         ▼
┌────────────────────┐
│  LibraryIndexer    │
│                    │
│  ┌──────────────┐  │
│  │  indexer.rs   │  │  scan_directory, repair_library
│  ├──────────────┤  │
│  │event_handler │  │  handle_file_discovered, handle_path_deleted,
│  │     .rs      │  │  handle_path_renamed, handle_directory_*
│  ├──────────────┤  │
│  │ classifier   │  │  classify_file_entry (fan-out puro)
│  │     .rs      │  │
│  └──────────────┘  │
└────────┬───────────┘
         │ LedgerCommand
         ▼
┌────────────────────┐
│  Ledger (CQRS)     │
│  SQLite + Saga     │
└────────────────────┘
```

### Módulos

| Arquivo            | Responsabilidade                                                                 |
| ------------------ | -------------------------------------------------------------------------------- |
| `indexer.rs`       | Struct `LibraryIndexer`, pipeline de scan paralelo, repair, folder cache builder |
| `event_handler.rs` | Listener de eventos, handlers individuais (add, rename, delete, move recovery)   |
| `classifier.rs`    | Classificação stateless de arquivos para o fan-out do scan                       |

---

## Como funciona

### Scan Inicial (`scan_directory`)

Quando o usuário adiciona uma biblioteca, o Indexer executa um scan completo em 6 fases:

1. **Walk** — Um único `spawn_blocking` percorre toda a árvore de diretórios via `WalkDir`, coletando arquivos e pastas.
2. **Folder Cache** — Pastas são processadas sequencialmente em ordem de profundidade (pais antes de filhos). Pastas inexistentes são criadas no banco e cacheadas em um `HashMap<PathBuf, String>`.
3. **Comparison Cache** — Carrega `(path, size, modified_at)` de todos os assets já indexados para comparação diferencial O(1).
4. **Fan-out** — Cada arquivo é classificado por uma task assíncrona limitada por `Semaphore(200)`. A classificação compara metadados do disco com o cache e decide: `NewAsset`, `ExistingAsset`, ou `Error`.
5. **Consumer** — Os resultados são drenados em batches de 100 via `Ledger.BatchCreate`.
6. **Prune** — Assets e pastas que existem no banco mas não foram encontrados no disco são removidos.

### Monitoramento em Tempo Real (`event_handler`)

Após o scan inicial, o Indexer escuta o `EventBus` via `start_event_listener` e processa cada `DomainEvent` individualmente, **sem re-scan**:

| DomainEvent              | Handler                        | Operação                                            |
| ------------------------ | ------------------------------ | --------------------------------------------------- |
| `FsFileDiscovered`       | `handle_file_discovered`       | Move recovery (Fast Match) ou criação de asset      |
| `FsPathDeleted`          | `handle_path_deleted`          | Exclusão + cache em `recent_removals` por 5s        |
| `FsPathRenamed`          | `handle_path_renamed`          | Update de path (arquivo) ou rename/criação (pasta)  |
| `FsDirectoryDiscovered`  | `handle_directory_discovered`  | Criação de folder + scan interno                    |
| `FsDirectoryDeleted`     | `handle_directory_deleted`     | Remoção de folder (cascata via CTE recursiva no DB) |

### Move Recovery (Fast Match)

Quando um arquivo é deletado e, dentro de 5 segundos, um arquivo com o mesmo `size + created_at` aparece em outro lugar, o Indexer reconhece isso como um **Move** ao invés de `DELETE + CREATE`. Isso preserva ID, tags, cores extraídas e thumbnails.

---

## Operações Monitoradas

### Arquivos

| Operação                 | Evento Debounced             | Ação do Indexer                                                      |
| ------------------------ | ---------------------------- | -------------------------------------------------------------------- |
| Criação                  | `FsFileDiscovered`           | Cria asset com formato, size, folder_id                              |
| Exclusão                 | `FsPathDeleted`              | Remove asset do DB; cacheia metadados para move recovery             |
| Renomeação               | `FsPathRenamed`              | Update do path no registro existente (preserva ID/tags/thumbnails)   |
| Movimentação             | `FsPathRenamed` ou Fast Match| Update do path + folder_id (preserva tudo)                           |
| Envio para lixeira       | `FsPathDeleted` (com delay)  | Tratado como exclusão — o deletion guard de 3s confirma antes        |
| Restauração da lixeira   | `FsFileDiscovered`           | Fast Match recupera como move; caso contrário, cria novo asset       |

### Pastas

| Operação                 | Evento Debounced             | Ação do Indexer                                                      |
| ------------------------ | ---------------------------- | -------------------------------------------------------------------- |
| Criação                  | `FsDirectoryDiscovered`      | Cria folder + scan imediato do conteúdo                              |
| Criação (macOS Finder)   | `FsPathRenamed`              | "Pasta Sem Título" → nome real tratado como criação                  |
| Exclusão                 | `FsDirectoryDeleted`         | Remove folder via CTE recursiva (cascata para subpastas)             |
| Renomeação               | `FsPathRenamed`              | Update de path e nome no registro existente                          |
| Restauração da lixeira   | `FsDirectoryDiscovered`      | Cria folder + scan descobre assets internos com folder_id correto    |

### Arquivos Ignorados

| Arquivo/Padrão     | Motivo                                                                           |
| ------------------- | -------------------------------------------------------------------------------- |
| `.DS_Store`         | Metadado do Finder (macOS). Excluído pela heurística `is_likely_directory`       |
| `.` (dotfiles)      | Arquivos ocultos do sistema. Classificados como "não-diretório" pelo debouncer   |
| Extensões não suportadas | Filtrados pelo `FormatRegistry.is_supported_extension()` durante walk/classify |

---

## Reestruturação de raízes monitoradas

Ao cadastrar uma pasta, o Ledger vincula as raízes existentes cujo pai físico é
essa pasta, dentro da mesma transação de criação. Por exemplo, cadastrar
`C:\Library\Photos` antes de `C:\Library` mantém o ID de `Photos`, mas atualiza
seu `parent_id` para a nova pasta. A recarga de locations reflete essa relação
no `FolderTreeSidebarPanel`, sem reconstruir a hierarquia no frontend.

Se houver níveis intermediários (`C:\Library\Projects\Photos`), a adoção de
`Photos` acontece quando o scan, ordenado por profundidade, cadastra `Projects`.
Assim, os níveis físicos são preservados, sem ligar todos os descendentes
diretamente à nova raiz.

A comparação em `infra/database/folder_hierarchy.rs` aceita `/` e `\` no Windows,
incluindo raízes de unidade e caminhos UNC, sem alterar os caminhos armazenados.
Em Unix, `\` continua sendo um caractere literal de nome. Os nomes são comparados
por prefixo literal: `%` e `_` não atuam como curingas SQL.

Os testes de regressão do handler podem ser executados com
`cargo test --locked --lib folder_hierarchy` em `src-tauri`.

## Detalhes por Sistema Operacional

### macOS (FSEvents) — ✅ Testado

O macOS utiliza FSEvents para monitoramento em nível de árvore, o que é altamente escalável (não requer um watch por diretório). Porém, apresenta particularidades críticas.

#### Filesystem Case-Insensitive (APFS/HFS+)

O sistema de arquivos padrão do macOS é case-insensitive. `Path::exists()` do Rust retorna `true` para `subpasta` mesmo quando o arquivo no disco é `Subpasta`.

**Solução implementada:** A função `path_exists_exact()` no debouncer enumera o diretório pai via `std::fs::read_dir` e compara o nome exato byte a byte, garantindo detecção correta de renomeações case-only.

#### Criação de Pastas no Finder

Ao criar uma nova pasta via Finder, o macOS executa dois eventos em sequência:
1. `Create "Pasta Sem Título"`
2. `Rename "Pasta Sem Título" → "nome_real"`

O debouncer parea esses dois eventos como um `FsPathRenamed`. O indexer detecta que o `from_path` ("Pasta Sem Título") nunca existiu no banco e trata como criação de pasta nova com o nome final.

#### Renomeações como Delete + Create

O macOS frequentemente emite renomeações como dois eventos separados (`Remove` + `Create`) ao invés de um único `Rename`. O debouncer utiliza uma heurística multi-estágio para parear:

1. **Strict Match** — `size_bytes + created_at` (permite cross-folder)
2. **Fallback (Buffer)** — Mesmo pai + mesma extensão + mesmo tipo (dir vs file)
3. **Fallback (Recent Emitted)** — Pareamento tardio com creates já emitidos

Cada estágio valida que o candidato `to_path` **ainda existe no disco** (`path_exists_exact`), prevenindo que duas exclusões rápidas sejam incorretamente pareadas como rename.

#### Deletion Guard (3 segundos)

Quando um arquivo desaparece, o debouncer aguarda 3 segundos antes de confirmar a exclusão. Durante essa janela, se o arquivo reaparecer (restauração rápida da lixeira), o evento é convertido em `Created` ao invés de `Deleted`.

#### Arquivos `.DS_Store`

O macOS gera/modifica `.DS_Store` automaticamente em qualquer pasta que o Finder abre. A heurística `is_likely_directory()` no debouncer exclui dotfiles da classificação como diretório, prevenindo pareamentos espúrios.

---

### Windows (ReadDirectoryChangesW) — ⚠️ Não Validado

> [!WARNING]
> As informações abaixo são baseadas em conhecimento teórico e documentação da API do Windows. **Necessitam validação prática.**

#### API Nativa

O Windows utiliza `ReadDirectoryChangesW` que fornece eventos tipados:
- `FILE_ACTION_ADDED`
- `FILE_ACTION_REMOVED`
- `FILE_ACTION_RENAMED_OLD_NAME`
- `FILE_ACTION_RENAMED_NEW_NAME`
- `FILE_ACTION_MODIFIED`

A crate `notify` no Windows abstrai esses eventos, e os eventos de rename **são emitidos como pares**, o que simplifica significativamente a heurística no debouncer (tracker-based `From`/`To` pairing já implementado).

#### Filesystem Case-Insensitive (NTFS)

NTFS é case-insensitive por padrão (similar ao APFS/HFS+). A função `path_exists_exact()` deverá funcionar corretamente no Windows, mas precisa ser validada.

#### File Identity

O Windows oferece `FILE_ID_128` via `GetFileInformationByHandleEx`, que persiste entre renames/moves. Isso é um recurso valioso para move detection que não foi implementado ainda.

#### USN Journal

O Windows possui o USN Journal, que permite consultar o histórico de alterações mesmo quando o Mundam estava fechado. Este é um recurso extremamente importante para incremental indexing futuro.

#### Buffer Overflow

`ERROR_NOTIFY_ENUM_DIR` pode ocorrer quando muitas alterações acontecem simultaneamente. O debouncer deverá tratar isso com um full rescan automático.

---

### Linux (inotify) — ⚠️ Não Validado

> [!WARNING]
> As informações abaixo são baseadas em conhecimento teórico e documentação do inotify. **Necessitam validação prática.**

#### API Nativa

O Linux utiliza `inotify` com eventos granulares:
- `IN_CREATE`, `IN_DELETE`, `IN_MODIFY`
- `IN_MOVED_FROM`, `IN_MOVED_TO` (pareados por cookie)
- `IN_ATTRIB`

O `notify` crate emite renames como `RenameMode::Both` no Linux, fornecendo `from` e `to` em um único evento. O debouncer já suporta isso via tracker-based pairing.

#### Filesystem Case-Sensitive (ext4, btrfs)

Ao contrário do macOS e Windows, sistemas de arquivos Linux são **case-sensitive por padrão**. Renomear `subpasta` para `Subpasta` gera um evento de rename normal sem ambiguidade. A função `path_exists_exact()` é efetivamente um no-op nesses sistemas (o `Path::exists()` padrão já é exato).

#### Limite de Watches

Cada diretório monitorado consome um watch do inotify. Para bibliotecas com muitos subdiretórios, pode ser necessário ajustar:

```bash
echo 524288 | sudo tee /proc/sys/fs/inotify/max_user_watches
```

#### Event Queue Overflow

`IN_Q_OVERFLOW` pode ocorrer quando o buffer do kernel é excedido. O debouncer deverá tratar isso com um full rescan automático.

#### Filesystems de Rede

SMB e NFS podem não notificar alterações corretamente. Recomenda-se um reconciliation scan periódico para bibliotecas em volumes de rede.

---

## Checklist de Testes Manuais

Esta checklist deve ser executada após qualquer alteração no Indexer, Debouncer, ou Ledger. Cada item deve ser verificado com a aplicação em execução (`npm run tauri dev`).

### Arquivos

| #    | Cenário                                                                          | Status |
| ---- | -------------------------------------------------------------------------------- | ------ |
| A.1  | Adicionar novo arquivo à pasta monitorada                                        | —     |
| A.2  | Renomear arquivo (nome completamente diferente)                                  | —     |
| A.2.1| Renomear alterando apenas maiúsculas/minúsculas (ex: `foto.jpg` → `Foto.jpg`)   | —     |
| A.2.2| Verificar que tags, cores e thumbnail se mantêm sem duplicar registro no DB      | —     |
| A.3  | Mover arquivo de pasta monitorada para outra pasta monitorada                    | —     |
| A.3.1| Verificar que tags, cores e thumbnail se mantêm sem duplicar registro no DB      | —     |
| A.4  | Excluir arquivo (mover para lixeira)                                             | —     |
| A.4.1| Excluir vários arquivos simultaneamente (seleção múltipla + delete)              | —     |
| A.4.2| Excluir vários arquivos em sequência rápida (um por um, sem esperar)             | —     |
| A.4.3| Excluir vários arquivos em sequência lenta (um por um, aguardando remoção)       | —     |
| A.5  | Restaurar arquivo da lixeira                                                     | —     |
| A.5.1| Restaurar instantaneamente (antes do delete processar — <3s)                     | —     |
| A.5.2| Restaurar logo após a remoção da aplicação (3-8s)                                | —     |
| A.5.3| Restaurar depois de um tempo (>10s)                                              | —     |
| A.6  | Mover arquivo de pasta monitorada para pasta NÃO monitorada                      | —     |
| A.7  | Mover arquivo de pasta NÃO monitorada para pasta monitorada                      | —     |
| A.8  | Substituir arquivo existente (mesmo nome, conteúdo diferente)                    | —     |

### Pastas

| #    | Cenário                                                                          | Status |
| ---- | -------------------------------------------------------------------------------- | ------ |
| P.1  | Criar nova subpasta                                                              | —     |
| P.1.1| Verificar que pastas temporárias ("Pasta Sem Título") não aparecem no frontend   | —     |
| P.2  | Renomear subpasta (nome completamente diferente)                                 | —     |
| P.2.1| Renomear alterando apenas maiúsculas/minúsculas (ex: `Fotos` → `fotos`)         | —     |
| P.3  | Mover arquivo para dentro de uma subpasta                                        | —     |
| P.4  | Excluir subpasta vazia (mover para lixeira)                                      | —     |
| P.4.1| Excluir várias subpastas vazias simultaneamente                                  | —     |
| P.4.2| Excluir várias subpastas vazias em sequência rápida                              | —     |
| P.4.3| Excluir várias subpastas vazias em sequência lenta                               | —     |
| P.5  | Restaurar subpasta vazia da lixeira                                              | —     |
| P.5.1| Restaurar instantaneamente (<3s)                                                 | —     |
| P.5.2| Restaurar logo após remoção (3-8s)                                               | —     |
| P.5.3| Restaurar depois de um tempo (>10s)                                              | —     |
| P.6  | Excluir subpasta com arquivos dentro                                             | —     |
| P.7  | Restaurar subpasta com arquivos dentro                                           | —     |
| P.7.1| Verificar que assets internos ficam na subpasta (não na raiz)                    | —     |
| P.8  | Excluir subpasta aninhada (subpasta dentro de subpasta)                          | —     |
| P.9  | Mover subpasta para outra subpasta monitorada                                    | —     |
| P.10 | Monitorar uma pasta e depois cadastrar seu pai; conferir a árvore sem reiniciar | —     |
| P.10.1 | Cadastrar um ancestral vários níveis acima e conferir os níveis intermediários | —     |

---

## Futuras Melhorias

### 🔴 Alta Prioridade

| Melhoria                          | Descrição                                                                                                                                                                                                       |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **File Identity nativa**          | Utilizar `inode+dev` (Linux), `fileid+fsid` (macOS) e `FILE_ID_128` (Windows) para detecção precisa de renames/moves, eliminando a dependência da heurística baseada em `size+created_at`                       |
| **Reconciliation Scan periódico** | Scan incremental automático (ex: a cada 15 minutos) para detectar divergências causadas por overflow de eventos, sincronizadores de nuvem, ou alterações feitas com o Mundam fechado                             |
| **Tratamento de overflow**        | Detectar `IN_Q_OVERFLOW` (Linux), `ERROR_NOTIFY_ENUM_DIR` (Windows), `MustScanSubDirs` (macOS) e acionar rescan automático da árvore afetada                                                                   |
| **Validação em Windows e Linux**  | Executar a checklist de testes manuais completa em Windows (NTFS) e Linux (ext4), ajustando heurísticas conforme necessário                                                                                     |

### 🟡 Média Prioridade

| Melhoria                                  | Descrição                                                                                                                                                               |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Normalização Unicode (NFC/NFD)**        | Forçar `.nfc()` em todos os caminhos no ponto de entrada do debouncer para eliminar warnings residuais de "Normalization mismatch" em caminhos com acentos no macOS      |
| **USN Journal (Windows)**                 | Integrar com o USN Journal do NTFS para incremental indexing após reinicialização, eliminando a necessidade de full rescan ao iniciar                                    |
| **FSEvents Event ID (macOS)**             | Armazenar o último `event_id` processado para retomar o monitoramento de onde parou após reinicialização                                                                |
| **Detecção de substituição (Save-As)**    | Reconhecer o padrão `create temp → write → rename temp → delete original` usado por Photoshop, Office e Blender como uma modificação do arquivo original                |
| **Mover subpasta entre pastas monitoradas** | Atualmente as subpastas são renomeadas apenas quando o `from_path` existe no DB; o cenário de mover uma subpasta entre duas bibliotecas diferentes pode precisar de suporte adicional |

### 🟢 Baixa Prioridade

| Melhoria                                 | Descrição                                                                                                                                    |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| **Montagem/desmontagem de volumes**      | Detectar quando um HD externo ou volume de rede é desconectado e marcar assets como "indisponíveis" sem removê-los do banco                 |
| **Suporte a hard links e symlinks**      | Detectar quando múltiplos caminhos apontam para o mesmo arquivo e evitar duplicatas                                                          |
| **Monitoramento de permissões/atributos**| Detectar `chmod`/`chown` (Unix) ou alterações de ACL (Windows) e atualizar metadados de acesso                                               |
| **Batching inteligente para mudanças massivas** | Detectar operações tipo `git checkout` ou `unzip` que geram milhares de eventos e agrupá-las em um único rescan otimizado               |
| **Testes automatizados end-to-end**      | Criar suite de testes que simula operações de filesystem (via `tempdir`) e verifica a consistência do banco, eliminando dependência de testes manuais |
