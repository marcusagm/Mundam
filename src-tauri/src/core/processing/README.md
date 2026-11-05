# Contratos de processamento

O módulo concentra contratos independentes de Tauri, SQLite e arquivos gerados:

- `mod.rs`: etapas, tickets, resultados, outcomes e porta de persistência.
- `budget.rs` e `budget_tests.rs`: admissão compartilhada de operações e drenagem.
- `control.rs`: atividade e intenção de pausa da sessão.
- `diagnostics.rs`: causas seguras, escopos e confirmação de retry.

Contratos específicos da comparação ficam em [`duplicates/matching.rs`](../duplicates/matching.rs). Adaptadores SQL ficam em [`infra/database/processing`](../../infra/database/processing), e executors em [`processing/workers`](../../processing/workers).

Exemplo de dependência:

```rust
use crate::core::processing::{ProcessingKind, ProcessingRepository};
use crate::core::processing::control::{ProcessingControl, WorkerKind};
```

Invariantes e pendências estão na [revisão de organização](../../../../docs/backend/workers/2026-11-04-19-02-worker-module-organization.md).
