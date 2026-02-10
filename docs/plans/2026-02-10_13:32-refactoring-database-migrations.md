# Refatoração: Migrações de Banco de Dados com `sqlx migrate`

> **Data**: 10 de Fevereiro de 2026
> **Status**: Concluído
> **Objetivo**: Substituir o gerenciamento manual e imperativo do esquema SQLite por um sistema de migrações declarativo e versionado usando `sqlx`.

---

## 📅 Contexto
Durante a análise técnica do backend (`src-tauri/`), identificamos que o arquivo `database.rs` continha uma dívida técnica significativa: a criação de tabelas e adição de colunas era feita manualmente via código Rust (`PRAGMA table_info` + `ALTER TABLE`). Isso dificultava a manutenção e a escalabilidade do esquema.

Como a aplicação ainda está em fase de desenvolvimento e não possui base de usuários ativos, optamos por consolidar o estado atual em uma única migração inicial limpa.

---

## 🚀 Passo a Passo da Implementação

### 1. Preparação do Ambiente
Criamos o diretório de migrações e movemos o arquivo de esquema original para lá, renomeando-o para seguir o padrão do `sqlx`.

- **Comand**: `mkdir -p src-tauri/migrations`
- **Ação**: Mover `src-tauri/src/schema.sql` para `src-tauri/migrations/20260210000000_initial_schema.sql`.

### 2. Atualização das Dependências
Adicionamos a feature `macros` à crate `sqlx` no `Cargo.toml`. Esta feature é obrigatória para que a macro `sqlx::migrate!` funcione, permitindo que as migrações sejam validadas em tempo de compilação e embutidas no binário.

```toml
# src-tauri/Cargo.toml
sqlx = { version = "0.8.6", features = ["sqlite", "runtime-tokio", "chrono", "macros"] }
```

### 3. Consolidação do Esquema Inicial
Editamos o arquivo `20260210000000_initial_schema.sql` para refletir o estado **final** desejado das tabelas, incluindo colunas que antes eram adicionadas via patches manuais:
- Tabela `images`: Adicionadas as colunas `format`, `rating`, `notes`, `added_at`, `thumbnail_attempts` e `thumbnail_last_error`.
- Garantia de que todos os índices e triggers FTS5 estivessem presentes desde o início.

### 4. Refatoração do Código Rust
Limpamos o método `Db::new` no arquivo `src-tauri/src/database.rs`.

- **Removido**: Lógica de `include_str!("schema.sql")` e execução manual do script.
- **Removido**: Blocos de código `if !column_names.contains(...)` que executavam `ALTER TABLE`.
- **Adicionado**: Chamada unificada para o runner de migrações:

```rust
// src-tauri/src/database.rs

// ... (configuração do pool) ...

// Inicializa o esquema e roda todas as migrações pendentes
sqlx::migrate!("./migrations")
    .run(&pool)
    .await?;

// ...
```

### 5. Verificação
Executamos o comando `cargo check` dentro da pasta `src-tauri` para validar se o compilador Rust conseguia localizar e processar as migrações corretamente.

---

## 🏆 Resultados
- **Código Limpo**: Redução de ~40 linhas de código imperativo no `database.rs`.
- **Segurança**: O `sqlx` agora garante que o esquema do banco de dados reflita exatamente o que está definido nos arquivos de migração.
- **Escalabilidade**: Para futuras alterações, basta criar um novo arquivo `.sql` na pasta `migrations/`, mantendo o histórico de evolução do banco.

---

## 📝 Notas para o Futuro
Para desenvolvedores trabalhando no projeto:
1. Se houver erro de inconsistência no banco local, delete o arquivo `mundam.db` para que o novo sistema de migrações crie tudo do zero (visto que estamos em dev).
2. Nunca altere uma migração já executada; sempre crie uma nova para mudanças subsequentes.
