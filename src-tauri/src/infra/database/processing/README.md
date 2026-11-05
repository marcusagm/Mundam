# Adaptador da fila de processamento

`mod.rs` implementa a porta `ProcessingRepository`. A fachada mantém o pool e delega:

- Leituras a [`query_handlers/processing`](../query_handlers/processing).
- Escritas a [`handlers/processing`](../handlers/processing).
- Predicados compartilhados a `selection.rs`.
- Regressões contra migrations e adaptadores reais a `tests/`.

O Ledger usa o mesmo fencing de tentativas dentro da transação que publica o resultado. Separar arquivos não separa transações: reset de fingerprints/comparações, claims e publicação conservam suas garantias de atomicidade. As queries de diagnóstico e os handlers de retry compartilham origem e elegibilidade.

Exemplo de composição:

```rust
use crate::infra::database::processing::SqliteProcessingRepository;
```

O domínio não depende deste adaptador. Consulte [organização e pendências](../../../../../docs/backend/workers/2026-11-04-19-02-worker-module-organization.md) antes de acrescentar queries ou handlers.
