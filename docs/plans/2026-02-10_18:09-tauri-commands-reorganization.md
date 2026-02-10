# 2026-02-10_18:09 - Reorganização de Comandos Tauri

Este documento detalha o processo de refatoração para organizar os comandos do Tauri, que anteriormente estavam dispersos na raiz do projeto `src-tauri/src/`. A nova estrutura adota um padrão modular baseado em domínios semânticos (Option B: Library Umbrella).

---

## 🎯 Objetivo
Organizar o backend seguindo as diretrizes de [backend-rust.md](../guidelines/backend-rust.md), agrupando comandos relacionados sob módulos específicos para melhorar a manutenibilidade e escalabilidade.

---

## 🏗️ Nova Estrutura de Diretórios

```text
src-tauri/src/
├── library/                 # Domínio Principal da Biblioteca
│   ├── mod.rs               # Registro do módulo library
│   └── commands/            # Comandos de gerenciamento
│       ├── mod.rs           # Registro de sub-módulos de comandos
│       ├── tags.rs          # Antigo tag_commands.rs
│       ├── folders.rs       # Antigo location_commands.rs
│       ├── metadata.rs      # Antigo metadata_commands.rs
│       ├── smart_folders.rs # Antigo smart_folder_commands.rs
│       ├── formats.rs       # Antigo format_commands.rs
│       └── indexing.rs      # Extraído de lib.rs (start_indexing)
├── media/                   # Domínio de Mídia
│   ├── mod.rs               # Registro do módulo media
│   └── commands.rs          # Antigo audio_commands.rs
├── settings/                # Domínio de Configurações
│   ├── mod.rs               # Registro do módulo settings
│   └── commands.rs          # Antigo settings_commands.rs
└── thumbnails/              # Domínio de Thumbnails (Unificado)
    ├── mod.rs               # Atualizado para incluir commands
    └── commands.rs          # Antigo thumbnail_commands.rs
```

---

## 🛠️ Passo a Passo da Refatoração

### 1. Criação da Estrutura Base
- Criação dos diretórios `library/commands`, `media`, e `settings`.

### 2. Migração de Arquivos
- Movimentação de todos os arquivos de comandos da raiz para seus novos destinos.
- Extração do comando `start_indexing` do arquivo `lib.rs` para `library/commands/indexing.rs` para manter o padrão de "Thin Commands".

### 3. Implementação dos Entry Points (`mod.rs`)
- Criação de `library/mod.rs` registrando o submódulo `commands`.
- Criação de `library/commands/mod.rs` exportando todos os tipos de comandos da biblioteca.
- Criação de `media/mod.rs` e `settings/mod.rs`.
- Atualização do `thumbnails/mod.rs` para expor o novo submódulo `commands`.

### 4. Atualização do `lib.rs`
- Remoção das declarações `mod` para os arquivos antigos dispersos.
- Adição das novas declarações: `pub mod library`, `mod media`, `mod settings`.
- Atualização radial do `invoke_handler`. Exemplo de mudança:
  - Antigo: `tag_commands::create_tag`
  - Novo: `library::commands::tags::create_tag`
- Restauração de imports necessários (como `crate::indexer::Indexer`) que permanecem em uso no bloco `setup`.

### 5. Limpeza e Validação
- Remoção de todos os arquivos `.rs` de comandos da raiz (`src-tauri/src/*.rs`).
- Execução de `cargo check` para garantir que todas as referências cruzadas e resoluções de nomes estão corretas.

---

## ✅ Resultados
- **Organização**: A raiz do projeto está muito mais limpa, contendo apenas módulos core (`db`, `error`, `ffmpeg`, `protocols`, etc.).
- **Descoberta**: É mais intuitivo localizar onde um comando específico reside baseado em seu domínio.
- **Padrão**: O padrão de arquitetura modular iniciado pelo módulo `transcoding` agora é a regra em todo o backend.
