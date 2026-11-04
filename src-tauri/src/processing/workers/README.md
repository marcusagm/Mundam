# Workers de processamento

Este diretório concentra os executores de segundo plano iniciados por [`bootstrap/workers.rs`](../../bootstrap/workers.rs). Cada executor mantém seu loop, suas dependências e sua política de processamento; o bootstrap faz a composição e registra o lifecycle.

| Executor | Arquivo | Responsabilidade |
| --- | --- | --- |
| `ThumbnailWorker` | [`thumbnail_worker.rs`](thumbnail_worker.rs) | Thumbnails, metadados técnicos e prioridade do viewport |
| `ColorWorker` | [`color_worker.rs`](color_worker.rs) | Extração de paletas a partir das thumbnails |
| `DuplicateWorker` | [`duplicate_worker.rs`](duplicate_worker.rs) | Fingerprints automáticos e agendamento de comparações de duplicados |
| `AutoEmptyTrashWorker` | [`auto_empty_trash_worker.rs`](auto_empty_trash_worker.rs) | Manutenção periódica da lixeira conforme as configurações existentes |

## Padrão de organização

- Implementação em `<responsabilidade>_worker.rs`, com tipo público `<Responsabilidade>Worker`.
- Testes específicos em `<responsabilidade>_worker_tests.rs`, como módulo do próprio executor.
- Fixtures compartilhadas em [`test_support.rs`](test_support.rs).
- Exportação dos tipos públicos pela fachada [`mod.rs`](mod.rs), usada pelo bootstrap.

Exemplo de importação:

```rust
use crate::processing::workers::{ColorWorker, DuplicateWorker, ThumbnailWorker};
```

Os workers de extração usam a admissão compartilhada de [`ProcessingControl`](../../core/processing_control.rs), o bookkeeping de [`ProcessingRepository`](../../core/processing.rs) e o token de encerramento do lifecycle. Pausa, retomada, prioridades e retries continuam obedecendo a esses contratos.

A limpeza automática da lixeira é um executor de manutenção. Ela conserva sua configuração de habilitação e retenção e não é adicionada aos controles de extração da statusbar por esta reorganização.

## Limite entre workers e features

`processing/workers` contém o consumo contínuo e a execução técnica em segundo plano. A feature [`duplicates`](../../feature/duplicates) conserva comandos, consultas, regras de comparação, funções de hashes perceptuais e a coordenação comum com scans manuais. Um worker pode consumir essas capacidades sem tornar a feature responsável por iniciar um loop automático.

O nome `events.rs` foi removido da feature de duplicados: eventos apenas acordam o `DuplicateWorker`, cujo processamento efetivo já usa a fila persistida. Não há módulo de compatibilidade ou uma segunda cópia do executor no caminho antigo.

O cálculo automático e manual de fingerprints usa o mesmo [`FingerprintProcessor`](../../feature/duplicates/fingerprint_processor.rs), com extração técnica em [`processing/media/fingerprint.rs`](../media/fingerprint.rs). Os chamadores conservam a admissão pelo coordenador comum e pelos controles da statusbar. Elegibilidade, tentativas e persistência seguem a mesma política nos dois caminhos, descrita em [Processamento compartilhado de fingerprints](../../../../docs/backend/fingerprint-processing.md).

Os três processadores usam reservas persistentes com heartbeat e publicação transacional pelo Ledger. Thumbnails e previews produzidos pelo worker têm nomes por tentativa. Os contratos, a recuperação e os limites estão em [Reservas persistentes e publicação transacional](../../../../docs/backend/processing-leases.md). Comparações exatas e visuais também usam [jobs persistidos](../../../../docs/backend/duplicate-matching-jobs.md), com snapshot da política, publicação por fase e recuperação após crash. A capacidade de thumbnails, cores, fingerprints e comparações é compartilhada pelo [orçamento de operações](../../../../docs/backend/worker-resource-budget.md). Os limites de mídia aplicam sem reinício, com drenagem das operações admitidas e admissão antes das reservas persistentes.
