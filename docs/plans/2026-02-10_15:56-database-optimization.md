# Refatoração: Otimização de Banco de Dados e Hierarquia

**Data**: 10 de Fevereiro de 2026
**Status**: ✅ Concluído
**Objetivo**: Eliminar recursão no lado do Rust para criação de pastas e adicionar índices para filtros e ordenação frequentes.

---

## 🛠️ Passo a Passo da Implementação

### 1. Criação de Migração de Performance
Adicionamos índices técnicos para acelerar filtros de UI e listagens por data.
- **Arquivo**: `src-tauri/migrations/20260210000001_add_performance_indices.sql`
- **Índices**: 
  - `idx_images_format` (Busca por tipo de arquivo).
  - `idx_images_added_at` (Ordenação "Recentes").

### 2. Refatoração da Camada de Dados (`folders.rs`)
Para permitir transações em métodos internos, alteramos a estrutura de execução:

- **Helpers Internos**: Criados `get_folder_id_internal` e `upsert_folder_internal` que aceitam `&mut SqliteConnection`. Isso resolveu o problema de propriedade (*ownership*) e falta do trait `Copy` em executores genéricos durante transações.
- **Public API**: Mantivemos `get_folder_by_path` e `upsert_folder` intactos, mascarando a complexidade de adquirir conexões do pool para os consumidores externos.

### 3. Eliminação de Recursão Assíncrona
A função `ensure_folder_hierarchy` usava `Box::pin` para se auto-chamar recursivamente.
- **Nova Lógica**: Implementação **iterativa**.
- **Processo**:
  1. Verifica existência rápida fora da transação.
  2. Inicia `tx = self.pool.begin()`.
  3. Divide o path em componentes (`/`, `parent`, `child`).
  4. Percorre os componentes garantindo a existência de cada nível usando os helpers internos dentro da transação.
  5. `tx.commit()`.

### 4. Validação Técnica
- **Compilação**: Executado `cargo check` para garantir integridade dos tipos e traits do SQLx.
- **Consistência**: Garantido que toda a hierarquia é criada de forma atômica (falha em um nível reverte todos os outros).

---

## 🏁 Resultados
- **Risco de Stack Overflow**: Zero (removida recursão).
- **Performance de I/O**: Otimizada (pastas profundas agora fazem apenas um commit de disco).
- **UX**: Filtragem por formato e ordenação por data agora utilizam índices nativos do SQLite.
