# Relatório de Análise Técnica: Banco de Dados e Dados (Mundam)
> **Escopo**: `src-tauri/src/database.rs`, `src-tauri/src/schema.sql` (SQLite + SQLx)
> **Data**: 10 de Fevereiro de 2026

## 1. Visão Geral do Schema

O Mundam utiliza um banco de dados **SQLite** relacional, gerenciado através do ORM/Query Builder **SQLx** (Rust). A estrutura é normalizada e adequada para um DAM (Digital Asset Manager).

### Tabelas Principais
*   **`folders`**: Hierarquia de diretórios.
    *   **Árvore**: Utiliza `parent_id` (Adjacency List).
    *   **Paths**: Armazena o caminho completo (`path`) para facilitar queries, mas mantém a estrutura de árvore para navegação.
    *   **Performance**: Índices em `path` e `parent_id` garantem buscas rápidas.
*   **`images`**: A tabela central de ativos.
    *   **Metadados Ricos**: `width`, `height`, `size`, `format`, `rating`, `notes`.
    *   **Rastreamento**: `hash` (para duplicatas), `thumbnail_path` (cache), `thumbnail_last_error` (debug).
    *   **Timestamps Completo**: `created_at`, `modified_at`, `added_at`.
*   **`tags` & `image_tags`**: Sistema clássico de tagging Many-to-Many.
    *   **Hierarquia de Tags**: A tabela `tags` suporta aninhamento (`parent_id`), permitindo taxonomias complexas.
*   **`smart_folders`**: Armazena queries salvas em JSON, permitindo flexibilidade total para filtros dinâmicos.

### Recursos Avançados (SQLite)
*   **FTS5 (Full-Text Search)**: A tabela virtual `images_fts` implementa busca textual otimizada usando o tokenizador `trigram`. Isso permite buscas parciais (`LIKE %query%`) muito rápidas em `filename` e `notes`, mantendo-se sincronizada via Triggers (`AFTER INSERT/UPDATE/DELETE`).
*   **Performance Tuning**: O código ativa `WAL` (Write-Ahead Logging) e `synchronous = NORMAL`, configurações recomendadas para alta performance em aplicações desktop single-user.

---

## 2. Qualidade da Implementação (Rust)

### Pontos Fortes
*   **Queries Parametrizadas**: Quase todas as queries utilizam `sqlx::query(...).bind(...)`, prevenindo injeção de SQL e erros de sintaxe com caracteres especiais em nomes de arquivos.
*   **Tratamento de Conflitos**: O uso de `ON CONFLICT(path) DO UPDATE` no `save_image` é robusto para lidar com condições de corrida entre o Indexador e o Watcher.
*   **Lógica de "Adoção"**: A função `save_image` tenta inteligentemente detectar se um arquivo foi *movido* (mesmo tamanho e data de criação, mas caminho novo) antes de inseri-lo como novo, preservando ID, Tags e Rating.
*   **Manutenção**: Função `run_maintenance` executa `VACUUM` e `ANALYZE`, essencial para manter o SQLite rápido ao longo do tempo.

### Pontos Críticos (Dívida Técnica)
*   **Migrações Manuais**:
    *   **Problema**: O método `Db::new` verifica a existência de colunas manualmente (`PRAGMA table_info`) e executa `ALTER TABLE` uma a uma.
    *   **Risco**: Isso é propenso a falhas, difícil de testar e não escala. Se um usuário pular várias versões, o estado do banco pode ficar inconsistente.
    *   **Solução**: Adotar **SQLx Migrations**. O `sqlx-cli` gerencia pastas `migrations/` com arquivos `.sql` versionados (ex: `202402101200_init.sql`). O binário incorpora essas migrações e as aplica atomicamente na inicialização.

*   **Recursividade em Rust vs SQL**:
    *   **Problema**: A função `get_folder_counts_recursive` usa **Common Table Expressions (CTE)** (L314), o que é excelente. Porém, a função `ensure_folder_hierarchy` (L347) faz recursão no lado do Rust (`Box::pin`), o que pode ser lento para deep paths.
    *   **Melhoria**: Tentar resolver a criação de hierarquias também via CTE ou em uma única transação.

---

## 3. Análise de Índices e Performance

### Índices Existentes
*   `idx_images_path`, `idx_images_folder`: Essenciais para joins e lookups.
*   `idx_images_rating_created`, `idx_images_size`: Índices compostos e específicos para ordenação na UI (Sort by Rating, Size).

### Oportunidades
*   **Índice em `format`**: Não há índice na coluna `format`. Se o usuário filtrar muito por "Tipo de Arquivo" (ex: "Mostrar apenas Vídeos"), um índice aqui aceleraria a query.
*   **Índice em `added_at`**: Existe índice para `created_at` e `modified_at`. Como `added_at` ("Data de Importação") é um critério de ordenação comum, vale a pena indexá-lo.

---

## 4. Recomendações para o Banco de Dados

1.  **Refatoração de Migrações (Urgente)**:
    *   Instalar `sqlx-cli`.
    *   Mover o `schema.sql` atual para `migrations/20260210000000_initial_schema.sql`.
    *   Substituir a lógica manual no `Db::new` por `sqlx::migrate!().run(&pool).await`.

2.  **Otimização de Escrita**:
    *   Em operações de bulk insert (indexação inicial), envolver as chamadas em uma Transação (`BEGIN TRANSACTION ... COMMIT`). O SQLite é muito mais rápido com transações grandes do que com centenas de commits individuais.
    *   O `indexer` atualmente chama `save_image` um a um. Implementar um método `save_images_batch` que receba um vetor e use uma transação.

3.  **Segurança de Tipos**:
    *   Usar macros `sqlx::query!` em vez de strings onde possível para verificação de sintaxe em tempo de compilação (requer banco de desenvolvimento presente ou `sqlx-data.json`).

4.  **Busca por Cor (Futuro)**:
    *   Preparar o schema para a feature "Análise Cromática".
    *   Criar tabela `image_colors` (`image_id`, `r`, `g`, `b`, `prominence`) ou adicionar coluna `dominant_color_hex` na tabela `images`.

## 5. Conclusão

A camada de dados do Mundam é bem projetada para um aplicativo desktop local. O uso de SQLite + FTS5 + WAL garante performance e recursos de busca avançados sem a complexidade de servidores externos. A única fragilidade real é o sistema de migrações manuais, que deve ser saneado antes do lançamento da versão 1.0.

*   **Schema Design**: ⭐⭐⭐⭐⭐
*   **Integridade de Dados**: ⭐⭐⭐⭐⭐ (Foreign Keys com CASCADE)
*   **Manutenibilidade**: ⭐⭐⭐ (Devido às migrações manuais)
*   **Performance SQL**: ⭐⭐⭐⭐⭐ (Índices corretos, CTEs, FTS5)
