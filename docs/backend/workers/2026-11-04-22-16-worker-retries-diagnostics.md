# Retry por worker e diagnóstico de extração

> **Estado consolidado:** ciclo encerrado e validado pelo usuário em Windows/macOS. Este documento preserva o relato de sua etapa; referências a próximas entregas e validações pendentes devem ser lidas nesse contexto histórico. Consulte o [encerramento](2026-11-04-22-16-workers-closure.md) e as [evoluções futuras separadas](2026-11-04-22-16-workers-future-evolutions.md).


**Implementação:** 4 de novembro de 2026.

O painel de atividade permite repetir falhas esgotadas ou reavaliar resultados sem suporte, com confirmação das quantidades realmente enfileiradas. Os motivos são agrupados por códigos seguros, sem enviar logs, comandos ou caminhos de assets à interface.

## Ações e políticas

O comando `retry_processing_worker(worker, scope)` aceita os três workers controlados e dois escopos:

| Escopo | Seleção | Comportamento |
| --- | --- | --- |
| `failed` | Tentativas terminais `failed` do estágio | Reinicia geração e tentativas. Para duplicados, inclui comparações exatas/visuais que falharam. |
| `unsupported` | Resultados `skipped` do estágio | Solicita uma nova avaliação explícita, por exemplo após atualizar ferramentas e reiniciar o registro de formatos. Não inclui comparações. |

Retries em `retry_wait` conservam seu backoff. Assets removidos, na lixeira, offline, com outra origem ou com reserva válida não são alterados. Cores exigem uma thumbnail cadastrada, conforme a dependência existente. A solicitação não remove resultados válidos: paletas, thumbnails, previews, fingerprints e grupos permanecem até uma publicação normal do processamento.

O retry não altera pausa ou preferência automática. Se o worker estiver pausado, o trabalho aguarda retomada. Com detecção automática desligada, o usuário deve habilitá-la ou usar `Scan Now` para consumir o trabalho de duplicados. Nenhum scan completo ou rebuild é disparado implicitamente.

O reset de fingerprints e das comparações que falharam usa **a mesma transação** SQLite. A geração dos assets avança e reservas antigas não podem concluir sobre a solicitação nova. Uma falha na mutação de comparações desfaz também o reset dos assets. Solicitar novamente enquanto os registros já estão pendentes retorna zero alterações.

## Diagnóstico seguro

A migration `20261104020000_processing_diagnostics.sql` acrescenta `processing_tasks.issue_code` sem reiniciar tarefas antigas. Os workers registram causas tipadas quando conhecidas:

- Provider ausente ou capacidade de extração não disponível.
- Ferramenta obrigatória indisponível, incluindo resolução de FFmpeg, FFprobe e Assimp.
- Entrada indisponível, output inválido, timeout ou erro de armazenamento.
- Falha de extração sem uma causa tipada mais específica.

Mensagens antigas não são analisadas por heurísticas. Elas aparecem como falha ou ausência de suporte anterior, com orientação para obter um diagnóstico atual por retry/reavaliação. Mesmo um código desconhecido armazenado no banco é convertido em uma categoria segura.

O read model agrupa somente estados `failed`, `retry_wait` e `skipped` da origem atual de assets ativos. Uma whitelist finita de 13 códigos limita cada estágio a no máximo 39 grupos. A resposta contém apenas `issue`, `state` e `count`; `last_error` continua privado ao backend. Os diagnósticos compartilham o cache curto do snapshot e não criam polling por asset.

O diagnóstico descreve **resultados observados**, não um teste contínuo de instalação ou saúde de todas as ferramentas. Camadas legadas que encapsulam um erro em uma mensagem genérica continuam como falha de extração; não se presume uma ferramenta ausente a partir do texto. Registrar novos providers pode exigir reinício. Um fingerprint válido apenas para comparação exata continua sendo sucesso e não é repetido por este comando.

## Contratos e organização

| Módulo | Responsabilidade |
| --- | --- |
| [`core/processing/diagnostics.rs`](../../../src-tauri/src/core/processing/diagnostics.rs) | Códigos, escopos e contagens de retry |
| [`infra/database/query_handlers/processing/diagnostics.rs`](../../../src-tauri/src/infra/database/query_handlers/processing/diagnostics.rs) | Agrupamento seguro e reset transacional de filas |
| [`ProcessingControlService`](../../../src-tauri/src/feature/processing/control.rs) | Mapeamento de worker para estágio, invalidação do cache e snapshot de confirmação |
| [`delivery/tauri/processing.rs`](../../../src-tauri/src/delivery/tauri/processing.rs) | Adaptador IPC, com permissão própria para janelas da biblioteca |
| [`ProcessingClient`](../../../src/core/store/processing/client.ts) | Serialização de controles por worker, timeout e proteção contra respostas de instâncias anteriores |
| [`ProcessingRetryActions`](../../../src/components/features/statusbar/processing/ProcessingRetryActions.tsx) | Ações distintas de retry e reavaliação |
| [`ProcessingDiagnostics`](../../../src/components/features/statusbar/processing/ProcessingDiagnostics.tsx) | Descrições locais seguras em detalhes expansíveis |

Exemplo de solicitação:

```ts
await invoke('retry_processing_worker', { worker: 'thumbnails', scope: 'failed' });
```

A resposta contém `snapshot`, `assets` e `comparisons`. A interface mantém as falhas até a confirmação e mostra uma mensagem agregada de enfileiramento. Não anuncia que a extração terminou. Falhas ou timeout de confirmação preservam o último snapshot e orientam a reconciliação; não provam que a mutação deixou de ocorrer.

Enquanto uma ação está pendente, os controles daquele worker ficam desabilitados. A splashscreen não tem acesso ao comando. A ação não publica eventos de alteração da biblioteca por asset nem toasts individuais. Os resultados posteriores continuam pelo pipeline existente.

## Limites e próxima entrega

Não há instalação de extratores pela interface, lista de caminhos dos arquivos afetados ou inferência retroativa de erros antigos. Indexação e limpeza automática da lixeira conservam seus controles próprios. Esta etapa não muda concorrência ou interrompe tarefas nativas em andamento.

A entrega posterior [Orçamento compartilhado dos workers](2026-11-04-22-16-worker-resource-budget.md) aplica limites de operações em tempo de execução. Otimização orientada por medição e testes de carga permanecem como próxima etapa. Probes de capacidades e suspensão temporária de uma ferramenta indisponível exigem uma política adicional, além do diagnóstico de resultados entregue aqui.

## Validação

No Windows, **62 testes de processamento** e **45 testes frontend** passaram. Cobrem seleção por origem/estado, reserva ativa, preservação de outputs, idempotência, rollback entre assets e comparações, códigos tipados, privacidade, ACL real, pausa preservada, confirmação de quantidades e respostas antigas após reinício. Testes existentes de thumbnails também foram executados.

Typecheck, lint sem warnings, Prettier, build de produção com carregador nativo e checks de whitespace complementam a entrega. Os testes frontend usam jsdom e IPC simulado. Não houve validação visual ou execução desta etapa no macOS. O documento histórico `05-job-scheduler-workers.md` permanece inalterado.
