# 2026-02-10_14:18-database-type-safety.md

## Contexto
Dando continuidade à otimização do banco de dados, implementamos a migração estratégica de consultas baseadas em strings para macros `sqlx::query!`. O objetivo principal é garantir que erros de sintaxe SQL ou mudanças no esquema da tabela sejam detectados em tempo de compilação, e não em tempo de execução.

## Problema Resolvido
Como a aplicação é local e o banco de dados é criado dinamicamente, não havia um banco estático para o compilador do Rust validar as queries durante o `cargo build`. Isso criava uma vulnerabilidade onde renomear uma coluna em uma migração poderia quebrar o backend silenciosamente.

## Implementação Realizada

### 1. Setup do Ambiente de Compilação
- **Banco de Referência (`dev.db`)**: Criado um banco de dados SQLite local na pasta `src-tauri` para servir de "espectro" para o compilador.
- **Sincronização de Esquema**: O esquema inicial foi aplicado ao `dev.db` manualmente via comando `sqlite3`.
- **Configuração SQLx**: Criado arquivo `.env` com a variável `DATABASE_URL=sqlite:dev.db`, permitindo que o `sqlx` acesse o esquema durante a fase de compilação das macros.

### 2. Migração para Macros `sqlx::query!`
Foram refatoradas as seguintes funções no `database.rs`:
- `update_image_rating`
- `update_image_notes`
- `get_folder_path`
- `get_folder_by_path`
- `get_images_needing_thumbnails`
- `record_thumbnail_error`
- `update_thumbnail_path`
- `clear_thumbnail_path`
- `delete_folder`
- `get_all_root_folders`
- `get_folders_under_root`
- `get_folder_counts_recursive` e `get_folder_counts_direct`

### 3. Ajustes de Tipagem Estrita
- **Força de Não-Nulidade**: Uso da sintaxe `AS "coluna!"` para converter tipos `Option<T>` em `T` onde a coluna é garantidamente preenchida (ex: PKs e colunas `NOT NULL`).
- **Compatibilidade de Datas**: Mantido o uso de `query_as` em `get_file_comparison_data` para evitar conflitos de tipos entre structs `OffsetDateTime` (usadas internamente pelo SQLx) e `DateTime<Utc>` (usadas pelo resto da aplicação).

## Resultados Obtidos
- **Segurança de Tipos**: O compilador agora valida cada consulta SQL contra o schema real.
- **Autocompletividade**: Melhor suporte da IDE para os campos retornados pelas queries (agora retornam structs anônimas tipadas).
- **Robustez de Contrato**: Qualquer mudança no banco de dados agora exige a atualização imediata do código Rust correspondente.

## Verificação
- [x] Criação do ambiente `.env` e `dev.db`.
- [x] Migração dos blocos de código.
- [x] Sucesso na execução de `cargo check`.
