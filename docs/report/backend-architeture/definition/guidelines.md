# 🦀 Backend Guidelines (Rust + Tauri - Hexagonal)

Este documento dita os padrões de código, documentação e arquitetura exigidos para o backend do **Mundam**, estruturado em Rust sob os princípios da Arquitetura Hexagonal, CQRS e Orientação a Eventos (EDA).

---

## 🏗️ 1. Regras de Arquitetura (Hexagonal & CQRS)

### Separação de Camadas (Obrigatório)

O código nunca deve misturar IO de banco de dados, regras de negócio e chamadas de Tauri na mesma função.

- **`core/` (Domínio & Portas):** É o coração puro. Não sabe o que é Tauri, não toca no disco rigidamente. Aqui vivem as estruturas de dados, os Enums de Domínio (`AppError`, `DomainEvent`) e as Interfaces (Traits) como `TransactionalAssetLedger` e `FormatProvider`.
- **`feature/` (Application Layer / Handlers):** Contém a lógica orquestradora. Aqui residem os *Command Handlers* (Que alteram estado via Ledger) e os *Query Handlers* (Que leem dados velozmente via SQLx). Toda rota do Tauri `invoke()` deve repassar a chamada **imediatamente** para um Handler nesta pasta.
- **`infra/` (Adaptadores Ativos):** Implementa as Traits do Core. Se o `AssetLedger` exige um banco, o `SqliteLedgerAdapter` vive aqui, realizando as queries sujas do SQLx.
- **`delivery/` (Adaptadores Passivos):** Onde o mundo externo bate. Os comandos `#[tauri::command]` vivem em `delivery/tauri/`. O servidor de vídeo vive em `delivery/streaming/`. Nenhum deles contém regra de negócio, apenas convertem `AppResult<T>` em JSON e extraem parâmetros.

### Mutação Guiada por CQRS e EventBus

- **Comandos (Mutação):** Nunca dê `UPDATE` ou `INSERT` no SQLite diretamente nos Command Handlers. Repasse o payload (ex: `CreateAssetCommand`) para o **Asset Ledger**, que cuidará da Atomicidade.
- **Leitura (Query):** Leia à vontade e o mais rápido possível através da injeção de `Db` utilizando SQLx direto para as tabelas de Read-Model.
- **Reatividade:** Handlers não chamam Handlers. Handlers finalizam o trabalho reportando as mudanças de estado para o banco, e o Ledger publica um `DomainEvent` no EventBus. Outros interessados (como o JobScheduler de Thumbnails) ouvem as mudanças via `.subscribe()`.

---

## 📝 2. Padrões de Código (Clean Code)

### Nomenclatura Estrita

- **Jamais abrevie nomes de variáveis.** O código deve ser descritivamente explícito.
    ```rust
    // ✅ Correto
    let processed_thumbnail_buffer = ...;
    // ❌ Terminantemente Proibido
    let buf = ...;
    let thumb_buf = ...;
    ```
- **Funções e Variáveis**: `snake_case` (ex: `process_image`, `asset_id`).
- **Structs e Traits**: `PascalCase` (ex: `FormatProvider`, `AssetState`).
- **Constantes**: `SCREAMING_SNAKE_CASE` (ex: `MAX_WORKER_THREADS`).

### Princípios de Ouro

- **Single Responsibility Principle (SRP):** Cada função, adapter ou handler deve ter uma única responsabilidade. Funções monstras não passarão no Code Review.
- **Sem "Cleverness" Tóxica:** Prefira um código banal, explícito e longo a um macro obscuro ("Magic Rust") ou correntes de iteradores que ninguém entende.
- **Não Mascare Erros com `unwrap()`:** Use **sempre** o operador `?`. O uso de `.unwrap()` ou `.expect()` só é tolerado em configurações fixas no `main.rs` onde a quebra é intencional antes do boot.

### Assincronismo Limpo (Sem Freeze)

Nunca bloqueie o Executor do Tokio. Se uma função manipula o FileSystem via APIs síncronas brutas (C++ FFI, `image-rs`, FFmpeg nativo), jogue a carga para `spawn_blocking`.

```rust
// ✅ Delegando carga pesada de CPU / IO síncrono
pub async fn heavy_extraction() -> AppResult<()> {
    tokio::task::spawn_blocking(|| {
        perform_heavy_cpp_math_or_io()
    })
    .await
    .map_err(|_| AppError::ExtractionProcessTimeout)??;
    Ok(())
}
```

---

## 🛡️ 3. Tratamento Centralizado de Erros

A comunicação de falhas é contratual e ocorre via pacote `core/error/`.

- **Retorno Unitário:** Toda função falível da fronteira do Domínio e Tauri retorna `AppResult<T>` (Alias para `Result<T, AppError>`).
- **AppError Enum:** Centralizado em `thiserror`. Captura falhas primitivas (SQLx, std::io) por baixo, mas se apresenta ao desenvolvedor como Erro de Domínio.
- **Tradução FrontendSegura:** O `AppError` implementa `serde::Serialize` ativamente, mascarando rastros de pilha do SQL e Strings puras, devolvendo mapas JSON padronizados `{ "code": "ASSET_NOT_FOUND", "message": "..." }`.

---

## 📈 4. Tracing e Telemetria

Jamais polua o terminal com `println!`. O log oficial é feito pela crate `tracing`.

- Use `tracing::info!`, `debug!`, `warn!` ou `error!`.
- Adorne Command Handlers importantes com `#[tracing::instrument]` para que o OpenTelemetry rastreie as ramificações de tempo e parâmetros das funções automaticamente.

```rust
use tracing::{info, instrument, error};

#[instrument(skip(ledger))]
pub async fn ingest_assets(ledger: Arc<dyn TransactionalAssetLedger>, path: PathBuf) -> AppResult<()> {
    info!("Iniciando fluxo de ingestão de ativos");
    // ...
}
```

---

## 📚 5. Obrigações de Documentação (Rustdoc)

O código Rust do projeto precisa respirar via `rustdoc`. Se você assinou um Trait, comentou uma struct ou publicou um módulo, você deve explicá-lo com as barras triplas (`///`). 

**Regra Absoluta:** Explicar o **"Por que" (Motivação Arquitetural)** e não o **"O Que" (Descrição Trivial)** da mecânica.

1. **Sumário:** Linha direta descrevendo o objetivo do bloco de código.
2. **`# Arguments`:** Lista dos parâmetros obscuros.
3. **`# Errors` (Obrigatório!):** Se a função retorna `AppResult`, ela deve mapear explicitamente quando e por que ela atirará os erros previstos.

```rust
/// Resolve o provedor de extração adequado em complexidade O(1)
///
/// Compara a extensão do arquivo no `HashMap` registrado no Bootlace. Se falhar,
/// aciona a segunda rota de detecção profunda operando verificação de Magic Bytes
///
/// # Arguments
/// * `path` - Caminho canônico fornecido pelo SO.
/// * `header` - Array com os 512 primeiros bytes vitais. Utilize um file handle eficiente.
///
/// # Errors
/// Retorna `AppError::FormatNotSupported` caso o arquivo recuse identificação binária
/// absoluta após fallback O(N).
pub fn resolve(&self, path: &Path, header: &[u8]) -> Option<Arc<dyn FormatProvider>> { ... }
```

**Módulos (`mod.rs`)** devem conter um Big Picture no topo usando `//!` elucidando sua responsabilidade em face da Arquitetura Hexagonal.

## Organização dos adaptadores por domínio

Contratos relacionados ficam em pastas de domínio no `core`, por exemplo `core/processing/`, `core/duplicates/` e `core/trash/`. Executors contínuos ficam em `processing/workers/<domínio>/`, com testes junto da implementação.

No adaptador SQLite, fachadas de repositório delegam consultas a `infra/database/query_handlers/` e mutações a `infra/database/handlers/`. Agrupe responsabilidades extensas em subpastas como `matching/` e `processing/`. Uma consulta read-only não cria configurações padrão nem altera hierarquias. Leituras de validação dentro de uma transação de escrita permanecem no handler responsável por essa transação. Predicados compartilhados de elegibilidade ficam junto da fachada, evitando políticas divergentes entre contagem e admissão.

Ao mover módulos, atualize imports, testes, recursos incluídos e links da documentação. A revisão atual da estrutura e dos limites do plano está em [Organização dos módulos](../../../backend/workers/2026-11-04-22-16-worker-module-organization.md).
