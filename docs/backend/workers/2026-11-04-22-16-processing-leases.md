# Reservas persistentes e publicação transacional

> **Estado consolidado:** ciclo encerrado e validado pelo usuário em Windows/macOS. Este documento preserva o relato de sua etapa; referências a próximas entregas e validações pendentes devem ser lidas nesse contexto histórico. Consulte o [encerramento](2026-11-04-22-16-workers-closure.md) e as [evoluções futuras separadas](2026-11-04-22-16-workers-future-evolutions.md).


**Implementação:** 3 de novembro de 2026.

Thumbnails, cores e fingerprints agora usam uma reserva exclusiva persistida por asset e estágio. O resultado e o desfecho da tentativa são publicados na mesma transação SQLite, evitando que uma execução antiga substitua dados de uma solicitação mais recente.

## Reserva e recuperação

A migration `20261103010000_processing_leases.sql` acrescenta `lease_id`, `lease_expires_at` e o ponteiro do preview 3D publicado em `processing_tasks`. Os estados anteriores e seus retries são preservados; não há reprocessamento obrigatório do catálogo.

`ProcessingRepository::begin` seleciona e reserva em `BEGIN IMMEDIATE`. Cada admissão recebe um UUID novo, mesmo quando a geração e a origem são iguais às de uma tentativa expirada. Uma reserva válida fica fora da seleção de pendências. Outra solicitação explícita ou mudança de origem invalida a propriedade anterior.

| Operação | Regra |
| --- | --- |
| Aquisição | Prazo de 120 segundos, exclusividade por `(asset_id, kind)` |
| Renovação | A cada 30 segundos durante a execução; exige reserva ainda válida, geração atual e origem ativa |
| Liberação | Condicionada ao UUID e à geração; não consome tentativa de erro |
| Encerramento normal | Libera a reserva após sucesso, erro ou cancelamento |
| Unwinding/abort do worker | O guard interrompe o heartbeat e solicita liberação assíncrona |
| Crash ou falha de liberação | A reserva expira; a consulta periódica permite nova aquisição |
| Tentativa superada | Renovação, commit e desfecho não alteram a nova propriedade |

O guard [`ProcessingLease`](../../../src-tauri/src/processing/lease.rs) é usado nos três processadores. Falha de renovação encerra a renovação e libera aquela propriedade; o resultado só será aceito se a validação transacional ainda o permitir. Operações nativas já iniciadas não são interrompidas à força. Se continuarem depois da perda da reserva, seus resultados não podem substituir os da nova tentativa.

A pausa continua sendo um controle da sessão. Ela impede novas admissões e permite que as operações admitidas terminem com seus heartbeats ativos. Não muda a política de até três falhas e backoff de cinco e trinta segundos. Reservas válidas são excluídas da contagem de pendências; a atividade exibida pela statusbar continua sendo a atividade local, sem um painel de executores de outros processos.

## Commit protegido

[`TransactionalAssetLedger::commit_processing`](../../../src-tauri/src/core/ledger/port.rs) recebe um ticket e um `ProcessingResult`. O adaptador SQLite inicia uma transação de escrita e delega ao [handler de processamento](../../../src-tauri/src/infra/database/handlers/processing_handler.rs), que:

1. Confere UUID da reserva, prazo, geração, origem e disponibilidade do asset dentro da transação.
2. Valida a correspondência entre estágio, asset e payload.
3. Persiste thumbnail, metadados, preview, paleta ou fingerprint pelos adaptadores existentes.
4. Conclui a tentativa e libera sua reserva na mesma transação. A geração de thumbnail também enfileira cores atomicamente.
5. Retorna os eventos para publicação somente após o commit.

Se a validação falhar, retorna `false` e não publica eventos. Falhas de escrita desfazem resultado, desfecho e solicitações dependentes juntos. Uma falha ao entregar eventos após commit é registrada no log; ela não transforma um resultado já publicado em erro de extração. Polling continua recuperando os resultados persistidos.

Exemplo após obter admissão nos controles e executar a extração:

```rust,ignore
let accepted = ledger
    .commit_processing(&ticket, ProcessingResult::Colors(extracted_colors))
    .await?;
if !accepted {
    // Ownership was superseded or expired; the current attempt retains its data.
}
```

`finish` permanece disponível para falhas e casos sem resultado. Também exige a reserva válida. Os resultados dos workers usam o commit protegido; adaptadores de Ledger que não o implementam falham explicitamente, sem recorrer a gravações sem proteção.

## Cache por tentativa

Os workers deixam de sobrescrever `asset-id.webp` e `asset-id.glb`. Produzem nomes imutáveis como `asset-id.lease-uuid.webp` e `asset-id.lease-uuid.glb`. O caminho da thumbnail fica em `assets.thumbnail_path`; o preview 3D publicado fica em `processing_tasks.preview_path`.

O protocolo `asset://` consulta esses ponteiros para servir thumbnails e previews 3D. Previews 3D legados conservam o fallback para `asset-id.glb`. As rotas e os comandos IPC existentes são preservados. Previews de imagem gerados sob demanda pelo protocolo continuam em seu fluxo anterior; esta etapa protege os artefatos produzidos pelos workers.

A limpeza ao concluir uma tentativa consulta diretamente seus três nomes possíveis, sem varrer todo o diretório por asset. A coleta geral ocorre na inicialização do loop de thumbnails e em intervalos de 60 segundos, quando o ciclo de orquestração avança. Ela enumera os arquivos antes de consultar os ponteiros publicados e reservas válidas, preserva arquivos publicados ou ainda reservados e remove os demais.

Um crash pode deixar arquivos não publicados até a reconciliação seguinte; eles não são servidos pelo protocolo. Se uma operação antiga produzir um arquivo depois de uma coleta, a coleta posterior o remove. A remoção de pastas/assets também reconhece os nomes por tentativa e remove seus caches. Os originais nunca são candidatos dessa limpeza.

## Limites

- A revisão da origem usa os dados do catálogo: caminho, tamanho e `modified_at`. Não fornece snapshot físico atômico do arquivo durante a leitura.
- Os arquivos são gravados antes de seus ponteiros. SQLite e filesystem não compartilham transação; arquivos não publicados podem permanecer após crash e ser recolhidos na reconciliação.
- As comparações e sua recuperação após crash foram implementadas na etapa posterior de [comparações persistentes](2026-11-04-22-16-duplicate-matching-jobs.md). Orçamento global de recursos e preferência de pausa persistente continuam fora desta entrega. O coordenador local permanece, acompanhado de reservas de comparação entre processos.
- A expiração usa o relógio do SQLite. Não há interrupção forçada de extratores ou garantia de processamento físico exatamente uma vez; há exclusividade de publicação sob a propriedade válida.

## Validação

Os testes focados usam migrations e SQLite reais, incluindo banco em arquivo fechado e reaberto. Cobrem exclusividade entre conexões, expiração e retomada, proteção contra tickets antigos com a mesma geração, renovação/liberação por guard, rejeição de assets inativos e origem superada, rollback provocado na conclusão, eventos apenas após commit, publicação de fingerprints e limpeza do cache sem tocar nos originais.

Validação final no Windows:

- `./scripts/test-backend-windows.ps1 processing_`: **45 testes aprovados**, incluindo geração/regeneração real pelo worker com arquivos imutáveis e preservação do original.
- Executável de testes com filtro `folder_`: **16 regressões aprovadas**, incluindo remoção de caches, cascade e integridade dos duplicados.
- Executável com filtro `delivery::protocols::asset::tests`: **três testes de parsing de URIs aprovados** para os formatos Windows/macOS. Isso não representa execução no macOS nem teste HTTP completo do protocolo.
- `git diff --check` e links locais da documentação: sem erros.

Não houve validação visual nem execução no macOS nesta etapa. O relatório histórico `05-job-scheduler-workers.md` permanece inalterado.
