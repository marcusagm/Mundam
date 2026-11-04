# Workers de processamento

Este diretório concentra os executores de segundo plano iniciados por [`bootstrap/workers.rs`](../../bootstrap/workers.rs). Cada executor mantém seu loop, suas dependências e sua política de processamento; o bootstrap faz a composição e registra o lifecycle.

| Executor | Arquivo | Responsabilidade |
| --- | --- | --- |
| `ThumbnailWorker` | [`thumbnails/mod.rs`](thumbnails/mod.rs) | Thumbnails, metadados técnicos e prioridade do viewport |
| `ColorWorker` | [`colors/mod.rs`](colors/mod.rs) | Extração de paletas a partir das thumbnails |
| `DuplicateWorker` | [`duplicates/mod.rs`](duplicates/mod.rs) | Fingerprints automáticos e agendamento de comparações de duplicados |
| `AutoEmptyTrashWorker` | [`trash/mod.rs`](trash/mod.rs) | Manutenção periódica da lixeira conforme as configurações existentes |

## Padrão de organização

- Cada responsabilidade tem uma pasta (`thumbnails`, `colors`, `duplicates`, `trash`) e sua implementação em `mod.rs`, com tipo público `<Responsabilidade>Worker`.
- Testes específicos ficam em `tests.rs` junto da implementação, como módulo do próprio executor.
- Fixtures compartilhadas em [`test_support.rs`](test_support.rs).
- Exportação dos tipos públicos pela fachada [`mod.rs`](mod.rs), usada pelo bootstrap.

Exemplo de importação:

```rust
use crate::processing::workers::{ColorWorker, DuplicateWorker, ThumbnailWorker};
```

Os workers de extração usam a admissão compartilhada de [`ProcessingControl`](../../core/processing/control.rs), o bookkeeping de [`ProcessingRepository`](../../core/processing/mod.rs) e o token de encerramento do lifecycle. Pausa, retomada, prioridades e retries continuam obedecendo a esses contratos.

A limpeza automática da lixeira é um executor de manutenção. Ela conserva sua configuração de habilitação e retenção e não é adicionada aos controles de extração da statusbar por esta reorganização.

## Limite entre workers e features

`processing/workers` contém o consumo contínuo e a execução técnica em segundo plano. A feature [`duplicates`](../../feature/duplicates) conserva comandos, consultas, regras de comparação, funções de hashes perceptuais e a coordenação comum com scans manuais. Um worker pode consumir essas capacidades sem tornar a feature responsável por iniciar um loop automático.

O nome `events.rs` foi removido da feature de duplicados: eventos apenas acordam o `DuplicateWorker`, cujo processamento efetivo já usa a fila persistida. Não há módulo de compatibilidade ou uma segunda cópia do executor no caminho antigo.

O cálculo automático e manual de fingerprints usa o mesmo [`FingerprintProcessor`](../../feature/duplicates/fingerprint_processor.rs), com extração técnica em [`processing/media/fingerprint.rs`](../media/fingerprint.rs). Os chamadores conservam a admissão pelo coordenador comum e pelos controles da statusbar. Elegibilidade, tentativas e persistência seguem a mesma política nos dois caminhos, descrita em [Processamento compartilhado de fingerprints](../../../../docs/backend/fingerprint-processing.md).

Os três processadores usam reservas persistentes com heartbeat e publicação transacional pelo Ledger. Thumbnails e previews produzidos pelo worker têm nomes por tentativa. Os contratos, a recuperação e os limites estão em [Reservas persistentes e publicação transacional](../../../../docs/backend/processing-leases.md). Comparações exatas e visuais também usam [jobs persistidos](../../../../docs/backend/duplicate-matching-jobs.md), com snapshot da política, publicação por fase e recuperação após crash. A capacidade de thumbnails, cores, fingerprints e comparações é compartilhada pelo [orçamento de operações](../../../../docs/backend/worker-resource-budget.md). Os limites de mídia aplicam sem reinício, com drenagem das operações admitidas e admissão antes das reservas persistentes.

O progresso de scans manuais alimenta o snapshot por um canal de último valor, sem eventos por asset. A comparação prepara variantes de hashes por snapshot e evita subgrades de crop sem corroboração. Medições por fase, benchmark reproduzível e o escopo da evolução incremental estão em [Progresso e custo do matching](../../../../docs/backend/worker-progress-matching-performance.md).
