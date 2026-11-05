# Estado global e pausa dos workers

> **Estado consolidado:** ciclo encerrado e validado pelo usuário em Windows/macOS. Este documento preserva o relato de sua etapa; referências a próximas entregas e validações pendentes devem ser lidas nesse contexto histórico. Consulte o [encerramento](2026-11-04-22-16-workers-closure.md) e as [evoluções futuras separadas](2026-11-04-22-16-workers-future-evolutions.md).


**Data:** 3 de novembro de 2026.  
**Escopo:** atividade de thumbnails/metadados, cores e duplicados na statusbar, com pausa e retomada por executor na instância atual da aplicação.

## Comportamento entregue

O painel **System Activity** usa um snapshot global para apresentar operações ativas, pendências elegíveis, retries adiados, falhas definitivas e resultados sem suporte. A mensagem central da statusbar também usa esse estado. `Ready` depende de uma consulta bem-sucedida e de workers online sem trabalho elegível, retries ou falhas; resultados sem suporte não são tratados como falhas.

O progresso existente do indexer continua visível. Esta etapa não adiciona pausa ao indexer: seu pipeline ainda não expõe admissão por execução para esse controle.

| Controle | Efeito |
| --- | --- |
| Pausar thumbnails/metadados | Bloqueia novas extrações, incluindo prioridades do viewport e regenerações solicitadas |
| Pausar cores | Bloqueia novas extrações e reextrações de paleta |
| Pausar duplicados | Bloqueia novos fingerprints, comparações automáticas, scans manuais e reconstruções do índice |
| Retomar | Reabre admissão; os loops existentes recuperam as pendências no banco |

A pausa é **da sessão** e reinicia desativada ao reabrir a aplicação, conforme informado no painel. Ela não apaga jobs ou resultados e não interrompe arbitrariamente conversões nativas. Enquanto uma operação admitida termina, a linha mostra **Pausing — finishing current operation**; depois passa a **Paused**.

Uma operação de duplicados pode ser um fingerprint, uma comparação ou um scan manual completo. Pausar durante um scan manual permite que ele conclua; o cancelamento já disponível na interface de duplicados continua sendo uma ação separada. Uma reconstrução só apaga o índice depois de obter a coordenação comum e a admissão, impedindo que um pedido bloqueado pela pausa apague resultados sem iniciar o scan.

`auto_detect_duplicates` permanece uma preferência independente: retomar não habilita uma detecção automática desativada nas configurações. O painel informa essa condição. Pedidos manuais continuam possíveis quando a automação está desativada, desde que o executor não esteja pausado.

## Contratos e integração

- [`core/processing/control.rs`](../../../src-tauri/src/core/processing/control.rs): pausa e admissão compartilham um único lock. Permits RAII mantêm operações e fases em memória, liberando atividade inclusive em erro ou unwind. Um guard de lifecycle distingue um worker ocioso de um orquestrador que saiu.
- [`feature/processing/control.rs`](../../../src-tauri/src/feature/processing/control.rs): reúne atividade, contagens persistidas e progresso do scan manual em respostas completas, com `instance_id` e `sequence` monotônica.
- [`infra/database/query_handlers/processing/counts.rs`](../../../src-tauri/src/infra/database/query_handlers/processing/counts.rs): reutiliza as condições de origem, dependência e resultado ausente da seleção de jobs. Exclui lixeira, offline e identidades de assets admitidas pelos workers automáticos. Cores só entra na fila após haver thumbnail utilizável.
- [`delivery/tauri/processing.rs`](../../../src-tauri/src/delivery/tauri/processing.rs): encaminha os comandos para o serviço. Ambos estão no invoke handler e têm permissões específicas concedidas às janelas `main` e `design-system`, sem acesso pela splashscreen.

O agregador depende das portas `ProcessingRepository`, `SettingsRepository` e `ManualScanActivity` do core. O composition root compartilha o repository de configurações com o serviço existente, e o serviço de duplicados fornece um adaptador de leitura do progresso. O snapshot não aciona handlers de comandos nem inicia trabalho para obter telemetria.

Chamadas do frontend:

```typescript
await invoke('get_processing_snapshot');
await invoke('set_processing_worker_paused', {
    worker: 'thumbnails', // Also 'colors' and 'duplicates'.
    paused: true
});
```

Os dois comandos retornam o snapshot completo. Exemplo de linha de um executor:

```json
{
  "worker": "thumbnails",
  "online": true,
  "paused": true,
  "active": 2,
  "phase": "thumbnail",
  "scheduled": false,
  "pending": 120,
  "retry_wait": 3,
  "failed": 1,
  "skipped": 4,
  "automatic_enabled": true
}
```

`active` conta operações, não uma fração do total da biblioteca. As contagens da fila são amostras de telemetria: o cache dura até um segundo e é renovado quando as identidades ativas mudam. Os três estágios são consultados separadamente, enquanto as admissões podem avançar. Não há garantia de uma transação única entre atividade em memória e contagens SQL. No scan manual legado, o progresso de arquivos aparece separadamente; sua operação ativa não possui uma identidade de asset por arquivo na nova fila.

`scheduled` conserva a intenção de comparação após os fingerprints, inclusive durante a pausa. Assim, o intervalo entre um lote e a comparação seguinte não é apresentado como `Ready`. A intenção continua em memória nesta etapa; sua persistência após crash depende dos jobs de matching planejados.

## Transporte e apresentação

[`ProcessingClient`](../../../src/core/store/processing/client.ts) é compartilhado pela mensagem central e pelo indicador. Há uma consulta global a cada dois segundos por webview com consumidores, sem uma chamada por linha ou por asset. O polling para quando o último consumidor desmonta e recupera o estado sem depender da entrega de eventos.

Respostas atrasadas são verificadas pela identidade da instância, sequência e época das ações. Um controle fica desabilitado enquanto aguarda confirmação. A interface só altera a pausa após a resposta do backend; falha ou timeout mantém o último snapshot conhecido, informa incerteza no painel e permite reconciliar o estado na consulta seguinte. Uma resposta que chega depois do timeout não muda o estado visual.

Telemetria não publica toasts, não recarrega listas, árvores, badges ou grupos e não reutiliza eventos de alteração de assets. O antigo sinal `thumbnail:queue-status`, sem emissor, deixou de orientar a statusbar. As propriedades legadas de `useSystem` foram mantidas para compatibilidade.

O painel usa o Popover existente, com foco, fechamento externo e teclado, preservando os botões de configurações e Design System. Os controles têm nomes acessíveis, estado expandido e indicação de confirmação pendente.

Atualizações periódicas preservam os elementos das linhas e o foco dos controles, mesmo quando o snapshot traz novos objetos para os mesmos workers.

## Limites desta etapa

O retry de falhas e a reavaliação explícita de resultados sem suporte foram acrescentados em [Retry por worker e diagnóstico](2026-11-04-22-16-worker-retries-diagnostics.md), com causas agrupadas e feedback de enfileiramento. A etapa de [Orçamento dos workers](2026-11-04-22-16-worker-resource-budget.md) acrescenta limites dinâmicos de operações e capacidade efetiva na statusbar. Os limites descritos abaixo registram o alcance original dos controles da sessão.

A revisão posterior das configurações e do finder está em [Integração dos workers com a interface](2026-11-04-22-16-workers-frontend-integration.md). Ela compartilha admissão e progresso manual com a statusbar, corrige as fases exibidas e relaciona as pendências atuais do plano.

O controle da pausa e a atividade exibida são locais à instância. Não implementam persistência da pausa, interrupção imediata de operações nativas, alteração dinâmica dos limites de concorrência ou repetição global de falhas. Essas partes continuam no plano de [padronização dos workers](2026-11-04-22-16-workers-processing-control.md). O cálculo manual e automático foi unificado em [Processamento compartilhado de fingerprints](2026-11-04-22-16-fingerprint-processing.md); a execução por asset passou a usar [reservas persistentes e publicação transacional](2026-11-04-22-16-processing-leases.md). Reservas válidas ficam fora da contagem de pendências, sem transformar o painel em um monitor de atividade de outros processos.

## Validação executada

- `./scripts/test-backend-windows.ps1 processing_`: **28 testes aprovados**. Inclui os dois novos comandos com ACL real, pausa/admissão/drenagem, cleanup por RAII, retenção de filas, contagens por resultado, reconstrução sem apagar dados antes da admissão e comparação agendada que sobrevive à pausa e conclui após a retomada.
- Vitest nos arquivos `processingClient.spec.ts`, `ProcessingActivityPanel.spec.tsx`, `thumbnailProcessing.spec.ts` e `Thumbnail.spec.tsx`: **25 testes aprovados**. Inclui confirmação dos controles, respostas fora de ordem, reinício do serviço, falha/timeout de consulta, limpeza de inscrições, foco preservado durante polling e regressões de thumbnails.
- `npm run typecheck`, ESLint dos arquivos frontend alterados e `git diff --check`: sem erros.

Não houve validação visual na aplicação, execução no macOS ou benchmark de catálogos grandes nesta entrega. A aplicação Tauri deve ser recompilada e reiniciada para carregar os novos serviços e permissões.
